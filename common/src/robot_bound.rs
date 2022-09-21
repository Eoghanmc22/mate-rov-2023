use anyhow::Context;
use serde::{Serialize, Deserialize};
use crate::types::{Filter, Meters, Movement};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Packet {
    Arm, // Enable Motors
    Disarm, // Disable Motors
    MovementCommand(bool, Option<Movement>), // Updates motor speed targets: absolute, movement
    DepthPid(Option<Meters>), // Sets depth target: target depth (meters)
    SetFilter(Filter), // Set intents
    Ping(u128), // Used to measure latency: wall clock time of send
}

impl TryInto<Vec<u8>> for &Packet {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        Ok(bincode::serialize(self).context("Encode robot bound packet")?)
    }
}

impl TryFrom<&[u8]> for Packet {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(bincode::deserialize(bytes).context("Decode robot bound packet")?)
    }
}