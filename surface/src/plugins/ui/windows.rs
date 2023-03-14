//! Egui window renders
//! Api not final

use anyhow::anyhow;
use anyhow::Context;
use bevy::prelude::*;
use egui::Ui;
use std::net::ToSocketAddrs;

use crate::plugins::{networking::NetworkEvent, notification::Notification};

use super::Renderable;

#[derive(Default)]
pub struct ConnectionWindow {
    typed: String,
}

impl Renderable for ConnectionWindow {
    fn render(&mut self, ui: &mut Ui, cmds: &mut Commands, entity: Entity) {
        ui.text_edit_singleline(&mut self.typed);
        if !ui.button("Connect").clicked() {
            return;
        }
        // TODO this logic should be else where
        match (self.typed.as_str(), 44444)
            .to_socket_addrs()
            .context("Create socket addrs")
            .and_then(|mut it| {
                it.filter(|it| it.is_ipv4())
                    .next()
                    .ok_or_else(|| anyhow!("No Socket address found"))
            }) {
            Ok(remote) => {
                cmds.add(move |world: &mut World| {
                    world.send_event(NetworkEvent::ConnectTo(remote));
                    world.despawn(entity);
                });
            }
            Err(error) => {
                cmds.add(|world: &mut World| {
                    world.send_event(Notification::Error(
                        "Could not resolve address".to_owned(),
                        error,
                    ));
                });
            }
        }
    }
}
