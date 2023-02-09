//! Code shared between both the surface and robot projects
#![feature(const_fn_floating_point_arithmetic, const_float_classify)]

use serde::{Deserialize, Serialize};

pub mod kvdata;
pub mod protocol;
pub mod state;
pub mod types;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}
