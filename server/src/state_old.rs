use crate::event;
use crate::unit_tasks::UnitTasks;
use common::model::OrientedPoint;
use common::model::{Health, PlayerId, Speed, TaskId, TimeStamp, UnitId};
use std::collections::BinaryHeap;
use std::collections::HashMap;
use tokio::sync::broadcast;

use common::model;
use std::collections::HashSet;

use crate::engine::EngineError;

// put this in a state mod and limit scope?
pub type SimulatedId = u64;
pub type SequenceNumber = u64;

#[derive(Clone, Debug)]
pub enum UnitLocation {
	ByMoveTask(TaskId),
	Fixed(model::OrientedPoint),
}

// #[derive(Default, Debug)]
// pub struct UnitTasks {
// 	current_simulation_id: Option<SimulatedId>,
// 	sequence_number: SequenceNumber,
// 	tasks: Vec<SimulatedId>,
// }

#[derive(Debug)]
pub struct TaskProgress {
	pub finish_time: TimeStamp,
	pub completion: TaskCompletion,
}

#[derive(Debug)]
pub struct SimulatedTask {
	pub id: SimulatedId,
	pub task: common::model::Task,
	// animation class? we need other things like player, health, image...
	// this should be just the info abot tasks: task animation...
	// break them up? no.
	pub animation: common::model::AnimationSegment,
	pub progress: TaskProgress,
	// move the task id here?
	//     yes, and then make this be by_id, and keep the sorted heap separate
	//     also, maintain the list of tasks per unit separately

	// todo: put the shape (volume) for collisions here
}

pub struct UnitTemplate {
	pub health: Option<Health>,
	pub speed: Option<Speed>,
	// keep this separate from the view shape?
	// pub model_shape: Option<model::Shape>,
	pub display_type: Option<model::UnitDisplayType>,
}

pub struct UnitCache {
	pub unit_id: UnitId,
	// pub position: model::Point,
	// shape, speed, location...
}

#[derive(Default, Debug)]
pub struct PlayersGamePerspective {
	last_update: HashMap<UnitId, SequenceNumber>,
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

	unit_tasks: HashMap<UnitId, UnitTasks>,
	simulated_tasks: HashMap<SimulatedId, SimulatedTask>,
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

	pub fn remove_completed_task(
		&mut self,
	) -> Result<TaskCompletion, EngineError> {
		if let Some(tp) = self.in_progress.pop() {
			Ok(tp.completion)
		} else {
			Err(EngineError::InternalError)
		}
	}

	pub fn advance_to_time(&mut self, game_time: TimeStamp) {
		self.last_time = game_time;
	}
	pub fn get_next_id(&mut self) -> u64 {
		let id = self.next_id;
		self.next_id += 1;
		id
	}

	pub fn queue_tasks_requested(
		&mut self,
		unit_id: UnitId,
		tasks: Vec<SimulatedTask>,
	) -> Result<(), EngineError> {
		// Todo: this should be updated while looping through the simulated tasks, or remove the locations....
		if let Some(first_task) = tasks.first() {
			// TODO: began moving?
			self.locations
				.insert(unit_id, UnitLocation::ByMoveTask(first_task.id));
		} else {
			println!("Queued 0 tasks for unit {}", unit_id);
		}
		if let Some(unit_tasks) = self.unit_tasks.get_mut(&unit_id) {
			//  this should be based on time...
			if let Some(task) = tasks.first() {
				unit_tasks.current_simulation_id = Some(task.id);
				self.in_progress.push(task.into());
			}
			for task in &tasks {
				unit_tasks.tasks.push(task.id);
			}
			unit_tasks.sequence_number += 1;
		} else {
			return Err(EngineError::MalformedRequest);
		}
		// self.simulated_tasks.extend(tasks.iter().map(|t| (t.id, t.clone())));
		for task in tasks {
			self.simulated_tasks.insert(task.id, task);
		}

		Ok(())
	}

