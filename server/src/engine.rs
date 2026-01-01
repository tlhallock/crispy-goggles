use crate::event::WarningContent;
use crate::state::GameState;

// use single_value_channel::channel_starting_with;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};

use common::model::{self};
use common::model::{Animatable, Shape};
use common::model::{Coord, TIME_PER_SECOND, TimeStamp};
use rand::Rng;

use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum EngineError {
	MalformedRequest,
	UnableToSend,
}

impl fmt::Display for EngineError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			EngineError::MalformedRequest => write!(f, "Malformed request"),
			EngineError::UnableToSend => write!(f, "Unable to send message"),
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
	tick_completion_sender: &mut broadcast::Sender<crate::event::PublishEvent>,
	_game_state: &mut GameState,
) -> Result<(), broadcast::error::SendError<crate::event::PublishEvent>> {
	// For now: game_time == wall_time (you can change this later)

	let wall_ms = wall_time();
	let game_time = wall_ms;

	tick_completion_sender.send(crate::event::PublishEvent::TickCompleted(
		crate::event::TickCompletedEvent {
			wall_ms: wall_ms,
			game_time: game_time,
		},
	))?;

	Ok(())
}

pub async fn run_engine(
	mut user_requests_receiver: mpsc::Receiver<crate::event::PlayerRequest>,
	mut tick_completion_sender: broadcast::Sender<crate::event::PublishEvent>,
) {
	let mut game_state = GameState::new();
	let (tick_sender, mut tick_receiver) =
		tokio::sync::watch::channel::<crate::event::EngineEvent>(
			crate::event::EngineEvent::Tick(wall_time()),
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
	tick_completion_sender: &mut broadcast::Sender<crate::event::PublishEvent>,
) -> Result<(), EngineError> {
	println!("Player joined: {}", player_id);
	Ok(())
}

async fn hanlde_player_left(
	player_id: u64,
	game_state: &mut GameState,
	tick_completion_sender: &mut broadcast::Sender<crate::event::PublishEvent>,
) -> Result<(), EngineError> {
	println!("Player left: {}", player_id);
	Ok(())
}

async fn handle_create_unit(
	unit_id: common::model::UnitId,
	game_state: &mut GameState,
	tick_completion_sender: &mut broadcast::Sender<crate::event::PublishEvent>,
) -> Result<(), EngineError> {
	println!("Create unit: {}", unit_id);

	let anim = create_unit(unit_id);

	game_state
		.unit_tasks
		.insert(unit_id, common::model::Tasks::default());

	tick_completion_sender
		.send(crate::event::PublishEvent::UnitCreated(anim))
		.map_err(|_| EngineError::UnableToSend)?;

	Ok(())
}

async fn handle_update_intentions(
	request: common::grpc::QueueRequest,
	game_state: &mut GameState,
	tick_completion_sender: &mut broadcast::Sender<crate::event::PublishEvent>,
) -> Result<(), EngineError> {
	let tasks = request
		.task
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

	if let Err(e) = game_state.queue_tasks(request.unit_id, tasks) {
		tick_completion_sender
			.send(crate::event::PublishEvent::Warning(WarningContent {
				// TODO
				user_id: 0,
				message: format!(
					"No tasks found for unit ID {}",
					request.unit_id
				),
			}))
			.map_err(|_| EngineError::UnableToSend)?;
		return Err(e);
	}

	Ok(())
}

async fn handle_user_request(
	request: crate::event::PlayerRequest,
	game_state: &mut GameState,
	tick_completion_sender: &mut broadcast::Sender<crate::event::PublishEvent>,
) -> Result<(), EngineError> {
	match request {
		crate::event::PlayerRequest::PlayerJoined(player_id) => {
			handle_player_joined(player_id, game_state, tick_completion_sender)
				.await?
		}
		crate::event::PlayerRequest::CreateUnit(unit_id) => {
			handle_create_unit(unit_id, game_state, tick_completion_sender)
				.await?
		}
		crate::event::PlayerRequest::UpdateIntentions(request) => {
			handle_update_intentions(
				request,
				game_state,
				tick_completion_sender,
			)
			.await?
		}
		crate::event::PlayerRequest::PlayerLeft(player_id) => {
			hanlde_player_left(player_id, game_state, tick_completion_sender)
				.await?
		}
	}

	Ok(())
}

fn make_random_anim(id: u64) -> Animatable {
	let mut rng = rand::rng();

	let shape = if rng.random_bool(0.5) {
		Shape::Circle(rng.random_range(10.0..80.0))
	} else {
		Shape::Rectangle(
			rng.random_range(20.0..140.0),
			rng.random_range(20.0..140.0),
		)
	};

	let color = (rng.random::<u8>(), rng.random::<u8>(), rng.random::<u8>());

	// todo: extract this to common function
	let wall_ms = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or(Duration::from_secs(0))
		.as_millis() as u64;

	let begin_time =
		wall_ms + rng.random_range(0..2 * TIME_PER_SECOND) as TimeStamp;
	let delta = model::Delta {
		dx: rng.random_range(-1.0..1.0),
		dy: rng.random_range(-1.0..1.0),
	}
	.normalize(5.0 * TIME_PER_SECOND as f64);
	let begin_location = model::Point {
		x: rng.random_range(100.0 as Coord..400.0 as Coord),
		y: rng.random_range(100.0 as Coord..400.0 as Coord),
	};
	let d_t = rng.random_range(5 * TIME_PER_SECOND..20 * TIME_PER_SECOND)
		as TimeStamp;
	let end_location = model::Point {
		x: begin_location.x + (d_t as f64 * delta.dx) as Coord,
		y: begin_location.y + (d_t as f64 * delta.dy) as Coord,
	};
	let d_orientation = rng.random_range(-180.0..180.0);
	let path = vec![
		common::model::PathSegment {
			begin_time,
			begin_location,
			delta: Some(delta),
			begin_orientation: rng.random_range(0.0..360.0),
			d_orientation: Some(d_orientation),
		},
		common::model::PathSegment {
			begin_time: begin_time + d_t,
			begin_location: end_location,
			delta: None,
			begin_orientation: 0.0,
			d_orientation: None,
		},
	];

	Animatable {
		id,
		shape,
		fill: rng.random_bool(0.7),
		color,
		path,
	}
}

fn create_unit(id: common::model::UnitId) -> Animatable {
	Animatable {
		id,
		shape: Shape::Circle(1.0),
		fill: true,
		color: (0, 255, 0), // green
		path: vec![common::model::PathSegment {
			begin_time: 0,
			begin_location: common::model::Point { x: 0.0, y: 0.0 },
			delta: None,
			begin_orientation: 0.0,
			d_orientation: None,
		}],
	}
}

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
