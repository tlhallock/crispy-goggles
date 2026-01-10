use common::model::{Health, Speed};

use common::model;


pub struct UnitTemplate {
	pub health: Option<Health>,
	pub speed: Option<Speed>,
	// keep this separate from the view shape?
	// pub model_shape: Option<model::Shape>,
	pub display_type: Option<model::UnitDisplayType>,
}