	pub fn clear_tasks_requested(
		&mut self,
		unit_id: UnitId,
	) -> Result<(), EngineError> {
		// let simulation_id = self.in_progress.
		// TODO:
		// we need to finish the current task
		// use the helpers...
		// update the sequence number

		// TODO: split these into managers
		// just call all the managers with a similar function:
		// manager.clear_tasks(unit_id);
		// manager.transition_simulation(unit_id, simulation_id, next_simulation_id);
		// etc...

		let current_location =
			self.get_unit_location(unit_id, self.last_time)?;
		self.locations
			.insert(unit_id, UnitLocation::Fixed(current_location));

		if let Some(unit_tasks) = self.unit_tasks.get_mut(&unit_id) {
			for simulated_id in unit_tasks.tasks.iter() {
				self.simulated_tasks.remove(simulated_id);
			}
			self.in_progress.retain(|tp| match tp.completion {
				TaskCompletion::DestinationReached {
					unit_id: uid,
					simulation_id: None,
				} => uid != unit_id, // || sid != *simulated_id,
				_ => true,
			});
			// let is_empty = unit_tasks.tasks.is_empty();
			unit_tasks.tasks.clear();
			unit_tasks.current_simulation_id = None;
			unit_tasks.sequence_number += 1;
			Ok(())
		} else {
			Err(EngineError::MalformedRequest)
		}
	}

	pub fn add_unit(
		&mut self,
		player_id: PlayerId,
		unit_id: UnitId,
		template: UnitTemplate,
		location: OrientedPoint,
	) {
		self.owners.insert(unit_id, player_id);
		self.unit_tasks.insert(unit_id, UnitTasks::default());
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
		// TODO: this is not efficient
		let mut units_to_add = vec![];
		let mut units_to_update = vec![];

		{
			// For each player perspective, send updates about new units
			let perspective = self
				.perspectives
				.get_mut(&player_id)
				.ok_or(EngineError::InternalError)?;

			for unit_id in self.units.iter() {
				if !perspective.last_update.contains_key(unit_id) {
					units_to_add.push(*unit_id);
					// todo: using the same sequnce number for two meanings (exists vs tasks)
					perspective.last_update.insert(
						*unit_id,
						self.unit_tasks
							.get(unit_id)
							.map_or(0, |ut| ut.sequence_number),
					);
				}
			}

			for (unit_id, tasks) in self.unit_tasks.iter() {
				if let Some(update_num) = perspective.last_update.get(unit_id) {
					if tasks.sequence_number > *update_num {
						units_to_update.push(*unit_id);
						perspective
							.last_update
							.insert(*unit_id, tasks.sequence_number);
					}
				}
			}
		}

		for unit_id in units_to_add.iter() {
			// TODO: send unit created event
			println!(
				"Player {}: sending create for unit {}",
				player_id, unit_id
			);
			let animatable = self.animate(player_id, *unit_id)?;
			if let Some(animatable) = animatable {
				tick_completion_sender
					.send(crate::event::PublishEvent::UnitCreated(animatable))
					.map_err(|_e| EngineError::UnableToSend)?;
			} else {
				println!(
					"Player {}: no animatable for unit {}",
					player_id, unit_id
				);
			}
		}

		for unit_id in units_to_update.iter() {
			println!(
				"Player {}: sending update for unit {}",
				player_id, unit_id
			);
			let animatable = self.animate(player_id, *unit_id)?;
			if let Some(animatable) = animatable {
				tick_completion_sender
					.send(crate::event::PublishEvent::TasksUpdated(
						event::TasksUpdatedEvent {
							unit_id: *unit_id,
							tasks: animatable
								.queue
								.into_iter()
								.map(Into::into)
								.collect(),
						},
					))
					.map_err(|_e| EngineError::UnableToSend)?;
			} else {
				println!(
					"Player {}: no animatable for unit {}",
					player_id, unit_id
				);
			}
		}
		Ok(())
	}

