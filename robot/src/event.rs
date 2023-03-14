use std::net::SocketAddr;

use common::{protocol::Protocol, store::Update};

/// Repersents a message a system can brodcast
#[derive(Debug)]
pub enum Event {
    PeerConnected(SocketAddr),

    PacketTx(Protocol),
    PacketRx(Protocol),

    Store(Update),
    SyncStore,

    Error(anyhow::Error),
}
