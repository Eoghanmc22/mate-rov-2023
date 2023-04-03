use egui::{vec2, Align, Direction, Layout, TextureId, Widget};

#[derive(Debug)]
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
