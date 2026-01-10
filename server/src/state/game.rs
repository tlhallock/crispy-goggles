use crate::event;
use crate::state::perspective::PerspectiveUpdates;
use crate::state::perspective::PlayersGamePerspective;
use crate::state::tasks::CompletedTask;
use crate::state::tasks::SimulatedTask;
use crate::state::tasks::SimulationEvent;
use crate::state::tasks::TaskCompletion;
use crate::state::tasks::TaskManager;
use crate::state::tasks::TaskTransition;
use crate::state::templates::UnitTemplate;
use crate::state::types::SimulatedId;
use common::model::OrientedPoint;
use common::model::{Health, PlayerId, Speed, TaskId, TimeStamp, UnitId};
use std::collections::BinaryHeap;
use std::collections::HashMap;
use tokio::sync::broadcast;

use common::model;
use std::collections::HashSet;

use crate::engine::EngineError;
use crate::engine::EngineErrorKind;
use crate::engine_error;

// put this in a state mod and limit scope?

#[derive(Clone, Debug)]
pub enum UnitLocation {
	ByMoveTask(TaskId),
	Fixed(model::OrientedPoint),
}

#[derive(Debug, Clone)]
pub struct TaskProgress {
	pub finish_time: TimeStamp,
	pub completion: SimulationEvent,
}

pub struct UnitCache {
	pub unit_id: UnitId,
	// pub position: model::Point,
	// shape, speed, location...
}

#[derive(Default)]
pub struct GameState {
	next_id: u64,
	begin_time: TimeStamp,

	// pub current_time: TimeStamp,
	last_time: TimeStamp,
	last_wall_ms: TimeStamp,

	health: HashMap<UnitId, Health>,
	speeds: HashMap<UnitId, Speed>,
	owners: HashMap<UnitId, PlayerId>,
	locations: HashMap<UnitId, UnitLocation>,
	unit_display_types: HashMap<UnitId, model::UnitDisplayType>,

	tasks: TaskManager,

	pub in_progress: BinaryHeap<TaskProgress>,

	// inventory change listeners
	// area change listeners
	perspectives: HashMap<PlayerId, PlayersGamePerspective>,

	// put sequence numbers here?
	units: HashSet<UnitId>,
}

impl GameState {
	pub fn get_next_completion(&self) -> Option<TimeStamp> {
		self.in_progress.peek().map(|tp| tp.finish_time)
	}

	pub fn advance_to_time(&mut self, game_time: TimeStamp) {
		self.last_time = game_time;
	}
	pub fn get_next_id(&mut self) -> u64 {
		let id = self.next_id;
		self.next_id += 1;
		id
	}

	// TODO: rename to remove next task, and return Option if none is past the time
	pub fn remove_completed_task(
		&mut self,
	) -> Result<TaskProgress, EngineError> {
		if let Some(tp) = self.in_progress.pop() {
			Ok(tp)
		} else {
			Err(engine_error!(EngineErrorKind::InternalError))
		}
	}

	pub fn set_task_queue_requested(
		&mut self,
		unit_id: UnitId,
		time: TimeStamp,
		tasks: Vec<SimulatedTask>,
	) -> Result<(), EngineError> {
		// Todo: this should be updated while looping through the simulated tasks, or remove the locations....

		let current_location = Some(self.get_unit_location(unit_id, time)?);

		let transition = self.tasks.set_task_queue_requested(
			unit_id,
			tasks,
			self.last_time,
		)?;

		self.handle_task_transition(unit_id, &transition, current_location)?;

		Ok(())
	}

	// pub fn clear_tasks_requested(
	// 	&mut self,
	// 	unit_id: UnitId,
	// ) -> Result<(), EngineError> {
	// 	// let simulation_id = self.in_progress.
	// 	// TODO:
	// 	// we need to finish the current task
	// 	// use the helpers...
	// 	// update the sequence number

