use crate::event;
use common::grpc::Task;
use common::model::{Health, PlayerId, Speed, TaskId, TimeStamp, UnitId};
use common::model::{METERS, OrientedPoint};
use rand::rand_core::impls;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use tokio::sync::broadcast;

use common::model;
use std::collections::HashSet;

use crate::engine::EngineError;

type SimulatedId = u64;
type SequenceNumber = u64;

use crate::state;

#[derive(Default, Debug)]
pub struct UnitTasks {
	pub current_simulation_id: Option<state::SimulatedId>,
	pub sequence_number: state::SequenceNumber,
	pub tasks: Vec<state::SimulatedId>,
}
