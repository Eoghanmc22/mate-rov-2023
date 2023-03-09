use crate::{
    store::adapters::{Adapter, BackingType, TimestampedAdapter, TypeAdapter, TypedAdapter},
    store::{Key, Token},
    types::{
        Armed, Camera, DepthFrame, InertialFrame, MagFrame, Meters, MotorFrame, MotorId, Movement,
        Orientation, SystemInfo,
    },
};
use fxhash::FxHashMap as HashMap;
use std::time::Instant;

#[rustfmt::skip]
pub const SYSTEM_INFO: Token<SystemInfo> = Token::new_const("robot.system_info");

#[rustfmt::skip]
pub const CAMERAS: Token<Vec<Camera>> = Token::new_const("robot.cameras");

#[rustfmt::skip]
pub const ARMED: Token<(Armed, Instant)> = Token::new_const("robot.motors.armed");
#[rustfmt::skip]
pub const MOTOR_SPEED: Token<(HashMap<MotorId, MotorFrame>, Instant)> = Token::new_const("robot.motors.speed");

#[rustfmt::skip]
pub const MOVEMENT_JOYSTICK: Token<(Movement, Instant)> = Token::new_const("robot.movement.joystick");
#[rustfmt::skip]
pub const MOVEMENT_OPENCV: Token<(Movement, Instant)> = Token::new_const("robot.movement.ai");
#[rustfmt::skip]
pub const MOVEMENT_DEPTH: Token<(Movement, Instant)> = Token::new_const("robot.movement.depth");
#[rustfmt::skip]
pub const MOVEMENT_CALCULATED: Token<(Movement, Instant)> = Token::new_const("robot.movement.calculated");

#[rustfmt::skip]
pub const RAW_DEPTH: Token<(DepthFrame, Instant)> = Token::new_const("robot.sensors.depth");
#[rustfmt::skip]
pub const RAW_INERTIAL: Token<(InertialFrame, Instant)> = Token::new_const("robot.sensors.inertial");
#[rustfmt::skip]
pub const RAW_MAGNETIC: Token<(MagFrame, Instant)> = Token::new_const("robot.sensors.mag");
#[rustfmt::skip]
pub const ORIENTATION: Token<(Orientation, Instant)> = Token::new_const("robot.sensors.fusion");

#[rustfmt::skip]
pub const DEPTH_TARGET: Token<(Meters, Instant)> = Token::new_const("robot.ai.depth_target");

pub fn generate_adaptors() -> HashMap<Key, Box<dyn TypeAdapter<BackingType> + Send + Sync>> {
    fn from<A: TypedAdapter<BackingType> + Default + Send + Sync + 'static>(
        token: Token<A::Data>,
    ) -> (Key, Box<dyn TypeAdapter<BackingType> + Send + Sync>) {
        (token.0, Box::new(A::default()))
    }

    vec![
        from::<Adapter<_>>(SYSTEM_INFO),
        from::<Adapter<_>>(CAMERAS),
        from::<TimestampedAdapter<_>>(ARMED),
        from::<TimestampedAdapter<_>>(MOTOR_SPEED),
        from::<TimestampedAdapter<_>>(MOVEMENT_JOYSTICK),
        from::<TimestampedAdapter<_>>(MOVEMENT_OPENCV),
        from::<TimestampedAdapter<_>>(MOVEMENT_DEPTH),
        from::<TimestampedAdapter<_>>(MOVEMENT_CALCULATED),
        from::<TimestampedAdapter<_>>(RAW_DEPTH),
        from::<TimestampedAdapter<_>>(RAW_INERTIAL),
        from::<TimestampedAdapter<_>>(RAW_MAGNETIC),
        from::<TimestampedAdapter<_>>(ORIENTATION),
        from::<TimestampedAdapter<_>>(DEPTH_TARGET),
    ]
    .into_iter()
    .collect()
}
