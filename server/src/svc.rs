use crate::viewer::GameViewer;
use actix_web::cookie::time::Time;
use common::grpc::PathSegment;
use common::grpc::{
    CreateShapeRequest, CreateShapeResponse, Event, SubscribeRequest,
    shape_events_server::ShapeEvents,
};
use common::lobby::Player;
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
        user_requests_tx: tokio::sync::mpsc::Sender<crate::event::PlayerRequest>,
        tick_tx: broadcast::Sender<crate::event::PublishEvent>,
    ) -> Self {
        Self {
            next_id: Arc::new(AtomicU64::new(1)),
            player_requests_tx: user_requests_tx,
            tick_tx,
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

    fn create_unit(&self) -> Animatable {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        Animatable {
            id,
            shape: Shape::Circle(1.0),
            fill: true,
            color: (0, 255, 0), // green
            path: vec![common::model::PathSegment {
                begin_time: 0,
                begin_location: common::model::Point { x: 0.0, y: 0.0 },
                delta: None,
                begin_orientation: 0.0,
                d_orientation: None,
            }],
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
        let player_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let rx = self.tick_tx.subscribe();

        let (grpc_tx, grpc_rx) = tokio::sync::mpsc::channel::<Result<Event, Status>>(100);
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
        let anim = self.make_random_anim();
        let anim = self.create_unit();
        let id = anim.id;

        // Broadcast "Show" event to all subscribers
        let ev: Event = Message::Show(anim).into();
        // let _ = self.tx.send(ev);

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
