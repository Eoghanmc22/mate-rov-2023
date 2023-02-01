mod elements;
pub mod widgets;

use crate::plugins::networking::NetworkEvent;
use crate::plugins::robot::Robot;
use anyhow::{anyhow, Context};
use bevy::prelude::*;
use bevy_egui::{EguiContext, EguiPlugin};
use message_io::network::RemoteAddr;
use std::net::ToSocketAddrs;

use super::notification::Notification;

// todo Display errors

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

    elements::menu_bar(ctx, &mut cmd, state, store);
    elements::side_bar(ctx, &mut cmd, state, store);
    elements::top_panel(ctx, &mut cmd, state, store);
}

fn draw_connection_window(
    mut cmd: Commands,
    mut window: Option<ResMut<ConnectWindow>>,
    mut egui_context: ResMut<EguiContext>,
    mut net: EventWriter<NetworkEvent>,
    mut errors: EventWriter<Notification>,
) {
    let ctx = egui_context.ctx_mut();

    if let Some(ref mut window) = window {
        egui::Window::new("Connection").show(ctx, |ui| {
            ui.text_edit_singleline(&mut window.0);
            if ui.button("Connect").clicked() {
                println!("Hit");
                match (window.0.as_str(), 44444)
                    .to_socket_addrs()
                    .context("Create socket addrs")
                    .and_then(|it| {
                        it.filter(|it| it.is_ipv4())
                            .map(RemoteAddr::Socket)
                            .next()
                            .ok_or_else(|| anyhow!("No Socket address found"))
                    }) {
                    Ok(remote) => {
                        println!("\"{:?}\"", remote);
                        net.send(NetworkEvent::ConnectTo(remote));
                        cmd.remove_resource::<ConnectWindow>();
                    }
                    Err(error) => {
                        errors.send(Notification::Error(
                            "Could not resolve address".to_owned(),
                            error,
                        ));
                    }
                }
            }
        });
    }
}
