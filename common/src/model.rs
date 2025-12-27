use serde::{Deserialize, Serialize};

pub type Coord = f32;
pub type TimeStamp = u64;
pub type Id = u64;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Shape {
    Circle(Coord),
    Rectangle(Coord, Coord),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathSegment {
    pub begin_location: (Coord, Coord),
    pub end_location: Option<(Coord, Coord)>,
    pub begin_time: TimeStamp,
    pub end_time: Option<TimeStamp>,
    pub begin_orientation: f32,
    pub end_orientation: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Animatable {
    pub id: Id,
    pub shape: Shape,
    pub fill: bool,
    pub color: (u8, u8, u8),
    pub path: Vec<PathSegment>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Message {
    Begin(TimeStamp),
    Show(Animatable),
    Update(Id, Vec<PathSegment>),
    Hide(Id),
}
