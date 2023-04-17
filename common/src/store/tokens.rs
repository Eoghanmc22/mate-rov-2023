//! Definitions of everything that can be stored in the global store

use crate::{
    store::adapters::{Adapter, BackingType, TypeAdapter},
    store::{Key, Token},
    types::{
        Armed, Camera, DepthFrame, InertialFrame, LevelingCorrection, LevelingMode, MagFrame,
        Meters, MotorFrame, MotorId, Movement, Orientation, PidConfig, PidResult, RobotStatus,
        SystemInfo,
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
pub const LEVELING_MODE: Token<LevelingMode> = Token::new_const("robot.leveling.mode");
#[rustfmt::skip]
pub const LEVELING_PID: Token<PidConfig> = Token::new_const("robot.leveling.pid");
#[rustfmt::skip]
pub const LEVELING_PITCH_RESULT: Token<PidResult> = Token::new_const("robot.leveling.pitch");
#[rustfmt::skip]
pub const LEVELING_ROLL_RESULT: Token<PidResult> = Token::new_const("robot.leveling.roll");
#[rustfmt::skip]
pub const LEVELING_CORRECTION: Token<LevelingCorrection> = Token::new_const("robot.leveling.correction");

#[rustfmt::skip]
pub const MOVEMENT_JOYSTICK: Token<Movement> = Token::new_const("robot.movement.joystick");
#[rustfmt::skip]
pub const MOVEMENT_OPENCV: Token<Movement> = Token::new_const("robot.movement.opencv");
#[rustfmt::skip]
pub const MOVEMENT_LEVELING: Token<Movement> = Token::new_const("robot.movement.leveling");
#[rustfmt::skip]
pub const MOVEMENT_DEPTH: Token<Movement> = Token::new_const("robot.movement.depth");
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
        from(LEVELING_MODE),
        from(LEVELING_PID),
        from(LEVELING_PITCH_RESULT),
        from(LEVELING_ROLL_RESULT),
        from(LEVELING_CORRECTION),
        from(MOVEMENT_JOYSTICK),
        from(MOVEMENT_OPENCV),
        from(MOVEMENT_LEVELING),
        from(MOVEMENT_DEPTH),
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
