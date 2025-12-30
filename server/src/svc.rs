use common::grpc::PathSegment;
use common::grpc::{
    CreateShapeRequest, CreateShapeResponse, Event, SubscribeRequest,
    shape_events_server::ShapeEvents,
};
use common::model::{Animatable, Message, Shape};
use common::model::{Coord, TIME_PER_SECOND, TimeStamp};
use common::{convert::*, model};
use rand::Rng;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;
use tokio_stream::{Stream, StreamExt, wrappers::BroadcastStream};
use tonic::{Request, Response, Status};

use tokio::time::Duration;
//use std::time::Duration;

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

        // todo: extract this to common function
        let wall_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;

        let begin_time = wall_ms + rng.random_range(0..2 * TIME_PER_SECOND) as TimeStamp;
        let delta = model::Delta {
            dx: rng.random_range(-1.0..1.0),
            dy: rng.random_range(-1.0..1.0),
        }
        .normalize(5.0 * TIME_PER_SECOND as f64);
        let begin_location = model::Point {
            x: rng.random_range(100.0 as Coord..400.0 as Coord),
            y: rng.random_range(100.0 as Coord..400.0 as Coord),
        };
        let d_t = rng.random_range(5 * TIME_PER_SECOND..20 * TIME_PER_SECOND) as TimeStamp;
        let end_location = model::Point {
            x: begin_location.x + (d_t as f64 * delta.dx) as Coord,
            y: begin_location.y + (d_t as f64 * delta.dy) as Coord,
        };
        let d_orientation = rng.random_range(-180.0..180.0);
        let path = vec![
            common::model::PathSegment {
                begin_time,
                begin_location,
                delta: Some(delta),
                begin_orientation: rng.random_range(0.0..360.0),
                d_orientation: Some(d_orientation),
            },
            common::model::PathSegment {
                begin_time: begin_time + d_t,
                begin_location: end_location,
                delta: None,
                begin_orientation: 0.0,
                d_orientation: None,
            },
        ];

        Animatable {
            id,
            shape,
            fill: rng.random_bool(0.7),
            color,
            path,
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
