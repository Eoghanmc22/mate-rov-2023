mod components;
mod panes;
mod widgets;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin};
use crossbeam::channel::{bounded, Receiver, Sender};
use egui::{Context, Ui};
use fxhash::FxHashMap as HashMap;
use rand::{distributions::Standard, prelude::Distribution, Rng};
use std::fmt::Debug;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin);
        // app.insert_resource(EguiSettings { scale_factor: 0.5, default_open_url_target: None });
        app.init_resource::<UiMessages>();
        app.add_system(handle_ui);
    }
}

struct UiState(HashMap<PaneId, Pane>);

impl FromWorld for UiState {
    fn from_world(_world: &mut World) -> Self {
        let mut panes = HashMap::default();

        panes.insert(PaneId::MenuBar, panes::menu_bar());
        panes.insert(PaneId::StatusBar, panes::status_bar());
        panes.insert(PaneId::DataPane, panes::data_panel());
        panes.insert(PaneId::CameraBar, panes::camera_bar());
        panes.insert(PaneId::Video, panes::video_panel());
        panes.insert(PaneId::Notifications, panes::notification_popup());

        UiState(panes)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum PaneId {
    MenuBar,
    StatusBar,
    DataPane,
    CameraBar,
    Video,
    Notifications,
    Extension(ExtensionId),
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ExtensionId(u128);

impl Distribution<ExtensionId> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ExtensionId {
        ExtensionId(rng.gen())
    }
}

type DynComponent = Box<dyn UiComponent + Send + Sync>;
type DynUiConstructor =
    Box<dyn for<'a> Fn(&egui::Context, Box<dyn FnMut(&mut Ui) + 'a>) + Send + Sync>;

pub struct Pane {
    components: Vec<DynComponent>,
    constructor: DynUiConstructor,
}

impl Pane {
    pub fn new<
        C: for<'a> Fn(&egui::Context, Box<dyn FnMut(&mut Ui) + 'a>) + Send + Sync + 'static,
    >(
        constructor: C,
    ) -> Self {
        let constructor = Box::new(constructor);

        Self {
            components: Default::default(),
            constructor,
        }
    }

    pub fn add<C: UiComponent + Send + Sync + 'static>(&mut self, component: C) {
        self.components.push(Box::new(component));
    }

    pub fn update(&mut self, world: &World, commands: &mut Commands) {
        for component in &mut self.components {
            component.pre_draw(world, commands);
        }
    }

    pub fn render(&mut self, ctx: &Context, commands: &mut Commands) {
        let renderer = |ui: &mut Ui| {
            for component in &mut self.components {
                component.draw(ctx, &mut *ui, commands);
            }
        };
        (self.constructor)(ctx, Box::new(renderer));
    }
}

pub trait UiComponent: Debug {
    fn pre_draw(&mut self, _world: &World, _commands: &mut Commands) {}
    fn draw(&mut self, ctx: &Context, ui: &mut Ui, commands: &mut Commands);
}

#[derive(Resource)]
pub struct UiMessages(Sender<UiMessage>, Receiver<UiMessage>);

impl Default for UiMessages {
    fn default() -> Self {
        let (tx, rx) = bounded(30);
        UiMessages(tx, rx)
    }
}

pub enum UiMessage {
    OpenPanel(PaneId, Pane),
    ClosePanel(PaneId),
}

fn handle_ui(
    mut state: Local<UiState>,
    mut commands: Commands,
    messages: Res<UiMessages>,
    mut set: ParamSet<(&World, EguiContexts)>,
) {
    // Handle ui updates
    // This should be in a seperate system but cant be due as
    // the state is kept in a `Local<T>`
    for message in messages.1.try_iter() {
        match message {
            UiMessage::OpenPanel(id, pane) => {
                state.0.insert(id, pane);
            }
            UiMessage::ClosePanel(id) => {
                state.0.remove(&id);
            }
        }
    }

    // Define render order
    let filters: &[&dyn Fn(&(&PaneId, &mut Pane)) -> bool] = &[
        &|(id, _pane)| matches!(id, PaneId::MenuBar),
        &|(id, _pane)| matches!(id, PaneId::StatusBar),
        &|(id, _pane)| matches!(id, PaneId::DataPane),
        &|(id, _pane)| matches!(id, PaneId::CameraBar),
        &|(id, _pane)| matches!(id, PaneId::Video),
        &|(id, _pane)| matches!(id, PaneId::Notifications),
        &|(id, _pane)| matches!(id, PaneId::Extension(_)),
    ];

    // Render the ui
    for filter in filters {
        for (id, pane) in state.0.iter_mut().filter(filter) {
            pane.update(set.p0(), &mut commands);

            let mut egui = set.p1();
            pane.render(egui.ctx_mut(), &mut commands);
        }
    }
}
