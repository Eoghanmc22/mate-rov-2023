use std::{mem, net::SocketAddr};

use bevy::{prelude::*, utils::HashMap};
use bevy_egui::EguiContext;
use egui::{vec2, Align, Layout, Ui};

use super::{ui::widgets, MateStage};

pub struct VideoPlugin;

impl Plugin for VideoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VideoState>();
        app.add_startup_system(video_setup);
        app.add_system_to_stage(CoreStage::PostUpdate, video_add);
        app.add_system_to_stage(CoreStage::PostUpdate, video_remove);
        app.add_system_to_stage(MateStage::RenderVideo, video_render);
    }
}

#[derive(Bundle)]
pub struct Video {
    pub name: VideoName,
    pub source: VideoSource,
    pub position: VideoPosition,
}

impl Video {
    pub fn new(name: String, src: SocketAddr, pos: Position) -> Self {
        Self {
            name: VideoName(name),
            source: VideoSource(src),
            position: VideoPosition(pos),
        }
    }
}

#[derive(Component)]
pub struct VideoName(pub String);

#[derive(Component)]
pub struct VideoSource(pub SocketAddr);

#[derive(Component)]
pub struct VideoPosition(pub Position);

#[derive(Component)]
pub struct VideoRemove;

#[derive(Default)]
struct VideoState(HashMap<Position, VideoTree>);

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Position {
    Center,
}

#[derive(Debug)]
enum VideoTree {
    Node(Box<VideoTree>, Box<VideoTree>),
    Leaf(Entity),
    Empty,
}

impl VideoTree {
    pub fn insert(&mut self, entity: Entity) {
        match self {
            VideoTree::Node(a, b) => {
                // TODO better balancing
                if let VideoTree::Node(_, _) = **b {
                    a.insert(entity);
                } else {
                    b.insert(entity);
                }
            }
            VideoTree::Leaf(cur) => {
                *self = VideoTree::Node(
                    Box::new(VideoTree::Leaf(*cur)),
                    Box::new(VideoTree::Leaf(entity)),
                );
            }
            VideoTree::Empty => {
                *self = VideoTree::Leaf(entity);
            }
        }
    }

    pub fn remove(&mut self, entity: Entity) {
        match self {
            VideoTree::Node(a, b) => {
                a.remove(entity);
                b.remove(entity);

                // Not sure why this compiles
                if let VideoTree::Empty = **a {
                    *self = mem::take(b);
                } else if let VideoTree::Empty = **b {
                    *self = mem::take(a);
                }
            }
            VideoTree::Leaf(it) => {
                if *it == entity {
                    *self = VideoTree::Empty;
                }
            }
            VideoTree::Empty => {}
        }
    }
}

impl Default for VideoTree {
    fn default() -> Self {
        Self::Empty
    }
}

fn video_setup(mut _cmds: Commands, mut video: ResMut<VideoState>) {
    video.0.insert(Position::Center, VideoTree::Empty);
}

fn video_add(
    mut video: ResMut<VideoState>,
    cameras: Query<(Entity, &VideoSource, &VideoPosition), Added<VideoSource>>,
) {
    for (entity, _src, pos) in &cameras {
        let tree = video.0.entry(pos.0.clone()).or_default();
        tree.insert(entity);

        // TODO start video thread
    }
}
fn video_remove(
    mut cmd: Commands,
    mut video: ResMut<VideoState>,
    cameras: Query<(Entity, &VideoPosition), With<VideoRemove>>,
) {
    for (entity, pos) in &cameras {
        if let Some(tree) = video.0.get_mut(&pos.0) {
            tree.remove(entity);

            // TODO stop video thread

            cmd.entity(entity).despawn_recursive();
        }
    }
}

fn video_render(
    mut cmds: Commands,
    mut egui_context: ResMut<EguiContext>,
    video: Res<VideoState>,
    cameras: Query<&VideoName>,
) {
    let ctx = egui_context.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(tree) = video.0.get(&Position::Center) {
            render(&mut cmds, ui, tree, &cameras);
        }
    });
}

fn render(cmds: &mut Commands, ui: &mut Ui, tree: &VideoTree, cameras: &Query<&VideoName>) {
    match tree {
        VideoTree::Node(a, b) => {
            let available = ui.available_size();
            let (layout, size) = if available.x > available.y {
                (
                    Layout::left_to_right(Align::LEFT),
                    vec2(available.x / 2.0, available.y),
                )
            } else {
                (
                    Layout::top_down(Align::LEFT),
                    vec2(available.x, available.y / 2.0),
                )
            };

            ui.with_layout(layout, |ui| {
                ui.allocate_ui(size, |ui| {
                    ui.set_min_size(size);
                    render(cmds, ui, a, cameras);
                });
                ui.allocate_ui(size, |ui| {
                    ui.set_min_size(size);
                    render(cmds, ui, b, cameras);
                });
            });
        }
        VideoTree::Leaf(entity) => {
            if let Ok(name) = cameras.get(*entity) {
                let mut video = widgets::Video::new(&name.0);

                ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                    ui.add(&mut video);
                });

                if video.should_delete {
                    cmds.entity(*entity).insert(VideoRemove);
                }
            }
        }
        VideoTree::Empty => {
            ui.add_sized(ui.available_size(), |ui: &mut Ui| ui.heading("Empty"));
        }
    }
}
