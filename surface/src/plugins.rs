use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub mod error;
pub mod gamepad;
pub mod movement;
pub mod networking;
pub mod robot;
pub mod ui;

pub struct MatePlugins;

impl PluginGroup for MatePlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(SchedulePlugin);
        group.add(robot::RobotPlugin);
        group.add(networking::NetworkPlugin);
        group.add(ui::UiPlugin);
        group.add(error::ErrorPlugin);
        group.add(gamepad::GamepadPlugin);
        group.add(movement::MovementPlugin);
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
    UpdateStateLate,
    NetworkWrite,
    ErrorHandling,
}
