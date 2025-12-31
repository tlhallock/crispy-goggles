use crate::state::GameState;

use anyhow::Result;
use common::grpc::shape_events_server::ShapeEventsServer;
use http::header::{HeaderName, HeaderValue};
use std::net::SocketAddr;

use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};

// pub fn simulate_tick(
//     game_state: &Arc<GameState>,
//     tx: &tokio::sync::broadcast::Sender<common::grpc::Event>,
//     wall_ms: u64,
// ) {
//     // Simulation logic would go here

//     let state = game_state.as_mut();

//     let game_ms = wall_ms;

//     let ev = common::grpc::Event {
//         kind: Some(common::grpc::event::Kind::Synchronize(
//             common::grpc::Synchronize {
//                 wall_time: wall_ms,
//                 game_time: game_ms,
//             },
//         )),
//     };

//     let _ = tx.send(ev);
// }

async fn tick(
    user_requests_receiver: &mut mpsc::Receiver<crate::event::PlayerRequest>,
    tick_completion_sender: &mut broadcast::Sender<crate::event::PublishEvent>,
    game_state: &mut GameState,
    // wall_ns?
    wall_ms: u64,
) {
    // For now: game_time == wall_time (you can change this later)
    let game_time = wall_ms;

    tick_completion_sender.send(crate::event::PublishEvent::TickCompleted(
        crate::event::TickCompletedEvent {
            wall_ms: wall_ms,
            game_time: game_time,
        },
    ));

    // TODO: move
    // let ev = ;

    // let _ = tx.send(ev);
}

pub async fn run_engine(
    mut user_requests_receiver: mpsc::Receiver<crate::event::PlayerRequest>,
    mut tick_completion_sender: broadcast::Sender<crate::event::PublishEvent>,
) {
    let mut game_state = GameState::new();
    let mut ticker = interval(Duration::from_millis(30));
    loop {
        let wall_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;

        tick(
            &mut user_requests_receiver,
            &mut tick_completion_sender,
            &mut game_state,
            wall_ms,
        )
        .await;

        ticker.tick().await;
    }
}
