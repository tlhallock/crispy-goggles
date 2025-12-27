use anyhow::Result;
use common::grpc::shape_events_server::ShapeEventsServer;
use http::header::{HeaderName, HeaderValue};
use std::net::SocketAddr;
use tokio::sync::broadcast;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};

mod svc;

#[tokio::main]
async fn main() -> Result<()> {
    // broadcast buffer
    let (tx, _rx) = broadcast::channel::<common::grpc::Event>(1024);

    let service = svc::ShapeSvc::new(tx);

    // CORS for browsers (grpc-web). You can tighten this later.
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::any())
        .allow_headers(tower_http::cors::Any)
        .expose_headers([
            HeaderName::from_static("grpc-status"),
            HeaderName::from_static("grpc-message"),
            HeaderName::from_static("grpc-status-details-bin"),
        ])
        .allow_methods(tower_http::cors::Any)
        .allow_credentials(false)
        .max_age(std::time::Duration::from_secs(60 * 60))
        .allow_origin(AllowOrigin::any())
        .allow_headers(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_private_network(true)
        .allow_origin(AllowOrigin::any());

    let addr: SocketAddr = "127.0.0.1:50051".parse()?;

    Server::builder()
        // needed for grpc-web in dev (HTTP/1). Still accepts HTTP/2 for native gRPC. :contentReference[oaicite:4]{index=4}
        .accept_http1(true)
        .layer(cors)
        .layer(GrpcWebLayer::new())
        .add_service(ShapeEventsServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
