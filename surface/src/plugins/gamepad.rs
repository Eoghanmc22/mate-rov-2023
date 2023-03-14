use std::time::Instant;

use bevy::{
    input::gamepad::{GamepadConnection, GamepadEvent},
    prelude::*,
};
use common::{
    store::tokens,
    types::{Movement, Speed},
};

use super::robot::Updater;

pub struct GamepadPlugin;

impl Plugin for GamepadPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(gamepad_connections.in_base_set(CoreSet::PreUpdate))
            .add_system(gamepad_input);
    }
}

#[derive(Resource)]
struct CurrentGamepad(Gamepad);

/// Listens to the connection and disconnection of gamepads
fn gamepad_connections(
    mut commands: Commands,
    current_gamepad: Option<Res<CurrentGamepad>>,
    mut gamepad_evr: EventReader<GamepadEvent>,
) {
    for event in gamepad_evr.iter() {
        match event {
            GamepadEvent::Connection(event) => match &event.connection {
                GamepadConnection::Connected(info) => {
                    info!(
                        "New gamepad ({}) connected with ID: {:?}",
                        info.name, event.gamepad
                    );

                    if current_gamepad.is_none() {
                        commands.insert_resource(CurrentGamepad(event.gamepad));
                    }
                }
                GamepadConnection::Disconnected => {
                    info!("Lost gamepad connection with ID: {:?}", event.gamepad);

                    if let Some(CurrentGamepad(gamepad_lost)) = current_gamepad.as_deref() {
                        if *gamepad_lost == event.gamepad {
                            commands.remove_resource::<CurrentGamepad>();
                        }
                    }
                }
            },
            GamepadEvent::Button(_) => {}
            GamepadEvent::Axis(_) => {}
        }
    }
}

/// Processes gamepad input and adds it to the global store
fn gamepad_input(
    axes: Res<Axis<GamepadAxis>>,
    buttons: Res<Input<GamepadButton>>,
    current_gamepad: Option<Res<CurrentGamepad>>,
    updater: Local<Updater>,
) {
    if let Some(gamepad) = current_gamepad {
        let axis_lx = GamepadAxis::new(gamepad.0, GamepadAxisType::LeftStickX);
        let axis_ly = GamepadAxis::new(gamepad.0, GamepadAxisType::LeftStickY);
        let axis_rx = GamepadAxis::new(gamepad.0, GamepadAxisType::RightStickX);
        let axis_ry = GamepadAxis::new(gamepad.0, GamepadAxisType::RightStickY);

        let up_button = GamepadButton::new(gamepad.0, GamepadButtonType::RightTrigger);
        let down_button = GamepadButton::new(gamepad.0, GamepadButtonType::LeftTrigger);

        if let (Some(lx), Some(ly), Some(rx), Some(ry)) = (
            axes.get(axis_lx),
            axes.get(axis_ly),
            axes.get(axis_rx),
            axes.get(axis_ry),
        ) {
            let ry = {
                let up = if buttons.pressed(up_button) { 1.0 } else { 0.0 };
                let down = if buttons.pressed(down_button) {
                    -1.0
                } else {
                    0.0
                };
                up + down + ry
            };

            let movement = Movement {
                x: Speed::new(rx as f64),
                y: Speed::new(ly as f64),
                z: Speed::new(ry as f64),
                x_rot: Speed::new(0.0),
                y_rot: Speed::new(0.0),
                z_rot: Speed::new(lx as f64),
            };

            updater.emit_update(&tokens::MOVEMENT_JOYSTICK, (movement, Instant::now()));
        }
    }
}
