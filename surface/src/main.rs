#![warn(meta_variable_misuse)]

mod plugins;

use crate::plugins::MatePlugins;
use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(FixedTime::new_from_secs(1.0 / 100.0))
        .add_plugins(DefaultPlugins)
        .add_plugins(MatePlugins)
        .run();
}
