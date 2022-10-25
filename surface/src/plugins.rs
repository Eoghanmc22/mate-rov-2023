use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub mod robot;
pub mod networking;
pub mod ui;
pub mod error;

pub struct MatePlugins;

impl PluginGroup for MatePlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(SchedulePlugin);
        group.add(robot::RobotPlugin);
        group.add(networking::NetworkPlugin);
        group.add(ui::UiPlugin);
        group.add(error::ErrorPlugin);
        // TODO gamepad
    }
}

struct SchedulePlugin;

impl Plugin for SchedulePlugin {
    fn build(&self, app: &mut App) {
        app.add_stage_after(CoreStage::PreUpdate, MateStage::NetworkRead, SystemStage::single_threaded());
        app.add_stage_after(MateStage::NetworkRead, MateStage::UpdateState, SystemStage::single_threaded());
        app.add_stage_after(CoreStage::Update, MateStage::NetworkWrite, SystemStage::single_threaded());
        app.add_stage_after(CoreStage::PostUpdate, MateStage::ErrorHandling, SystemStage::single_threaded());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(StageLabel)]
pub enum MateStage {
    NetworkRead,
    UpdateState,
    // Normal update stage
    NetworkWrite,
    ErrorHandling
}
