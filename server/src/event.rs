use common::model::PlayerId;
use common::model::TimeStamp;
use common::model::UnitId;

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
    CreateUnit(UnitId),
    UpdateIntentions {
        // intentions: common::grpc::PlayerIntentions,
    },
    PlayerLeft(PlayerId),
}

#[derive(Debug, Clone)]
pub enum PublishEvent {
    UnitCreated(common::model::Animatable),
    TickCompleted(TickCompletedEvent),
}

pub enum EngineEvent {
    Tick(TimeStamp),
    PlayerRequest(PlayerRequest),
}

pub enum GameServerEvent {
    Engine(EngineEvent),
    PublishEvent(PublishEvent),
}
