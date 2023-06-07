use std::collections::HashMap;

use bevy::{
    input::gamepad::{
        GamepadAxisChangedEvent, GamepadButtonChangedEvent, GamepadConnection,
        GamepadConnectionEvent,
    },
    prelude::*,
};
use common::{
    store::tokens,
    types::{DepthControlMode, LevelingMode, Meters, MotorId, Movement, Percent},
};

use super::robot::{Robot, Updater};

pub struct GamepadPlugin;

impl Plugin for GamepadPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(gamepad_connections.in_base_set(CoreSet::PreUpdate));
        app.add_system(gamepad_buttons.in_base_set(CoreSet::PreUpdate));
        app.add_system(gamepad_axis.in_base_set(CoreSet::PreUpdate));
        app.add_system(emit_updates.in_schedule(CoreSchedule::FixedUpdate));
    }
}

#[derive(Resource, Clone, Debug)]
pub struct CurrentGamepad(pub Gamepad, pub InputState);

#[derive(Clone, Debug)]
pub struct InputState {
    pub movement: Movement,
    pub servo: MotorId,

    pub maps: ControllerMappings,
    pub selected_map: &'static str,

    pub gain: f32,
    pub hold_axis: bool,

    pub servo_position_normal: f32,
    pub servo_position_inverted: f32,
}

