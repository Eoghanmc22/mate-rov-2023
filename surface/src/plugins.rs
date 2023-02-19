use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub mod gamepad;
pub mod networking;
pub mod notification;
pub mod robot;
pub mod ui;
pub mod video;

pub struct MatePlugins;

impl PluginGroup for MatePlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(SchedulePlugin);
        group.add(robot::RobotPlugin);
        group.add(networking::NetworkPlugin);
        group.add(ui::UiPlugin);
        group.add(video::VideoPlugin);
        group.add(notification::NotificationPlugin);
        group.add(gamepad::GamepadPlugin);
    }
}

struct SchedulePlugin;

impl Plugin for SchedulePlugin {
    fn build(&self, app: &mut App) {
        app.add_stage_before(
            CoreStage::PreUpdate,
            MateStage::NetworkRead,
            SystemStage::single_threaded(),
        );
        app.add_stage_after(
            MateStage::NetworkRead,
            MateStage::UpdateStateEarly,
            SystemStage::single_threaded(),
        );
        app.add_stage_after(
            CoreStage::PostUpdate,
            MateStage::RenderVideo,
            SystemStage::single_threaded(),
        );
        app.add_stage_after(
            MateStage::RenderVideo,
            MateStage::UpdateStateLate,
            SystemStage::single_threaded(),
        );
        app.add_stage_after(
            MateStage::UpdateStateLate,
            MateStage::NetworkWrite,
            SystemStage::single_threaded(),
        );
        app.add_stage_after(
            MateStage::NetworkWrite,
            MateStage::ErrorHandling,
            SystemStage::single_threaded(),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, StageLabel)]
pub enum MateStage {
    NetworkRead,
    UpdateStateEarly,
    // Pre update stage
    // Normal update stage
    // Post update stage
    RenderVideo,
    UpdateStateLate,
    NetworkWrite,
    ErrorHandling,
}
