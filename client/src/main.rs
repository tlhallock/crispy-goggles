use common::grpc::shape_events_client::ShapeEventsClient;
use common::grpc::{CreateShapeRequest, SubscribeRequest};
use tonic::transport::Channel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build a transport channel explicitly (works even when generated client has no `connect()`).
    let channel = Channel::from_static("http://127.0.0.1:50051")
        .connect()
        .await?;

    let mut client = ShapeEventsClient::new(channel);

    // Subscribe
    let mut stream = client.subscribe(SubscribeRequest {}).await?.into_inner();

    // Also create a shape once, just to test unary RPC
    let resp = client
        .create_shape(CreateShapeRequest {})
        .await?
        .into_inner();
    println!("Created shape id={}", resp.id);

    println!("Subscribed. Waiting for events...");
    while let Some(ev) = stream.message().await? {
        println!("{ev:?}");
    }

    Ok(())
}
