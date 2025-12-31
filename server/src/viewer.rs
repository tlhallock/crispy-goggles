use crate::event::PublishEvent;
use common::model::PlayerId;



pub struct GameViewer {
    player_id: PlayerId,
    grpc_tx: tokio::sync::mpsc::Sender<Result<common::grpc::Event, tonic::Status>>,
    rx: tokio::sync::broadcast::Receiver<PublishEvent>,
}

impl GameViewer {
    pub fn new(
        player_id: PlayerId,
        grpc_tx: tokio::sync::mpsc::Sender<Result<common::grpc::Event, tonic::Status>>,
        rx: tokio::sync::broadcast::Receiver<PublishEvent>,
    ) -> Self {
        Self {
            player_id,
            grpc_tx,
            rx,
        }
    }

    pub async fn handle_events(&mut self) -> Result<(), tonic::Status> {
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
                        .map_err(|_e| tonic::Status::internal("failed to send event"))?;
                }
            }
        }
        Ok(())
    }
}
