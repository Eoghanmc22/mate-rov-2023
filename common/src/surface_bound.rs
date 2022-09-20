use std::net::SocketAddr;
use anyhow::Context;
use serde::{Serialize, Deserialize};
use crate::types::{DepthFrame, InertialFrame, MotorFrame, Orientation, Role};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Packet {
    OrientationUpdate(Orientation),
    InertialUpdate(InertialFrame),
    DepthUpdate(DepthFrame),
    MotorUpdate(MotorFrame),
    AddCameras(Vec<SocketAddr>),
    RemoveCameras(Vec<SocketAddr>),
    Log(String),
    Role(Role),
    Armed(bool),
    Pong(u128, u128),
}

impl TryInto<Vec<u8>> for &Packet {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        Ok(bincode::serialize(self).context("Encode surface bound packet")?)
    }
}

impl TryFrom<&[u8]> for Packet {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(bincode::deserialize(bytes).context("Decode surface bound packet")?)
    }
}