use common::grpc::{
    Event, SubscribeRequest,
    shape_events_server::{ShapeEvents, ShapeEventsServer},
};
use std::pin::Pin;
use tokio::sync::broadcast;
use tokio_stream::{Stream, StreamExt, wrappers::BroadcastStream};
use tonic::{Request, Response, Status};

#[derive(Clone)]
pub struct ShapeGrpcService {
    tx: broadcast::Sender<Event>,
}

impl ShapeGrpcService {
    pub fn new(tx: broadcast::Sender<Event>) -> Self {
        Self { tx }
    }

    pub fn server(self) -> ShapeEventsServer<Self> {
        ShapeEventsServer::new(self)
    }
}

type EventStream = Pin<Box<dyn Stream<Item = Result<Event, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl ShapeEvents for ShapeGrpcService {
    type SubscribeStream = EventStream;

    async fn subscribe(
        &self,
        _req: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let rx = self.tx.subscribe();

        let s = BroadcastStream::new(rx).map(|item| match item {
            Ok(ev) => Ok(ev),
            Err(_lagged) => Err(Status::unavailable("client lagged behind broadcast buffer")),
        });

        Ok(Response::new(Box::pin(s) as Self::SubscribeStream))
    }
}
