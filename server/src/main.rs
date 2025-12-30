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
    let (tx, _rx) = broadcast::channel::<common::grpc::Event>(1024);

    {
        let tx = tx.clone();
        tokio::spawn(async move {
            use tokio::time::{Duration, interval};
            let mut ticker = interval(Duration::from_millis(30));
            loop {
                ticker.tick().await;

                let wall_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_millis() as u64;

                // For now: game_time == wall_time (you can change this later)
                let game_ms = wall_ms;

                let ev = common::grpc::Event {
                    kind: Some(common::grpc::event::Kind::Synchronize(
                        common::grpc::Synchronize {
                            wall_time: wall_ms,
                            game_time: game_ms,
                        },
                    )),
                };

                let _ = tx.send(ev);
            }
        });
    }

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
