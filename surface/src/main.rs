mod plugins;

use bevy::prelude::*;
use crate::plugins::MatePlugins;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MatePlugins)
        .run();
}
