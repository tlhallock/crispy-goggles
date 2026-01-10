use crate::event;
use crate::state::tasks::UnitTasks;
use common::model::OrientedPoint;
use common::model::{Health, PlayerId, Speed, TaskId, TimeStamp, UnitId};
use std::collections::BinaryHeap;
use std::collections::HashMap;
use tokio::sync::broadcast;

use common::model;
use std::collections::HashSet;

use crate::engine::EngineError;

pub struct UnitTemplate {
	pub health: Option<Health>,
	pub speed: Option<Speed>,
	// keep this separate from the view shape?
	// pub model_shape: Option<model::Shape>,
	pub display_type: Option<model::UnitDisplayType>,
}
