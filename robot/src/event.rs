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

    SensorFrame(SensorFrame),

    Error(anyhow::Error),
    Exit,
}

#[derive(Debug, Copy, Clone)]
pub enum SensorFrame {
    Imu(InertialFrame),
    Mag(MagFrame),
}
