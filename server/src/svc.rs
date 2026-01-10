use crate::event::PlayerRequest;
use crate::viewer::GameViewer;
use common::grpc::{
	CreateShapeRequest, CreateShapeResponse, Event, SubscribeRequest,
	shape_events_server::ShapeEvents,
};
use common::model::PlayerId;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

//use std::time::Duration;

#[derive(Clone)]
pub struct ShapeSvc {
	// to remove
	next_id: Arc<AtomicU64>,
	player_requests_tx: tokio::sync::mpsc::Sender<crate::event::PlayerRequest>,
	tick_tx: broadcast::Sender<crate::event::PublishEvent>,

	secrets: HashMap<PlayerId, String>,
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
			secrets: HashMap::new(),
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
		req: Request<CreateShapeRequest>,
	) -> Result<Response<CreateShapeResponse>, Status> {
		// Extract player ID from metadata
		let player_id = req
			.metadata()
			.get("player-id")
			.and_then(|v| v.to_str().ok())
			.and_then(|s| s.parse::<u64>().ok())
			.ok_or_else(|| {
				Status::unauthenticated("missing or invalid player-id header")
			})?;

		let id = self.next_id.fetch_add(1, Ordering::Relaxed);
		self.player_requests_tx
			.send(PlayerRequest::CreateUnit(player_id, id))
			.await
			.map_err(|_e| {
				Status::internal("failed to send create unit request")
			})?;

		Ok(Response::new(CreateShapeResponse { id }))
	}

	async fn queue(
		&self,
		req: Request<common::grpc::SetQueueRequest>,
	) -> Result<Response<common::grpc::SetQueueResponse>, Status> {
		// Extract player ID from metadata
		let _player_id = req
			.metadata()
			.get("player-id")
			.and_then(|v| v.to_str().ok())
			.and_then(|s| s.parse::<u64>().ok())
			.ok_or_else(|| {
				Status::unauthenticated("missing or invalid player-id header")
			})?;

		self.player_requests_tx
			.send(crate::event::PlayerRequest::UpdateIntentions(
				req.into_inner(),
			))
			.await
			.map_err(|_e| {
				Status::internal("failed to send update intentions request")
			})?;
		Ok(Response::new(common::grpc::SetQueueResponse {
			valid: true,
		}))
	}

	async fn clear_queue(
		&self,
		req: Request<common::grpc::ClearQueueRequest>,
	) -> Result<Response<common::grpc::ClearQueueResponse>, Status> {
		// TODO: send an empty queue
		// Extract player ID from metadata
		let _player_id = req
			.metadata()
			.get("player-id")
			.and_then(|v| v.to_str().ok())
			.and_then(|s| s.parse::<u64>().ok())
			.ok_or_else(|| {
				Status::unauthenticated("missing or invalid player-id header")
			})?;

		self.player_requests_tx
			.send(crate::event::PlayerRequest::ClearQueue(
				req.into_inner().unit_id,
			))
			.await
			.map_err(|_e| {
				Status::internal("failed to send update intentions request")
			})?;
		Ok(Response::new(common::grpc::ClearQueueResponse {}))
	}
}
