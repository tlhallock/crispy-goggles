use crate::engine::EngineError;
use crate::state::game::TaskProgress;
use crate::state::templates::UnitTemplate;
use crate::state::types::{SequenceNumber, SimulatedId};
use common::model::TimeStamp;
use common::model::UnitId;
use std::collections::HashMap;

use crate::state::perspective::PerspectiveUpdates;
use crate::state::perspective::PlayersGamePerspective;

pub struct TaskTransition {
	pub unit_id: UnitId,
	pub game_time: TimeStamp,
	pub from: Option<(common::model::Task, SimulatedId, bool)>,
	// a task is also inside the task progress...
	pub to: Option<(common::model::Task, TaskProgress)>,
}

// This could go in the engine as well
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

#[derive(Default, Debug)]
pub struct UnitTasks {
	pub current_simulation_id: Option<SimulatedId>,
	pub sequence_number: SequenceNumber,
	pub tasks: Vec<SimulatedId>,
}

// rename to unittask completed
#[derive(Debug, Clone)]
pub enum TaskCompletion {
	DestinationReached {
		unit_id: UnitId,
		// This should really be a location, it doesn't matter how
		// it got there
		simulation_id: Option<SimulatedId>,
	},
	// TransferCompleted,
}

#[derive(Debug, Clone)]
pub struct CompletedTask {
	pub unit_id: UnitId,
	pub simulation_id: SimulatedId,
	pub task: common::model::Task,
}

// todo move
#[derive(Debug, Clone, Ord, PartialEq, PartialOrd, Eq)]
pub enum SimulationEvent {
	TaskCompleted(CompletedTask),
	// Collision,
}

#[derive(Debug, Default)]
pub struct TaskManager {
	pub unit_tasks: HashMap<UnitId, UnitTasks>,
	pub simulated_tasks: HashMap<SimulatedId, SimulatedTask>,
}

impl TaskManager {
	pub fn unit_created(&mut self, unit_id: UnitId) {
		self.unit_tasks.insert(unit_id, UnitTasks::default());
	}

	// fn get_current_task(
	// 	&self,
	// 	unit_id: UnitId,
	// ) -> Result<Option<common::model::Task>, EngineError> {
	// 	let unit_tasks = self
	// 		.unit_tasks
	// 		.get(&unit_id)
	// 		.ok_or(EngineError::InvalidUnitId)?;
	// 	if let Some(current_sim_id) = unit_tasks.current_simulation_id {
	// 		let simulated_task = self
	// 			.simulated_tasks
	// 			.get(&current_sim_id)
	// 			.ok_or(EngineError::InternalError)?;
	// 		Ok(Some(simulated_task.task.clone()))
	// 	} else {
	// 		Ok(None)
	// 	}
	// }

	pub fn set_task_queue_requested(
		&mut self,
		unit_id: UnitId,
		simulated_tasks: Vec<SimulatedTask>,
		game_time: TimeStamp,
	) -> Result<TaskTransition, EngineError> {
		let unit_tasks = self
			.unit_tasks
			.get_mut(&unit_id)
			.ok_or(EngineError::InvalidUnitId)?;

		let current_task =
			if let Some(current_sim_id) = unit_tasks.current_simulation_id {
				let simulated_task = self
					.simulated_tasks
					.get(&current_sim_id)
					.ok_or(EngineError::InternalError)?;
				Some((simulated_task.task.clone(), current_sim_id, false))
			} else {
				None
			};

		let changed =
			!unit_tasks.tasks.is_empty() || !simulated_tasks.is_empty();

		for sid in &unit_tasks.tasks {
			self.simulated_tasks.remove(sid);
		}

		let mut next_task = None;
		//  this should be based on time...
		if let Some(task) = &simulated_tasks.first() {
			unit_tasks.current_simulation_id = Some(task.id);
			next_task = Some((task.task.clone(), task.progress.clone()));
		}
		unit_tasks.tasks.clear();
		unit_tasks
			.tasks
			.extend(simulated_tasks.iter().map(|t| t.id));

		self.simulated_tasks
			.extend(simulated_tasks.into_iter().map(|t| (t.id, t)));

		if changed {
			unit_tasks.sequence_number += 1;
		}

		Ok(TaskTransition {
			unit_id,
			game_time,
			from: current_task,
			to: next_task,
		})
	}

