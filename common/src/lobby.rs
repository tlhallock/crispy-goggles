use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub id: u64,
    pub name: String,
    pub ready: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lobby {
    pub players: Vec<Player>,
}
