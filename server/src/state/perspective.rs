use crate::event;
use crate::state::types::SequenceNumber;
use common::model::{PlayerId, UnitId};
use std::collections::HashMap;
use tokio::sync::broadcast;

use crate::engine::EngineError;
use crate::engine::EngineErrorKind;
use crate::engine_error;

#[derive(Default, Debug)]
pub struct PlayersGamePerspective {
	pub last_update: HashMap<UnitId, SequenceNumber>,
}

// TODO: this is not efficient
#[derive(Debug)]
pub struct PerspectiveUpdates {
	player_id: PlayerId,
	pub units_to_upsert: Vec<(UnitId, SequenceNumber, bool)>,
	pub units_to_remove: Vec<UnitId>,
}

impl PlayersGamePerspective {
	pub fn apply_changes(&mut self, updates: &PerspectiveUpdates) {
		for (unit_id, sequence_number, _) in &updates.units_to_upsert {
			self.last_update.insert(*unit_id, *sequence_number);
		}
		for unit_id in &updates.units_to_remove {
			self.last_update.remove(unit_id);
		}
	}
	pub fn unit_exists(
		&self,
		unit_id: &UnitId,
		sequence_number: SequenceNumber,
		perspective_updates: &mut PerspectiveUpdates,
	) {
		if let Some(last_seq) = self.last_update.get(unit_id) {
			if *last_seq < sequence_number {
				perspective_updates.units_to_upsert.push((
					*unit_id,
					sequence_number,
					false,
				));
			}
		} else {
			perspective_updates.units_to_upsert.push((
				*unit_id,
				sequence_number,
				true,
			));
		}
	}
}

impl PerspectiveUpdates {
	pub fn new(player_id: PlayerId) -> Self {
		Self {
			player_id,
			units_to_upsert: Vec::new(),
			units_to_remove: Vec::new(),
		}
	}
	pub fn send_changes(
		&self,
		game: &crate::state::game::GameState,
		tick_completion_sender: &broadcast::Sender<event::PublishEvent>,
	) -> Result<(), EngineError> {
		for (unit_id, _, is_new) in self.units_to_upsert.iter() {
			if *is_new {
				// TODO: send unit created event
				println!(
					"Player {}: sending create for unit {}",
					self.player_id, unit_id
				);
				let animatable = game.animate(self.player_id, *unit_id)?;
				if let Some(animatable) = animatable {
					tick_completion_sender
						.send(crate::event::PublishEvent::UnitCreated(
							animatable,
						))
						.map_err(|_e| {
							engine_error!(EngineErrorKind::UnableToSend)
						})?;
				} else {
					println!(
						"Player {}: no animatable for unit {}",
						self.player_id, unit_id
					);
				}
			} else {
				println!(
					"Player {}: sending update for unit {}",
					self.player_id, unit_id
				);
				let animatable = game.animate(self.player_id, *unit_id)?;
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
						.map_err(|_e| {
							engine_error!(EngineErrorKind::UnableToSend)
						})?;
				} else {
					println!(
						"Player {}: no animatable for unit {}",
						self.player_id, unit_id
					);
				}
			}
		}
		Ok(())
	}
}
