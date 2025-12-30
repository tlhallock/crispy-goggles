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

impl From<model::Point> for grpc::Point {
    fn from(p: model::Point) -> Self {
        grpc::Point { x: p.x, y: p.y }
    }
}

impl From<model::Delta> for grpc::Delta {
    fn from(d: model::Delta) -> Self {
        grpc::Delta { dx: d.dx, dy: d.dy }
    }
}

impl From<model::PathSegment> for grpc::PathSegment {
    fn from(p: model::PathSegment) -> Self {
        grpc::PathSegment {
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
