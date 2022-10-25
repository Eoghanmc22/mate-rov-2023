use bevy::prelude::*;
use bevy_egui::{EguiContext, EguiPlugin};

// todo Display errors

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin);
        app.add_system(draw_ui);
        //todo!()
    }
}

fn draw_ui(mut egui_context: ResMut<EguiContext>) {
    let ctx = egui_context.ctx_mut();
    egui::SidePanel::left("Panel Left").show(ctx, |ui| {
        ui.label("Test");
    });
    egui::TopBottomPanel::top("Panel Top").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.button("Test1");
            ui.button("Test2");
            ui.button("Test3");
            ui.button("Test4");
        })
    });
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.label("Test3");
    });
}
