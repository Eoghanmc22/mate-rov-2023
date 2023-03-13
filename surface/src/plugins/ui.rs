mod elements;
pub mod widgets;
pub mod windows;

use crate::plugins::robot::Robot;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin};
use egui::{Id, Ui};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin);
        // app.insert_resource(EguiSettings { scale_factor: 0.5, default_open_url_target: None });
        app.add_system(draw_ui);
        app.add_system(draw_windows);
    }
}

#[derive(Component)]
pub struct WindowComponent(String, Box<dyn Renderable + Sync + Send>);

impl WindowComponent {
    pub fn new(name: String, window: impl Renderable + Send + Sync + 'static) -> Self {
        Self(name, Box::new(window))
    }
}

pub trait Renderable {
    fn render(&mut self, ui: &mut Ui, cmds: &mut Commands, entity: Entity);
    fn close(&mut self, _cmds: &mut Commands, _entity: Entity) {}
}

fn draw_ui(mut cmds: Commands, robot: Res<Robot>, mut egui_context: EguiContexts) {
    let ctx = egui_context.ctx_mut();
    let store = robot.store();

    elements::menu_bar(ctx, &mut cmds, store);
    elements::side_bar(ctx, &mut cmds, store);
    elements::top_panel(ctx, &mut cmds, store);
}

fn draw_windows(
    mut cmds: Commands,
    mut egui_context: EguiContexts,
    mut windows: Query<(Entity, &mut WindowComponent)>,
) {
    let ctx = egui_context.ctx_mut();

    for (entity, mut window) in windows.iter_mut() {
        let mut open = true;

        egui::Window::new(&window.0)
            .id(Id::new(entity))
            .open(&mut open)
            .show(ctx, |ui| {
                window.1.render(ui, &mut cmds, entity);
            });

        if !open {
            window.1.close(&mut cmds, entity);
            cmds.entity(entity).despawn_recursive();
        }
    }
}