impl InputState {
    pub fn handle_event(&mut self, input: Input, value: f32, commands: &mut Commands) {
        let Some(map) = self.maps.get(&self.selected_map) else {
            error!("Map {} not found!", self.selected_map);

            return;
        };

        if let Some(action) = map.get(&input) {
            match action {
                Action::Arm => {
                    if value == 0.0 {
                        return;
                    }

                    commands.add(|world: &mut World| {
                        if let Some(mut robot) = world.get_resource_mut::<Robot>() {
                            robot.arm();
                        } else {
                            error!("No robot resource");
                        }
                    });
                }
                Action::Disarm => {
                    if value == 0.0 {
                        return;
                    }

                    commands.add(|world: &mut World| {
                        if let Some(mut robot) = world.get_resource_mut::<Robot>() {
                            robot.disarm();
                        } else {
                            error!("No robot resource");
                        }
                    });
                }
                Action::SetControlMapping(name) => {
                    if value == 0.0 {
                        return;
                    }

                    self.selected_map = name;
                }
                Action::CenterServo => {
                    if value == 0.0 {
                        return;
                    }

                    self.movement.set_by_id(self.servo, Percent::ZERO);
                }
                Action::SelectServoIncrement => {
                    if value == 0.0 {
                        return;
                    }

                    self.servo = next_servo(self.servo);
                }
                Action::SelectServoDecrement => {
                    if value == 0.0 {
                        return;
                    }

                    self.servo = last_servo(self.servo);
                }
                Action::IncreaseGain => {
                    if value == 0.0 {
                        return;
                    }

                    self.gain += 0.1;
                }
                Action::DecreaseGain => {
                    if value == 0.0 {
                        return;
                    }

                    self.gain -= 0.1;
                }
                Action::ResetGain => {
                    if value == 0.0 {
                        return;
                    }

                    self.gain = 1.0;
                }
                Action::TrimPitch => todo!(),
                Action::TrimPitchInverted => todo!(),
                Action::TrimRoll => todo!(),
                Action::TrimRollInverted => todo!(),
                Action::SetRobotMode() => todo!(),
                Action::HoldAxis => {
                    if value == 0.0 {
                        return;
                    }

                    self.hold_axis = !self.hold_axis;

                    if !self.hold_axis {
                        self.movement = Default::default();
                    }
                }
                Action::RotateServo => {
                    self.servo_position_normal = value;

                    self.movement.set_by_id(
                        self.servo,
                        Percent::new(
                            (self.servo_position_normal - self.servo_position_inverted) as f64,
                        ),
                    );
                }
                Action::RotateServoInverted => {
                    self.servo_position_inverted = value;

                    self.movement.set_by_id(
                        self.servo,
                        Percent::new(
                            (self.servo_position_normal - self.servo_position_inverted) as f64,
                        ),
                    );
                }
                Action::SetServo(pct, servo) => {
                    self.movement.set_by_id(*servo, *pct);
                }
                Action::Pitch => {
                    if self.hold_axis {
                        return;
                    }

                    self.movement.x_rot = Percent::new((value * self.gain) as f64);
                }
                Action::Roll => {
                    if self.hold_axis {
                        return;
                    }

                    self.movement.y_rot = Percent::new((value * self.gain) as f64);
                }
                Action::Yaw => {
                    if self.hold_axis {
                        return;
                    }

                    self.movement.z_rot = Percent::new((value * self.gain * 0.5) as f64);
                }
                Action::Forward => {
                    if self.hold_axis {
                        return;
                    }

                    self.movement.y = Percent::new((value * self.gain) as f64);
                }
                Action::Lateral => {
                    if self.hold_axis {
                        return;
                    }

                    self.movement.x = Percent::new((value * self.gain) as f64);
                }
                Action::Vertical => {
                    if self.hold_axis {
                        return;
                    }

                    self.movement.z = Percent::new((value * self.gain) as f64);
                }
                Action::ToggleLeveling(vec) => {
                    if value == 0.0 {
                        return;
                    }

                    let vec = *vec;
                    commands.add(move |world: &mut World| {
                        if let Some(robot) = world.get_resource::<Robot>() {
                            let old_mode = robot.store().get(&tokens::LEVELING_MODE).map(|it| *it);
                            let new_mode = match old_mode {
                                Some(LevelingMode::Enabled(_)) => LevelingMode::Disabled,
                                _ => LevelingMode::Enabled(vec.into()),
                            };
                            Updater::from_world(world)
                                .emit_update(&tokens::LEVELING_MODE, new_mode);
                        } else {
                            error!("No robot resource");
                        }
                    })
                }
                Action::ToggleDepth(depth) => {
                    if value == 0.0 {
                        return;
                    }

                    let depth = *depth;
                    commands.add(move |world: &mut World| {
                        if let Some(robot) = world.get_resource::<Robot>() {
                            if let Some(depth) = depth.or_else(|| {
                                robot.store().get(&tokens::RAW_DEPTH).map(|it| it.depth)
                            }) {
                                let old_mode =
                                    robot.store().get(&tokens::DEPTH_CONTROL_MODE).map(|it| *it);
                                let new_mode = match old_mode {
                                    Some(DepthControlMode::Enabled(_)) => {
                                        DepthControlMode::Disabled
                                    }
                                    _ => DepthControlMode::Enabled(depth),
                                };
                                Updater::from_world(world)
                                    .emit_update(&tokens::DEPTH_CONTROL_MODE, new_mode);
                            } else {
                                Updater::from_world(world).emit_update(
                                    &tokens::DEPTH_CONTROL_MODE,
                                    DepthControlMode::Disabled,
                                );
                            }
                        } else {
                            error!("No robot resource");
                        }
                    })
                }
            }
        } else {
            warn!("No action bound to {input:?}");
        }
    }
}

// fn next_servo(id: MotorId) -> MotorId {
//     match id {
//         MotorId::FrontLeftBottom
//         | MotorId::FrontLeftTop
//         | MotorId::FrontRightBottom
//         | MotorId::FrontRightTop
//         | MotorId::BackLeftBottom
//         | MotorId::BackLeftTop
//         | MotorId::BackRightBottom
//         | MotorId::BackRightTop => {
//             unimplemented!()
//         }
//         MotorId::Camera1 => MotorId::Camera2,
//         MotorId::Camera2 => MotorId::Camera3,
//         MotorId::Camera3 => MotorId::Camera4,
//         MotorId::Camera4 => MotorId::Aux1,
//         MotorId::Aux1 => MotorId::Aux2,
//         MotorId::Aux2 => MotorId::Aux3,
//         MotorId::Aux3 => MotorId::Aux4,
//         MotorId::Aux4 => MotorId::Camera1,
//     }
// }
//
// fn last_servo(id: MotorId) -> MotorId {
//     match id {
//         MotorId::FrontLeftBottom
//         | MotorId::FrontLeftTop
//         | MotorId::FrontRightBottom
//         | MotorId::FrontRightTop
//         | MotorId::BackLeftBottom
//         | MotorId::BackLeftTop
//         | MotorId::BackRightBottom
//         | MotorId::BackRightTop => {
//             unimplemented!()
//         }
//         MotorId::Camera1 => MotorId::Aux4,
//         MotorId::Camera2 => MotorId::Camera1,
//         MotorId::Camera3 => MotorId::Camera2,
//         MotorId::Camera4 => MotorId::Camera3,
//         MotorId::Aux1 => MotorId::Camera4,
//         MotorId::Aux2 => MotorId::Aux1,
//         MotorId::Aux3 => MotorId::Aux2,
//         MotorId::Aux4 => MotorId::Aux3,
//     }
// }

