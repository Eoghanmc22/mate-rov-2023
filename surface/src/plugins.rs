use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub mod gamepad;
pub mod networking;
pub mod notification;
pub mod opencv;
pub mod robot;
pub mod ui;
pub mod video;

pub struct MatePlugins;

impl PluginGroup for MatePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(robot::RobotPlugin)
            .add(networking::NetworkPlugin)
            .add(ui::UiPlugin)
            .add(video::VideoPlugin)
            .add(notification::NotificationPlugin)
            .add(gamepad::GamepadPlugin)
            .add(opencv::OpenCvPlugin)
    }
}
