


type SimulatedId = u64;
type SequenceNumber = u64;

use crate::state;

#[derive(Default, Debug)]
pub struct UnitTasks {
	pub current_simulation_id: Option<state::SimulatedId>,
	pub sequence_number: state::SequenceNumber,
	pub tasks: Vec<state::SimulatedId>,
}
