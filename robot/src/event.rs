use common::{protocol::Protocol, store::Update};

#[derive(Debug)]
pub enum Event {
    PacketTx(Protocol),
    PacketRx(Protocol),

    Store(Update),
    SyncStore,

    Error(anyhow::Error),
}