fn next_servo(id: MotorId) -> MotorId {
    match id {
        MotorId::Camera3 => MotorId::Camera1,
        _ => MotorId::Camera3,
    }
}

fn last_servo(id: MotorId) -> MotorId {
    match id {
        MotorId::Camera3 => MotorId::Camera1,
        _ => MotorId::Camera3,
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            movement: Default::default(),
            servo: MotorId::Camera3,
            maps: create_mapping(),
            selected_map: "default",
            gain: 1.0,
            hold_axis: false,
            servo_position_normal: 0.0,
            servo_position_inverted: 0.0,
        }
    }
}

/// Listens to the connection and disconnection of gamepads
fn gamepad_connections(
    mut commands: Commands,
    mut connections: EventReader<GamepadConnectionEvent>,
    current_gamepad: Option<Res<CurrentGamepad>>,
) {
    for event in connections.iter() {
        match &event.connection {
            GamepadConnection::Connected(info) => {
                info!(
                    "New gamepad ({}) connected with ID: {:?}",
                    info.name, event.gamepad
                );

                if current_gamepad.is_none() {
                    commands.insert_resource(CurrentGamepad(event.gamepad, Default::default()));
                }
            }
            GamepadConnection::Disconnected => {
                info!("Lost gamepad connection with ID: {:?}", event.gamepad);

                if let Some(CurrentGamepad(gamepad_lost, _)) = current_gamepad.as_deref() {
                    if *gamepad_lost == event.gamepad {
                        commands.remove_resource::<CurrentGamepad>();
                    }
                }
            }
        }
    }
}

/// Listens to axis changes
fn gamepad_axis(
    mut commands: Commands,
    mut current_gamepad: Option<ResMut<CurrentGamepad>>,
    mut axis: EventReader<GamepadAxisChangedEvent>,
) {
    for event in axis.iter() {
        if let Some(CurrentGamepad(gamepad, state)) = current_gamepad.as_deref_mut() {
            if event.gamepad == *gamepad {
                state.handle_event(Input::Axis(event.axis_type), event.value, &mut commands);
            }
        }
    }
}

/// Listens to axis changes
fn gamepad_buttons(
    mut commands: Commands,
    mut current_gamepad: Option<ResMut<CurrentGamepad>>,
    mut axis: EventReader<GamepadButtonChangedEvent>,
) {
    for event in axis.iter() {
        if let Some(CurrentGamepad(gamepad, state)) = current_gamepad.as_deref_mut() {
            if event.gamepad == *gamepad {
                state.handle_event(Input::Button(event.button_type), event.value, &mut commands);
            }
        }
    }
}

