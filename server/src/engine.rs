use crate::state::game::GameState;
use crate::state::game::TaskProgress;
use crate::state::tasks;
use crate::state::tasks::CompletedTask;
use crate::state::tasks::SimulatedTask;
use crate::state::templates::UnitTemplate;

// use single_value_channel::channel_starting_with;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};

use crate::event;
use crate::state;
use common::model::{self};
use common::model::{Coord, TimeStamp};

use common::grpc;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum EngineError {
	MalformedRequest,
	UnableToSend,
	InternalError,
	InvalidUnitId,
}

impl fmt::Display for EngineError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			EngineError::MalformedRequest => write!(f, "Malformed request"),
			EngineError::UnableToSend => write!(f, "Unable to send message"),
			EngineError::InternalError => write!(f, "Internal error"),
			EngineError::InvalidUnitId => write!(f, "Invalid unit ID"),
		}
	}
}

impl Error for EngineError {}

fn wall_time() -> u64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or(Duration::from_secs(0))
		.as_millis() as u64
}

async fn tick(
	tick_completion_sender: &mut broadcast::Sender<event::PublishEvent>,
	game_state: &mut GameState,
) -> Result<(), EngineError> {
	// For now: game_time == wall_time (you can change this later)

	let wall_ms = wall_time();
	let game_time = wall_ms;

	while let Some(finish_time) = game_state.get_next_completion() {
		if finish_time > game_time {
			break;
		}
		println!(
			"Next task completion at {}, current game time {}",
			finish_time, game_time
		);
		let progress = game_state.remove_completed_task()?;
		match progress.completion {
			tasks::SimulationEvent::TaskCompleted(ct) => {
				game_state.task_completed(progress.finish_time, ct)?;
			}
		}
	}

	game_state.advance_to_time(game_time);
	game_state.send_incremental_updates(tick_completion_sender)?;

	tick_completion_sender
		.send(event::PublishEvent::TickCompleted(
			event::TickCompletedEvent {
				wall_ms: wall_ms,
				game_time: game_time,
			},
		))
		.map_err(|_e| EngineError::UnableToSend)?;

	Ok(())
}

pub async fn run_engine(
	mut user_requests_receiver: mpsc::Receiver<event::PlayerRequest>,
	mut tick_completion_sender: broadcast::Sender<event::PublishEvent>,
) {
	let mut game_state = GameState::default();
	let (tick_sender, mut tick_receiver) =
		tokio::sync::watch::channel::<event::EngineEvent>(
			event::EngineEvent::Tick(wall_time()),
		);

	spawn_ticker(tick_sender);

	loop {
		tokio::select! {
			Ok(_) = tick_receiver.changed() => {
				match tick(
					&mut tick_completion_sender,
					&mut game_state,
				).await {
					Ok(_) => {},
					Err(e) => {
						eprintln!("Error during tick: {:?}", e);
					}
				}
			},
			Some(request) = user_requests_receiver.recv() => {
				match handle_user_request(request, &mut game_state,
					 &mut tick_completion_sender).await {
					Ok(_) => {},
					Err(e) => {
						eprintln!("Error handling user request: {:?}", e);
					}
				}
			}
		}
	}
}

async fn handle_player_joined(
	player_id: u64,
	game_state: &mut GameState,
	_tick_completion_sender: &mut broadcast::Sender<event::PublishEvent>,
) -> Result<(), EngineError> {
	println!("Player joined: {}", player_id);

	// todo: send existing units to player...
	game_state.add_player(player_id);

	Ok(())
}

async fn handle_player_left(
	player_id: u64,
	_game_state: &mut GameState,
	_tick_completion_sender: &mut broadcast::Sender<event::PublishEvent>,
) -> Result<(), EngineError> {
	println!("Player left: {}", player_id);
	Ok(())
}

async fn handle_create_unit(
	player_id: model::PlayerId,
	unit_id: model::UnitId,
	game_state: &mut GameState,
	_tick_completion_sender: &mut broadcast::Sender<event::PublishEvent>,
) -> Result<(), EngineError> {
	println!("Create unit: {}", unit_id);
	game_state.add_unit(
		player_id,
		unit_id,
		UnitTemplate::default(),
		model::OrientedPoint {
			point: model::Point { x: 0.0, y: 0.0 },
			orientation: 0.0,
		},
	);
	Ok(())
}

