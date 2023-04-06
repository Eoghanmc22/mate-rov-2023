#![warn(meta_variable_misuse, clippy::nursery)]

mod plugins;

use crate::plugins::MatePlugins;
use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(FixedTime::new_from_secs(1.0 / 10.0))
        // .insert_resource(FixedTime::new_from_secs(1.0 / 100.0))
        .add_plugins(DefaultPlugins)
        .add_plugins(MatePlugins)
        .run();
}
