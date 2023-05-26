use std::mem;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Context;
use bevy::prelude::Entity;
use bevy::prelude::FromWorld;
use bevy::{
    app::AppExit,
    prelude::{Commands, World},
};
use common::store::Token;
use common::types::DepthControlMode;
use common::types::DepthCorrection;
use common::types::LevelingCorrection;
use common::types::LevelingMode;
use common::types::MovementOverride;
use common::types::Percent;
use common::types::PidConfig;
use common::types::PidResult;
use common::types::RobotStatus;
use common::{
    error::LogErrorExt,
    protocol::Protocol,
    store::tokens,
    types::{
        Camera, Celsius, DepthFrame, InertialFrame, MagFrame, MotorFrame, MotorId, Movement,
        Orientation, SystemInfo,
    },
};
use egui::ComboBox;
use egui::Direction;
use egui::Slider;
use egui::{vec2, Align, Layout};
use egui::{Color32, Frame};
use egui_extras::{Column, TableBuilder};
use fxhash::FxHashMap as HashMap;
use glam::EulerRot;
use glam::Quat;
use std::net::ToSocketAddrs;
use tracing::error;

use crate::plugins::gamepad::CurrentGamepad;
use crate::plugins::notification::NotificationResource;
use crate::plugins::orientation::OrientationDisplay;
use crate::plugins::robot::Updater;
use crate::plugins::video::pipeline::MatId;
use crate::plugins::video::pipeline::PipelineStage;
use crate::plugins::video::VideoCamera;
use crate::plugins::video::VideoCaptureFrames;
use crate::plugins::video::VideoCaptureMovement;
use crate::plugins::video::VideoCaptureMovementEnabled;
use crate::plugins::video::VideoCapturePipeline;
use crate::plugins::video::VideoCaptureThread;
use crate::plugins::video::VideoMessage;
use crate::plugins::video::VideoSink;
use crate::plugins::video::VideoSinkMarker;
use crate::plugins::video::VideoSinkMat;
use crate::plugins::video::VideoSinkPeer;
use crate::plugins::video::VideoSinkRemove;
use crate::plugins::video::VideoSinkTexture;
use crate::plugins::video::VideoTree;
use crate::plugins::{
    networking::NetworkEvent, notification::Notification, robot::Robot, ui::UiComponent,
};

use super::widgets::MovementWidget;
use super::widgets::PidWidget;
use super::{panes, ExtensionId, PaneId, UiMessage, UiMessages};

const TABLE_ROW_HEIGHT: f32 = 15.0;

#[derive(Debug, Default)]
pub struct MenuBar;

impl UiComponent for MenuBar {
    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, commands: &mut Commands) {
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "File", |ui| {
                if ui.button("Quit").clicked() {
                    commands.add(|world: &mut World| {
                        world.send_event(AppExit);
                    });
                }
            });
            egui::menu::menu_button(ui, "Networking", |ui| {
                if ui.button("Connect").clicked() {
                    commands.add(|world: &mut World| {
                        if let Some(ui) = world.get_resource::<UiMessages>() {
                            let id = rand::random();
                            ui.0.try_send(UiMessage::OpenPanel(
                                PaneId::Extension(id),
                                panes::connect_window(id, ui.0.clone()),
                            ))
                            .log_error("Connect to robot");
                        } else {
                            error!("No UiMessage resource found");
                        }
                    });
                }
                if ui.button("Resync").clicked() {
                    commands.add(move |world: &mut World| {
                        world.send_event(NetworkEvent::SendPacket(Protocol::RequestSync));
                    });
                }
            });
            egui::menu::menu_button(ui, "PID", |ui| {
                if ui.button("Tune Leveling PID").clicked() {
                    commands.add(|world: &mut World| {
                        if let Some(ui) = world.get_resource::<UiMessages>() {
                            let id = rand::random();
                            ui.0.try_send(UiMessage::OpenPanel(
                                PaneId::Extension(id),
                                panes::leveling_pid_window(id, ui.0.clone()),
                            ))
                            .log_error("Open leveling tuner");
                        } else {
                            error!("No UiMessage resource found");
                        }
                    });
                }
                if ui.button("Tune Depth PID").clicked() {
                    commands.add(|world: &mut World| {
                        if let Some(ui) = world.get_resource::<UiMessages>() {
                            let id = rand::random();
                            ui.0.try_send(UiMessage::OpenPanel(
                                PaneId::Extension(id),
                                panes::depth_pid_window(id, ui.0.clone()),
                            ))
                            .log_error("Open depth tuner");
                        } else {
                            error!("No UiMessage resource found");
                        }
                    });
                }
            });
            egui::menu::menu_button(ui, "Motors", |ui| {
                if ui.button("Arm Robot").clicked() {
                    commands.add(|world: &mut World| {
                        if let Some(mut robot) = world.get_resource_mut::<Robot>() {
                            robot.arm();
                        } else {
                            error!("No robot resource");
                        }
                    });
                }
                if ui.button("Disarm Robot").clicked() {
                    commands.add(|world: &mut World| {
                        if let Some(mut robot) = world.get_resource_mut::<Robot>() {
                            robot.disarm();
                        } else {
                            error!("No robot resource");
                        }
                    });
                }
                if ui.button("Motor overrides").clicked() {
                    commands.add(|world: &mut World| {
                        if let Some(ui) = world.get_resource::<UiMessages>() {
                            let id = rand::random();
                            ui.0.try_send(UiMessage::OpenPanel(
                                PaneId::Extension(id),
                                panes::motor_override_window(id, ui.0.clone()),
                            ))
                            .log_error("Open motor window");
                        } else {
                            error!("No UiMessage resource found");
                        }
                    });
                }
            });
            egui::menu::menu_button(ui, "View", |ui| {
                if ui.button("Orientation").clicked() {
                    commands.add(|world: &mut World| {
                        if let Some(ui) = world.get_resource::<UiMessages>() {
                            let id = rand::random();
                            ui.0.try_send(UiMessage::OpenPanel(
                                PaneId::Extension(id),
                                panes::orientation_display_window(id, ui.0.clone()),
                            ))
                            .log_error("Open orientation display");
                        } else {
                            error!("No UiMessage resource found");
                        }
                    });
                }
                if ui.button("Video").clicked() {
                    commands.add(|world: &mut World| {
                        if let Some(ui) = world.get_resource::<UiMessages>() {
                            let id = rand::random();
                            ui.0.try_send(UiMessage::OpenPanel(
                                PaneId::Extension(id),
                                panes::video_window(id, ui.0.clone()),
                            ))
                            .log_error("Open video display");
                        } else {
                            error!("No UiMessage resource found");
                        }
                    });
                }
            });
            egui::menu::menu_button(ui, "Debug", |ui| {
                if ui.button("Egui Settings").clicked() {
                    commands.add(|world: &mut World| {
                        if let Some(ui) = world.get_resource::<UiMessages>() {
                            let id = rand::random();
                            ui.0.try_send(UiMessage::OpenPanel(
                                PaneId::Extension(id),
                                panes::debug_egui_window(id, ui.0.clone()),
                            ))
                            .log_error("Open egui debugger");
                        } else {
                            error!("No UiMessage resource found");
                        }
                    });
                }
            });
        });
    }
}

