use bevy::prelude::{Commands, World};
use common::{
    protocol::Protocol,
    store::{tokens, Store},
};
use egui::Context;

use crate::plugins::{
    networking::NetworkEvent,
    opencv::VideoCapturePeer,
    video::{Position, Video},
};

use super::{
    widgets::{
        CamerasUi, MotorsUi, MovementUi, OrientationUi, RawSensorDataUi, RemoteSystemUi, RobotUi,
    },
    windows::ConnectionWindow,
    WindowComponent,
};

/// Information panel on left of window
pub fn side_bar<C>(ctx: &Context, _cmd: &mut Commands, store: &Store<C>) {
    egui::SidePanel::left("Panel Left")
        .min_width(200.0)
        .show(ctx, |ui| {
            ui.collapsing("Robot", |ui| {
                ui.add(&mut RobotUi::new(store));
            });
            ui.collapsing("Orientation", |ui| {
                ui.add(&mut OrientationUi::new(store));
            });
            ui.collapsing("Movement", |ui| {
                ui.add(&mut MovementUi::new(store));
            });
            ui.collapsing("Raw Sensor Data", |ui| {
                ui.add(&mut RawSensorDataUi::new(store));
            });
            ui.collapsing("Motors", |ui| {
                ui.add(&mut MotorsUi::new(store));
            });
            ui.collapsing("Cameras", |ui| {
                ui.add(&mut CamerasUi::new(store));
            });
            ui.collapsing("System", |ui| {
                ui.add(&mut RemoteSystemUi::new(store));
            });
            ui.allocate_space(ui.available_size());
        });
}

/// Menu bar at top of screen
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
                    cmd.spawn_empty().insert(WindowComponent::new(
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

/// Camera select bar
pub fn top_panel<C>(ctx: &Context, cmd: &mut Commands, store: &Store<C>) {
    egui::TopBottomPanel::top("Panel Top").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if let Some(cameras) = store.get(&tokens::CAMERAS) {
                for camera in cameras.iter() {
                    if ui.button(&camera.name).clicked() {
                        cmd.spawn(Video::new(camera.name.to_owned(), Position::Center))
                            .insert(VideoCapturePeer(camera.to_owned()));
                    }
                }
            }
        });
    });
}
