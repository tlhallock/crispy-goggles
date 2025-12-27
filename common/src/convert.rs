use crate::{grpc, model};

impl From<model::Shape> for grpc::Shape {
    fn from(s: model::Shape) -> Self {
        match s {
            model::Shape::Circle(r) => grpc::Shape {
                kind: Some(grpc::shape::Kind::Circle(grpc::Circle { radius: r })),
            },
            model::Shape::Rectangle(w, h) => grpc::Shape {
                kind: Some(grpc::shape::Kind::Rectangle(grpc::Rectangle { w, h })),
            },
        }
    }
}

impl From<model::PathSegment> for grpc::PathSegment {
    fn from(p: model::PathSegment) -> Self {
        let (bx, by) = p.begin_location;
        let (has_end_location, ex, ey) = match p.end_location {
            Some((x, y)) => (true, x, y),
            None => (false, 0.0, 0.0),
        };
        let (has_end_time, et) = match p.end_time {
            Some(t) => (true, t),
            None => (false, 0),
        };
        let (has_end_orientation, eo) = match p.end_orientation {
            Some(o) => (true, o),
            None => (false, 0.0),
        };

        grpc::PathSegment {
            begin_x: bx,
            begin_y: by,
            has_end_location,
            end_x: ex,
            end_y: ey,
            begin_time: p.begin_time,
            has_end_time,
            end_time: et,
            begin_orientation: p.begin_orientation,
            has_end_orientation,
            end_orientation: eo,
        }
    }
}

impl From<model::Animatable> for grpc::Animatable {
    fn from(a: model::Animatable) -> Self {
        grpc::Animatable {
            id: a.id,
            shape: Some(a.shape.into()),
            fill: a.fill,
            color: Some(grpc::Color {
                r: a.color.0 as u32,
                g: a.color.1 as u32,
                b: a.color.2 as u32,
            }),
            path: a.path.into_iter().map(Into::into).collect(),
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
                kind: Some(Kind::Show(grpc::Show {
                    anim: Some(anim.into()),
                })),
            },
            model::Message::Update(id, path) => grpc::Event {
                kind: Some(Kind::Update(grpc::Update {
                    id,
                    path: path.into_iter().map(Into::into).collect(),
                })),
            },
            model::Message::Hide(id) => grpc::Event {
                kind: Some(Kind::Hide(grpc::Hide { id })),
            },
        }
    }
}
