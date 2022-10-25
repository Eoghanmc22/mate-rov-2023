use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::net::SocketAddr;
use std::time::Instant;
use crate::types::{Armed, DepthFrame, InertialFrame, MagFrame, Meters, MotorFrame, MotorId, Movement, Orientation};
use serde::{Serialize, Deserialize};

#[derive(Default)]
pub struct RobotState {
    armed: Armed,
    orientation: Option<(Orientation, Instant)>,
    movement: Option<(Movement, Instant)>,
    depth: Option<(DepthFrame, Instant)>,
    inertial: Option<(InertialFrame, Instant)>,
    mag: Option<(MagFrame, Instant)>,
    motors: HashMap<MotorId, (MotorFrame, Instant)>,
    cameras: HashSet<SocketAddr>,
    depth_target: Option<(Meters, Instant)>,

    callback: Option<Box<dyn Fn(&RobotStateUpdate, &mut RobotState) + Send + Sync + 'static>>,
}

impl RobotState {
    pub fn new(motor_ids: &[MotorId]) -> Self {
        let mut motors = HashMap::new();
        for motor in motor_ids {
            motors.insert(*motor, (MotorFrame::default(), Instant::now()));
        }

        Self {
            motors,
            ..Default::default()
        }
    }

    pub fn armed(&self) -> Armed {
        self.armed
    }

    pub fn orientation(&self) -> Option<(Orientation, Instant)> {
        self.orientation
    }

    pub fn movement(&self) -> Option<(Movement, Instant)> {
        self.movement
    }

    pub fn depth(&self) -> Option<(DepthFrame, Instant)> {
        self.depth
    }

    pub fn inertial(&self) -> Option<(InertialFrame, Instant)> {
        self.inertial
    }

    pub fn mag(&self) -> Option<(MagFrame, Instant)> {
        self.mag
    }

    pub fn motor(&self, motor: MotorId) -> Option<(MotorFrame, Instant)> {
        self.motors.get(&motor).copied()
    }

    pub fn motors(&self) -> &HashMap<MotorId, (MotorFrame, Instant)> {
        &self.motors
    }

    pub fn cameras(&self) -> &HashSet<SocketAddr> {
        &self.cameras
    }

    pub fn depth_target(&self) -> Option<(Meters, Instant)> {
        self.depth_target
    }

    pub fn set_callback<F: Fn(&RobotStateUpdate, &mut RobotState) + Send + Sync + 'static>(&mut self, callback: F) {
        self.callback = Some(Box::new(callback));
    }

    pub fn update(&mut self, update: &RobotStateUpdate) {
        let now = Instant::now();

        let changed = match update {
            RobotStateUpdate::Armed(armed) => {
                if self.armed != *armed {
                    self.armed = *armed;
                    true
                } else {
                    false
                }
            },
            RobotStateUpdate::Orientation(orientation) => {
                if self.orientation.as_ref().map(|it| &it.0) != Some(orientation) {
                    self.orientation = Some((*orientation, now));
                    true
                } else {
                    false
                }
            },
            RobotStateUpdate::Movement(movement) => {
                if self.movement.as_ref().map(|it| &it.0) != Some(movement) {
                    self.movement = Some((*movement, now));
                    true
                } else {
                    false
                }
            },
            RobotStateUpdate::Depth(depth) => {
                if self.depth.as_ref().map(|it| &it.0) != Some(depth) {
                    self.depth = Some((*depth, now));
                    true
                } else {
                    false
                }
            },
            RobotStateUpdate::Inertial(inertial) => {
                if self.inertial.as_ref().map(|it| &it.0) != Some(inertial) {
                    self.inertial = Some((*inertial, now));
                    true
                } else {
                    false
                }
            },
            RobotStateUpdate::Magnetometer(magnetometer) => {
                if self.mag.as_ref().map(|it| &it.0) != Some(magnetometer) {
                    self.mag = Some((*magnetometer, now));
                    true
                } else {
                    false
                }
            },
            RobotStateUpdate::Motor(motor_id, motor) => {
                let last = self.motors.insert(*motor_id, (*motor, now));

                last.as_ref().map(|it| &it.0) != Some(motor)
            },
            RobotStateUpdate::DepthTarget(depth_target) => {
                if self.depth_target.as_ref().map(|it| &it.0) != Some(depth_target) {
                    self.depth_target = Some((*depth_target, now));
                    true
                } else {
                    false
                }
            },
            RobotStateUpdate::Camera(action) => {
                match action {
                    CameraAction::Add(camera) => {
                        self.cameras.insert(*camera)
                    }
                    CameraAction::Remove(camera) => {
                        self.cameras.remove(camera)
                    }
                    CameraAction::Set(cameras) => {
                        if &self.cameras != cameras {
                            self.cameras = cameras.to_owned();
                            true
                        } else {
                            false
                        }
                    }
                }
            },
        };

        if let Some(callback) = self.callback.take() {
            if changed {
                (callback)(update, self);
            }
            self.callback = Some(callback);
        }
    }

    pub fn to_updates(&self) -> Vec<RobotStateUpdate> {
        let mut vec = Vec::new();

        vec.push(RobotStateUpdate::Armed(self.armed()));

        if let Some((orientation, _)) = self.orientation() {
            vec.push(RobotStateUpdate::Orientation(orientation));
        }

        if let Some((movement, _)) = self.movement() {
            vec.push(RobotStateUpdate::Movement(movement));
        }

        if let Some((depth, _)) = self.depth() {
            vec.push(RobotStateUpdate::Depth(depth));
        }

        if let Some((inertial, _)) = self.inertial() {
            vec.push(RobotStateUpdate::Inertial(inertial));
        }

        if let Some((mag, _)) = self.mag() {
            vec.push(RobotStateUpdate::Magnetometer(mag));
        }

        for (motor_id, (motor, _)) in self.motors() {
            vec.push(RobotStateUpdate::Motor(*motor_id, *motor));
        }

        vec.push(RobotStateUpdate::Camera(CameraAction::Set(self.cameras.to_owned())));

        if let Some((depth_target, _)) = self.depth_target() {
            vec.push(RobotStateUpdate::DepthTarget(depth_target));
        }

        vec
    }
}

impl Debug for RobotState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RobotState")
            .field("armed", &self.armed)
            .field("orientation", &self.orientation)
            .field("movement", &self.movement)
            .field("depth", &self.depth)
            .field("inertial", &self.inertial)
            .field("mag", &self.mag)
            .field("motors", &self.motors)
            .field("cameras", &self.cameras)
            .field("depth_target", &self.depth_target)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RobotStateUpdate {
    Armed(Armed),
    Orientation(Orientation),
    Movement(Movement),
    Depth(DepthFrame),
    Inertial(InertialFrame),
    Magnetometer(MagFrame),
    Motor(MotorId, MotorFrame),
    Camera(CameraAction),
    DepthTarget(Meters),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CameraAction {
    Add(SocketAddr),
    Remove(SocketAddr),
    Set(HashSet<SocketAddr>),
}
