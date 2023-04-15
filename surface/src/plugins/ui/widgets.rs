use common::types::Movement;
use egui::{vec2, Align, Direction, Layout, TextureId, Widget};

#[derive(Debug)]
pub struct Video<'a> {
    name: &'a str,
    texture: Option<TextureId>,
    pub should_delete: bool,
}

impl<'a> Video<'a> {
    pub const fn new(name: &'a str, texture: Option<TextureId>) -> Self {
        Self {
            name,
            texture,
            should_delete: false,
        }
    }
}

impl Widget for &mut Video<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
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
        })
        .response
    }
}

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
