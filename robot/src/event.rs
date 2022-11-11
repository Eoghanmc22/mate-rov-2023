use common::{protocol::Packet, state::RobotStateUpdate};

#[derive(Debug, Clone)]
pub enum Event {
    PacketSend(Packet),

    StateUpdate(Vec<RobotStateUpdate>),
    StateRefresh,
}
