use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum ServerMessage {
    Init { id: u32, width: u32, height: u32 },
    Move { id: u32, position: Position },
    Leave { id: u32 },
    Shoot { id: u32 },
}

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Move { position: Position },
    Shoot,
    Die,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
}
