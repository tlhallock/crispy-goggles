use crate::grpc;
use serde::{Deserialize, Serialize};

// Make this an int as well
pub type Coord = f32;
pub type TimeStamp = u64;
pub type Id = u64;
pub type Orientation = f32;
pub type UnitId = u64;
pub type TaskId = u64;
pub type Speed = f32;
pub type TemplateId = u32;

pub type ResourceId = u64;
pub type PlayerId = u64;

pub const METERS: Coord = 1_000 as Coord;
pub const SECONDS: TimeStamp = 1_000_000 as TimeStamp;

// todo just do nanoseconds...
// pub const TIME_PER_SECOND: u64 = 1_000;

#[derive(Clone, Debug, Serialize, Deserialize, Copy)]
pub enum UnitDisplayType {
	SimpleUnit = 1,
}

pub struct Health {
	pub current: i32,
	pub max: i32,
}

#[derive(Debug, Clone)]
pub enum Task {
	MoveTo(Point),
	Transfer(Transfer),
}

pub struct Tasks {
	pub tasks: Vec<(u64, Task)>,
}

#[derive(Debug, Clone)]
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
	pub dx: Coord,
	pub dy: Coord,
}

// TODO...  too many structs for the same thing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrientedPoint {
	pub point: Point,
	pub orientation: Orientation,
}

// this should be any task
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationSegment {
	pub begin_time: TimeStamp,

	pub begin_location: Point,
	pub delta: Option<Delta>,

	// move orientation out...
	pub begin_orientation: Orientation,
	pub d_orientation: Option<Orientation>,
	// add a progress here?
}

impl AnimationSegment {
	pub fn place_at(&self, time: TimeStamp) -> OrientedPoint {
		let d_t = time.saturating_sub(self.begin_time) as f64;
		let dx = self
			.delta
			.as_ref()
			.map_or(0.0, |d| d.dx as f64 * (d_t as f64));
		let dy = self
			.delta
			.as_ref()
			.map_or(0.0, |d| d.dy as f64 * (d_t as f64));
		OrientedPoint {
			point: Point {
				x: self.begin_location.x + dx as Coord,
				y: self.begin_location.y + dy as Coord,
			},
			orientation: self.begin_orientation
				+ self.d_orientation.unwrap_or(0.0),
		}
	}
}

#[derive(Clone, Debug)]
pub struct Animatable {
	pub unit_id: UnitId,
	pub display_type: UnitDisplayType,
	pub queue: Vec<AnimationSegment>,
	// pub queue: Vec<grpc::AnimationSegment>,
	// the location for stationary objects...
}

#[derive(Clone, Debug)]
pub enum Message {
	Begin(TimeStamp),
	Show(Animatable),
	Update(Id, Vec<AnimationSegment>),
	Hide(Id),
}