#[derive(Debug, Default)]
pub struct StatusBar {
    status: Option<Arc<RobotStatus>>,
    leak: Option<Arc<bool>>,
    leveling: Option<Arc<LevelingMode>>,
    depth: Option<Arc<DepthControlMode>>,
    movement_override: Option<Arc<MovementOverride>>,
}

impl UiComponent for StatusBar {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        let Some(robot) = world.get_resource::<Robot>() else {
            return;
        };
        self.status = robot.store().get(&tokens::STATUS);
        self.leak = robot.store().get(&tokens::LEAK);
        self.leveling = robot.store().get(&tokens::LEVELING_MODE);
        self.depth = robot.store().get(&tokens::DEPTH_CONTROL_MODE);
        self.movement_override = robot.store().get(&tokens::MOVEMENT_OVERRIDE);
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        ui.horizontal_wrapped(|ui| {
            if let Some(ref status) = self.status {
                let color = match &**status {
                    RobotStatus::Moving(_) => Color32::LIGHT_GREEN,
                    RobotStatus::Ready => Color32::GREEN,
                    RobotStatus::Disarmed => Color32::RED,
                    RobotStatus::NoPeer => Color32::LIGHT_BLUE,
                };
                ui.colored_label(color, format!("Status: {status:?}"));
            } else {
                ui.label("No status data");
            }

            if let Some(ref leak) = self.leak {
                let color = if **leak { Color32::RED } else { Color32::GREEN };
                ui.colored_label(color, format!("Leak detected: {leak:?}"));
            } else {
                ui.label("No leak data");
            }

            if let Some(ref leveling) = self.leveling {
                let color = if matches!(**leveling, LevelingMode::Enabled(_)) {
                    Color32::GREEN
                } else {
                    Color32::BLUE
                };
                ui.colored_label(color, format!("Leveling: {leveling:?}"));
            } else {
                ui.label("No leveling data");
            }

            if let Some(ref depth_control) = self.depth {
                let color = if matches!(**depth_control, DepthControlMode::Enabled(_)) {
                    Color32::GREEN
                } else {
                    Color32::BLUE
                };
                ui.colored_label(color, format!("Depth control: {depth_control:?}"));
            } else {
                ui.label("No depth control data");
            }

            if let Some(_) = self.movement_override {
                ui.colored_label(Color32::RED, format!("Movement Override is SET"));
            } else {
                ui.label("No movement override");
            }
        });
    }
}

#[derive(Debug, Default)]
pub struct RemoteSystemUi(Option<Arc<SystemInfo>>);

