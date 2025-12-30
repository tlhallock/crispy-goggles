use serde::{Deserialize, Serialize};

// Make this an int as well
pub type Coord = f32;
pub type TimeStamp = u64;
pub type Id = u64;
pub type Orientation = f32;

// todo just do nanoseconds...
pub const TIME_PER_SECOND: u64 = 1_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Shape {
    Circle(Coord),
    Rectangle(Coord, Coord),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Point {
    pub x: Coord,
    pub y: Coord,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delta {
    pub dx: f64,
    pub dy: f64,
}

impl Delta {
    pub fn normalize(&self, radius: f64) -> Delta {
        // todo: can use the radius to scale differently...
        let len = (self.dx * self.dx + self.dy * self.dy).sqrt();
        if len < 1e-6 {
            Delta { dx: 0.0, dy: 0.0 }
        } else {
            Delta {
                dx: self.dx / len * radius,
                dy: self.dy / len * radius,
            }
        }
    }
}

// TODO...
pub struct OrientedPoint {
    pub point: Point,
    pub orientation: Orientation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathSegment {
    pub begin_time: TimeStamp,

    pub begin_location: Point,
    pub delta: Option<Delta>,

    // move orientation out...
    pub begin_orientation: Orientation,
    pub d_orientation: Option<Orientation>,
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

// impl Delta {
//     pub fn add_to(&self, opoint: OrientedPoint) -> OrientedPoint {
//         OrientedPoint {
//             point: Point {
//                 x: opoint.point.x + self.dx,
//                 y: opoint.point.y + self.dy,
//             },
//             orientation: opoint.orientation,
//         }
//     }
// }

// impl OrientedPoint {
//     pub fn add_delta(&self, delta: Option<Delta>) -> OrientedPoint {
//         if let Some(delta) = delta {
//             OrientedPoint {
//                 point: Point {
//                     x: self.point.x + delta.dx,
//                     y: self.point.y + delta.dy,
//                 },
//                 orientation: self.orientation,
//             }
//         } else {
//             self.clone()
//         }
//     }
