//! Repersents the protocol used for two way communication

use crate::types::LogLevel;
use anyhow::Context;
use bincode::{DefaultOptions, Options};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Representation of all messages that can be communicated between peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Protocol {
    /// Update peers data store
    Store(String, Option<Vec<u8>>),
    /// Requests that the peer sends the contents of its RobotState
    RequestSync,
    /// Logs a message on the peer's console
    Log(LogLevel, String),
    /// Asks the peer to reply with a Pong, used to measure communication latency
    Ping(SystemTime),
    /// Response to a Ping, used to measure communication latency
    Pong(SystemTime, SystemTime),
}

impl networking::Packet for Protocol {
    fn expected_size(&self) -> anyhow::Result<u64> {
        options()
            .serialized_size(self)
            .context("Could not compute expected size")
    }

    fn write_buf(self, buffer: &mut &mut [u8]) -> anyhow::Result<()> {
        options()
            .serialize_into(buffer, &self)
            .context("Could not serialize packet")
    }

    fn read_buf(buffer: &mut &[u8]) -> anyhow::Result<Self> {
        options()
            .deserialize_from(buffer)
            .context("Could not deserialize packet")
    }
}

fn options() -> impl Options {
    DefaultOptions::new()
}
