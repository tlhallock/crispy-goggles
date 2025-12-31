use crate::viewer::GameViewer;
use common::grpc::{
    CreateShapeRequest, CreateShapeResponse, Event, SubscribeRequest,
    shape_events_server::ShapeEvents,
};
use common::model::{self, PlayerId};
use common::model::{Animatable, Message, Shape};
use common::model::{Coord, TIME_PER_SECOND, TimeStamp};
use rand::Rng;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use tokio::time::Duration;
//use std::time::Duration;

#[derive(Clone)]
pub struct ShapeSvc {
    // to remove
    next_id: Arc<AtomicU64>,
    player_requests_tx: tokio::sync::mpsc::Sender<crate::event::PlayerRequest>,
    tick_tx: broadcast::Sender<crate::event::PublishEvent>,
}

impl ShapeSvc {
    pub fn new(
        user_requests_tx: tokio::sync::mpsc::Sender<
            crate::event::PlayerRequest,
        >,
        tick_tx: broadcast::Sender<crate::event::PublishEvent>,
    ) -> Self {
        Self {
            next_id: Arc::new(AtomicU64::new(1)),
            player_requests_tx: user_requests_tx,
            tick_tx,
        }
    }
}

type EventStream =
    Pin<Box<dyn Stream<Item = Result<Event, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl ShapeEvents for ShapeSvc {
    type SubscribeStream = EventStream;

    async fn subscribe(
        &self,
        _req: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let player_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let rx = self.tick_tx.subscribe();

        let (grpc_tx, grpc_rx) =
            tokio::sync::mpsc::channel::<Result<Event, Status>>(100);
        let mut viewer = GameViewer::new(player_id as PlayerId, grpc_tx, rx);

        self.player_requests_tx
            .send(crate::event::PlayerRequest::PlayerJoined(player_id))
            .await
            .map_err(|_e| Status::internal("failed to send join request"))?;

        tokio::spawn(async move {
            match viewer.handle_events().await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error in viewer event handling: {:?}", e);
                }
            }
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(grpc_rx);
        Ok(Response::new(Box::pin(stream) as Self::SubscribeStream))
    }

    async fn create_shape(
        &self,
        _req: Request<CreateShapeRequest>,
    ) -> Result<Response<CreateShapeResponse>, Status> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Broadcast "Show" event to all subscribers
        // let _ = self.tx.send(ev);
        self.player_requests_tx
            .send(crate::event::PlayerRequest::CreateUnit(id))
            .await
            .map_err(|_e| {
                Status::internal("failed to send create unit request")
            })?;

        Ok(Response::new(CreateShapeResponse { id }))
    }

    async fn queue(
        &self,
        _req: Request<common::grpc::QueueRequest>,
    ) -> Result<Response<common::grpc::QueueResponse>, Status> {
        Err(Status::unimplemented("not implemented yet"))
    }

    async fn clear_queue(
        &self,
        _req: Request<common::grpc::ClearQueueRequest>,
    ) -> Result<Response<common::grpc::ClearQueueResponse>, Status> {
        Err(Status::unimplemented("not implemented yet"))
    }
}