	// 	// TODO: split these into managers
	// 	// just call all the managers with a similar function:
	// 	// manager.clear_tasks(unit_id);
	// 	// manager.transition_simulation(unit_id, simulation_id, next_simulation_id);
	// 	// etc...

	// 	let transition =
	// 		self.tasks.clear_tasks_requested(unit_id, self.last_time)?;
	// 	self.handle_task_transition(unit_id, &transition)?;

	// 	Ok(())
	// }

	fn clear_upcoming_by_unit(&mut self, unit_id: UnitId) {
		self.in_progress.retain(|tp| match tp.completion {
			SimulationEvent::TaskCompleted(ref completed_task) => {
				completed_task.unit_id != unit_id
			}
		});
	}
	fn clear_upcoming_by_simulation_id(&mut self, simulation_id: SimulatedId) {
		self.in_progress.retain(|tp| match tp.completion {
			SimulationEvent::TaskCompleted(ref completed_task) => {
				completed_task.simulation_id != simulation_id
			}
		});
	}

	pub fn add_unit(
		&mut self,
		player_id: PlayerId,
		unit_id: UnitId,
		template: UnitTemplate,
		location: OrientedPoint,
	) {
		self.owners.insert(unit_id, player_id);
		self.tasks.unit_created(unit_id);

		if let Some(health) = template.health {
			self.health.insert(unit_id, health);
		}
		if let Some(speed) = template.speed {
			self.speeds.insert(unit_id, speed);
		}
		// if let Some(shape) = template.shape {
		// 	self.locations
		// 		.insert(unit_id, UnitLocation::Fixed(location));
		// }
		if let Some(display_type) = template.display_type {
			self.unit_display_types.insert(unit_id, display_type);
		}
		self.units.insert(unit_id);
		self.locations
			.insert(unit_id, UnitLocation::Fixed(location));
	}

	pub fn add_player(&mut self, player_id: PlayerId) {
		self.perspectives
			.insert(player_id, PlayersGamePerspective::default());
	}

	pub fn send_incremental_updates(
		&mut self,
		tick_completion_sender: &mut broadcast::Sender<event::PublishEvent>,
	) -> Result<(), EngineError> {
		// For each player perspective, send updates about new units
		let player_ids = self.perspectives.keys().cloned().collect::<Vec<_>>();
		for player_id in player_ids {
			self.send_perspective_updates(player_id, tick_completion_sender)?;
		}
		Ok(())
	}

	pub fn send_perspective_updates(
		&mut self,
		player_id: PlayerId,
		tick_completion_sender: &mut broadcast::Sender<
			crate::event::PublishEvent,
		>,
	) -> Result<(), EngineError> {
		let mut updates = PerspectiveUpdates::new(player_id);

		{
			// For each player perspective, send updates about new units
			let perspective = self
				.perspectives
				.get(&player_id)
				.ok_or(engine_error!(EngineErrorKind::InternalError))?;

			// The tasks shouldn't be in charge of this
			self.tasks.show_perspective(perspective, &mut updates);
		}

		{
			let perspective = self
				.perspectives
				.get_mut(&player_id)
				.ok_or(engine_error!(EngineErrorKind::InternalError))?;
			perspective.apply_changes(&updates);
		}

		updates.send_changes(self, tick_completion_sender)?;

		Ok(())
	}

	pub fn animate(
		&self,
		_player_perspective: PlayerId,
		unit_id: UnitId,
	) -> Result<Option<model::Animatable>, EngineError> {
		// Create an Animatable for the unit

		let display_type = match self.unit_display_types.get(&unit_id) {
			Some(dt) => Ok(dt),
			None => Err(engine_error!(EngineErrorKind::InternalError)),
		}?;
		let queue: Option<Vec<model::AnimationSegment>> =
			match self.locations.get(&unit_id) {
				Some(UnitLocation::ByMoveTask(_)) => Some(
					self.tasks
						.unit_tasks
						.get(&unit_id)
						.ok_or(engine_error!(EngineErrorKind::InternalError))?
						.tasks
						.iter()
						.map(|simulation_id| {
							self.tasks
								.simulated_tasks
								.get(simulation_id)
								.map(|sim| sim.animation.clone())
								.ok_or(engine_error!(
									EngineErrorKind::InternalError
								))
						})
						.collect::<Result<Vec<_>, EngineError>>()?,
				),
				Some(UnitLocation::Fixed(position)) => {
					Some(vec![model::AnimationSegment {
						begin_time: self.last_time,
						begin_location: position.point.clone(),
						delta: None,
						begin_orientation: position.orientation,
						d_orientation: None,
					}])
				}
				_ => None,
			};
		Ok(queue.map(|queue| model::Animatable {
			unit_id,
			display_type: *display_type,
			queue,
		}))
	}

