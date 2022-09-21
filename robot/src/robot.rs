use std::collections::{HashMap, HashSet};
use std::time::Instant;
use lazy_static::lazy_static;
use common::types::{DepthFrame, InertialFrame, MagFrame, Meters, MotorFrame, MotorId, Movement, Orientation};
use crate::event::Notify;

lazy_static! {
    pub static ref ROBOT: Robot = Robot::new();
}

pub struct Robot {
    orientation: Notify<Option<(Orientation, Instant)>>,
    movement: Notify<Option<(Movement, Instant)>>,
    depth: Notify<Option<(DepthFrame, Instant)>>,
    inertial: Notify<Option<(InertialFrame, Instant)>>,
    mag: Notify<Option<(MagFrame, Instant)>>,
    motors: HashMap<MotorId, Notify<Option<(MotorFrame, Instant)>>>,
    armed: Notify<bool>,
    depth_target: Notify<Option<(Meters, Instant)>>
}

impl Robot {
    fn new(motor_ids: HashSet<MotorId>) -> Self {
        let mut motors = HashMap::new();

        for motor in motor_ids {
            motors.insert(motor, Default::default());
        }

        Robot {
            orientation: Default::default(),
            movement: Default::default(),
            depth: Default::default(),
            inertial: Default::default(),
            mag: Default::default(),
            motors,
            armed: Default::default(),
            depth_target: Default::default()
        }
    }


    pub fn orientation(&self) -> &Notify<Option<(Orientation, Instant)>> {
        &self.orientation
    }
    pub fn movement(&self) -> &Notify<Option<(Movement, Instant)>> {
        &self.movement
    }
    pub fn depth(&self) -> &Notify<Option<(DepthFrame, Instant)>> {
        &self.depth
    }
    pub fn inertial(&self) -> &Notify<Option<(InertialFrame, Instant)>> {
        &self.inertial
    }
    pub fn mag(&self) -> &Notify<Option<(MagFrame, Instant)>> {
        &self.mag
    }
    pub fn motors(&self) -> &HashMap<MotorId, Notify<Option<(MotorFrame, Instant)>>> {
        &self.motors
    }
    pub fn armed(&self) -> &Notify<bool> {
        &self.armed
    }
    pub fn depth_target(&self) -> &Notify<Option<(Meters, Instant)>> {
        &self.depth_target
    }
}
