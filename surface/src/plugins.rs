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
            .add(SchedulePlugin)
            .add(robot::RobotPlugin)
            .add(networking::NetworkPlugin)
            .add(ui::UiPlugin)
            .add(video::VideoPlugin)
            .add(notification::NotificationPlugin)
            .add(gamepad::GamepadPlugin)
            .add(opencv::OpenCvPlugin)
    }
}

struct SchedulePlugin;

impl Plugin for SchedulePlugin {
    fn build(&self, app: &mut App) {
        // app.configure_sets(
        //     (
        //         MateSet::NetworkRead,
        //         CoreSet::PreUpdate,
        //         MateSet::UpdateStateEarly,
        //         CoreSet::Update,
        //         CoreSet::PostUpdate,
        //         MateSet::RenderVideo,
        //         MateSet::UpdateStateLate,
        //         MateSet::NetworkWrite,
        //         MateSet::ErrorHandling,
        //         CoreSet::PostUpdateFlush,
        //     )
        //         .chain(),
        // );
        // app.add_stage_before(
        //     CoreStage::PreUpdate,
        //     MateStage::NetworkRead,
        //     SystemStage::single_threaded(),
        // );
        // app.add_stage_after(
        //     MateStage::NetworkRead,
        //     MateStage::UpdateStateEarly,
        //     SystemStage::single_threaded(),
        // );
        // app.add_stage_after(
        //     CoreStage::PostUpdate,
        //     MateStage::RenderVideo,
        //     SystemStage::single_threaded(),
        // );
        // app.add_stage_after(
        //     MateStage::RenderVideo,
        //     MateStage::UpdateStateLate,
        //     SystemStage::single_threaded(),
        // );
        // app.configure_set(add_stage_after(
        //     MateStage::UpdateStateLate,
        //     MateStage::NetworkWrite,
        //     SystemStage::single_threaded(),
        // );
        // app.add_stage_after(
        //     MateStage::NetworkWrite,
        //     MateStage::ErrorHandling,
        //     SystemStage::single_threaded(),
        // );
    }
}
