use common::state::RobotStateUpdate;

pub enum Events {
    StateChanged(RobotStateUpdate),

    Connected(/* TODO */),
    ConnectionFailed(/* TODO */),
    Disconnected(/* TODO */),

}
