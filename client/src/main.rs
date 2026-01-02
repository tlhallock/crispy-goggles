use common::grpc::shape_events_client::ShapeEventsClient;
use common::grpc::{CreateShapeRequest, CreateShapeResponse, SubscribeRequest};
use tonic::Request;
use tonic::transport::Channel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Build a transport channel explicitly (works even when generated client has no `connect()`).
	let channel = Channel::from_static("http://127.0.0.1:50051")
		.connect()
		.await?;

	let mut client = ShapeEventsClient::new(channel);
	let mut identity = None;

	// Subscribe
	let mut stream = client.subscribe(SubscribeRequest {}).await?.into_inner();

	println!("Subscribed. Waiting for events...");
	while let Some(ev) = stream.message().await? {
		match &ev.kind {
			Some(common::grpc::event::Kind::Synchronize(_)) => {
				// skip logging synchronize events
			}
			Some(common::grpc::event::Kind::Warning(warning)) => {
				eprintln!("Warning from server: {}", warning.message);
			}
			Some(common::grpc::event::Kind::PlayerIdentity(ident)) => {
				identity = Some(ident.player_id);
				println!("Received player identity event");

				let mut request = Request::new(CreateShapeRequest {});
				request.metadata_mut().insert(
					"player-id",
					ident.player_id.to_string().parse().unwrap(),
				);

				let resp: CreateShapeResponse =
					client.create_shape(request).await?.into_inner();
				println!("Created shape id={}", resp.id);
			}
			_ => {
				println!("{ev:?}");
			}
		}
		if matches!(ev.kind, Some(common::grpc::event::Kind::Synchronize(_))) {
			// skip logging synchronize events
		} else {
		}
	}

	Ok(())
}
