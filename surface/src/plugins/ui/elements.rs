use bevy::prelude::Commands;
use common::{
    kvdata::{Key, Store, Value},
    state::RobotState,
};
use egui::Context;

use crate::plugins::video::{Position, Video};

use super::{
    widgets::{Cameras, Motors, Movement, Orientation, RawSensorData, RemoteSystem},
    windows::ConnectionWindow,
    WindowComponent,
};

pub fn side_bar(ctx: &Context, _cmd: &mut Commands, state: &RobotState, store: &Store) {
    egui::SidePanel::left("Panel Left")
        .min_width(200.0)
        .show(ctx, |ui| {
            ui.collapsing("Orientation", |ui| {
                ui.add(&mut Orientation::new(state));
            });
            ui.collapsing("Movement", |ui| {
                ui.add(&mut Movement::new(state));
            });
            ui.collapsing("Raw Sensor Data", |ui| {
                ui.add(&mut RawSensorData::new(state));
            });
            ui.collapsing("Motors", |ui| {
                ui.add(&mut Motors::new(state));
            });
            ui.collapsing("Cameras", |ui| {
                ui.add(&mut Cameras::new(store));
            });
            ui.collapsing("System", |ui| {
                ui.add(&mut RemoteSystem::new(store));
            });
            ui.allocate_space(ui.available_size());
        });
}

pub fn menu_bar(ctx: &Context, cmd: &mut Commands, _state: &RobotState, _store: &Store) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "File", |ui| {
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });
            egui::menu::menu_button(ui, "Robot", |ui| {
                if ui.button("Connect").clicked() {
                    cmd.spawn().insert(WindowComponent::new(
                        "Connect to robot".to_string(),
                        ConnectionWindow::default(),
                    ));
                }
            });
        });
    });
}

pub fn top_panel(ctx: &Context, cmd: &mut Commands, _state: &RobotState, store: &Store) {
    egui::TopBottomPanel::top("Panel Top").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if let Some(Value::Cameras(cameras)) = store.get(&Key::Cameras) {
                for (name, addrs) in cameras {
                    if ui.button(name).clicked() {
                        cmd.spawn().insert_bundle(Video::new(
                            name.to_owned(),
                            addrs.to_owned(),
                            Position::Center,
                        ));
                    }
                }
            }
        });
    });
}
