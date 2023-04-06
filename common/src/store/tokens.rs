//! Definitions of everything that can be stored in the global store

use crate::{
    store::adapters::{Adapter, BackingType, TypeAdapter},
    store::{Key, Token},
    types::{
        Armed, Camera, DepthFrame, InertialFrame, MagFrame, Meters, MotorFrame, MotorId, Movement,
        Orientation, RobotStatus, SystemInfo,
    },
};
use fxhash::FxHashMap as HashMap;
use serde::{Deserialize, Serialize};

// Adaptor Definitions

#[rustfmt::skip]
pub const SYSTEM_INFO: Token<SystemInfo> = Token::new_const("robot.system_info");

#[rustfmt::skip]
pub const STATUS: Token<RobotStatus> = Token::new_const("robot.status");
#[rustfmt::skip]
pub const LEAK: Token<bool> = Token::new_const("robot.status.leak");

#[rustfmt::skip]
pub const CAMERAS: Token<Vec<Camera>> = Token::new_const("robot.cameras");

#[rustfmt::skip]
pub const ARMED: Token<Armed> = Token::new_const("robot.motors.armed");
#[rustfmt::skip]
pub const MOTOR_SPEED: Token<HashMap<MotorId, MotorFrame>> = Token::new_const("robot.motors.speed");

#[rustfmt::skip]
pub const MOVEMENT_JOYSTICK: Token<Movement> = Token::new_const("robot.movement.joystick");
#[rustfmt::skip]
pub const MOVEMENT_OPENCV: Token<Movement> = Token::new_const("robot.movement.opencv");
#[rustfmt::skip]
pub const MOVEMENT_AI: Token<Movement> = Token::new_const("robot.movement.ai");
#[rustfmt::skip]
pub const DEPTH_TARGET: Token<Meters> = Token::new_const("robot.movement.ai.depth.target");
#[rustfmt::skip]
pub const MOVEMENT_CALCULATED: Token<Movement> = Token::new_const("robot.movement.calculated");

#[rustfmt::skip]
pub const RAW_DEPTH: Token<DepthFrame> = Token::new_const("robot.sensors.depth");
#[rustfmt::skip]
pub const RAW_INERTIAL: Token<InertialFrame> = Token::new_const("robot.sensors.inertial");
#[rustfmt::skip]
pub const RAW_MAGNETIC: Token<MagFrame> = Token::new_const("robot.sensors.mag");
#[rustfmt::skip]
pub const ORIENTATION: Token<Orientation> = Token::new_const("robot.sensors.fusion");

/// Returns a map between `Key` and `TypeAdapter`
/// Used to convert the binary data for key into the correct struct
pub fn generate_adaptors() -> HashMap<Key, Box<dyn TypeAdapter<BackingType> + Send + Sync>> {
    fn from<T>(token: Token<T>) -> (Key, Box<dyn TypeAdapter<BackingType> + Send + Sync>)
    where
        for<'a> T: Send + Sync + Serialize + Deserialize<'a> + 'static,
    {
        (token.0, Box::<Adapter<T>>::default())
    }

    vec![
        from(SYSTEM_INFO),
        from(STATUS),
        from(LEAK),
        from(CAMERAS),
        from(ARMED),
        from(MOTOR_SPEED),
        from(MOVEMENT_JOYSTICK),
        from(MOVEMENT_OPENCV),
        from(MOVEMENT_AI),
        from(DEPTH_TARGET),
        from(MOVEMENT_CALCULATED),
        from(RAW_DEPTH),
        from(RAW_INERTIAL),
        from(RAW_MAGNETIC),
        from(ORIENTATION),
    ]
    .into_iter()
    .collect()
}