	pub fn get_unit_location(
		&self,
		unit_id: UnitId,
		at_time: TimeStamp,
	) -> Result<model::OrientedPoint, EngineError> {
		match self.locations.get(&unit_id) {
			Some(UnitLocation::Fixed(pos)) => {
				println!(
					"Getting fixed location for unit {} at time {}: {:?}",
					unit_id, at_time, pos
				);
				Ok(pos.clone())
			}
			Some(UnitLocation::ByMoveTask(task_id)) => {
				let simulated_task = self
					.tasks
					.simulated_tasks
					.get(task_id)
					.ok_or(engine_error!(EngineErrorKind::InternalError))?;
				assert!(at_time >= simulated_task.animation.begin_time);
				assert!(at_time <= simulated_task.progress.finish_time);
				Ok(simulated_task.animation.place_at(at_time))
			}
			None => Err(engine_error!(EngineErrorKind::InternalError)),
		}
	}

	/// Evaluates an animation segment at a specific time to get the position
	fn eval_animation_segment(
		&self,
		seg: &model::AnimationSegment,
		at_time: TimeStamp,
	) -> Result<model::OrientedPoint, EngineError> {
		let d_t = at_time.saturating_sub(seg.begin_time) as f64;

		Ok(model::OrientedPoint {
			point: model::Point {
				x: seg.begin_location.x
					+ (seg
						.delta
						.as_ref()
						.map_or(0.0, |d| d.dx as f64 * (d_t as f64)))
						as model::Coord,
				y: seg.begin_location.y
					+ (seg
						.delta
						.as_ref()
						.map_or(0.0, |d| d.dy as f64 * (d_t as f64)))
						as model::Coord,
			},
			orientation: seg.begin_orientation
				+ seg.d_orientation.unwrap_or(0.0) * (d_t as f64) as f32,
		})
	}

	// TDO: should accept a wall time..
	pub fn get_current_time(&self) -> TimeStamp {
		self.last_time
	}

	pub fn get_unit_speed(
		&self,
		unit_id: UnitId,
	) -> Result<Speed, EngineError> {
		match self.speeds.get(&unit_id) {
			Some(speed) => Ok(*speed),
			_ => Err(engine_error!(EngineErrorKind::InternalError)),
		}
	}

	fn get_next_task(
		&self,
		unit_id: UnitId,
		simulation_id: SimulatedId,
	) -> Option<SimulatedId> {
		self.tasks
			.unit_tasks
			.get(&unit_id)
			.map(|ut| {
				if let Some(index) =
					ut.tasks.iter().position(|&sid| sid == simulation_id)
				{
					if index + 1 < ut.tasks.len() {
						Some(ut.tasks[index + 1])
					} else {
						None
					}
				} else {
					None
				}
			})
			.flatten()
	}

	fn clear_tasks(
		&mut self,
		unit_id: UnitId,
		// tick_completion_sender: &mut broadcast::Sender<event::PublishEvent>,
	) -> Result<(), EngineError> {
		if let Some(unit_tasks) = self.tasks.unit_tasks.get_mut(&unit_id) {
			unit_tasks.tasks.clear();
			unit_tasks.current_simulation_id = None;
			unit_tasks.sequence_number += 1;
		}
		// is this needed?
		// tick_completion_sender
		// 	.send(crate::event::PublishEvent::TasksCompleted(unit_id))
		// 	.map_err(|_e| EngineError::UnableToSend)?;
		Ok(())
	}

