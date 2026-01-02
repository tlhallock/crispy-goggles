use crate::event::PublishEvent;
use common::model::PlayerId;

pub struct GameViewer {
	player_id: PlayerId,
	grpc_tx:
		tokio::sync::mpsc::Sender<Result<common::grpc::Event, tonic::Status>>,
	rx: tokio::sync::broadcast::Receiver<PublishEvent>,
}

impl GameViewer {
	pub fn new(
		player_id: PlayerId,
		grpc_tx: tokio::sync::mpsc::Sender<
			Result<common::grpc::Event, tonic::Status>,
		>,
		rx: tokio::sync::broadcast::Receiver<PublishEvent>,
	) -> Self {
		Self {
			player_id,
			grpc_tx,
			rx,
		}
	}

	pub async fn handle_events(&mut self) -> Result<(), tonic::Status> {
		// Send player identity as the first event
		self.grpc_tx
			.send(Ok(common::grpc::Event {
				kind: Some(common::grpc::event::Kind::PlayerIdentity(
					common::grpc::PlayerIdentity {
						player_id: self.player_id,
					},
				)),
			}))
			.await
			.map_err(|_e| {
				tonic::Status::internal("failed to send player identity")
			})?;

		while let Ok(publish_event) = self.rx.recv().await {
			match publish_event {
				PublishEvent::TickCompleted(event) => {
					self.grpc_tx
						.send(Ok(common::grpc::Event {
							kind: Some(common::grpc::event::Kind::Synchronize(
								common::grpc::Synchronize {
									wall_time: event.wall_ms,
									game_time: event.game_time,
								},
							)),
						}))
						.await
						.map_err(|_e| {
							tonic::Status::internal("failed to send event")
						})?;
				}
				PublishEvent::UnitCreated(anim) => {
					self.grpc_tx
						.send(Ok(common::grpc::Event {
							kind: Some(common::grpc::event::Kind::Show(
								anim.into(),
							)),
						}))
						.await
						.map_err(|_e| {
							// TODO: should be an .into()
							tonic::Status::internal("failed to send event")
						})?;
				}
				PublishEvent::Warning(warning) => {
					self.grpc_tx
						.send(Ok(common::grpc::Event {
							kind: Some(common::grpc::event::Kind::Warning(
								common::grpc::Warning {
									message: warning.message,
								},
							)),
						}))
						.await
						.map_err(|_e| {
							tonic::Status::internal("failed to send event")
						})?;
				}
				PublishEvent::TasksUpdated(updates) => {
					self.grpc_tx
						.send(Ok(common::grpc::Event {
							kind: Some(common::grpc::event::Kind::Update(
								common::grpc::Update {
									unit_id: updates.unit_id,
									queue: updates
										.tasks
										.iter()
										.map(|t| t.clone())
										.collect(),
									details: None,
								},
							)),
						}))
						.await
						.map_err(|_e| {
							tonic::Status::internal("failed to send event")
						})?;
				}
			}
		}
		Ok(())
	}
}
