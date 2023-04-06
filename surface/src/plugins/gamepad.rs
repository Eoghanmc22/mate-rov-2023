use std::collections::HashMap;

use bevy::{
    input::gamepad::{GamepadConnection, GamepadEvent},
    prelude::*,
};
use common::{
    store::tokens,
    types::{MotorId, Movement, Percent},
};

use super::robot::{Robot, Updater};

pub struct GamepadPlugin;

impl Plugin for GamepadPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(gamepad_connections.in_base_set(CoreSet::PreUpdate));
        app.add_system(emit_updates.in_schedule(CoreSchedule::FixedUpdate));
    }
}

#[derive(Resource)]
struct CurrentGamepad(Gamepad, InputState);

struct InputState {
    movement: Movement,
    servo: MotorId,

    maps: ControllerMappings,
    selected_map: &'static str,

    gain: f32,
    hold_axis: bool,

    servo_position_normal: f32,
    servo_position_inverted: f32,
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

                    self.movement.z_rot = Percent::new((value * self.gain) as f64);
                }
                Action::Forward => {
                    println!("a");
                    if self.hold_axis {
                        return;
                    }

                    println!("HIT");

                    self.movement.y_rot = Percent::new((value * self.gain) as f64);
                }
                Action::Lateral => {
                    if self.hold_axis {
                        return;
                    }

                    self.movement.x_rot = Percent::new((value * self.gain) as f64);
                }
                Action::Vertical => {
                    if self.hold_axis {
                        return;
                    }

                    self.movement.z_rot = Percent::new((value * self.gain) as f64);
                }
            }
        } else {
            println!("Bad input");
        }
    }
}

fn next_servo(id: MotorId) -> MotorId {
    match id {
        MotorId::FrontLeftBottom
        | MotorId::FrontLeftTop
        | MotorId::FrontRightBottom
        | MotorId::FrontRightTop
        | MotorId::BackLeftBottom
        | MotorId::BaclLeftTop
        | MotorId::BackRightBottom
        | MotorId::RearRightTop => {
            unimplemented!()
        }
        MotorId::Camera1 => MotorId::Camera2,
        MotorId::Camera2 => MotorId::Camera3,
        MotorId::Camera3 => MotorId::Camera4,
        MotorId::Camera4 => MotorId::Aux1,
        MotorId::Aux1 => MotorId::Aux2,
        MotorId::Aux2 => MotorId::Aux3,
        MotorId::Aux3 => MotorId::Aux4,
        MotorId::Aux4 => MotorId::Camera1,
    }
}

fn last_servo(id: MotorId) -> MotorId {
    match id {
        MotorId::FrontLeftBottom
        | MotorId::FrontLeftTop
        | MotorId::FrontRightBottom
        | MotorId::FrontRightTop
        | MotorId::BackLeftBottom
        | MotorId::BaclLeftTop
        | MotorId::BackRightBottom
        | MotorId::RearRightTop => {
            unimplemented!()
        }
        MotorId::Camera1 => MotorId::Aux4,
        MotorId::Camera2 => MotorId::Camera1,
        MotorId::Camera3 => MotorId::Camera2,
        MotorId::Camera4 => MotorId::Camera3,
        MotorId::Aux1 => MotorId::Camera4,
        MotorId::Aux2 => MotorId::Aux1,
        MotorId::Aux3 => MotorId::Aux2,
        MotorId::Aux4 => MotorId::Aux3,
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            movement: Default::default(),
            servo: MotorId::Camera1,
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
    mut current_gamepad: Option<ResMut<CurrentGamepad>>,
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
            },
            GamepadEvent::Button(event) => {
                if let Some(CurrentGamepad(gamepad, state)) = current_gamepad.as_deref_mut() {
                    if event.gamepad == *gamepad {
                        state.handle_event(
                            Input::Button(event.button_type),
                            event.value,
                            &mut commands,
                        );
                    } else {
                        println!("Bad pad");
                    }
                } else {
                    println!("no pad");
                }
            }
            GamepadEvent::Axis(event) => {
                if let Some(CurrentGamepad(gamepad, state)) = current_gamepad.as_deref_mut() {
                    if event.gamepad == *gamepad {
                        state.handle_event(
                            Input::Axis(event.axis_type),
                            event.value,
                            &mut commands,
                        );
                    }
                }
            }
        }
    }
}

#[rustfmt::skip]
fn create_mapping() -> ControllerMappings {
    let default_mapping: ControllerMapping = [
        (Input::Button(GamepadButtonType::Select), Action::Disarm),
        (Input::Button(GamepadButtonType::Start), Action::Arm),
        (Input::Button(GamepadButtonType::LeftThumb), Action::ResetGain),
        (Input::Button(GamepadButtonType::RightThumb), Action::HoldAxis),
        (Input::Button(GamepadButtonType::DPadUp), Action::IncreaseGain),
        (Input::Button(GamepadButtonType::DPadDown), Action::DecreaseGain),
        (Input::Button(GamepadButtonType::DPadRight), Action::SelectServoIncrement),
        (Input::Button(GamepadButtonType::DPadLeft), Action::SelectServoDecrement),
        (Input::Button(GamepadButtonType::Mode), Action::SetControlMapping("pitch and roll")),
        // (Input::Button(GamepadButtonType::South), Action::SetControlMapping("trim")),
        (Input::Button(GamepadButtonType::LeftTrigger2), Action::RotateServoInverted),
        (Input::Button(GamepadButtonType::RightTrigger2), Action::RotateServo),
        (Input::Axis(GamepadAxisType::LeftStickX), Action::Lateral),
        (Input::Axis(GamepadAxisType::LeftStickY), Action::Forward),
        (Input::Axis(GamepadAxisType::RightStickX), Action::Yaw),
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
        updater.emit_update(&tokens::MOVEMENT_JOYSTICK, Default::default());
    }
}

type ControllerMapping = HashMap<Input, Action>;
type ControllerMappings = HashMap<&'static str, ControllerMapping>;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
enum Input {
    Button(GamepadButtonType),
    Axis(GamepadAxisType),
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Arm,
    Disarm,

    SetControlMapping(&'static str),

    CenterServo,
    SelectServoIncrement,
    SelectServoDecrement,
    RotateServo,
    RotateServoInverted,

    IncreaseGain,
    DecreaseGain,
    ResetGain,

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