	fn finished_moving(&mut self, unit_id: UnitId, destination: model::Point) {
		println!(
			"Unit {} finished moving to destination {:?}",
			unit_id, destination
		);
		self.locations.insert(
			unit_id,
			UnitLocation::Fixed(OrientedPoint {
				point: destination,
				orientation: 0.0, // TODO: set proper orientation
			}),
		);
	}

	fn last_simulation_completed(
		&mut self,
		unit_id: UnitId,
		simulation_id: SimulatedId,
		// tick_completion_sender: &mut broadcast::Sender<
		// 	crate::event::PublishEvent,
		// >,
	) -> Result<(), EngineError> {
		let current_simulation = self
			.tasks
			.simulated_tasks
			.get(&simulation_id)
			.ok_or(engine_error!(EngineErrorKind::InternalError))?;

		match &current_simulation.task {
			model::Task::MoveTo(location) => {
				self.finished_moving(unit_id, location.clone());
			}
			_ => {
				// For other task types, we might not need to update the location
			}
		}
		self.clear_tasks(unit_id)?;
		Ok(())
	}

	// fn transition_simulation(
	// 	&mut self,
	// 	unit_id: UnitId,
	// 	simulation_id: SimulatedId,
	// 	next_simulation_id: SimulatedId,
	// ) -> Result<(), EngineError> {
	// 	let current_simulation = self
	// 		.tasks
	// 		.simulated_tasks
	// 		.get(&simulation_id)
	// 		.ok_or(engine_error!(EngineErrorKind::InternalError))?;

	// 	let next_simulation = self
	// 		.tasks
	// 		.simulated_tasks
	// 		.get(&next_simulation_id)
	// 		.ok_or(engine_error!(EngineErrorKind::InternalError))?;

	// 	let next = next_simulation.into();

	// 	{
	// 		match &current_simulation.task {
	// 			model::Task::MoveTo(destination) => {
	// 				match &next_simulation.task {
	// 			}
	// 			_ => {
	// 				// For other task types, we might not need to update the location
	// 			}
	// 		}
	// 	}
	// 	if let Some(unit_tasks) = self.tasks.unit_tasks.get_mut(&unit_id) {
	// 		unit_tasks.current_simulation_id = Some(next_simulation_id);
	// 	}
	// 	self.in_progress.push(next);
	// 	Ok(())
	// }

	pub fn task_completed(
		&mut self,
		game_time: TimeStamp,
		completion: CompletedTask,
		// tick_completion_sender: &mut broadcast::Sender<event::PublishEvent>,
	) -> Result<(), EngineError> {
		let transition = self.tasks.task_completed(game_time, completion)?;
		self.handle_task_transition(transition.unit_id, &transition, None)?;

		Ok(())
	}

	// pub fn simulation_completed(
	// 	&mut self,
	// 	completion: TaskProgress,
	// 	// tick_completion_sender: &mut broadcast::Sender<event::PublishEvent>,
	// ) -> Result<(), EngineError> {
	// 	if let Some(next_simulation_id) =
	// 		self.get_next_task(unit_id, simulation_id)
	// 	{
	// 		self.transition_simulation(
	// 			unit_id,
	// 			simulation_id,
	// 			next_simulation_id,
	// 		)?;
	// 	} else {
	// 		self.last_simulation_completed(unit_id, simulation_id)?;
	// 	}
	// 	Ok(())
	// }

	// pub async fn task_completed(
	// 	&mut self,
	// 	completion: TaskCompletion,
	// 	_tick_completion_sender: &mut broadcast::Sender<
	// 		crate::event::PublishEvent,
	// 	>,
	// ) -> Result<(), EngineError> {
	// 	if let Some(_simulation_id) = completion.get_simulation_id() {}
	// 	Ok(())
	// }

