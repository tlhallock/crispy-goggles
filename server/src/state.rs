use common::model::{Tasks, TimeStamp, UnitId};
use std::collections::BinaryHeap;
use std::collections::HashMap;

use crate::engine::EngineError;

struct TaskProgress {
	unit_id: UnitId,
	finish_time: TimeStamp,
	task_id: u64,
}

pub struct GameState {
	pub next_id: u64,
	pub begin_time: TimeStamp,

	pub last_time: TimeStamp,
	pub last_wall_ms: TimeStamp,

	pub unit_tasks: HashMap<UnitId, Tasks>,
	pub in_progress: BinaryHeap<TaskProgress>,
}

impl GameState {
	pub fn new() -> Self {
		Self {
			next_id: 1,
			begin_time: 0,
			last_time: 0,
			last_wall_ms: 0,
			unit_tasks: HashMap::new(),
			in_progress: BinaryHeap::new(),
		}
	}

	pub fn get_next_id(&mut self) -> u64 {
		let id = self.next_id;
		self.next_id += 1;
		id
	}

	pub fn queue_tasks(
		&mut self,
		unit_id: UnitId,
		tasks: Vec<common::model::Task>,
	) -> Result<(), EngineError> {
		let len = tasks.len();
		let range = self.next_id..self.next_id + len as u64;
		if let Some(unit_tasks) = self.unit_tasks.get_mut(&unit_id) {
			unit_tasks.tasks.extend(range.zip(tasks.into_iter()));
			self.next_id += len as u64;
			Ok(())
		} else {
			Err(EngineError::MalformedRequest)
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
			.then_with(|| self.unit_id.cmp(&other.unit_id).reverse())
			.then_with(|| self.task_id.cmp(&other.task_id).reverse())
	}
}
impl PartialEq for TaskProgress {
	fn eq(&self, other: &Self) -> bool {
		self.task_id == other.task_id
	}
}
impl Eq for TaskProgress {}
