//! Egui widget implementations

use common::{
    store::{tokens, Store},
    types::{Camera, Celsius, MotorFrame},
};
use egui::{vec2, Align, Direction, Layout, TextureId, Widget};
use egui_extras::{Column, TableBuilder};
use glam::EulerRot;

const TABLE_ROW_HEIGHT: f32 = 15.0;

pub struct RemoteSystemUi<'a, C> {
    data: &'a Store<C>,
}

impl<'a, C> RemoteSystemUi<'a, C> {
    pub fn new(data: &'a Store<C>) -> Self {
        Self { data }
    }
}

impl<C> Widget for &mut RemoteSystemUi<'_, C> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(vec2(ui.available_width(), 0.0), |ui| {
            if let Some(hw_state) = self.data.get(&tokens::SYSTEM_INFO) {
                ui.collapsing("CPU", |ui| {
                    ui.set_max_height(500.0);
                    ui.label(format!(
                        "Load avg: {:.2}, {:.2}, {:.2}",
                        hw_state.load_average.0, hw_state.load_average.1, hw_state.load_average.2,
                    ));
                    ui.label(format!(
                        "Physical core count: {}",
                        hw_state.core_count.unwrap_or(0)
                    ));
                    TableBuilder::new(ui)
                        .striped(true)
                        .columns(Column::remainder(), 3)
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
                    ui.set_max_height(500.0);
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .column(Column::auto())
                        .columns(Column::exact(60.0), 4)
                        .header(TABLE_ROW_HEIGHT, |mut row| {
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
                                        ui.label(
                                            process.user.as_deref().unwrap_or("None").to_string(),
                                        );
                                    });
                                },
                            );
                        });
                });
                ui.collapsing("Networks", |ui| {
                    ui.set_max_height(500.0);
                    TableBuilder::new(ui)
                        .striped(true)
                        .columns(Column::remainder(), 7)
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
                    ui.set_max_height(500.0);
                    TableBuilder::new(ui)
                        .striped(true)
                        .columns(Column::remainder(), 4)
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
                    ui.set_max_height(500.0);
                    TableBuilder::new(ui)
                        .striped(true)
                        .columns(Column::remainder(), 5)
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
                            body.rows(TABLE_ROW_HEIGHT, hw_state.disks.len(), |disk, mut row| {
                                let disk = &hw_state.disks[disk];
                                row.col(|ui| {
                                    ui.label(&disk.name);
                                });
                                row.col(|ui| {
                                    ui.label(&disk.mount_point);
                                });
                                row.col(|ui| {
                                    ui.label(format!(
                                        "{:.2}MB",
                                        disk.total_space as f64 / 1048576.0
                                    ));
                                });
                                row.col(|ui| {
                                    ui.label(format!(
                                        "{:.2}MB",
                                        disk.available_space as f64 / 1048576.0
                                    ));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{}", disk.removable));
                                });
                            });
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
        .response
    }
}

pub struct OrientationUi<'a, C> {
    data: &'a Store<C>,
}

impl<'a, C> OrientationUi<'a, C> {
    pub fn new(data: &'a Store<C>) -> Self {
        Self { data }
    }
}

impl<C> Widget for &mut OrientationUi<'_, C> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(vec2(ui.available_width(), 0.0), |ui| {
            if let Some(data) = self.data.get(&tokens::ORIENTATION) {
                let (orientation, _) = &*data;

                let (yaw, pitch, roll) = orientation.0.to_euler(EulerRot::YXZ);
                ui.label(format!("Yaw: {yaw}"));
                ui.label(format!("Pitch: {pitch}"));
                ui.label(format!("Roll: {roll}"));
                // TODO visual
            } else {
                ui.label("No orientation data");
            }
        })
        .response
    }
}

pub struct MovementUi<'a, C> {
    data: &'a Store<C>,
}

impl<'a, C> MovementUi<'a, C> {
    pub fn new(data: &'a Store<C>) -> Self {
        Self { data }
    }
}

impl<C> Widget for &mut MovementUi<'_, C> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(vec2(ui.available_width(), 0.0), |ui| {
            if let Some(data) = self.data.get(&tokens::MOVEMENT_CALCULATED) {
                let (movement, _) = &*data;

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
            if let Some(data) = self.data.get(&tokens::MOVEMENT_JOYSTICK) {
                let (movement, _) = &*data;

                ui.collapsing("Joystick", |ui| {
                    ui.label(format!("X: {}", movement.x));
                    ui.label(format!("Y: {}", movement.y));
                    ui.label(format!("Z: {}", movement.z));
                    ui.add_space(5.0);
                    ui.label(format!("Yaw: {}", movement.z_rot));
                    ui.label(format!("Pitch: {}", movement.x_rot));
                    ui.label(format!("Roll: {}", movement.y_rot));
                    // TODO visual
                });
            }
            if let Some(data) = self.data.get(&tokens::MOVEMENT_OPENCV) {
                let (movement, _) = &*data;

                ui.collapsing("Open CV", |ui| {
                    ui.label(format!("X: {}", movement.x));
                    ui.label(format!("Y: {}", movement.y));
                    ui.label(format!("Z: {}", movement.z));
                    ui.add_space(5.0);
                    ui.label(format!("Yaw: {}", movement.z_rot));
                    ui.label(format!("Pitch: {}", movement.x_rot));
                    ui.label(format!("Roll: {}", movement.y_rot));
                    // TODO visual
                });
            }
            if let Some(data) = self.data.get(&tokens::MOVEMENT_AI) {
                let (movement, _) = &*data;

                ui.collapsing("Depth Correction", |ui| {
                    ui.label(format!("X: {}", movement.x));
                    ui.label(format!("Y: {}", movement.y));
                    ui.label(format!("Z: {}", movement.z));
                    ui.add_space(5.0);
                    ui.label(format!("Yaw: {}", movement.z_rot));
                    ui.label(format!("Pitch: {}", movement.x_rot));
                    ui.label(format!("Roll: {}", movement.y_rot));
                    // TODO visual
                });
            }
        })
        .response
    }
}