	// pub fn clear_tasks_requested(
	// 	&mut self,
	// 	unit_id: UnitId,
	// 	game_time: TimeStamp,
	// ) -> Result<TaskTransition, EngineError> {
	// 	let unit_tasks = self
	// 		.unit_tasks
	// 		.get_mut(&unit_id)
	// 		.ok_or(EngineError::InvalidUnitId)?;

	// 	let current_task =
	// 		if let Some(current_sim_id) = unit_tasks.current_simulation_id {
	// 			let simulated_task = self
	// 				.simulated_tasks
	// 				.get(&current_sim_id)
	// 				.ok_or(EngineError::InternalError)?;
	// 			Some(simulated_task.task.clone())
	// 		} else {
	// 			None
	// 		};

	// 	for simulated_id in unit_tasks.tasks.iter() {
	// 		self.simulated_tasks.remove(simulated_id);
	// 	}
	// 	let is_empty = unit_tasks.tasks.is_empty();
	// 	unit_tasks.tasks.clear();
	// 	unit_tasks.current_simulation_id = None;
	// 	if !is_empty {
	// 		unit_tasks.sequence_number += 1;
	// 	}
	// 	Ok(TaskTransition {
	// 		unit_id,
	// 		game_time,
	// 		complete: true,
	// 		from: current_task,
	// 		to: None,
	// 	})
	// }

	pub fn task_completed(
		&mut self,
		game_time: TimeStamp,
		completion: CompletedTask,
	) -> Result<TaskTransition, EngineError> {
		let unit_tasks = self
			.unit_tasks
			.get_mut(&completion.unit_id)
			.ok_or(EngineError::InvalidUnitId)?;

		assert!(
			unit_tasks.current_simulation_id == Some(completion.simulation_id),
			"Completed task does not match current task"
		);
		let first = unit_tasks.tasks.remove(0);
		assert!(
			first == completion.simulation_id,
			"Completed task is not at the front of the queue"
		);

		let from = {
			let simulated_task = self
				.simulated_tasks
				.remove(&completion.simulation_id)
				.ok_or(EngineError::InternalError)?;

			assert!(
				simulated_task.task == completion.task,
				"Completed task does not match simulated task"
			);
			Some((simulated_task.task, completion.simulation_id, true))
		};

		let to = if let Some(next_sim_id) = unit_tasks.tasks.first() {
			unit_tasks.current_simulation_id = Some(*next_sim_id);
			let simulated_task = self
				.simulated_tasks
				.get(next_sim_id)
				.ok_or(EngineError::InternalError)?;
			Some((simulated_task.task.clone(), simulated_task.progress.clone()))
		} else {
			unit_tasks.current_simulation_id = None;
			None
		};

		Ok(TaskTransition {
			unit_id: completion.unit_id,
			game_time: game_time,
			from: from,
			to: to,
		})
	}

	pub fn get_sequence_number(
		&self,
		unit_id: UnitId,
	) -> Result<SequenceNumber, EngineError> {
		let unit_tasks = self
			.unit_tasks
			.get(&unit_id)
			.ok_or(EngineError::InvalidUnitId)?;
		Ok(unit_tasks.sequence_number)
	}

	pub fn show_perspective(
		&self,
		perspective: &PlayersGamePerspective,
		updates: &mut PerspectiveUpdates,
	) {
		for (unit_id, unit_tasks) in &self.unit_tasks {
			perspective.unit_exists(
				unit_id,
				unit_tasks.sequence_number,
				updates,
			);
		}
	}
}

impl Ord for CompletedTask {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.unit_id
			.cmp(&other.unit_id)
			.then(self.simulation_id.cmp(&other.simulation_id))
	}
}

impl PartialOrd for CompletedTask {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Eq for CompletedTask {}

impl PartialEq for CompletedTask {
	fn eq(&self, other: &Self) -> bool {
		self.unit_id == other.unit_id
			&& self.simulation_id == other.simulation_id
	}
}