impl UiComponent for RemoteSystemUi {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        let Some(robot) = world.get_resource::<Robot>() else {
            return;
        };
        self.0 = robot.store().get(&tokens::SYSTEM_INFO);
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        ui.collapsing("Remote System", |ui| {
            if let Some(ref hw_state) = self.0 {
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
                        .columns(Column::remainder().clip(false).resizable(true), 3)
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
                        .resizable(true)
                        .max_scroll_height(500.0)
                        .column(Column::auto())
                        .columns(Column::remainder().clip(false).resizable(true), 4)
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
                    TableBuilder::new(ui)
                        .striped(true)
                        .columns(Column::remainder().clip(false).resizable(true), 7)
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
                        .columns(Column::remainder().clip(false).resizable(true), 4)
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
                        .columns(Column::remainder().clip(false).resizable(true), 5)
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
                ui.label("No data");
            }
        });
    }
}

#[derive(Debug, Default)]
pub struct OrientationUi(Option<Arc<Orientation>>);

impl UiComponent for OrientationUi {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        let Some(robot) = world.get_resource::<Robot>() else {
            return;
        };
        self.0 = robot.store().get(&tokens::ORIENTATION);
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        ui.collapsing("Orientation", |ui| {
            if let Some(ref orientation) = self.0 {
                let orientation = Quat::from(orientation.0);
                let (yaw, pitch, roll) = orientation.to_euler(EulerRot::ZXY);
                ui.label(format!("Yaw: {:.3}", yaw.to_degrees()));
                ui.label(format!("Pitch: {:.3}", pitch.to_degrees()));
                ui.label(format!("Roll: {:.3}", roll.to_degrees()));

                // TODO visual
            } else {
                ui.label("No orientation data");
            }
        });
    }
}

#[derive(Debug, Default)]
pub struct MovementUi {
    calculated: Option<Arc<Movement>>,
    joystick: Option<Arc<Movement>>,
    opencv: Option<Arc<Movement>>,
    leveling: Option<Arc<Movement>>,
    depth: Option<Arc<Movement>>,
}

impl UiComponent for MovementUi {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        let Some(robot) = world.get_resource::<Robot>() else {
            return;
        };
        self.calculated = robot.store().get(&tokens::MOVEMENT_CALCULATED);
        self.joystick = robot.store().get(&tokens::MOVEMENT_JOYSTICK);
        self.opencv = robot.store().get(&tokens::MOVEMENT_OPENCV);
        self.leveling = robot.store().get(&tokens::MOVEMENT_LEVELING);
        self.depth = robot.store().get(&tokens::MOVEMENT_DEPTH);
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        ui.collapsing("Movement", |ui| {
            if let Some(ref movement) = self.calculated {
                ui.add(MovementWidget(movement));
            } else {
                ui.label("No movement data");
            }
            if let Some(ref movement) = self.joystick {
                ui.collapsing("Joystick", |ui| {
                    ui.add(MovementWidget(movement));
                });
            }
            if let Some(ref movement) = self.opencv {
                ui.collapsing("Open CV", |ui| {
                    ui.add(MovementWidget(movement));
                });
            }
            if let Some(ref movement) = self.leveling {
                ui.collapsing("Leveling Correction", |ui| {
                    ui.add(MovementWidget(movement));
                });
            }
            if let Some(ref movement) = self.depth {
                ui.collapsing("Depth Correction", |ui| {
                    ui.add(MovementWidget(movement));
                });
            }
        });
    }
}

#[derive(Debug, Default)]
pub struct RawSensorDataUi {
    inertial: Option<Arc<InertialFrame>>,
    magnetic: Option<Arc<MagFrame>>,
    depth: Option<Arc<DepthFrame>>,
}

impl UiComponent for RawSensorDataUi {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        let Some(robot) = world.get_resource::<Robot>() else {
            return;
        };
        self.inertial = robot.store().get(&tokens::RAW_INERTIAL);
        self.magnetic = robot.store().get(&tokens::RAW_MAGNETIC);
        self.depth = robot.store().get(&tokens::RAW_DEPTH);
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        ui.collapsing("Sensors", |ui| {
            ui.collapsing("Imu", |ui| {
                if let Some(ref inertial) = self.inertial {
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
                if let Some(ref mag) = self.magnetic {
                    ui.label("Mag");
                    ui.label(format!("X: {}", mag.mag_x));
                    ui.label(format!("Y: {}", mag.mag_y));
                    ui.label(format!("Z: {}", mag.mag_z));

                    // TODO visual
                } else {
                    ui.label("No magnetometer data");
                }
            });
            ui.collapsing("Depth", |ui| {
                if let Some(ref depth) = self.depth {
                    ui.label(format!("Pressure: {}", depth.pressure));
                    ui.label(format!("Depth: {}", depth.depth));
                    ui.label(format!("Attitude: {}", depth.altitude));
                    ui.label(format!("Temp: {}", depth.temperature));
                } else {
                    ui.label("No depth data");
                }
            });
        });
    }
}

#[derive(Debug, Default)]
pub struct MotorsUi(Option<Arc<HashMap<MotorId, MotorFrame>>>);

