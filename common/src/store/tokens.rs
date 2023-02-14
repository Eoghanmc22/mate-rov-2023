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

pub const SYSTEM_INFO: Token<SystemInfo> = Token::new("robot.system_info");

pub const CAMERAS: Token<Vec<Camera>> = Token::new("robot.cameras");

pub const ARMED: Token<(Armed, Instant)> = Token::new("robot.motors.armed");
pub const MOTOR_SPEED: Token<(HashMap<MotorId, MotorFrame>, Instant)> =
    Token::new("robot.motors.speed");

pub const MOVEMENT_JOYSTICK: Token<(Movement, Instant)> = Token::new("robot.movement.joystick");
pub const MOVEMENT_OPENCV: Token<(Movement, Instant)> = Token::new("robot.movement.ai");
pub const MOVEMENT_DEPTH: Token<(Movement, Instant)> = Token::new("robot.movement.depth");
pub const MOVEMENT_CALCULATED: Token<(Movement, Instant)> = Token::new("robot.movement.calculated");

pub const RAW_DEPTH: Token<(DepthFrame, Instant)> = Token::new("robot.sensors.depth");
pub const RAW_INERTIAL: Token<(InertialFrame, Instant)> = Token::new("robot.sensors.inertial");
pub const RAW_MAGNETIC: Token<(MagFrame, Instant)> = Token::new("robot.sensors.mag");
pub const ORIENTATION: Token<Orientation> = Token::new("robot.sensors.fusion");

pub const DEPTH_TARGET: Token<(Meters, Instant)> = Token::new("robot.ai.depth_target");

pub fn generate_adaptors() -> HashMap<Key, Box<dyn TypeAdapter<BackingType>>> {
    fn from<A: TypedAdapter<BackingType> + Default + 'static>(
        token: Token<A::Data>,
    ) -> (Key, Box<dyn TypeAdapter<BackingType>>) {
        (token.0, Box::new(A::default()))
    }

    vec![
        from::<Adapter<_>>(SYSTEM_INFO),
        from::<Adapter<_>>(CAMERAS),
        from::<TimestampedAdapter<_>>(ARMED),
        from::<TimestampedAdapter<_>>(MOVEMENT_JOYSTICK),
        from::<TimestampedAdapter<_>>(MOVEMENT_OPENCV),
        from::<TimestampedAdapter<_>>(MOVEMENT_DEPTH),
        from::<TimestampedAdapter<_>>(MOVEMENT_CALCULATED),
        from::<TimestampedAdapter<_>>(RAW_DEPTH),
        from::<TimestampedAdapter<_>>(RAW_INERTIAL),
        from::<TimestampedAdapter<_>>(RAW_MAGNETIC),
        from::<Adapter<_>>(ORIENTATION),
        from::<TimestampedAdapter<_>>(DEPTH_TARGET),
    ]
    .into_iter()
    .collect()
}