async fn handle_update_intentions(
	request: grpc::SetQueueRequest,
	game_state: &mut GameState,
	_tick_completion_sender: &mut broadcast::Sender<event::PublishEvent>,
) -> Result<(), EngineError> {
	// get current position...

	let tasks = request
		.tasks
		.iter()
		.map(|x| {
			return <Result<
                common::model::Task,
                common::convert::ParseError,
            >>::from(x);
		})
		.collect::<Result<Vec<common::model::Task>, common::convert::ParseError>>(
		)
		.map_err(|_| EngineError::MalformedRequest)?;

	let simulated = simulate_tasks(game_state, request.unit_id, tasks)?;
	game_state.set_task_queue_requested(request.unit_id, simulated)?;

	// TODO: move to the tick loop? (sequence number?)
	// tick_completion_sender
	// 	.send(event::PublishEvent::TasksUpdated(
	// 		event::TasksUpdatedEvent {
	// 			unit_id: request.unit_id,
	// 			// I guess this only gets the first one for now...
	// 			tasks: vec![res.animation.into()],
	// 		},
	// 	))
	// 	.map_err(|_| EngineError::UnableToSend)?;

	// // TODO: this should be more simple when it is done inside the tick...
	// let mut evt = None;
	// if let Some(model::Task::MoveTo(t)) = tasks.first() {
	// 	let res = simulate_move(
	// 		request.unit_id,
	// 		// game_state.get_unit_location(request.unit_id)?,
	// 		model::Point { x: 0.0, y: 0.0 },
	// 		t.clone(),
	// 		// todo: which time to use here?
	// 		0,   // game_state.last_time,
	// 		5.0, // speed
	// 		0,   // game_state.next_id, // this should be the next task id
	// 	)?;
	// 	evt = Some(event::PublishEvent::TasksUpdated(
	// 		event::TasksUpdatedEvent {
	// 			unit_id: request.unit_id,
	// 			// I guess this only gets the first one for now...
	// 			tasks: vec![res.animation.into()],
	// 		},
	// 	));
	// }

	// if let Err(e) = game_state.queue_tasks(request.unit_id, tasks) {
	// 	tick_completion_sender
	// 		.send(event::PublishEvent::Warning(event::WarningContent {
	// 			// TODO
	// 			user_id: 0,
	// 			message: format!(
	// 				"No tasks found for unit ID {}",
	// 				request.unit_id
	// 			),
	// 		}))
	// 		.map_err(|_| EngineError::UnableToSend)?;
	// 	return Err(e);
	// }
	// if let Some(evt) = evt {
	// 	tick_completion_sender
	// 		.send(evt)
	// 		.map_err(|_| EngineError::UnableToSend)?;
	// }

	Ok(())
}

async fn handle_user_request(
	request: event::PlayerRequest,
	game_state: &mut GameState,
	tick_completion_sender: &mut broadcast::Sender<event::PublishEvent>,
) -> Result<(), EngineError> {
	match request {
		event::PlayerRequest::PlayerJoined(player_id) => {
			handle_player_joined(player_id, game_state, tick_completion_sender)
				.await?
		}
		event::PlayerRequest::CreateUnit(player_id, unit_id) => {
			handle_create_unit(
				player_id,
				unit_id,
				game_state,
				tick_completion_sender,
			)
			.await?
		}
		event::PlayerRequest::UpdateIntentions(request) => {
			handle_update_intentions(
				request,
				game_state,
				tick_completion_sender,
			)
			.await?
		}
		event::PlayerRequest::PlayerLeft(player_id) => {
			handle_player_left(player_id, game_state, tick_completion_sender)
				.await?
		}
		event::PlayerRequest::ClearQueue(unit_id) => {
			game_state.set_task_queue_requested(unit_id, vec![])?;
			tick_completion_sender
				.send(event::PublishEvent::TasksUpdated(
					event::TasksUpdatedEvent {
						unit_id,
						tasks: vec![],
					},
				))
				.map_err(|_| EngineError::UnableToSend)?;
		}
	}

	Ok(())
}

// fn make_random_anim(id: u64) -> Animatable {
// 	let mut rng = rand::rng();

// 	let shape = if rng.random_bool(0.5) {
// 		Shape::Circle(rng.random_range(10.0..80.0))
// 	} else {
// 		Shape::Rectangle(
// 			rng.random_range(20.0..140.0),
// 			rng.random_range(20.0..140.0),
// 		)
// 	};

// 	let color = (rng.random::<u8>(), rng.random::<u8>(), rng.random::<u8>());

// 	// todo: extract this to common function
// 	let wall_ms = std::time::SystemTime::now()
// 		.duration_since(std::time::UNIX_EPOCH)
// 		.unwrap_or(Duration::from_secs(0))
// 		.as_millis() as u64;

