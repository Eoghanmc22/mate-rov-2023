use anyhow::Context;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Packet {
    // TODO
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