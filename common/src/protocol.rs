use std::time::SystemTime;
use anyhow::Context;
use serde::{Serialize, Deserialize};
use crate::state::RobotStateUpdate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Packet {
    StateUpdate(Vec<RobotStateUpdate>),
    RequestSync,
    Log(String),
    Ping(SystemTime),
    Pong(SystemTime, SystemTime),
}

impl TryInto<Vec<u8>> for &Packet {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        bincode::serialize(self).context("Encode packet")
    }
}

impl TryFrom<&[u8]> for Packet {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bincode::deserialize(bytes).context("Decode packet")
    }
}