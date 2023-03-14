mod plugins;

use crate::plugins::MatePlugins;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MatePlugins)
        .run();
}
