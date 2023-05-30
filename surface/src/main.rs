#![feature(hash_drain_filter, drain_filter)]
#![warn(meta_variable_misuse)]

mod plugins;

use crate::plugins::MatePlugins;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    winit::WinitSettings,
};

fn main() {
    App::new()
        .insert_resource(FixedTime::new_from_secs(1.0 / 100.0))
        .add_plugins(DefaultPlugins)
        .add_plugins(MatePlugins)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .run();
}
