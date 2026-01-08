use crate::{
	grpc::{self},
	model,
};

#[derive(Debug)]
pub enum ParseError {
	MissingRequiredField(&'static str),
	InvalidValue(&'static str),
}

impl std::fmt::Display for ParseError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ParseError::MissingRequiredField(field) => {
				write!(f, "Missing required field: {}", field)
			}
			ParseError::InvalidValue(field) => {
				write!(f, "Invalid value for field: {}", field)
			}
		}
	}
}

impl From<model::Shape> for grpc::Shape {
	fn from(s: model::Shape) -> Self {
		match s {
			model::Shape::Circle(r) => grpc::Shape {
				kind: Some(grpc::shape::Kind::Circle(grpc::Circle {
					radius: r,
				})),
			},
			model::Shape::Rectangle(w, h) => grpc::Shape {
				kind: Some(grpc::shape::Kind::Rectangle(grpc::Rectangle {
					w,
					h,
				})),
			},
		}
	}
}

impl From<model::Point> for grpc::Point {
	fn from(p: model::Point) -> Self {
		grpc::Point { x: p.x, y: p.y }
	}
}

impl From<model::Delta> for grpc::Delta {
	fn from(d: model::Delta) -> Self {
		grpc::Delta {
			dx: d.dx as f64,
			dy: d.dy as f64,
		}
	}
}

impl From<model::AnimationSegment> for grpc::AnimationSegment {
	fn from(p: model::AnimationSegment) -> Self {
		grpc::AnimationSegment {
			begin_location: Some(p.begin_location.into()),
			delta: p.delta.map(Into::into),
			begin_time: p.begin_time,
			begin_orientation: p.begin_orientation,
			d_orientation: p.d_orientation,
		}
	}
}

impl From<model::Animatable> for grpc::Animatable {
	fn from(a: model::Animatable) -> Self {
		grpc::Animatable {
			unit_id: a.unit_id,
			display_type: a.display_type as u32,
			// shape: Some(a.shape.into()),
			// fill: a.fill,
			// color: Some(grpc::Color {
			// 	r: a.color.0 as u32,
			// 	g: a.color.1 as u32,
			// 	b: a.color.2 as u32,
			// }),
			queue: a.queue.into_iter().map(Into::into).collect(),
		}
	}
}

impl From<model::Animatable> for grpc::Show {
	fn from(anim: model::Animatable) -> Self {
		// let location =
		// 	anim.path.first().map(|p| p.begin_location.clone().into());
		grpc::Show {
			unit_id: anim.unit_id,
			anim: Some(anim.into()),
			details: None,
			// todo fill in..
		}
	}
}

impl From<model::Message> for grpc::Event {
	fn from(m: model::Message) -> Self {
		use grpc::event::Kind;

		match m {
			model::Message::Begin(ts) => grpc::Event {
				kind: Some(Kind::Begin(grpc::Begin { timestamp: ts })),
			},
			model::Message::Show(anim) => grpc::Event {
				kind: Some(Kind::Show(anim.into())),
			},
			model::Message::Update(id, path) => grpc::Event {
				kind: Some(Kind::Update(grpc::Update {
					unit_id: id,
					queue: path.into_iter().map(Into::into).collect(),
					details: None,
				})),
			},
			model::Message::Hide(id) => grpc::Event {
				kind: Some(Kind::Hide(grpc::Hide { id })),
			},
		}
	}
}

// Result<model::Task, ParseError>
impl From<grpc::AnimationSegment>
	for Result<model::AnimationSegment, ParseError>
{
	fn from(p: grpc::AnimationSegment) -> Self {
		Ok(model::AnimationSegment {
			begin_location: p
				.begin_location
				.map(|b| model::Point::from(b))
				.ok_or_else(|| {
					ParseError::MissingRequiredField(
						"AnimationSegment.begin_location",
					)
				})?,
			delta: p.delta.map(|d| model::Delta {
				dx: d.dx as f32,
				dy: d.dy as f32,
			}),
			begin_time: p.begin_time,
			begin_orientation: p.begin_orientation,
			d_orientation: p.d_orientation,
		})
	}
}

pub fn parse_animation_segment(
	p: &grpc::AnimationSegment,
) -> Result<model::AnimationSegment, ParseError> {
	Ok(model::AnimationSegment {
		begin_location: p
			.begin_location
			.as_ref()
			.map(|b| model::Point::from(b))
			.ok_or_else(|| {
				ParseError::MissingRequiredField(
					"AnimationSegment.begin_location",
				)
			})?,
		delta: p.delta.as_ref().map(
			// Into::into
			|d| model::Delta {
				dx: d.dx as f32,
				dy: d.dy as f32,
			},
		),
		begin_time: p.begin_time,
		begin_orientation: p.begin_orientation,
		d_orientation: p.d_orientation,
	})
}

// impl From<model::WarningContent> for grpc::Warning {
// 	fn from(w: model::WarningContent) -> Self {
// 		grpc::Warning { message: w.message }
// 	}
// }

impl From<grpc::Point> for model::Point {
	fn from(t: grpc::Point) -> Self {
		model::Point { x: t.x, y: t.y }
	}
}

impl From<&grpc::Task> for Result<model::Task, ParseError> {
	fn from(t: &grpc::Task) -> Self {
		match t
			.kind
			.ok_or_else(|| ParseError::MissingRequiredField("Task.kind"))
			.map(|k| k)?
		{
			grpc::task::Kind::Move(m) => Ok(model::Task::MoveTo(
				m.destination
					.ok_or_else(|| {
						ParseError::MissingRequiredField(
							"Task.Move.destination",
						)
					})?
					.into(),
			)),
			grpc::task::Kind::Transfer(_) => {
				Err(ParseError::MissingRequiredField(
					"Task.Transfer not implemented",
				))
			}
		}
	}
}

// todo how to clean up these double definitions?

impl From<&grpc::Point> for model::Point {
	fn from(p: &grpc::Point) -> Self {
		model::Point { x: p.x, y: p.y }
	}
}
impl From<&&grpc::Point> for model::Point {
	fn from(p: &&grpc::Point) -> Self {
		model::Point { x: p.x, y: p.y }
	}
}

impl From<&grpc::Shape> for model::Shape {
	fn from(s: &grpc::Shape) -> Self {
		match &s.kind {
			Some(grpc::shape::Kind::Circle(c)) => {
				model::Shape::Circle(c.radius)
			}
			Some(grpc::shape::Kind::Rectangle(r)) => {
				model::Shape::Rectangle(r.w, r.h)
			}
			_ => panic!("Unknown shape kind"),
		}
	}
}

impl From<&&grpc::Shape> for model::Shape {
	fn from(s: &&grpc::Shape) -> Self {
		match &s.kind {
			Some(grpc::shape::Kind::Circle(c)) => {
				model::Shape::Circle(c.radius)
			}
			Some(grpc::shape::Kind::Rectangle(r)) => {
				model::Shape::Rectangle(r.w, r.h)
			}
			_ => panic!("Unknown shape kind"),
		}
	}
}

// impl From<model::Task> for grpc::PathSegment {
// 	fn from(t: model::Task) -> Self {
// 		match t {
// 			model::Task::MoveTo(dest) => grpc::PathSegment {
// 				begin_location: Some(dest.into()),
// 				// Uh oh...
// 				delta: None,
// 				begin_time: 0,
// 				begin_orientation: 0.0,
// 				d_orientation: None,
// 			},
// 			model::Task::Transfer(_) => {
// 				panic!("Task::Transfer not implemented")
// 			}
// 		}
// 	}
// }
