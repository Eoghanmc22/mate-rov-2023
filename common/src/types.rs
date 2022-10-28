use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign, Neg, Sub};
use glam::Quat;
use serde::{Serialize, Deserialize};

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Orientation(pub Quat);

/// +X: Right, +Y: Forwards, +Z: Up
/// +XR: Pitch Up, +YR: Roll Counterclockwise, +ZR: Yaw Clockwise (top view)
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Movement {
    pub x: Speed,      // Right
    pub y: Speed,      // Forwards
    pub z: Speed,      // Up

    pub x_rot: Speed,  // Pitch Up
    pub y_rot: Speed,  // Roll Counterclockwise
    pub z_rot: Speed,  // Yaw Clockwise (top view)
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
            z_rot: self.z_rot + rhs.z_rot
        }
    }
}

impl AddAssign for Movement {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MotorId {
    UpF,
    UpB,
    UpL,
    UpR,
    FrontL,
    FrontR,
    RearL,
    RearR,
}


// Raw Data Frames

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DepthFrame {
    pub depth: Meters,
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
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MagFrame {
    pub mag_x: Gauss,
    pub mag_y: Gauss,
    pub mag_z: Gauss,
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MotorFrame(pub Speed);

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub enum Armed {
    Armed,
    #[default]
    Disarmed
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
pub struct Speed(f64);

impl Speed {
    pub const MAX_VAL: Speed = Speed(1.0);
    pub const MIN_VAL: Speed = Speed(-1.0);
    pub const ZERO: Speed = Speed(0.0);

    /// Creates a new `Speed`. Input should be between -1.0 and 1.0
    pub const fn new(speed: f64) -> Self {
        assert!(speed.is_normal());
        Self(speed).clamp(Self::MIN_VAL, Self::MAX_VAL)
    }

    /// Clamps a speed to be between `min` and `max`
    pub const fn clamp(self, min: Speed, max: Speed) -> Speed {
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

impl Add<Speed> for Speed {
    type Output = Speed;

    fn add(self, rhs: Speed) -> Self::Output {
        Speed::new(self.0 + rhs.0)
    }
}

impl Sub<Speed> for Speed {
    type Output = Speed;

    fn sub(self, rhs: Speed) -> Self::Output {
        Speed::new(self.0 - rhs.0)
    }
}

impl Neg for Speed {
    type Output = Speed;

    fn neg(self) -> Self::Output {
        Speed(-self.0)
    }
}

impl Display for Speed {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{:.2}%", self.0 * 100.0))
    }
}
