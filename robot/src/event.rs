use std::net::SocketAddr;

use common::{
    protocol::Protocol,
    store::Update,
    types::{InertialFrame, MagFrame},
};

/// Repersents a message a system can brodcast
#[derive(Debug)]
pub enum Event {
    PeerConnected(SocketAddr),
    PeerDisconnected(Option<SocketAddr>),

    PacketTx(Protocol),
    PacketRx(Protocol),

    Store(Update),
    SyncStore,
    ResetForignStore,

    SensorFrame(SensorBatch),

    Error(anyhow::Error),
    Exit,
}

// Repersents 20ms of sensor data
#[derive(Debug, Copy, Clone)]
pub struct SensorBatch {
    pub inertial: [InertialFrame; 20],
    pub mag: [MagFrame; 2],
}