impl Delta {
	pub fn between(from: &Point, to: &Point) -> Self {
		Self {
			dx: (to.x - from.x) as Coord,
			dy: (to.y - from.y) as Coord,
		}
	}
	pub fn normalize(&self, radius: Coord) -> Delta {
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

impl Point {
	pub fn distance_to(&self, other: &Point) -> f64 {
		let dx = self.x - other.x;
		let dy = self.y - other.y;
		(dx * dx + dy * dy).sqrt() as f64
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

impl From<(&grpc::Shape, &grpc::Point)> for PositionedShape {
	fn from(t: (&grpc::Shape, &grpc::Point)) -> Self {
		(&Shape::from(t.0), &Point::from(t.1)).into()
	}
}

impl From<(&&grpc::Shape, &&grpc::Point)> for PositionedShape {
	fn from(t: (&&grpc::Shape, &&grpc::Point)) -> Self {
		(&Shape::from(t.0), &Point::from(t.1)).into()
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

	pub fn center(&self) -> Point {
		match self {
			PositionedShape::Circle(circ) => circ.center.clone(),
			PositionedShape::Rectangle(rec) => rec.center(),
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

	pub fn intersects_circle(&self, other: &CenteredCircle) -> bool {
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

pub struct ShapeVolume {
	// add orientation
	shape: Shape,
	begin: (TimeStamp, Point),
	end: (TimeStamp, Point),
}

impl ShapeVolume {
	pub fn moved_rect_intersects_rect(
		t1: TimeStamp,
		r1: &Rec,
		t2: TimeStamp,
		r2: &Rec,
	) -> bool {
		/* t, x, y
		 such that:

		 a = (t - t1) / (t2 - t1)

		x >= self.min.x && point.x <= self.max.x && point.y >= self.min.y && point.y <= self.max.y
		*/
		false
	}

	pub fn intersects(&self, other: &ShapeVolume) -> bool {
		false
	}
}

// https://chatgpt.com/c/69560574-5948-832f-9839-40b252a6e799
// fn axis_interval(
// 	t1: f64,
// 	t2: f64,
// 	w1: f64,
// 	w2: f64,
// 	d0: f64,
// 	d1: f64,
// ) -> Option<(f64, f64)> {
// 	let dt = t2 - t1;
// 	let l = -w2;
// 	let u = w1;

// 	let v = (d1 - d0) / dt;

// 	if v == 0.0 {
// 		if d0 >= l && d0 <= u {
// 			return Some((t1, t2));
// 		} else {
// 			return None;
// 		}
// 	}

// 	let ta = t1 + (l - d0) / v;
// 	let tb = t1 + (u - d0) / v;

// 	let enter = ta.min(tb);
// 	let exit = ta.max(tb);

// 	let start = t1.max(enter);
// 	let end = t2.min(exit);

// 	if start <= end {
// 		Some((start, end))
// 	} else {
// 		None
// 	}
// }

// fn rects_intersect_sometime(
// 	t1: f64,
// 	t2: f64,
// 	r1: Rect,
// 	r2: Rect,
// 	r1t2: Rect,
// 	r2t2: Rect,
// ) -> bool {
// 	let w1x = r1.max.x - r1.min.x;
// 	let w1y = r1.max.y - r1.min.y;
// 	let w2x = r2.max.x - r2.min.x;
// 	let w2y = r2.max.y - r2.min.y;

// 	let dx0 = r2.min.x - r1.min.x;
// 	let dx1 = r2t2.min.x - r1t2.min.x;
// 	let dy0 = r2.min.y - r1.min.y;
// 	let dy1 = r2t2.min.y - r1t2.min.y;

// 	let ix = axis_interval(t1, t2, w1x, w2x, dx0, dx1);
// 	let iy = axis_interval(t1, t2, w1y, w2y, dy0, dy1);

// 	match (ix, iy) {
// 		(Some((sx, ex)), Some((sy, ey))) => (sx.max(sy) <= ex.min(ey)),
// 		_ => false,
// 	}
// }

// #[derive(Clone, Copy)]
// struct Point { x: f64, y: f64 }
// #[derive(Clone, Copy)]
// struct Rect { min: Point, max: Point }

// fn dot(a: Point, b: Point) -> f64 { a.x*b.x + a.y*b.y }
// fn sub(a: Point, b: Point) -> Point { Point { x: a.x-b.x, y: a.y-b.y } }
// fn add(a: Point, b: Point) -> Point { Point { x: a.x+b.x, y: a.y+b.y } }
// fn mul(a: Point, k: f64) -> Point { Point { x: a.x*k, y: a.y*k } }

// fn quad_has_solution_in_interval(a: f64, b: f64, c: f64, s0: f64, s1: f64) -> bool {
//     // Check if a*s^2 + b*s + c <= 0 has any s in [s0,s1]
//     if a == 0.0 {
//         if b == 0.0 { return c <= 0.0; }
//         // linear: b*s + c <= 0  -> s <= -c/b (if b>0) else s >= -c/b
//         let r = -c / b;
//         return if b > 0.0 { s0 <= r } else { r <= s1 };
//     }
//     let d = b*b - 4.0*a*c;
//     if d < 0.0 {
//         // No real roots: inequality holds everywhere or nowhere depending on value at a point
//         let mid = 0.5*(s0+s1);
//         return a*mid*mid + b*mid + c <= 0.0;
//     }
//     let sqrt_d = d.sqrt();
//     let r1 = (-b - sqrt_d) / (2.0*a);
//     let r2 = (-b + sqrt_d) / (2.0*a);
//     let lo = r1.min(r2);
//     let hi = r1.max(r2);

//     if a > 0.0 {
//         // <=0 between roots
//         let seg_lo = lo.max(s0);
//         let seg_hi = hi.min(s1);
//         seg_lo <= seg_hi
//     } else {
//         // <=0 outside roots
//         // true if [s0,s1] intersects (-inf,lo] or [hi,inf)
//         s0 <= lo || hi <= s1
//     }
// }

// fn circles_intersect_sometime(
//     t1: f64, t2: f64,
//     a0: Point, a1: Point, ra: f64,
//     b0: Point, b1: Point, rb: f64,
// ) -> bool {
//     let dt = t2 - t1;
//     let d0 = sub(b0, a0);
//     let va = mul(sub(a1, a0), 1.0/dt);
//     let vb = mul(sub(b1, b0), 1.0/dt);
//     let v  = sub(vb, va);

//     let r = ra + rb;
//     let a = dot(v, v);
//     let b = 2.0 * dot(d0, v);
//     let c = dot(d0, d0) - r*r;

//     quad_has_solution_in_interval(a, b, c, 0.0, dt)
// }

// fn rect_circle_intersect_sometime(
//     t1: f64, t2: f64,
//     rect0: Rect, rect1: Rect,
//     c0: Point, c1: Point, r: f64,
// ) -> bool {
//     let dt = t2 - t1;

//     let w = rect0.max.x - rect0.min.x;
//     let h = rect0.max.y - rect0.min.y;

//     // rect frame: rect is [0,w]x[0,h]; circle center is q(s)=q0+u*s
//     let q0 = sub(c0, rect0.min);
//     let q1 = sub(c1, rect1.min);
//     let u  = mul(sub(q1, q0), 1.0/dt);

//     // breakpoints where qx hits 0 or w, qy hits 0 or h
//     let mut cuts = vec![0.0, dt];
//     if u.x != 0.0 {
//         for &bnd in &[0.0, w] {
//             let s = (bnd - q0.x) / u.x;
//             if 0.0 < s && s < dt { cuts.push(s); }
//         }
//     }
//     if u.y != 0.0 {
//         for &bnd in &[0.0, h] {
//             let s = (bnd - q0.y) / u.y;
//             if 0.0 < s && s < dt { cuts.push(s); }
//         }
//     }
//     cuts.sort_by(|a,b| a.partial_cmp(b).unwrap());
//     cuts.dedup_by(|a,b| (*a-*b).abs() < 1e-12);

//     // check each piece: dx,dy are linear (or 0) => quadratic inequality
//     for win in cuts.windows(2) {
//         let s0 = win[0];
//         let s1 = win[1];
//         let sm = 0.5*(s0+s1);
//         let qm = add(q0, mul(u, sm));

//         // dx(s) = ax*s + bx (or 0) depending on region in this interval
//         let (ax, bx) = if qm.x < 0.0 {
//             // dx = -qx = -(q0x + ux*s)
//             (-u.x, -q0.x)
//         } else if qm.x > w {
//             // dx = qx - w
//             (u.x, q0.x - w)
//         } else {
//             (0.0, 0.0)
//         };

//         let (ay, by) = if qm.y < 0.0 {
//             (-u.y, -q0.y)
//         } else if qm.y > h {
//             (u.y, q0.y - h)
//         } else {
//             (0.0, 0.0)
//         };

//         // (ax*s+bx)^2 + (ay*s+by)^2 - r^2 <= 0
//         let a = ax*ax + ay*ay;
//         let b = 2.0*(ax*bx + ay*by);
//         let c = (bx*bx + by*by) - r*r;

//         if quad_has_solution_in_interval(a, b, c, s0, s1) {
//             return true;
//         }
//     }
//     false
// }

impl UnitDisplayType {
	pub fn get_color(&self) -> (u8, u8, u8) {
		match self {
			UnitDisplayType::SimpleUnit => (0, 128, 255), // Example color
		}
	}

	pub fn get_shape(&self) -> Shape {
		match self {
			UnitDisplayType::SimpleUnit => Shape::Circle(0.5), // Example shape
		}
	}

	pub fn get_fill(&self) -> bool {
		match self {
			UnitDisplayType::SimpleUnit => true,
		}
	}

	// pub fn get_int(self) -> u32 {
	// 	match self {
	// 		UnitDisplayType::SimpleUnit => UnitDisplayType::SimpleUnit as u32,
	// 	}
	// }

	pub fn parse(
		value: u32,
	) -> Result<UnitDisplayType, crate::convert::ParseError> {
		match value {
			1 => Ok(UnitDisplayType::SimpleUnit),
			_ => {
				Err(crate::convert::ParseError::InvalidValue("UnitDisplayType"))
			}
		}
	}
}
