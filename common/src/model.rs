use serde::{Deserialize, Serialize};

// Make this an int as well
pub type Coord = f32;
pub type TimeStamp = u64;
pub type Id = u64;
pub type Orientation = f32;
pub type UnitId = u64;

pub type ResourceId = u64;
pub type PlayerId = u64;

pub const METERS: Coord = 1_000 as Coord;
pub const SECONDS: TimeStamp = 1_000_000 as TimeStamp;

// todo just do nanoseconds...
pub const TIME_PER_SECOND: u64 = 1_000;

pub enum Task {
	MoveTo(Point),
	Transfer(Transfer),
}

pub struct Tasks {
	pub tasks: Vec<(u64, Task)>,
}

pub struct Transfer {
	pub resource_id: ResourceId,
	pub amount: i32,

	pub source_id: Id,
	pub destination_id: Id,
}

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

// TODO...
pub struct OrientedPoint {
	pub point: Point,
	pub orientation: Orientation,
}

// this should be any task
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

impl Default for Tasks {
	fn default() -> Self {
		Self { tasks: Vec::new() }
	}
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

pub enum PositionedShape {
	Circle(CenteredCircle),
	Rectangle(Rec),
}

impl From<(&Shape, &Point)> for PositionedShape {
	fn from(t: (&Shape, &Point)) -> Self {
		match t.0 {
			Shape::Circle(r) => PositionedShape::Circle(CenteredCircle {
				center: Point { x: t.1.x, y: t.1.y },
				radius: *r,
			}),
			Shape::Rectangle(w, h) => PositionedShape::Rectangle(Rec {
				min: Point {
					x: t.1.x - *w / (2.0 as Coord),
					y: t.1.y - *h / (2.0 as Coord),
				},
				max: Point {
					x: t.1.x + *w / (2.0 as Coord),
					y: t.1.y + *h / (2.0 as Coord),
				},
			}),
		}
	}
}

impl PositionedShape {
	pub fn intersects(&self, other: &PositionedShape) -> bool {
		match self {
			PositionedShape::Circle(circ) => match other {
				PositionedShape::Circle(c) => circ.intersects_circle(c),
				PositionedShape::Rectangle(r) => r.intersects_circle(circ),
			},
			PositionedShape::Rectangle(r) => match other {
				PositionedShape::Circle(c) => r.intersects_circle(c),
				PositionedShape::Rectangle(rect) => {
					r.intersects_rectangle(rect)
				}
			},
		}
	}

	pub fn contains_point(&self, point: &Point) -> bool {
		match self {
			PositionedShape::Circle(circ) => circ.contains_point(point),
			PositionedShape::Rectangle(rec) => rec.contains_point(point),
		}
	}
}

pub struct Rec {
	pub min: Point,
	pub max: Point,
}

impl From<(Point, Point)> for Rec {
	fn from(t: (Point, Point)) -> Self {
		Self {
			min: Point {
				x: t.0.x.min(t.1.x),
				y: t.0.y.min(t.1.y),
			},
			max: Point {
				x: t.0.x.max(t.1.x),
				y: t.0.y.max(t.1.y),
			},
		}
	}
}

impl Rec {
	pub fn contains_point(&self, point: &Point) -> bool {
		point.x >= self.min.x
			&& point.x <= self.max.x
			&& point.y >= self.min.y
			&& point.y <= self.max.y
	}

	pub fn intersects_rectangle(&self, other: &Rec) -> bool {
		!(self.max.x < other.min.x
			|| self.min.x > other.max.x
			|| self.max.y < other.min.y
			|| self.min.y > other.max.y)
	}

	pub fn intersects_circle(&self, other: &PositionedCircle) -> bool {
		let closest_x = other.center.x.clamp(self.min.x, self.max.x);
		let closest_y = other.center.y.clamp(self.min.y, self.max.y);

		let distance_x = other.center.x - closest_x;
		let distance_y = other.center.y - closest_y;

		let distance_squared =
			distance_x * distance_x + distance_y * distance_y;
		distance_squared < (other.radius * other.radius)
	}

	pub fn width(&self) -> Coord {
		self.max.x - self.min.x
	}
	pub fn height(&self) -> Coord {
		self.max.y - self.min.y
	}
	pub fn center(&self) -> Point {
		Point {
			x: (self.min.x + self.max.x) / 2.0,
			y: (self.min.y + self.max.y) / 2.0,
		}
	}
}

pub struct CenteredCircle {
	pub center: Point,
	pub radius: Coord,
}

impl From<(Point, Coord)> for CenteredCircle {
	fn from(t: (Point, Coord)) -> Self {
		Self {
			center: t.0,
			radius: t.1,
		}
	}
}

impl CenteredCircle {
	pub fn contains_point(&self, point: &Point) -> bool {
		let dx = point.x - self.center.x;
		let dy = point.y - self.center.y;
		(dx * dx + dy * dy) < (self.radius * self.radius)
	}

	pub fn intersects_circle(&self, other: &CenteredCircle) -> bool {
		let dx = other.center.x - self.center.x;
		let dy = other.center.y - self.center.y;
		let distance_squared = dx * dx + dy * dy;
		let radius_sum = self.radius + other.radius;
		distance_squared < (radius_sum * radius_sum)
	}
}
