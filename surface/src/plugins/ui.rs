use bevy::prelude::*;
use bevy_egui::{EguiContext, EguiPlugin, EguiSettings};
use common::types::MotorFrame;
use crate::plugins::robot::Robot;

// todo Display errors

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin);
        app.insert_resource(EguiSettings { scale_factor: 0.5, default_open_url_target: None });
        app.add_system(draw_ui);
        //todo!()
    }
}

fn draw_ui(state: Res<Robot>, mut egui_context: ResMut<EguiContext>) {
    let ctx = egui_context.ctx_mut();
    let state = state.state();

    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "File", |ui| {
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });
        });
    });
    egui::SidePanel::left("Panel Left").resizable(false).min_width(200.0).show(ctx, |ui| {
        ui.collapsing("Orientation", |ui| {
            if let Some((orientation, _)) = state.orientation() {
                let (yaw, pitch, roll) = orientation.0.to_euler(EulerRot::YXZ);
                ui.label(format!("Yaw: {yaw}"));
                ui.label(format!("Pitch: {pitch}"));
                ui.label(format!("Roll: {roll}"));
                // TODO visual
            } else {
                ui.label("No orientation data");
            }
        });
        ui.collapsing("Movement", |ui| {
            if let Some((movement, _)) = state.movement() {
                ui.label(format!("Mode: {:?}", movement.mode));
                ui.add_space(3.0);
                ui.label(format!("X: {}", movement.x));
                ui.label(format!("Y: {}", movement.y));
                ui.label(format!("Z: {}", movement.z));
                ui.add_space(3.0);
                ui.label(format!("X: {}", movement.x_rot));
                ui.label(format!("Y: {}", movement.y_rot));
                ui.label(format!("Z: {}", movement.z_rot));
                // TODO visual
            } else {
                ui.label("No movement data");
            }
        });
        ui.collapsing("Raw Sensor Data", |ui| {
            ui.collapsing("Accelerometer", |ui| {
                if let Some((inertial, _)) = state.inertial() {
                    ui.label(format!("X: {}", inertial.accel_x));
                    ui.label(format!("Y: {}", inertial.accel_y));
                    ui.label(format!("Z: {}", inertial.accel_z));
                    // TODO visual
                } else {
                    ui.label("No accelerometer data");
                }
            });
            ui.collapsing("Gyro", |ui| {
                if let Some((inertial, _)) = state.inertial() {
                    ui.label(format!("X: {}", inertial.gyro_x));
                    ui.label(format!("Y: {}", inertial.gyro_y));
                    ui.label(format!("Z: {}", inertial.gyro_z));
                    // TODO visual
                } else {
                    ui.label("No gyro data");
                }
            });
            ui.collapsing("Depth", |ui| {
                if let Some((depth, _)) = state.depth() {
                    ui.label(format!("Depth: {}", depth.depth));
                    ui.label(format!("Temp: {}", depth.temperature));
                } else {
                    ui.label("No depth data");
                }
                if let Some((target, _)) = state.depth_target() {
                    ui.label(format!("Depth Target: {target}"));
                } else {
                    ui.label("Depth Target: None");
                }
            });
        });
        ui.collapsing("Motors", |ui| {
            for (motor, (MotorFrame(speed), _)) in state.motors().iter() {
                ui.label(format!("{motor:?}: {speed}"));
            }
            // TODO maybe draw thrust diagram
        });
        ui.collapsing("Cameras", |ui| {
            for (name, addrs) in state.cameras().iter() {
                ui.label(format!("{name}: {addrs}"));
                // TODO Maybe show preview
            }
        });
    });
    egui::TopBottomPanel::top("Panel Top").show(ctx, |ui| {
        ui.horizontal(|ui| {
            for (name, addrs) in state.cameras().iter() {
                if ui.button(name).clicked() {
                    todo!();
                }
            }
        });
    });
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Test");
    });
}