// 	let begin_time =
// 		wall_ms + rng.random_range(0..2 * TIME_PER_SECOND) as TimeStamp;
// 	let delta = model::Delta {
// 		dx: rng.random_range(-1.0..1.0),
// 		dy: rng.random_range(-1.0..1.0),
// 	}
// 	.normalize(5.0 * TIME_PER_SECOND as f64);
// 	let begin_location = model::Point {
// 		x: rng.random_range(100.0 as Coord..400.0 as Coord),
// 		y: rng.random_range(100.0 as Coord..400.0 as Coord),
// 	};
// 	let d_t = rng.random_range(5 * TIME_PER_SECOND..20 * TIME_PER_SECOND)
// 		as TimeStamp;
// 	let end_location = model::Point {
// 		x: begin_location.x + (d_t as f64 * delta.dx) as Coord,
// 		y: begin_location.y + (d_t as f64 * delta.dy) as Coord,
// 	};
// 	let d_orientation = rng.random_range(-180.0..180.0);
// 	let path = vec![
// 		common::model::PathSegment {
// 			begin_time,
// 			begin_location,
// 			delta: Some(delta),
// 			begin_orientation: rng.random_range(0.0..360.0),
// 			d_orientation: Some(d_orientation),
// 		},
// 		common::model::PathSegment {
// 			begin_time: begin_time + d_t,
// 			begin_location: end_location,
// 			delta: None,
// 			begin_orientation: 0.0,
// 			d_orientation: None,
// 		},
// 	];

// 	Animatable {
// 		id,
// 		shape,
// 		fill: rng.random_bool(0.7),
// 		color,
// 		path,
// 	}
// }

// fn create_unit(id: common::model::UnitId) -> Animatable {
// 	Animatable {
// 		id,
// 		shape: Shape::Circle(1.0),
// 		fill: true,
// 		color: (0, 255, 0), // green
// 		path: vec![common::model::PathSegment {
// 			begin_time: 0,
// 			begin_location: common::model::Point { x: 0.0, y: 0.0 },
// 			delta: None,
// 			begin_orientation: 0.0,
// 			d_orientation: None,
// 		}],
// 	}
// }

fn spawn_ticker(
	tick_sender: tokio::sync::watch::Sender<crate::event::EngineEvent>,
) {
	tokio::spawn(async move {
		let mut ticker = interval(Duration::from_millis(30));
		let wall_ms = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap_or(Duration::from_secs(0))
			.as_millis() as u64;
		loop {
			ticker.tick().await;

			if let Err(_) =
				tick_sender.send(crate::event::EngineEvent::Tick(wall_ms))
			{
				println!("Error sending tick, stopping ticker.");
				break;
			}
		}
	});
}

struct SimScratchPad {
	current_time: TimeStamp,
	current_location: model::OrientedPoint,
}

fn simulate_move(
	game_state: &mut GameState,
	unit_id: model::UnitId,
	task: model::Task,
	scratch_pad: &mut SimScratchPad,

	to: model::Point,
) -> Result<SimulatedTask, EngineError> {
	let speed = game_state.get_unit_speed(unit_id)?;
	if speed < 1e-6 as Coord {
		return Err(EngineError::MalformedRequest);
	}
	let dist = scratch_pad.current_location.point.distance_to(&to);
	if dist < 1e-6 {
		return Err(EngineError::MalformedRequest);
	}
	let duration = dist / speed as f64;
	let delta = model::Delta::between(&scratch_pad.current_location.point, &to)
		.normalize(speed);
	let finish_time = scratch_pad.current_time + duration as TimeStamp;
	let simulation_id = game_state.get_next_id();

	let ret = SimulatedTask {
		id: simulation_id,
		task: task,
		animation: model::AnimationSegment {
			begin_time: scratch_pad.current_time,
			begin_location: scratch_pad.current_location.point.clone(),
			delta: Some(delta),
			begin_orientation: 0.0,
			d_orientation: None,
		},
		progress: TaskProgress {
			finish_time,
			completion: tasks::SimulationEvent::TaskCompleted(CompletedTask {
				unit_id,
				simulation_id: simulation_id,
				task: model::Task::MoveTo(to.clone()),
			}),
		},
	};

	scratch_pad.current_time = finish_time;
	scratch_pad.current_location = model::OrientedPoint {
		point: to,
		orientation: 0.0,
	};

	return Ok(ret);
}

fn simulate_task(
	game_state: &mut GameState,
	unit_id: model::UnitId,
	task: model::Task,
	scratch_pad: &mut SimScratchPad,
) -> Result<SimulatedTask, EngineError> {
	// todo: no need to clone
	let t = task.clone();
	match task {
		model::Task::MoveTo(to) => {
			simulate_move(game_state, unit_id, t, scratch_pad, to)
		}
		_ => Err(EngineError::MalformedRequest),
	}
}

fn simulate_tasks(
	game_state: &mut GameState,
	unit_id: model::UnitId,
	tasks: Vec<model::Task>,
) -> Result<Vec<SimulatedTask>, EngineError> {
	let begin_time = game_state.get_current_time();
	let mut scratch_pad = SimScratchPad {
		current_time: begin_time,
		current_location: game_state.get_unit_location(unit_id, begin_time)?,
	};

	let mut simulated_tasks = Vec::new();
	for task in tasks {
		simulated_tasks.push(simulate_task(
			game_state,
			unit_id,
			task,
			&mut scratch_pad,
		)?);
	}
	Ok(simulated_tasks)
}
