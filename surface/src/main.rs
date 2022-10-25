pub mod ui;

use bevy::prelude::*;
use bevy_egui::{EguiContext, EguiPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        // Systems that create Egui widgets should be run during the `CoreStage::Update` stage,
        // or after the `EguiSystem::BeginFrame` system (which belongs to the `CoreStage::PreUpdate` stage).
        .add_system(ui_example)
        .run();
}

fn ui_example(mut egui_context: ResMut<EguiContext>) {
    let ctx = egui_context.ctx_mut();
    egui::SidePanel::left("Panel Left").show(ctx, |ui| {
        ui.label("Test");
    });
    egui::TopBottomPanel::top("Panel Top").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.button("Test2");
            ui.button("Test2");
            ui.button("Test2");
            ui.button("Test2");
        })
    });
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.label("Test3");
    });
}
