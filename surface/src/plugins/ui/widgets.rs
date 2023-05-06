use common::types::{Movement, PidConfig};
use egui::{vec2, DragValue, Widget};

#[derive(Debug)]
pub struct MovementWidget<'a>(pub &'a Movement);

impl Widget for MovementWidget<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.group(|ui| {
            ui.label(format!("X: {}", self.0.x));
            ui.label(format!("Y: {}", self.0.y));
            ui.label(format!("Z: {}", self.0.z));
            ui.add_space(5.0);
            ui.label(format!("Yaw: {}", self.0.z_rot));
            ui.label(format!("Pitch: {}", self.0.x_rot));
            ui.label(format!("Roll: {}", self.0.y_rot));
            // TODO visual

            ui.allocate_space(vec2(ui.available_width(), 0.0));
        })
        .response
    }
}

#[derive(Debug)]
pub struct PidWidget<'a>(pub &'a mut PidConfig);

impl Widget for PidWidget<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.group(|ui| {
            ui.add(DragValue::new(&mut self.0.kp).speed(0.1).prefix("kp: "));
            ui.add(DragValue::new(&mut self.0.ki).speed(0.1).prefix("ki: "));
            ui.add(DragValue::new(&mut self.0.kd).speed(0.1).prefix("kd: "));
            ui.add(DragValue::new(&mut self.0.max_integral).prefix("max i: "));
            ui.allocate_space(vec2(ui.available_width(), 0.0));
        })
        .response
    }
}
