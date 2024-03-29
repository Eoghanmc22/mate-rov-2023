//! Definitions of important types used throughout the project

use fxhash::FxHashMap as HashMap;
use mint::{Quaternion, Vector3};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::iter::Sum;
use std::net::SocketAddr;
use std::ops::{Add, AddAssign, Neg, Sub};
use std::time::Duration;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub struct Orientation(pub Quaternion<f32>);

impl Default for Orientation {
    fn default() -> Self {
        Self(Quaternion::from([0.0, 0.0, 0.0, 1.0]))
    }
}

pub type MovementOverride = HashMap<MotorId, Percent>;

/// +X: Right, +Y: Forwards, +Z: Up
/// +XR: Pitch Up, +YR: Roll Clockwise, +ZR: Yaw Clockwise (top view)
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Movement {
    /// Right
    pub x: Percent,
    /// Forwards
    pub y: Percent,
    /// Up
    pub z: Percent,

    /// Pitch Up
    pub x_rot: Percent,
    /// Roll Clockwise
    pub y_rot: Percent,
    /// Yaw Clockwise (top view)
    pub z_rot: Percent,

    /// Servo for camera 1
    pub cam_1: Percent,
    /// Servo for camera 2
    pub cam_2: Percent,
    /// Servo for camera 3
    pub cam_3: Percent,
    /// Servo for camera 4
    pub cam_4: Percent,

    /// Auxilary control 1
    pub aux_1: Percent,
    /// Auxilary control 2
    pub aux_2: Percent,
    /// Auxilary control 3
    pub aux_3: Percent,
    /// Auxilary control 4
    pub aux_4: Percent,
}

impl Movement {
    pub fn get_by_id(&self, id: MotorId) -> Percent {
        match id {
            MotorId::FrontLeftBottom
            | MotorId::FrontLeftTop
            | MotorId::FrontRightBottom
            | MotorId::FrontRightTop
            | MotorId::BackLeftBottom
            | MotorId::BackLeftTop
            | MotorId::BackRightBottom
            | MotorId::BackRightTop => {
                unimplemented!()
            }

            MotorId::Camera1 => self.cam_1,
            MotorId::Camera2 => self.cam_2,
            MotorId::Camera3 => self.cam_3,
            MotorId::Camera4 => self.cam_4,
            MotorId::Aux1 => self.aux_1,
            MotorId::Aux2 => self.aux_2,
            MotorId::Aux3 => self.aux_3,
            MotorId::Aux4 => self.aux_4,
        }
    }

    pub fn set_by_id(&mut self, id: MotorId, value: Percent) {
        match id {
            MotorId::FrontLeftBottom
            | MotorId::FrontLeftTop
            | MotorId::FrontRightBottom
            | MotorId::FrontRightTop
            | MotorId::BackLeftBottom
            | MotorId::BackLeftTop
            | MotorId::BackRightBottom
            | MotorId::BackRightTop => {
                unimplemented!()
            }

            MotorId::Camera1 => self.cam_1 = value,
            MotorId::Camera2 => self.cam_2 = value,
            MotorId::Camera3 => self.cam_3 = value,
            MotorId::Camera4 => self.cam_4 = value,
            MotorId::Aux1 => self.aux_1 = value,
            MotorId::Aux2 => self.aux_2 = value,
            MotorId::Aux3 => self.aux_3 = value,
            MotorId::Aux4 => self.aux_4 = value,
        }
    }
}

impl Add for Movement {
    type Output = Movement;

    fn add(self, rhs: Self) -> Self::Output {
        Movement {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            x_rot: self.x_rot + rhs.x_rot,
            y_rot: self.y_rot + rhs.y_rot,
            z_rot: self.z_rot + rhs.z_rot,

            cam_1: self.cam_1 + rhs.cam_1,
            cam_2: self.cam_2 + rhs.cam_2,
            cam_3: self.cam_3 + rhs.cam_3,
            cam_4: self.cam_4 + rhs.cam_4,

            aux_1: self.aux_1 + rhs.aux_1,
            aux_2: self.aux_2 + rhs.aux_2,
            aux_3: self.aux_3 + rhs.aux_3,
            aux_4: self.aux_4 + rhs.aux_4,
        }
    }
}

impl AddAssign for Movement {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for Movement {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), Self::add)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MotorId {
    FrontLeftBottom,
    FrontLeftTop,
    FrontRightBottom,
    FrontRightTop,
    BackLeftBottom,
    BackLeftTop,
    BackRightBottom,
    BackRightTop,

    Camera1,
    Camera2,
    Camera3,
    Camera4,

    Aux1,
    Aux2,
    Aux3,
    Aux4,
}