pub struct RawSensorDataUi<'a, C> {
    data: &'a Store<C>,
}

impl<'a, C> RawSensorDataUi<'a, C> {
    pub fn new(data: &'a Store<C>) -> Self {
        Self { data }
    }
}

impl<C> Widget for &mut RawSensorDataUi<'_, C> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(vec2(ui.available_width(), 0.0), |ui| {
            ui.collapsing("Imu", |ui| {
                if let Some(data) = self.data.get(&tokens::RAW_INERTIAL) {
                    let (inertial, _) = &*data;

                    ui.label("Accel");
                    ui.label(format!("X: {}", inertial.accel_x));
                    ui.label(format!("Y: {}", inertial.accel_y));
                    ui.label(format!("Z: {}", inertial.accel_z));

                    ui.label("Gyro");
                    ui.label(format!("X: {}", inertial.gyro_x));
                    ui.label(format!("Y: {}", inertial.gyro_y));
                    ui.label(format!("Z: {}", inertial.gyro_z));

                    ui.label("Temp");
                    ui.label(format!("In robot: {}", inertial.tempature));

                    // TODO visual
                } else {
                    ui.label("No accelerometer data");
                }
            });
            ui.collapsing("Mag", |ui| {
                if let Some(data) = self.data.get(&tokens::RAW_MAGNETIC) {
                    let (mag, _) = &*data;

                    ui.label("Mag");
                    ui.label(format!("X: {}", mag.mag_x));
                    ui.label(format!("Y: {}", mag.mag_y));
                    ui.label(format!("Z: {}", mag.mag_z));

                    // TODO visual
                } else {
                    ui.label("No magnetometer data");
                }
            });
            ui.collapsing("Fusion", |ui| {
                ui.label("TODO");
            });
            ui.collapsing("Depth", |ui| {
                if let Some(data) = self.data.get(&tokens::RAW_DEPTH) {
                    let (depth, _) = &*data;

                    ui.label(format!("Pressure: {}", depth.pressure));
                    ui.label(format!("Depth: {}", depth.depth));
                    ui.label(format!("Attitude: {}", depth.altitude));
                    ui.label(format!("Temp: {}", depth.temperature));
                } else {
                    ui.label("No depth data");
                }
                if let Some(data) = self.data.get(&tokens::DEPTH_TARGET) {
                    let (target, _) = &*data;

                    ui.label(format!("Depth Target: {target}"));
                } else {
                    ui.label("Depth Target: None");
                }
            });
        })
        .response
    }
}

pub struct MotorsUi<'a, C> {
    data: &'a Store<C>,
}

impl<'a, C> MotorsUi<'a, C> {
    pub fn new(data: &'a Store<C>) -> Self {
        Self { data }
    }
}

impl<C> Widget for &mut MotorsUi<'_, C> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(vec2(ui.available_width(), 0.0), |ui| {
            if let Some(data) = self.data.get(&tokens::MOTOR_SPEED) {
                let (speeds, _) = &*data;

                for (motor, MotorFrame(speed)) in speeds.iter() {
                    ui.label(format!("{motor:?}: {speed}"));
                }
            }
            // TODO maybe draw thrust diagram
        })
        .response
    }
}

pub struct CamerasUi<'a, C> {
    data: &'a Store<C>,
}

impl<'a, C> CamerasUi<'a, C> {
    pub fn new(data: &'a Store<C>) -> Self {
        Self { data }
    }
}

impl<C> Widget for &mut CamerasUi<'_, C> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(vec2(ui.available_width(), 0.0), |ui| {
            if let Some(cameras) = self.data.get(&tokens::CAMERAS) {
                for Camera { name, location } in &*cameras {
                    ui.label(format!("{name}: {location}"));
                    // TODO Maybe show preview
                }
            }
        })
        .response
    }
}

pub struct Video<'a> {
    name: &'a str,
    texture: Option<TextureId>,
    pub should_delete: bool,
}

impl<'a> Video<'a> {
    pub fn new(name: &'a str, texture: Option<TextureId>) -> Self {
        Self {
            name,
            texture,
            should_delete: false,
        }
    }
}

impl Widget for &mut Video<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(ui.available_size(), |ui| {
            ui.group(|ui| {
                ui.allocate_ui_with_layout(
                    vec2(ui.available_width(), 1.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.group(|ui| {
                            ui.label(self.name);
                            if ui.small_button("Close").clicked() {
                                self.should_delete = true;
                            }
                            ui.allocate_space(ui.available_size());
                        });
                    },
                );

                if let Some(texture) = self.texture {
                    ui.with_layout(
                        Layout::centered_and_justified(Direction::LeftToRight),
                        |ui| {
                            let available = ui.available_size();
                            let x = available.x;
                            let y = x / 16.0 * 9.0;

                            ui.image(texture, (x, y));
                        },
                    );
                } else {
                    ui.label("No video");
                }

                ui.allocate_space(ui.available_size());
            });
        })
        .response
    }
}

pub struct RobotUi<'a, C> {
    data: &'a Store<C>,
}

impl<'a, C> RobotUi<'a, C> {
    pub fn new(data: &'a Store<C>) -> Self {
        Self { data }
    }
}

impl<C> Widget for &mut RobotUi<'_, C> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(vec2(ui.available_width(), 0.0), |ui| ui.label("TODO"))
            .response
    }
}
