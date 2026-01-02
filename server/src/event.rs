use common::model;
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
	CreateUnit(PlayerId, UnitId),
	UpdateIntentions(common::grpc::QueueRequest),
	ClearQueue(UnitId),
	PlayerLeft(PlayerId),
}

#[derive(Debug, Clone)]
pub struct WarningContent {
	pub user_id: PlayerId,
	pub message: String,
}

#[derive(Debug, Clone)]
pub struct TasksUpdatedEvent {
	pub unit_id: UnitId,
	pub tasks: Vec<common::grpc::AnimationSegment>,
}

#[derive(Debug, Clone)]
pub enum PublishEvent {
	Warning(WarningContent),
	UnitCreated(/*model::UnitDetails, */ model::Animatable),
	TickCompleted(TickCompletedEvent),
	TasksUpdated(TasksUpdatedEvent),
}

pub enum EngineEvent {
	Tick(TimeStamp),
	PlayerRequest(PlayerRequest),
}

pub enum GameServerEvent {
	Engine(EngineEvent),
	PublishEvent(PublishEvent),
}
