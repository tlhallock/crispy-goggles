use common::model::{Animatable, Message, Shape};
use common::model::{Coord, TIME_PER_SECOND, TimeStamp};

pub struct GameState {
    pub next_id: u64,
    pub begin_time: TimeStamp,

    pub last_time: TimeStamp,
    pub last_wall_ms: TimeStamp,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            begin_time: 0,
            last_time: 0,
            last_wall_ms: 0,
        }
    }
}