impl UiComponent for MotorsUi {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        let Some(robot) = world.get_resource::<Robot>() else {
            return;
        };
        self.0 = robot.store().get(&tokens::MOTOR_SPEED);
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        ui.collapsing("Motors", |ui| {
            if let Some(ref speeds) = self.0 {
                let mut speeds: Vec<(_, _)> = speeds.iter().collect();
                speeds.sort_by_key(|(name, _)| format!("{name:?}"));

                TableBuilder::new(ui)
                    .striped(true)
                    .columns(Column::remainder().clip(false).resizable(true), 2)
                    .header(TABLE_ROW_HEIGHT, |mut row| {
                        row.col(|ui| {
                            ui.label("Motor");
                        });
                        row.col(|ui| {
                            ui.label("Speed");
                        });
                    })
                    .body(|body| {
                        body.rows(TABLE_ROW_HEIGHT, speeds.len(), |idx, mut row| {
                            let (name, speed) = speeds[idx];

                            row.col(|ui| {
                                ui.label(format!("{name:?}"));
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.2?}", speed.to_f64()));
                            });
                        });
                    });
            } else {
                ui.label("No motor data");
            }
            // TODO maybe draw thrust diagram
        });
    }
}

#[derive(Debug, Default)]
pub struct CamerasUi(Option<Arc<Vec<Camera>>>);

impl UiComponent for CamerasUi {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        let Some(robot) = world.get_resource::<Robot>() else {
            return;
        };
        self.0 = robot.store().get(&tokens::CAMERAS);
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        ui.collapsing("Cameras", |ui| {
            if let Some(ref cameras) = self.0 {
                for Camera { name, location } in &**cameras {
                    ui.label(format!("{name}: {location}"));
                    // TODO Maybe show preview
                }
            } else {
                ui.label("No cameras found");
            }
        });
    }
}

#[derive(Debug, Default)]
pub struct InputUi(Option<CurrentGamepad>);

impl UiComponent for InputUi {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        self.0 = world.get_resource::<CurrentGamepad>().cloned();
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        ui.collapsing("Input", |ui| {
            if let Some(ref gamepad) = self.0 {
                ui.label(format!("Gamepad id: {}", gamepad.0.id));
                ui.label(format!("Selected servo: {:?}", gamepad.1.servo));
                ui.label(format!("Selected map: {:?}", gamepad.1.selected_map));
                ui.label(format!("Gain: {:.2?}", gamepad.1.gain));
                ui.label(format!("Hold: {:?}", gamepad.1.hold_axis));

                ui.collapsing("Joystick Calculated", |ui| {
                    ui.add(MovementWidget(&gamepad.1.movement));
                    ui.group(|ui| {
                        ui.label(format!(
                            "Servo Normal: {:.2?}",
                            gamepad.1.servo_position_normal
                        ));
                        ui.label(format!(
                            "Servo Inverted: {:.2?}",
                            gamepad.1.servo_position_inverted
                        ));
                        ui.label(format!(
                            "Servo Calculated: {:.2?}",
                            gamepad.1.servo_position_normal - gamepad.1.servo_position_inverted
                        ));
                        ui.allocate_space(vec2(ui.available_width(), 0.0));
                    })
                });
                ui.collapsing("Selected Map", |ui| {
                    if let Some(map) = gamepad.1.maps.get(gamepad.1.selected_map) {
                        let mut mappings: Vec<(_, _)> = map.iter().collect();
                        mappings.sort_by_key(|(button, _)| format!("{button:?}"));

                        TableBuilder::new(ui)
                            .striped(true)
                            .columns(Column::remainder().clip(false).resizable(true), 2)
                            .header(TABLE_ROW_HEIGHT, |mut row| {
                                row.col(|ui| {
                                    ui.label("Button");
                                });
                                row.col(|ui| {
                                    ui.label("Action");
                                });
                            })
                            .body(|body| {
                                body.rows(TABLE_ROW_HEIGHT, mappings.len(), |idx, mut row| {
                                    let (button, action) = mappings[idx];

                                    row.col(|ui| {
                                        ui.label(format!("{button:?}"));
                                    });
                                    row.col(|ui| {
                                        ui.label(format!("{action:?}"));
                                    });
                                });
                            });
                    }
                });
            } else {
                ui.label("No gamepad found");
            }
        });
    }
}

#[derive(Debug)]
pub struct ConnectUi(String, ExtensionId);

impl ConnectUi {
    pub fn new(id: ExtensionId) -> Self {
        Self(Default::default(), id)
    }
}

