use crate::event;
use common::model::OrientedPoint;
use common::model::{
	Health, PlayerId, Speed, TaskId, TimeStamp, UnitId,
};
use std::collections::BinaryHeap;
use std::collections::HashMap;
use tokio::sync::broadcast;

use common::model;
use std::collections::HashSet;

use crate::engine::EngineError;

type SimulatedId = u64;

pub enum UnitLocation {
	ByMoveTask(TaskId),
	Fixed(model::OrientedPoint),
}

pub struct TaskProgress {
	pub finish_time: TimeStamp,
	pub simulation_id: SimulatedId,
}

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

#[derive(Default)]
pub struct PlayersGamePerspective {
	known_units: HashSet<UnitId>,
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

	unit_tasks: HashMap<UnitId, Vec<SimulatedId>>,
	simulated_tasks: HashMap<SimulatedId, SimulatedTask>,
	in_progress: BinaryHeap<TaskProgress>,

	// inventory change listeners
	// area change listeners
	perspectives: HashMap<PlayerId, PlayersGamePerspective>,

	units: HashSet<UnitId>,
}

impl GameState {
	pub fn get_next_id(&mut self) -> u64 {
		let id = self.next_id;
		self.next_id += 1;
		id
	}

	pub fn queue_tasks(
		&mut self,
		unit_id: UnitId,
		tasks: Vec<SimulatedTask>,
	) -> Result<(), EngineError> {
		if let Some(unit_tasks) = self.unit_tasks.get_mut(&unit_id) {
			if let Some(task) = tasks.first() {
				self.in_progress.push(task.into());
			}
			for task in &tasks {
				unit_tasks.push(task.id);
			}
		} else {
			return Err(EngineError::MalformedRequest);
		}
		// self.simulated_tasks.extend(tasks.iter().map(|t| (t.id, t.clone())));
		for task in tasks {
			self.simulated_tasks.insert(task.id, task);
		}
		Ok(())
	}

	pub fn clear_tasks(&mut self, unit_id: UnitId) -> Result<(), EngineError> {
		if let Some(unit_tasks) = self.unit_tasks.get_mut(&unit_id) {
			for simulated_id in unit_tasks.iter() {
				self.simulated_tasks.remove(simulated_id);
			}
			self.in_progress
				.retain(|tp| !unit_tasks.contains(&tp.simulation_id));
			unit_tasks.clear();
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
		_location: OrientedPoint,
	) {
		self.owners.insert(unit_id, player_id);
		self.unit_tasks.insert(unit_id, vec![]);
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

		{
			// For each player perspective, send updates about new units
			let perspective = self
				.perspectives
				.get_mut(&player_id)
				.ok_or(EngineError::InternalError)?;

			for unit_id in self.units.iter() {
				if !perspective.known_units.contains(unit_id) {
					units_to_add.push(*unit_id);
				}
				perspective.known_units.insert(*unit_id);
			}
		}

		for unit_id in units_to_add.iter() {
			// TODO: send unit created event
			let animatable = self.animate(player_id, *unit_id)?;

			if let Some(animatable) = animatable {
				tick_completion_sender
					.send(crate::event::PublishEvent::UnitCreated(animatable))
					.map_err(|_e| EngineError::UnableToSend)?;
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

		let _position = match self.locations.get(&unit_id) {
			Some(UnitLocation::Fixed(pos)) => Some(pos.clone()),
			_ => None,
		};
		let queue: Option<Vec<model::AnimationSegment>> =
			match self.locations.get(&unit_id) {
				Some(UnitLocation::ByMoveTask(_)) => Some(
					self.unit_tasks
						.get(&unit_id)
						.ok_or(EngineError::InternalError)?
						.iter()
						.map(|simulation_id| {
							self.simulated_tasks
								.get(simulation_id)
								.map(|sim| sim.animation.clone())
								.ok_or(EngineError::InternalError)
						})
						.collect::<Result<Vec<_>, EngineError>>()?,
				),
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
		_time: TimeStamp,
	) -> Result<model::OrientedPoint, EngineError> {
		match self.locations.get(&unit_id) {
			Some(UnitLocation::Fixed(pos)) => Ok(pos.clone()),
			// TODO: move binary search from the ui/main.rs to common
			_ => Err(EngineError::InternalError),
		}
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
				self.simulation_id.cmp(&other.simulation_id).reverse()
			})
	}
}
impl PartialEq for TaskProgress {
	fn eq(&self, other: &Self) -> bool {
		self.simulation_id == other.simulation_id
	}
}
impl Eq for TaskProgress {}

impl Default for UnitTemplate {
	fn default() -> Self {
		Self {
			health: Some(model::Health {
				current: 100,
				max: 100,
			}),
			speed: Some(1.0 as model::Speed),
			display_type: Some(model::UnitDisplayType::SimpleUnit),
		}
	}
}

impl From<&SimulatedTask> for TaskProgress {
	fn from(task: &SimulatedTask) -> Self {
		Self {
			finish_time: task.progress.finish_time,
			simulation_id: task.id,
		}
	}
}