// Raw Data Frames

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DepthFrame {
    pub depth: Meters,
    pub altitude: Meters,
    pub pressure: Mbar,

    pub temperature: Celsius,
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct InertialFrame {
    pub gyro_x: Dps,
    pub gyro_y: Dps,
    pub gyro_z: Dps,

    pub accel_x: GForce,
    pub accel_y: GForce,
    pub accel_z: GForce,

    pub tempature: Celsius,
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MagFrame {
    pub mag_x: Gauss,
    pub mag_y: Gauss,
    pub mag_z: Gauss,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum MotorFrame {
    Percent(Percent),
    Raw(Duration),
}

impl MotorFrame {
    pub fn to_f64(&self) -> f64 {
        match self {
            MotorFrame::Percent(pct) => pct.get(),
            MotorFrame::Raw(raw) => {
                let us = raw.as_micros() as f64;
                let dist_center = us - 1500.0;
                dist_center / (1900.0 - 1500.0)
            }
        }
    }
}

impl Default for MotorFrame {
    fn default() -> Self {
        Self::Percent(Percent::ZERO)
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub enum Armed {
    Armed,
    #[default]
    Disarmed,
}

// Basic Units

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Meters(pub f64);

impl Display for Meters {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}M", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Mbar(pub f64);

impl Display for Mbar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}mbar", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Celsius(pub f64);

impl Display for Celsius {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}°C", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct GForce(pub f64);

impl Display for GForce {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}g", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Radians(pub f64);

impl Display for Radians {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}rad", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Degrees(pub f64);

impl Display for Degrees {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}deg", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Dps(pub f64);

impl Display for Dps {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}dps", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Gauss(pub f64);

impl Display for Gauss {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}Gs", self.0))
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Percent(f64);

impl Percent {
    pub const MAX_VAL: Percent = Percent(1.0);
    pub const MIN_VAL: Percent = Percent(-1.0);
    pub const ZERO: Percent = Percent(0.0);

    /// Creates a new `Speed`. Input should be between -1.0 and 1.0
    pub const fn new(speed: f64) -> Self {
        if !speed.is_normal() {
            return Self::ZERO;
        }
        Self(speed).clamp(Self::MIN_VAL, Self::MAX_VAL)
    }

    /// Clamps a speed to be between `min` and `max`
    pub const fn clamp(self, min: Percent, max: Percent) -> Percent {
        if self.0 > max.0 {
            max
        } else if self.0 < min.0 {
            min
        } else {
            self
        }
    }

    /// Get the speed as a float between -1.0 and 1.0
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl Add<Percent> for Percent {
    type Output = Percent;

    fn add(self, rhs: Percent) -> Self::Output {
        Percent::new(self.0 + rhs.0)
    }
}

impl Sub<Percent> for Percent {
    type Output = Percent;

    fn sub(self, rhs: Percent) -> Self::Output {
        Percent::new(self.0 - rhs.0)
    }
}

impl Neg for Percent {
    type Output = Percent;

    fn neg(self) -> Self::Output {
        Percent(-self.0)
    }
}

impl Display for Percent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}%", self.0 * 100.0))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub processes: Vec<Process>,
    /// one min, five min, fifteen min
    pub load_average: (f64, f64, f64),
    pub networks: Vec<Network>,
    pub cpu_total: Cpu,
    pub cpus: Vec<Cpu>,
    pub core_count: Option<usize>,
    pub memory: Memory,
    pub components: Vec<Component>,
    pub disks: Vec<Disk>,
    pub uptime: Duration,
    pub name: Option<String>,
    pub kernel_version: Option<String>,
    pub os_version: Option<String>,
    pub distro: String,
    pub host_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Process {
    pub name: String,
    pub pid: u32,
    pub memory: u64,
    pub cpu_usage: f32,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cpu {
    pub frequency: u64,
    pub usage: f32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub total_mem: u64,
    pub used_mem: u64,
    pub free_mem: u64,

    pub total_swap: u64,
    pub used_swap: u64,
    pub free_swap: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub tempature: Celsius,
    pub tempature_max: Celsius,
    pub tempature_critical: Option<Celsius>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disk {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
    pub removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Camera {
    pub name: String,
    pub location: SocketAddr,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RobotStatus {
    // No peer is connected
    NoPeer,
    // Peer is connected and robot is disarmed
    Disarmed,
    // Peer is connected and robot is armed
    Ready,
    // The robot is moving, includes speed
    Moving(Percent),
}

#[derive(Clone, Copy)]
pub struct PidController {
    last_error: Option<f64>,
    integral: f64,
    interval: Duration,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PidConfig {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,

    pub max_integral: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PidResult {
    pub p: f64,
    pub i: f64,
    pub d: f64,
}

impl PidResult {
    pub const fn correction(&self) -> f64 {
        self.p + self.i + self.d
    }
}

impl PidController {
    pub fn new(interval: Duration) -> Self {
        Self {
            last_error: None,
            integral: 0.0,
            interval,
        }
    }

    pub fn update(&mut self, error: f64, config: PidConfig) -> PidResult {
        let cfg = config;
        let interval = self.interval.as_secs_f64();

        self.integral += error * interval;
        self.integral = self.integral.clamp(-cfg.max_integral, cfg.max_integral);

        let proportional = error;
        let integral = self.integral;
        let derivative = (error - self.last_error.unwrap_or(error)) / interval;

        self.last_error = Some(error);

        let p = cfg.kp * proportional;
        let i = cfg.ki * integral;
        let d = cfg.kd * derivative;

        PidResult { p, i, d }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LevelingMode {
    Enabled(Vector3<f32>),
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DepthControlMode {
    Enabled(Meters),
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LevelingCorrection {
    pub pitch: f64,
    pub roll: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DepthCorrection {
    pub depth: f64,
}