impl UiComponent for ConnectUi {
    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, commands: &mut Commands) {
        ui.text_edit_singleline(&mut self.0);
        if !ui.button("Connect").clicked() {
            return;
        }

        // TODO this is slow and should be async
        match (self.0.as_str(), 44444)
            .to_socket_addrs()
            .context("Create socket addrs")
            .and_then(|mut it| {
                it.find(|it| it.is_ipv4())
                    .ok_or_else(|| anyhow!("No Socket address found"))
            }) {
            Ok(remote) => {
                let id = self.1;
                commands.add(move |world: &mut World| {
                    world.send_event(NetworkEvent::ConnectTo(remote));
                    world
                        .resource::<UiMessages>()
                        .0
                        .try_send(UiMessage::ClosePanel(PaneId::Extension(id)))
                        .log_error("Close connetion window");
                });
            }
            Err(error) => {
                commands.add(|world: &mut World| {
                    world.send_event(Notification::Error(
                        "Could not resolve address".to_owned(),
                        error,
                    ));
                });
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct VideoUi {
    tree: VideoTree,
    sinks: HashMap<
        Entity,
        (
            VideoCamera,
            VideoSinkMat,
            Option<VideoSinkTexture>,
            Option<VideoSinkPeer>,
            Option<VideoCapturePipeline>,
            Option<VideoCaptureFrames>,
            Option<VideoCaptureMovement>,
            Option<VideoCaptureMovementEnabled>,
        ),
    >,
    cameras: Option<Arc<Vec<Camera>>>,
}

impl VideoUi {
    fn render(&mut self, cmds: &mut Commands, ui: &mut egui::Ui, tree: &mut VideoTree) -> bool {
        let mut should_remove = false;

        match tree {
            VideoTree::Node(a, b) => {
                let available = ui.available_size();
                let (layout, size) = if available.x > available.y {
                    (
                        Layout::left_to_right(Align::LEFT),
                        vec2(available.x / 2.0 - 4.0, available.y),
                    )
                } else {
                    (
                        Layout::top_down(Align::LEFT),
                        vec2(available.x, available.y / 2.0 - 4.0),
                    )
                };

                let remove = ui
                    .with_layout(layout, |ui| {
                        let remove_a = ui
                            .allocate_ui(size, |ui| {
                                ui.set_min_size(size);
                                self.render(cmds, ui, a)
                            })
                            .inner;
                        let remove_b = ui
                            .allocate_ui(size, |ui| {
                                ui.set_min_size(size);
                                self.render(cmds, ui, b)
                            })
                            .inner;

                        (remove_a, remove_b)
                    })
                    .inner;

                match remove {
                    (true, true) => *tree = VideoTree::Empty,
                    (true, false) => *tree = mem::take(b),
                    (false, true) => *tree = mem::take(a),
                    (false, false) => {}
                }
            }
            VideoTree::Leaf(entity) => {
                // Get around some silly rust rule
                let entity = *entity;
                if let Some((
                    camera,
                    mat,
                    texture,
                    peer,
                    pipeline,
                    frames,
                    movement,
                    movement_enabled,
                )) = self.sinks.get(&entity)
                {
                    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.group(|ui| {
                                    ui.push_id(entity, |ui| {
                                        ComboBox::from_id_source("name")
                                            .selected_text(&format!("{:?}", camera.0.name))
                                            .show_ui(ui, |ui| {
                                                if let Some(ref cameras) = self.cameras {
                                                    for camera_option in &**cameras {
                                                        if ui
                                                            .selectable_label(
                                                                camera_option == &camera.0,
                                                                &format!(
                                                                    "{:?}",
                                                                    camera_option.name
                                                                ),
                                                            )
                                                            .clicked()
                                                        {
                                                            cmds.entity(entity).insert(
                                                                VideoCamera(camera_option.clone()),
                                                            );
                                                        }
                                                    }
                                                }
                                            });
                                        ComboBox::from_id_source("mat")
                                            .selected_text(&format!("{:?}", mat.0))
                                            .show_ui(ui, |ui| {
                                                if let Some(ref frames) = frames {
                                                    for available_mat in &frames.1 {
                                                        if ui
                                                            .selectable_label(
                                                                available_mat == &mat.0,
                                                                &format!("{:?}", available_mat),
                                                            )
                                                            .clicked()
                                                        {
                                                            cmds.entity(entity).insert(
                                                                VideoSinkMat(*available_mat),
                                                            );
                                                        }
                                                    }
                                                }
                                            });
                                        ComboBox::from_id_source("pipeline")
                                            .selected_text("Pipeline")
                                            .show_ui(ui, |ui| {
                                                if let (Some(ref pipeline), Some(ref peer)) =
                                                    (pipeline, peer)
                                                {
                                                    for stage in PipelineStage::all() {
                                                        let selected = pipeline.0.contains(&stage);

                                                        if ui
                                                            .selectable_label(
                                                                selected,
                                                                &format!("{:?}", stage),
                                                            )
                                                            .clicked()
                                                        {
                                                            if !selected {
                                                                // Add this stage
                                                                let stage = stage;
                                                                let peer = peer.0;
                                                                cmds.add(move |world: &mut World| {
                                                                    if let Some(mut peer) = world.get_entity_mut(peer) {
                                                                        if let Some(mut pipeline) = peer.get_mut::<VideoCapturePipeline>() {
                                                                            pipeline.0.push(stage);
                                                                            pipeline.0.sort();
                                                                        }
                                                                    }
                                                                });
                                                            } else {
                                                                // Remove this stage
                                                                let stage = stage;
                                                                let peer = peer.0;
                                                                cmds.add(move |world: &mut World| {
                                                                    if let Some(mut peer) = world.get_entity_mut(peer) {
                                                                        if let Some(mut pipeline) = peer.get_mut::<VideoCapturePipeline>() {
                                                                            pipeline.0.drain_filter(|it| it == &stage);
                                                                        }
                                                                    }
                                                                });
                                                            }
                                                        }
                                                    }
                                                }
                                            });
                                        if ui.selectable_label(movement_enabled.is_some(), "Movement").on_hover_ui(|ui| {
                                            ui.set_max_width(100.0);
                                            if let Some(movement) = movement {
                                                ui.add(MovementWidget(&movement.0));
                                            } else {
                                                ui.label("No movement");
                                            }
                                        }).clicked() {
                                            if let Some(peer) = peer {
                                                match movement_enabled {
                                                    Some(_) => {
                                                        cmds.entity(peer.0).remove::<VideoCaptureMovementEnabled>();
                                                    },
                                                    None => {
                                                        cmds.entity(peer.0).insert(VideoCaptureMovementEnabled);
                                                    },
                                                }
                                            }
                                        }
                                        if ui.small_button("Capture").clicked() {
                                            if let Some(peer) = peer {
                                                let peer = peer.0;
                                                let mat = mat.0;
                                                cmds.add(move |world: &mut World| {
                                                    if let Some(peer) = world.get_entity(peer) {
                                                        if let Some(thread) = peer.get::<VideoCaptureThread>() {
                                                            thread.0.try_send(VideoMessage::SaveFrame(format!("images.png"), mat)).log_error("Request screenshot");
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                        if ui.small_button("Split").clicked() {
                                            let node = mem::take(tree);
                                            *tree = VideoTree::Node(
                                                Box::new(node),
                                                Box::new(VideoTree::Empty),
                                            )
                                        }
                                        if ui.small_button("Close").clicked() {
                                            cmds.entity(entity).insert(VideoSinkRemove);
                                            should_remove = true;
                                        }
                                    });
                                });
                            });

                            if let Some(texture) = texture {
                                ui.with_layout(
                                    Layout::centered_and_justified(Direction::LeftToRight),
                                    |ui| {
                                        let available = ui.available_size();
                                        let x = available.x;
                                        let y = x / 16.0 * 9.0;

                                        ui.image(texture.0, (x, y));
                                    },
                                );
                            } else {
                                ui.label("No video");
                            }
                        });
                    });
                } else {
                    ui.label("Waiting on sink");
                }
            }
            VideoTree::Empty => {
                ui.vertical_centered_justified(|ui| {
                    if let Some(ref cameras) = self.cameras {
                        for camera in &**cameras {
                            if ui.button(&camera.name).clicked() {
                                let sink = cmds
                                    .spawn(VideoSink {
                                        camera: VideoCamera(camera.clone()),
                                        mat: VideoSinkMat(MatId::Camera),
                                        marker: VideoSinkMarker,
                                    })
                                    .id();
                                *tree = VideoTree::Leaf(sink);
                            }
                        }
                    } else {
                        ui.label("No cameras found");
                    }
                });
            }
        }

        should_remove
    }
}

impl UiComponent for VideoUi {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        let mut sinks = HashMap::default();

        for sink in self.tree.entities() {
            if let Some(entity) = world.get_entity(sink) {
                let camera = entity.get::<VideoCamera>().cloned();
                let mat = entity.get::<VideoSinkMat>().cloned();
                let texture = entity.get::<VideoSinkTexture>().cloned();
                let peer = entity.get::<VideoSinkPeer>().cloned();

                let (pipeline, frames, movement, movement_enabled) =
                    if let Some(peer) = peer.as_ref().and_then(|it| world.get_entity(it.0)) {
                        let pipeline = peer.get::<VideoCapturePipeline>().cloned();
                        let frames = peer.get::<VideoCaptureFrames>().cloned();
                        let movement = peer.get::<VideoCaptureMovement>().cloned();
                        let movement_enabled = peer.get::<VideoCaptureMovementEnabled>().cloned();

                        (pipeline, frames, movement, movement_enabled)
                    } else {
                        (None, None, None, None)
                    };

                if let Some((camera, mat)) = Option::zip(camera, mat) {
                    sinks.insert(
                        sink,
                        (
                            camera,
                            mat,
                            texture,
                            peer,
                            pipeline,
                            frames,
                            movement,
                            movement_enabled,
                        ),
                    );
                } else {
                    error!("Found invalid sink");
                }
            } else {
                error!("Could not find sink");
            }
        }

        self.sinks = sinks;

        let Some(robot) = world.get_resource::<Robot>() else {
            return;
        };
        self.cameras = robot.store().get(&tokens::CAMERAS);
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, commands: &mut Commands) {
        // The borrow checker is fun
        let mut tree = mem::take(&mut self.tree);

        let should_remove = self.render(commands, ui, &mut tree);

        if !should_remove {
            self.tree = tree;
        } else {
            self.tree = VideoTree::Empty;
        }
    }
}

#[derive(Debug, Default)]
pub struct NotificationUi(Option<NotificationResource>);

impl UiComponent for NotificationUi {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        self.0 = world.get_resource::<NotificationResource>().cloned();
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        if let Some(ref notifs) = self.0 {
            for (notif, _) in &notifs.0 {
                ui.allocate_space(vec2(0.0, 5.0));

                Frame::popup(ui.style()).show(ui, |ui| {
                    ui.heading(&notif.title);
                    ui.label(&notif.description);
                });
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct OrientationDisplayUi(Option<OrientationDisplay>);

impl UiComponent for OrientationDisplayUi {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        self.0 = world
            .get_resource::<OrientationDisplay>()
            .map(|it| it.to_owned());
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        if let Some(ref texture) = self.0 {
            ui.image(texture.1, (512.0, 512.0));
        }
    }
}

#[derive(Default, Debug)]
pub struct DebugEguiUi;

impl UiComponent for DebugEguiUi {
    fn draw(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        ui.collapsing("Memory", |ui| {
            ctx.memory_ui(ui);
        });
        ui.collapsing("Settings", |ui| {
            ctx.settings_ui(ui);
        });
        ui.collapsing("Inspect", |ui| {
            ctx.inspection_ui(ui);
        });
    }
}

#[derive(Debug, Default)]
pub struct PreserveSize;

impl UiComponent for PreserveSize {
    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        ui.allocate_space(ui.available_size());
    }
}

#[derive(Debug)]
pub struct PidEditorUi(PidConfig, Token<PidConfig>);

impl PidEditorUi {
    pub fn new(token: Token<PidConfig>) -> Self {
        Self(Default::default(), token)
    }
}

impl UiComponent for PidEditorUi {
    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, commands: &mut Commands) {
        ui.add(PidWidget(&mut self.0));
        ui.horizontal(|ui| {
            if ui.button("Defaults").clicked() {
                self.0 = Default::default();
            }
            if ui.button("Unset").clicked() {
                let token = self.1.clone();

                commands.add(move |world: &mut World| {
                    Updater::from_world(world).emit_delete(&token);
                });
            }
            if ui.button("Apply").clicked() {
                let config = self.0.clone();
                let token = self.1.clone();

                commands.add(move |world: &mut World| {
                    Updater::from_world(world).emit_update(&token, config);
                });
            }
        });
    }
}

#[derive(Debug, Default)]
pub struct LevelingUi {
    mode: Option<Arc<LevelingMode>>,
    pid_override: Option<Arc<PidConfig>>,
    correction: Option<Arc<LevelingCorrection>>,
    pitch: Option<Arc<PidResult>>,
    roll: Option<Arc<PidResult>>,
    calculated: Option<Arc<Movement>>,
}

impl UiComponent for LevelingUi {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        let Some(robot) = world.get_resource::<Robot>() else {
            return;
        };
        self.mode = robot.store().get(&tokens::LEVELING_MODE);
        self.pid_override = robot.store().get(&tokens::LEVELING_PID_OVERRIDE);
        self.correction = robot.store().get(&tokens::LEVELING_CORRECTION);
        self.pitch = robot.store().get(&tokens::LEVELING_PITCH_RESULT);
        self.roll = robot.store().get(&tokens::LEVELING_ROLL_RESULT);
        self.calculated = robot.store().get(&tokens::MOVEMENT_LEVELING);
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        ui.collapsing("Leveling", |ui| {
            if let Some(ref mode) = self.mode {
                ui.label(format!("Mode: {mode:?}"));
            } else {
                ui.label(format!("No mode set"));
            }

            ui.collapsing("Pid Override", |ui| {
                if let Some(ref pid) = self.pid_override {
                    ui.monospace(format!("{pid:#?}"));
                } else {
                    ui.label(format!("No pid override"));
                }
            });

            ui.collapsing("Correction", |ui| {
                if let Some(ref correction) = self.correction {
                    ui.label(format!("Pitch: {}", correction.pitch));
                    ui.label(format!("Roll: {}", correction.roll));
                } else {
                    ui.label(format!("No correction"));
                }

                ui.collapsing("Pitch", |ui| {
                    if let Some(ref pitch) = self.pitch {
                        ui.monospace(format!("{pitch:#?}"));
                    } else {
                        ui.label(format!("No pitch correction data"));
                    }
                });

                ui.collapsing("Roll", |ui| {
                    if let Some(ref roll) = self.roll {
                        ui.monospace(format!("{roll:#?}"));
                    } else {
                        ui.label(format!("No roll correction data"));
                    }
                });
            });

            ui.collapsing("Calculated Movement", |ui| {
                if let Some(ref calculated) = self.calculated {
                    ui.add(MovementWidget(&calculated));
                } else {
                    ui.label(format!("No movement calculated"));
                }
            });
        });
    }
}

#[derive(Debug, Default)]
pub struct DepthControlUi {
    mode: Option<Arc<DepthControlMode>>,
    pid_override: Option<Arc<PidConfig>>,
    correction: Option<Arc<DepthCorrection>>,
    depth: Option<Arc<PidResult>>,
    calculated: Option<Arc<Movement>>,
}

impl UiComponent for DepthControlUi {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        let Some(robot) = world.get_resource::<Robot>() else {
            return;
        };
        self.mode = robot.store().get(&tokens::DEPTH_CONTROL_MODE);
        self.pid_override = robot.store().get(&tokens::DEPTH_CONTROL_PID_OVERRIDE);
        self.correction = robot.store().get(&tokens::DEPTH_CONTROL_CORRECTION);
        self.depth = robot.store().get(&tokens::DEPTH_CONTROL_RESULT);
        self.calculated = robot.store().get(&tokens::MOVEMENT_DEPTH);
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, _commands: &mut Commands) {
        ui.collapsing("Depth Control", |ui| {
            if let Some(ref mode) = self.mode {
                ui.label(format!("Mode: {mode:?}"));
            } else {
                ui.label(format!("No mode set"));
            }

            ui.collapsing("Pid Override", |ui| {
                if let Some(ref pid) = self.pid_override {
                    ui.monospace(format!("{pid:#?}"));
                } else {
                    ui.label(format!("No pid override"));
                }
            });

            ui.collapsing("Correction", |ui| {
                if let Some(ref correction) = self.correction {
                    ui.label(format!("Depth: {}", correction.depth));
                } else {
                    ui.label(format!("No correction"));
                }

                ui.collapsing("Depth", |ui| {
                    if let Some(ref depth) = self.depth {
                        ui.monospace(format!("{depth:#?}"));
                    } else {
                        ui.label(format!("No depth correction data"));
                    }
                });
            });

            ui.collapsing("Calculated Movement", |ui| {
                if let Some(ref calculated) = self.calculated {
                    ui.add(MovementWidget(&calculated));
                } else {
                    ui.label(format!("No movement calculated"));
                }
            });
        });
    }
}

