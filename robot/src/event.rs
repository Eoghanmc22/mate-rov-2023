use common::{protocol::Protocol, state::RobotStateUpdate};

#[derive(Debug)]
pub enum Event {
    PacketSend(Protocol),

    StateUpdate(Vec<RobotStateUpdate>),
    StateRefresh,

    Error(anyhow::Error),
}