	fn animate(
		&self,
		_player_perspective: PlayerId,
		unit_id: UnitId,
	) -> Result<Option<model::Animatable>, EngineError> {
		// Create an Animatable for the unit

		let display_type = match self.unit_display_types.get(&unit_id) {
			Some(dt) => Ok(dt),
			None => Err(EngineError::InternalError),
		}?;
		let queue: Option<Vec<model::AnimationSegment>> =
			match self.locations.get(&unit_id) {
				Some(UnitLocation::ByMoveTask(_)) => Some(
					self.unit_tasks
						.get(&unit_id)
						.ok_or(EngineError::InternalError)?
						.tasks
						.iter()
						.map(|simulation_id| {
							self.simulated_tasks
								.get(simulation_id)
								.map(|sim| sim.animation.clone())
								.ok_or(EngineError::InternalError)
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
					.simulated_tasks
					.get(task_id)
					.ok_or(EngineError::InternalError)?;
				assert!(at_time >= simulated_task.animation.begin_time);
				assert!(at_time <= simulated_task.progress.finish_time);
				Ok(simulated_task.animation.place_at(at_time))
			}
			None => Err(EngineError::InternalError),
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

	pub fn get_current_time(&self) -> TimeStamp {
		self.last_time
	}

	pub fn get_unit_speed(
		&self,
		unit_id: UnitId,
	) -> Result<Speed, EngineError> {
		match self.speeds.get(&unit_id) {
			Some(speed) => Ok(*speed),
			_ => Err(EngineError::InternalError),
		}
	}

	fn get_next_task(
		&self,
		unit_id: UnitId,
		simulation_id: SimulatedId,
	) -> Option<SimulatedId> {
		self.unit_tasks
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
		if let Some(unit_tasks) = self.unit_tasks.get_mut(&unit_id) {
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
			.simulated_tasks
			.get(&simulation_id)
			.ok_or(EngineError::InternalError)?;

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

	fn transition_simulation(
		&mut self,
		unit_id: UnitId,
		simulation_id: SimulatedId,
		next_simulation_id: SimulatedId,
	) -> Result<(), EngineError> {
		let current_simulation = self
			.simulated_tasks
			.get(&simulation_id)
			.ok_or(EngineError::InternalError)?;

		let next_simulation = self
			.simulated_tasks
			.get(&next_simulation_id)
			.ok_or(EngineError::InternalError)?;

		let next = next_simulation.into();

		{
			match &current_simulation.task {
				model::Task::MoveTo(destination) => {
					match &next_simulation.task {
						model::Task::MoveTo(_) => {
							// continue moving
							self.locations.insert(
								unit_id,
								UnitLocation::ByMoveTask(next_simulation_id),
							);
						}
						_ => {
							// finished moving, now do something else
							self.finished_moving(unit_id, destination.clone());
						}
					}
				}
				_ => {
					// For other task types, we might not need to update the location
				}
			}
		}
		if let Some(unit_tasks) = self.unit_tasks.get_mut(&unit_id) {
			unit_tasks.current_simulation_id = Some(next_simulation_id);
		}
		self.in_progress.push(next);
		Ok(())
	}

	pub fn simulation_completed(
		&mut self,
		completion: TaskCompletion,
		// tick_completion_sender: &mut broadcast::Sender<event::PublishEvent>,
	) -> Result<(), EngineError> {
		let unit_id =
			// don't worry about this until we have examples of other simulations
			// when we do, todo: clean this up
			completion.get_unit_id().ok_or(EngineError::InternalError)?;

		let simulation_id = completion
			.get_simulation_id()
			.ok_or(EngineError::InternalError)?;

		if let Some(next_simulation_id) =
			self.get_next_task(unit_id, simulation_id)
		{
			self.transition_simulation(
				unit_id,
				simulation_id,
				next_simulation_id,
			)?;
		} else {
			self.last_simulation_completed(unit_id, simulation_id)?;
		}
		Ok(())
	}

	pub async fn task_completed(
		&mut self,
		completion: TaskCompletion,
		_tick_completion_sender: &mut broadcast::Sender<
			crate::event::PublishEvent,
		>,
	) -> Result<(), EngineError> {
		if let Some(_simulation_id) = completion.get_simulation_id() {}
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
			.then_with(|| self.finish_time.cmp(&other.finish_time).reverse())
			.then_with(|| {
				// tie-breaker: compare by unit id and simulation id
				match (&self.completion, &other.completion) {
					(
						TaskCompletion::DestinationReached {
							unit_id: u1,
							simulation_id: s1,
						},
						TaskCompletion::DestinationReached {
							unit_id: u2,
							simulation_id: s2,
						},
					) => u1.cmp(u2).then_with(|| s1.cmp(s2)),
				}
			})
	}
}

impl PartialEq for TaskProgress {
	fn eq(&self, other: &Self) -> bool {
		self.completion == other.completion
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
			completion: TaskCompletion::DestinationReached {
				unit_id: match &task.progress.completion {
					TaskCompletion::DestinationReached { unit_id, .. } => {
						*unit_id
					}
				},
				simulation_id: Some(task.id),
			},
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
impl TaskProgress {
	fn get_simulation_id(&self) -> Option<SimulatedId> {
		match &self.completion {
			TaskCompletion::DestinationReached { simulation_id, .. } => {
				simulation_id.clone()
			}
		}
	}
}
