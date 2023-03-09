use bevy::prelude::{Commands, World};
use common::{
    protocol::Protocol,
    store::{tokens, Store},
    types::Camera,
};
use egui::Context;

use crate::plugins::{
    networking::NetworkEvent,
    video::{Position, Video},
};

use super::{
    widgets::{Cameras, Motors, Movement, Orientation, RawSensorData, RemoteSystem},
    windows::ConnectionWindow,
    WindowComponent,
};

pub fn side_bar<C>(ctx: &Context, _cmd: &mut Commands, store: &Store<C>) {
    egui::SidePanel::left("Panel Left")
        .min_width(200.0)
        .show(ctx, |ui| {
            ui.collapsing("Orientation", |ui| {
                ui.add(&mut Orientation::new(store));
            });
            ui.collapsing("Movement", |ui| {
                ui.add(&mut Movement::new(store));
            });
            ui.collapsing("Raw Sensor Data", |ui| {
                ui.add(&mut RawSensorData::new(store));
            });
            ui.collapsing("Motors", |ui| {
                ui.add(&mut Motors::new(store));
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

pub fn menu_bar<C>(ctx: &Context, cmd: &mut Commands, _store: &Store<C>) {
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
                if ui.button("Resync").clicked() {
                    cmd.add(move |world: &mut World| {
                        world.send_event(NetworkEvent::SendPacket(Protocol::RequestSync));
                    });
                }
            });
        });
    });
}

pub fn top_panel<C>(ctx: &Context, cmd: &mut Commands, store: &Store<C>) {
    egui::TopBottomPanel::top("Panel Top").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if let Some(cameras) = store.get(&tokens::CAMERAS) {
                for Camera { name, location } in cameras.iter() {
                    if ui.button(name).clicked() {
                        cmd.spawn().insert_bundle(Video::new(
                            name.to_owned(),
                            location.to_owned(),
                            Position::Center,
                        ));
                    }
                }
            }
        });
    });
}