	fn handle_task_transition(
		&mut self,
		unit_id: UnitId,
		transition: &TaskTransition,

		// todo...
		location: Option<model::OrientedPoint>,
	) -> Result<(), EngineError> {
		// update the locations
		self.locations_transition(unit_id, transition, location)?;

		// update the in-progress tasks
		if let Some((_, simulation_id, false)) = &transition.from {
			self.clear_upcoming_by_simulation_id(*simulation_id);
		}
		if let Some((_, progress)) = &transition.to {
			self.in_progress.push(progress.clone());
		} else {
			self.clear_upcoming_by_unit(unit_id);
		}
		Ok(())
	}

	fn locations_transition(
		&mut self,
		unit_id: UnitId,
		transition: &TaskTransition,
		location: Option<model::OrientedPoint>,
	) -> Result<(), EngineError> {
		// match multiple levels?
		let next_moving_sim_id =
			if let Some((model::Task::MoveTo(_), progress)) = &transition.to {
				match &progress.completion {
					SimulationEvent::TaskCompleted(c) => Some(c.simulation_id),
					_ => None,
				}
			} else {
				None
			};

		// TODO

		if let Some(next_moving_sim_id) = next_moving_sim_id {
			self.locations
				.insert(unit_id, UnitLocation::ByMoveTask(next_moving_sim_id));
		} else {
			// Shouldn't have to recalculate where it is...
			let current_location = match &transition.from {
				Some((model::Task::MoveTo(destination), _, true)) => {
					OrientedPoint {
						point: destination.clone(),
						orientation: 0.0, // TODO: set proper orientation
					}
				}
				_ => location
					.ok_or(engine_error!(EngineErrorKind::InternalError))?,
			};

			// finished moving, now do something else
			self.locations
				.insert(unit_id, UnitLocation::Fixed(current_location));
		}

		Ok(())
	}
}

impl PartialOrd for TaskProgress {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}
impl Ord for TaskProgress {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		other
			.finish_time
			.cmp(&self.finish_time)
			.reverse()
			.then_with(|| self.completion.cmp(&other.completion))
	}
}

impl PartialEq for TaskProgress {
	fn eq(&self, other: &Self) -> bool {
		self.finish_time == other.finish_time
			&& self.completion == other.completion
	}
}
impl Eq for TaskProgress {}
impl PartialEq for TaskCompletion {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(
				TaskCompletion::DestinationReached {
					unit_id: u1,
					simulation_id: s1,
				},
				TaskCompletion::DestinationReached {
					unit_id: u2,
					simulation_id: s2,
				},
			) => u1 == u2 && s1 == s2,
		}
	}
}
impl Eq for TaskCompletion {}

impl Default for UnitTemplate {
	fn default() -> Self {
		Self {
			health: Some(model::Health {
				current: 100,
				max: 100,
			}),
			speed: Some(1.0 * model::METERS / model::SECONDS as model::Speed),
			display_type: Some(model::UnitDisplayType::SimpleUnit),
		}
	}
}

impl From<&SimulatedTask> for TaskProgress {
	fn from(task: &SimulatedTask) -> Self {
		Self {
			finish_time: task.progress.finish_time,
			completion: SimulationEvent::TaskCompleted(CompletedTask {
				unit_id: match &task.progress.completion {
					SimulationEvent::TaskCompleted(ct) => ct.unit_id,
				},
				simulation_id: task.id,
				task: task.task.clone(),
			}),
		}
	}
}

impl TaskCompletion {
	fn get_simulation_id(&self) -> Option<SimulatedId> {
		match &self {
			TaskCompletion::DestinationReached {
				// unit_id, _, // _unit_id,
				simulation_id,
				..
			} => *simulation_id,
		}
	}
	fn get_unit_id(&self) -> Option<UnitId> {
		match &self {
			TaskCompletion::DestinationReached { unit_id, .. } => {
				Some(*unit_id)
			}
		}
	}
}
