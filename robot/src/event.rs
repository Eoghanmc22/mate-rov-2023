use std::net::SocketAddr;

use common::{protocol::Protocol, store::Update};

#[derive(Debug)]
pub enum Event {
    PeerConnected(SocketAddr),

    PacketTx(Protocol),
    PacketRx(Protocol),

    Store(Update),
    SyncStore,

    Error(anyhow::Error),
}