#[derive(Debug, Default)]
pub struct MovementOverrideUi {
    movement: Option<Arc<MovementOverride>>,
    auto_notify: bool,
}

impl UiComponent for MovementOverrideUi {
    fn pre_draw(&mut self, world: &World, _commands: &mut Commands) {
        let Some(robot) = world.get_resource::<Robot>() else {
            return;
        };
        self.movement = robot.store().get(&tokens::MOVEMENT_OVERRIDE);
    }

    fn draw(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, commands: &mut Commands) {
        let motor_ids = [
            MotorId::FrontLeftBottom,
            MotorId::FrontLeftTop,
            MotorId::FrontRightBottom,
            MotorId::FrontRightTop,
            MotorId::BackLeftBottom,
            MotorId::BackLeftTop,
            MotorId::BackRightBottom,
            MotorId::BackRightTop,
            MotorId::Camera1,
            MotorId::Camera2,
            MotorId::Camera3,
            MotorId::Camera4,
            MotorId::Aux1,
            MotorId::Aux2,
            MotorId::Aux3,
            MotorId::Aux4,
        ];

        if let Some(ref speed_overrides) = self.movement {
            let mut speed_overrides = (**speed_overrides).clone();

            for motor_id in motor_ids {
                let mut speed = speed_overrides
                    .get(&motor_id)
                    .cloned()
                    .unwrap_or(Percent::ZERO)
                    .get()
                    * 100.0;

                ui.add(
                    Slider::new(&mut speed, -100.0..=100.0)
                        .text(format!("{motor_id:?}"))
                        .suffix("%"),
                );

                speed_overrides.insert(motor_id, Percent::new(speed / 100.0));
            }

            if self.auto_notify {
                commands.add(|world: &mut World| {
                    Updater::from_world(world)
                        .emit_update(&tokens::MOVEMENT_OVERRIDE, speed_overrides);
                });
            }
        } else {
            ui.label("No override set");
        }

        ui.horizontal(|ui| {
            ui.toggle_value(&mut self.auto_notify, "Auto apply");
            if ui.button("Add override").clicked() {
                commands.add(|world: &mut World| {
                    Updater::from_world(world)
                        .emit_update(&tokens::MOVEMENT_OVERRIDE, Default::default());
                });
                self.auto_notify = true;
            }
            if ui.button("Remove override").clicked() {
                commands.add(|world: &mut World| {
                    Updater::from_world(world).emit_delete(&tokens::MOVEMENT_OVERRIDE);
                    let mut robot = world
                        .get_resource_mut::<Robot>()
                        .expect("No `Robot` resource");
                    robot.store_mut().remove(&tokens::MOVEMENT_OVERRIDE);
                });
                self.movement = None;
                self.auto_notify = false;
            }
        });
    }
}