#[rustfmt::skip]
fn create_mapping() -> ControllerMappings {
    let default_mapping: ControllerMapping = [
        (Input::Button(GamepadButtonType::Select), Action::Disarm),
        (Input::Button(GamepadButtonType::Start), Action::Arm),
        // (Input::Button(GamepadButtonType::LeftThumb), Action::ResetGain),
        // (Input::Button(GamepadButtonType::RightThumb), Action::HoldAxis),
        (Input::Button(GamepadButtonType::DPadUp), Action::IncreaseGain),
        (Input::Button(GamepadButtonType::DPadDown), Action::DecreaseGain),
        (Input::Button(GamepadButtonType::DPadRight), Action::SelectServoIncrement),
        (Input::Button(GamepadButtonType::DPadLeft), Action::SelectServoDecrement),
        (Input::Button(GamepadButtonType::Mode), Action::SetControlMapping("pitch and roll")),
        // (Input::Button(GamepadButtonType::South), Action::SetControlMapping("trim")),
        (Input::Button(GamepadButtonType::North), Action::ToggleLeveling(Vec3::NEG_Z)),
        (Input::Button(GamepadButtonType::East), Action::ToggleLeveling(Vec3::Z)),
        (Input::Button(GamepadButtonType::West), Action::ToggleDepth(None)),
        (Input::Button(GamepadButtonType::South), Action::ToggleDepth(Some(Meters(-1.0)))),
        (Input::Button(GamepadButtonType::LeftTrigger), Action::SetServo(Percent::new(-1.0), MotorId::Camera2)),
        (Input::Button(GamepadButtonType::RightTrigger), Action::SetServo(Percent::new(1.0), MotorId::Camera2)),
        (Input::Button(GamepadButtonType::LeftTrigger2), Action::RotateServoInverted),
        (Input::Button(GamepadButtonType::RightTrigger2), Action::RotateServo),
        (Input::Axis(GamepadAxisType::LeftStickX), Action::Yaw),
        (Input::Axis(GamepadAxisType::LeftStickY), Action::Forward),
        (Input::Axis(GamepadAxisType::RightStickX), Action::Lateral),
        (Input::Axis(GamepadAxisType::RightStickY), Action::Vertical),
        // TODO control modes
        // TODO Use for trigger buttons?
    ].into();

    let mut pr_buttom_mapping = default_mapping.clone();
    pr_buttom_mapping.extend::<ControllerMapping>(
        [
            (Input::Button(GamepadButtonType::Mode), Action::SetControlMapping("default")),
            (Input::Axis(GamepadAxisType::LeftStickX), Action::Roll),
            (Input::Axis(GamepadAxisType::LeftStickY), Action::Pitch),
        ].into(),
    );

    let mut trim_buttom_mapping = default_mapping.clone();
    trim_buttom_mapping.extend::<ControllerMapping>(
        [
            (Input::Button(GamepadButtonType::South), Action::SetControlMapping("default")),
            (Input::Button(GamepadButtonType::DPadUp), Action::TrimPitchInverted),
            (Input::Button(GamepadButtonType::DPadDown), Action::TrimPitch),
            (Input::Button(GamepadButtonType::DPadRight), Action::TrimRoll),
            (Input::Button(GamepadButtonType::DPadLeft), Action::TrimRollInverted),
        ].into(),
    );

   
    [
        ("default", default_mapping),
        ("pitch and roll", pr_buttom_mapping),
        ("trim", trim_buttom_mapping),
    ].into()
}

fn emit_updates(updater: Local<Updater>, current_gamepad: Option<ResMut<CurrentGamepad>>) {
    if let Some(CurrentGamepad(_, state)) = current_gamepad.as_deref() {
        updater.emit_update(&tokens::MOVEMENT_JOYSTICK, state.movement);
    } else {
        updater.emit_delete(&tokens::MOVEMENT_JOYSTICK);
    }
}

pub type ControllerMapping = HashMap<Input, Action>;
pub type ControllerMappings = HashMap<&'static str, ControllerMapping>;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Input {
    Button(GamepadButtonType),
    Axis(GamepadAxisType),
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Arm,
    Disarm,

    SetControlMapping(&'static str),

    CenterServo,
    SelectServoIncrement,
    SelectServoDecrement,
    RotateServo,
    RotateServoInverted,
    SetServo(Percent, MotorId),

    IncreaseGain,
    DecreaseGain,
    ResetGain,

    ToggleDepth(Option<Meters>),
    ToggleLeveling(Vec3),

    TrimPitch,
    TrimPitchInverted,
    TrimRoll,
    TrimRollInverted,

    SetRobotMode(),

    Pitch,
    Roll,
    Yaw,

    Forward,
    Lateral,
    Vertical,

    HoldAxis,
}
