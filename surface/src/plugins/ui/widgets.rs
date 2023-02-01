use bevy::prelude::EulerRot;
use common::{
    kvdata::{Key, Store, Value},
    state::RobotState,
    types::{Celsius, MotorFrame},
};
use egui::{vec2, Align, Layout, Response, Widget};
use egui_extras::{Size, TableBuilder};

const TABLE_ROW_HEIGHT: f32 = 15.0;

pub struct RemoteSystem<'a> {
    data: &'a Store,
}

impl<'a> RemoteSystem<'a> {
    pub fn new(data: &'a Store) -> Self {
        Self { data }
    }
}

impl Widget for &mut RemoteSystem<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(vec2(200.0, 0.0), |ui| {
            if let Some(Value::SystemInfo(hw_state)) = self.data.get(&Key::SystemInfo) {
                ui.collapsing("CPU", |ui| {
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
                    ui.set_max_height(500.0);
                    TableBuilder::new(ui)
                        .striped(true)
                        .column(Size::remainder().at_least(100.0))
                        .columns(Size::exact(60.0), 5)
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
                                        ui.label(format!(
                                            "{}",
                                            process
                                                .user
                                                .as_ref()
                                                .map(|it| it.as_str())
                                                .unwrap_or("None")
                                        ));
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

pub struct Orientation<'a> {
    data: &'a RobotState,
}

impl<'a> Orientation<'a> {
    pub fn new(data: &'a RobotState) -> Self {
        Self { data }
    }
}

impl Widget for &mut Orientation<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(vec2(200.0, 0.0), |ui| {
            if let Some((orientation, _)) = self.data.orientation() {
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

pub struct Movement<'a> {
    data: &'a RobotState,
}

impl<'a> Movement<'a> {
    pub fn new(data: &'a RobotState) -> Self {
        Self { data }
    }
}

impl Widget for &mut Movement<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(vec2(200.0, 0.0), |ui| {
            if let Some((movement, _)) = self.data.movement() {
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
        })
        .response
    }
}

pub struct RawSensorData<'a> {
    data: &'a RobotState,
}

impl<'a> RawSensorData<'a> {
    pub fn new(data: &'a RobotState) -> Self {
        Self { data }
    }
}

impl Widget for &mut RawSensorData<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(vec2(200.0, 0.0), |ui| {
            ui.collapsing("Accelerometer", |ui| {
                if let Some((inertial, _)) = self.data.inertial() {
                    ui.label(format!("X: {}", inertial.accel_x));
                    ui.label(format!("Y: {}", inertial.accel_y));
                    ui.label(format!("Z: {}", inertial.accel_z));
                    // TODO visual
                } else {
                    ui.label("No accelerometer data");
                }
            });
            ui.collapsing("Gyro", |ui| {
                if let Some((inertial, _)) = self.data.inertial() {
                    ui.label(format!("X: {}", inertial.gyro_x));
                    ui.label(format!("Y: {}", inertial.gyro_y));
                    ui.label(format!("Z: {}", inertial.gyro_z));
                    // TODO visual
                } else {
                    ui.label("No gyro data");
                }
            });
            ui.collapsing("Depth", |ui| {
                if let Some((depth, _)) = self.data.depth() {
                    ui.label(format!("Depth: {}", depth.depth));
                    ui.label(format!("Temp: {}", depth.temperature));
                } else {
                    ui.label("No depth data");
                }
                if let Some((target, _)) = self.data.depth_target() {
                    ui.label(format!("Depth Target: {target}"));
                } else {
                    ui.label("Depth Target: None");
                }
            });
        })
        .response
    }
}

pub struct Motors<'a> {
    data: &'a RobotState,
}

impl<'a> Motors<'a> {
    pub fn new(data: &'a RobotState) -> Self {
        Self { data }
    }
}

impl Widget for &mut Motors<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(vec2(200.0, 0.0), |ui| {
            for (motor, (MotorFrame(speed), _)) in self.data.motors().iter() {
                ui.label(format!("{motor:?}: {speed}"));
            }
            // TODO maybe draw thrust diagram
        })
        .response
    }
}

pub struct Cameras<'a> {
    data: &'a Store,
}

impl<'a> Cameras<'a> {
    pub fn new(data: &'a Store) -> Self {
        Self { data }
    }
}

impl Widget for &mut Cameras<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.allocate_ui(vec2(200.0, 0.0), |ui| {
            if let Some(Value::Cameras(cameras)) = self.data.get(&Key::Cameras) {
                for (name, addrs) in cameras {
                    ui.label(format!("{name}: {addrs}"));
                    // TODO Maybe show preview
                }
            }
        })
        .response
    }
}

pub struct Video<'a> {
    name: &'a str,
    pub should_delete: bool, // TODO
}

impl<'a> Video<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
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
                ui.label("end");
                ui.allocate_space(ui.available_size());
                // Todo
            });
        })
        .response
    }
}

// TODO
