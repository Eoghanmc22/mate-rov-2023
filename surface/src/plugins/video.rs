//! Handles video display

use std::mem;

use bevy::{prelude::*, utils::HashMap};
use bevy_egui::EguiContexts;
use common::error::LogErrorExt;
use egui::{vec2, Align, Layout, TextureId, Ui};

use crate::plugins::opencv::VideoMessage;

use super::{opencv::VideoCaptureThread, ui::widgets};

pub struct VideoPlugin;

impl Plugin for VideoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VideoState>();
        app.add_startup_system(video_setup);
        app.add_system(video_add);
        app.add_system(video_remove);
        app.add_system(video_frames);
        app.add_system(video_render.in_base_set(CoreSet::PostUpdate));
    }
}

#[derive(Bundle)]
pub struct Video {
    pub name: VideoName,
    pub position: VideoPosition,
}

impl Video {
    pub fn new(name: String, pos: Position) -> Self {
        Self {
            name: VideoName(name),
            position: VideoPosition(pos),
        }
    }
}

#[derive(Component)]
pub struct VideoName(pub String);
#[derive(Component)]
pub struct VideoPosition(pub Position);
#[derive(Component)]
pub struct VideoRemove;
#[derive(Component)]
pub struct VideoTexture(Handle<Image>, TextureId);
#[derive(Default, Resource)]
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

/// Sets up `VideoState`
fn video_setup(mut video: ResMut<VideoState>) {
    video.0.insert(Position::Center, VideoTree::Empty);
}

/// Add new camera entities to `VideoState`
fn video_add(
    mut video: ResMut<VideoState>,
    cameras: Query<(Entity, &VideoPosition), Added<VideoName>>,
) {
    for (entity, pos) in &cameras {
        let tree = video.0.entry(pos.0.clone()).or_default();
        tree.insert(entity);
    }
}

/// Process new frames from opencv
fn video_frames(
    mut cmds: Commands,
    mut images: ResMut<Assets<Image>>,
    mut egui_ctx: EguiContexts,
    mut cameras: Query<(Entity, &VideoCaptureThread, Option<&VideoTexture>)>,
) {
    for (entity, thread, texture) in cameras.iter_mut() {
        let mut new_image = None;

        for image in thread.1.try_iter() {
            if let Some(reuse_image) = new_image {
                thread
                    .0
                    .send(VideoMessage::ReuseImage(reuse_image))
                    .log_error("Reuse image");
            }

            new_image = Some(image);
        }

        if let Some(new_image) = new_image {
            if let Some(texture) = texture {
                let texture = images.get_mut(&texture.0).expect("Lookup image handle");
                let old_image = mem::replace(texture, new_image);

                thread
                    .0
                    .try_send(VideoMessage::ReuseImage(old_image))
                    .log_error("Reuse mats");
            } else {
                let texture = images.add(new_image);
                let texture_id = egui_ctx.add_image(texture.clone_weak());
                cmds.entity(entity)
                    .insert(VideoTexture(texture, texture_id));
            }
        }
    }
}

/// Handles the removal of video feeds
fn video_remove(
    mut cmd: Commands,
    mut video: ResMut<VideoState>,
    cameras: Query<(Entity, &VideoPosition), With<VideoRemove>>,
) {
    for (entity, pos) in &cameras {
        if let Some(tree) = video.0.get_mut(&pos.0) {
            tree.remove(entity);

            // Despawning the entity drops the `VideoCaptureThread` component and stops the thread
            cmd.entity(entity).despawn_recursive();
        }
    }
}

/// Renders the video panel
fn video_render(
    mut cmds: Commands,
    mut egui_context: EguiContexts,
    video: Res<VideoState>,
    cameras: Query<(&VideoName, Option<&VideoTexture>)>,
) {
    let ctx = egui_context.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(tree) = video.0.get(&Position::Center) {
            render(&mut cmds, ui, tree, &cameras);
        }
    });
}

/// Renders each node in the `VideoTree`
fn render(
    cmds: &mut Commands,
    ui: &mut Ui,
    tree: &VideoTree,
    cameras: &Query<(&VideoName, Option<&VideoTexture>)>,
) {
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
            if let Ok((name, texture)) = cameras.get(*entity) {
                let mut video = widgets::Video::new(&name.0, texture.map(|it| it.1));

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
