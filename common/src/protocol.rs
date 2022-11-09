use crate::state::RobotStateUpdate;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Representation of all messages that can be communicated between peers
// TODO use references
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Packet {
    /// Encodes the updates that should be made to the peer's RobotState
    StateUpdate(Vec<RobotStateUpdate>),
    /// Requests that the peer sends the contents of its RobotState
    RequestSync,
    /// Logs a message on the peer's console
    Log(String),
    /// Asks the peer to reply with a Pong, used to measure communication latency
    Ping(SystemTime),
    /// Response to a Ping, used to measure communication latency
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
