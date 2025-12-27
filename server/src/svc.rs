use common::convert::*;
use common::grpc::{
    CreateShapeRequest, CreateShapeResponse, Event, SubscribeRequest,
    shape_events_server::ShapeEvents,
};
use common::model::{Animatable, Message, Shape};
use rand::Rng;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;
use tokio_stream::{Stream, StreamExt, wrappers::BroadcastStream};
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct ShapeSvc {
    next_id: Arc<AtomicU64>,
    tx: broadcast::Sender<Event>,
}

impl ShapeSvc {
    pub fn new(tx: broadcast::Sender<Event>) -> Self {
        Self {
            next_id: Arc::new(AtomicU64::new(1)),
            tx,
        }
    }

    fn make_random_anim(&self) -> Animatable {
        let mut rng = rand::rng();

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let shape = if rng.random_bool(0.5) {
            Shape::Circle(rng.random_range(10.0..80.0))
        } else {
            Shape::Rectangle(rng.random_range(20.0..140.0), rng.random_range(20.0..140.0))
        };

        let color = (rng.random::<u8>(), rng.random::<u8>(), rng.random::<u8>());

        Animatable {
            id,
            shape,
            fill: rng.random_bool(0.7),
            color,
            path: vec![],
        }
    }
}

type EventStream = Pin<Box<dyn Stream<Item = Result<Event, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl ShapeEvents for ShapeSvc {
    type SubscribeStream = EventStream;

    async fn subscribe(
        &self,
        _req: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let rx = self.tx.subscribe();

        // BroadcastStream is behind tokio-stream feature "sync" :contentReference[oaicite:7]{index=7}
        let s = BroadcastStream::new(rx).map(|item| match item {
            Ok(ev) => Ok(ev),
            Err(_lagged) => Err(Status::unavailable("client lagged behind broadcast buffer")),
        });

        Ok(Response::new(Box::pin(s) as Self::SubscribeStream))
    }

    async fn create_shape(
        &self,
        _req: Request<CreateShapeRequest>,
    ) -> Result<Response<CreateShapeResponse>, Status> {
        let anim = self.make_random_anim();
        let id = anim.id;

        // Broadcast "Show" event to all subscribers
        let ev: Event = Message::Show(anim).into();
        let _ = self.tx.send(ev);

        Ok(Response::new(CreateShapeResponse { id }))
    }
}
