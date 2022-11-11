use crate::plugins::networking::NetworkEvent;
use crate::plugins::robot::Robot;
use anyhow::Context;
use bevy::prelude::*;
use bevy_egui::{EguiContext, EguiPlugin};
use common::{
    kvdata::{Key, Value},
    types::{Celsius, MotorFrame},
};
use egui_extras::{Size, TableBuilder};
use message_io::network::ToRemoteAddr;

// todo Display errors

const TABLE_ROW_HEIGHT: f32 = 20.0;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin);
        // app.insert_resource(EguiSettings { scale_factor: 0.5, default_open_url_target: None });
        app.add_system(draw_ui);
        app.add_system(draw_connection_window);
        //todo!()
    }
}

// TODO use components for ui elements
// TODO display errors
// TODO split up

#[derive(Default, Component)]
struct ConnectWindow(String);

fn draw_ui(mut cmd: Commands, robot: Res<Robot>, mut egui_context: ResMut<EguiContext>) {
    let ctx = egui_context.ctx_mut();
    let state = robot.state();
    let store = robot.store();

    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "File", |ui| {
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });
            egui::menu::menu_button(ui, "Robot", |ui| {
                if ui.button("Connect").clicked() {
                    cmd.init_resource::<ConnectWindow>();
                }
            });
        });
    });
    egui::SidePanel::left("Panel Left")
        .resizable(false)
        .min_width(200.0)
        .show(ctx, |ui| {
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
                    ui.label(format!("X: {}", movement.x));
                    ui.label(format!("Y: {}", movement.y));
                    ui.label(format!("Z: {}", movement.z));
                    ui.add_space(5.0);
                    ui.label(format!("Yaw: {}", movement.z_rot));
                    ui.label(format!("Pitch: {}", movement.x_rot));
                    ui.label(format!("Roll: {}", movement.y_rot));
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
                if let Some(Value::Cameras(cameras)) = store.get(&Key::Cameras) {
                    for (name, addrs) in cameras {
                        ui.label(format!("{name}: {addrs}"));
                        // TODO Maybe show preview
                    }
                }
            });
            ui.collapsing("System", |ui| {
                if let Some(Value::SystemInfo(hw_state)) = store.get(&Key::SystemInfo) {
                    ui.collapsing("CPU", |ui| {
                        ui.label(format!(
                            "Load avg: {:.2}, {:.2}, {:.2}",
                            hw_state.load_average.0,
                            hw_state.load_average.1,
                            hw_state.load_average.2,
                        ));
                        ui.label(format!(
                            "Physical core count: {}",
                            hw_state.core_count.unwrap_or(0)
                        ));
                        TableBuilder::new(ui)
                            .striped(true)
                            .columns(Size::remainder(), 3)
                            .header(TABLE_ROW_HEIGHT, |mut row| {
                                row.col(|ui| {
                                    ui.label("Name");
                                });
                                row.col(|ui| {
                                    ui.label("Usage");
                                });
                                row.col(|ui| {
                                    ui.label("Freq");
                                });
                            })
                            .body(|mut body| {
                                body.row(TABLE_ROW_HEIGHT, |mut row| {
                                    row.col(|ui| {
                                        ui.label(&hw_state.cpu_total.name);
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{:.2}%", hw_state.cpu_total.usage));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{}MHz", hw_state.cpu_total.frequency));
                                    });
                                });
                                body.rows(TABLE_ROW_HEIGHT, hw_state.cpus.len(), |cpu, mut row| {
                                    let cpu = &hw_state.cpus[cpu];
                                    row.col(|ui| {
                                        ui.label(&cpu.name);
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{:.2}%", cpu.usage));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{}MHz", cpu.frequency));
                                    });
                                });
                            });
                    });
                    ui.collapsing("Processes", |ui| {
                        TableBuilder::new(ui)
                            .striped(true)
                            .columns(Size::remainder(), 5)
                            .header(20.0, |mut row| {
                                row.col(|ui| {
                                    ui.label("Name");
                                });
                                row.col(|ui| {
                                    ui.label("PID");
                                });
                                row.col(|ui| {
                                    ui.label("CPU");
                                });
                                row.col(|ui| {
                                    ui.label("MEM");
                                });
                                row.col(|ui| {
                                    ui.label("User");
                                });
                            })
                            .body(|body| {
                                body.rows(
                                    TABLE_ROW_HEIGHT,
                                    hw_state.processes.len(),
                                    |process, mut row| {
                                        let process = &hw_state.processes[process];
                                        row.col(|ui| {
                                            ui.label(&process.name);
                                        });
                                        row.col(|ui| {
                                            ui.label(format!("{}", process.pid));
                                        });
                                        row.col(|ui| {
                                            ui.label(format!("{:.2}%", process.cpu_usage));
                                        });
                                        row.col(|ui| {
                                            ui.label(format!(
                                                "{:.2}MB",
                                                process.memory as f64 / 1048576.0
                                            ));
                                        });
                                        row.col(|ui| {
                                            ui.label(format!("{:?}", process.user));
                                        });
                                    },
                                );
                            });
                    });
                    ui.collapsing("Networks", |ui| {
                        TableBuilder::new(ui)
                            .striped(true)
                            .columns(Size::remainder(), 7)
                            .header(20.0, |mut row| {
                                row.col(|ui| {
                                    ui.label("Name");
                                });
                                row.col(|ui| {
                                    ui.label("TX Data");
                                });
                                row.col(|ui| {
                                    ui.label("RX Data");
                                });
                                row.col(|ui| {
                                    ui.label("TX Packets");
                                });
                                row.col(|ui| {
                                    ui.label("RX Packets");
                                });
                                row.col(|ui| {
                                    ui.label("TX Errors");
                                });
                                row.col(|ui| {
                                    ui.label("RX Errors");
                                });
                            })
                            .body(|body| {
                                body.rows(
                                    TABLE_ROW_HEIGHT,
                                    hw_state.networks.len(),
                                    |network, mut row| {
                                        let network = &hw_state.networks[network];
                                        row.col(|ui| {
                                            ui.label(&network.name);
                                        });
                                        row.col(|ui| {
                                            ui.label(format!(
                                                "{:.2}MB",
                                                network.tx_bytes as f64 / 1048576.0
                                            ));
                                        });
                                        row.col(|ui| {
                                            ui.label(format!(
                                                "{:.2}MB",
                                                network.rx_bytes as f64 / 1048576.0
                                            ));
                                        });
                                        row.col(|ui| {
                                            ui.label(format!("{}", network.tx_packets));
                                        });
                                        row.col(|ui| {
                                            ui.label(format!("{}", network.rx_packets));
                                        });
                                        row.col(|ui| {
                                            ui.label(format!("{}", network.tx_errors));
                                        });
                                        row.col(|ui| {
                                            ui.label(format!("{}", network.rx_errors));
                                        });
                                    },
                                );
                            });
                    });
                    ui.collapsing("Memory", |ui| {
                        let memory = &hw_state.memory;
                        ui.label(format!(
                            "Memory: {:.2}MB / {:.2}MB",
                            memory.used_mem as f64 / 1048576.0,
                            memory.total_mem as f64 / 1048576.0
                        ));
                        ui.label(format!(
                            "Free Memory: {:.2}MB",
                            memory.free_mem as f64 / 1048576.0
                        ));
                        ui.add_space(3.0);

                        ui.label(format!(
                            "Swap: {:.2}MB / {:.2}MB",
                            memory.used_swap as f64 / 1048576.0,
                            memory.total_swap as f64 / 1048576.0
                        ));
                        ui.label(format!(
                            "Free Swap: {:.2}MB",
                            memory.free_swap as f64 / 1048576.0
                        ));
                    });
                    ui.collapsing("Thermals", |ui| {
                        TableBuilder::new(ui)
                            .striped(true)
                            .columns(Size::remainder(), 4)
                            .header(20.0, |mut row| {
                                row.col(|ui| {
                                    ui.label("Name");
                                });
                                row.col(|ui| {
                                    ui.label("Temp");
                                });
                                row.col(|ui| {
                                    ui.label("Max Temp");
                                });
                                row.col(|ui| {
                                    ui.label("Critical Temp");
                                });
                            })
                            .body(|body| {
                                body.rows(
                                    TABLE_ROW_HEIGHT,
                                    hw_state.components.len(),
                                    |component, mut row| {
                                        let component = &hw_state.components[component];
                                        row.col(|ui| {
                                            ui.label(&component.name);
                                        });
                                        row.col(|ui| {
                                            ui.label(format!("{}", component.tempature));
                                        });
                                        row.col(|ui| {
                                            ui.label(format!("{}", component.tempature_max));
                                        });
                                        row.col(|ui| {
                                            ui.label(format!(
                                                "{}",
                                                component
                                                    .tempature_critical
                                                    .unwrap_or(Celsius(f64::NAN))
                                            ));
                                        });
                                    },
                                );
                            });
                    });
                    ui.collapsing("Disks", |ui| {
                        TableBuilder::new(ui)
                            .striped(true)
                            .columns(Size::remainder(), 5)
                            .header(20.0, |mut row| {
                                row.col(|ui| {
                                    ui.label("Name");
                                });
                                row.col(|ui| {
                                    ui.label("Mount");
                                });
                                row.col(|ui| {
                                    ui.label("Total");
                                });
                                row.col(|ui| {
                                    ui.label("Free");
                                });
                                row.col(|ui| {
                                    ui.label("Removable");
                                });
                            })
                            .body(|body| {
                                body.rows(
                                    TABLE_ROW_HEIGHT,
                                    hw_state.disks.len(),
                                    |disk, mut row| {
                                        let disk = &hw_state.disks[disk];
                                        row.col(|ui| {
                                            ui.label(&disk.name);
                                        });
                                        row.col(|ui| {
                                            ui.label(&disk.mount_point);
                                        });
                                        row.col(|ui| {
                                            ui.label(format!(
                                                "{}MB",
                                                disk.total_space as f64 / 1048576.0
                                            ));
                                        });
                                        row.col(|ui| {
                                            ui.label(format!(
                                                "{}MB",
                                                disk.available_space as f64 / 1048576.0
                                            ));
                                        });
                                        row.col(|ui| {
                                            ui.label(format!("{}", disk.removable));
                                        });
                                    },
                                );
                            });
                    });
                    ui.collapsing("General", |ui| {
                        ui.label(format!("System Name: {:?}", hw_state.name));
                        ui.label(format!("Kernel Version: {:?}", hw_state.kernel_version));
                        ui.label(format!("OS Version: {:?}", hw_state.os_version));
                        ui.label(format!("Distribution: {:?}", hw_state.distro));
                        ui.label(format!("Host Name: {:?}", hw_state.host_name));
                    });
                } else {
                    ui.label("No system data");
                }
            })
        });
    egui::TopBottomPanel::top("Panel Top").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if let Some(Value::Cameras(cameras)) = store.get(&Key::Cameras) {
                for (name, addrs) in cameras {
                    if ui.button(name).clicked() {
                        todo!();
                    }
                }
            }
        });
    });
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Test");
    });
}

fn draw_connection_window(
    mut cmd: Commands,
    mut window: Option<ResMut<ConnectWindow>>,
    mut egui_context: ResMut<EguiContext>,
    mut net: EventWriter<NetworkEvent>,
    mut errors: EventWriter<anyhow::Error>,
) {
    let ctx = egui_context.ctx_mut();

    if let Some(ref mut window) = window {
        egui::Window::new("Connection").show(ctx, |ui| {
            ui.text_edit_singleline(&mut window.0);
            if ui.button("Connect").clicked() {
                match (window.0.as_str(), 44444)
                    .to_remote_addr()
                    .context("Create remote addrs")
                {
                    Ok(remote) => {
                        net.send(NetworkEvent::ConnectTo(remote));
                        cmd.remove_resource::<ConnectWindow>();
                    }
                    Err(error) => {
                        errors.send(error);
                    }
                }
            }
        });
    }
}
