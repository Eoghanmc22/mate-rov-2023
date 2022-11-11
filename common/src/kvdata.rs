//! An simple way to communicate data
//!
//! This side steps the complexity and event-based nature of the main state machine
//! Useful for data that is not mission critical but should not have its own packet

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};

use crate::types::SystemInfo;

pub type Store = HashMap<Key, Value>;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Key {
    SystemInfo,
    Cameras,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    SystemInfo(Box<SystemInfo>),
    Cameras(Vec<(String, SocketAddr)>),
}
impl Value {
    pub const fn to_key(&self) -> Key {
        match self {
            Value::SystemInfo(..) => Key::SystemInfo,
            Value::Cameras(..) => Key::Cameras,
        }
    }
}
