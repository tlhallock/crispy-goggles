use common::model::PlayerId;
use common::model::TimeStamp;

pub struct TickEvent {
    pub wall_ms: TimeStamp,
}

#[derive(Debug, Clone)]
pub struct TickCompletedEvent {
    pub wall_ms: TimeStamp,
    pub game_time: TimeStamp,
}

pub struct UpdateIntentionsEvent {}

pub enum PlayerRequest {
    PlayerJoined(PlayerId),
    UpdateIntentions {
        // intentions: common::grpc::PlayerIntentions,
    },
    PlayerLeft(PlayerId),
}

#[derive(Debug, Clone)]
pub enum PublishEvent {
    TickCompleted(TickCompletedEvent),
}

pub enum GameServerEvent {
    Tick(TickEvent),
    PlayerRequest(PlayerRequest),
    PublishEvent(PublishEvent),
}
