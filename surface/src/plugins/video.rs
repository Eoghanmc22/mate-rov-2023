//! Handles video display

use std::mem;

use bevy::{prelude::*, utils::HashMap};
use bevy_egui::EguiContexts;
use common::error::LogErrorExt;
use egui::TextureId;

use crate::plugins::opencv::VideoMessage;

use super::{opencv::VideoCaptureThread, ui::ExtensionId};

pub struct VideoPlugin;

impl Plugin for VideoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VideoState>();
        app.add_startup_system(video_setup);
        app.add_system(video_add);
        app.add_system(video_remove);
        app.add_system(video_frames);
    }
}

#[derive(Bundle)]
pub struct Video {
    pub name: VideoName,
    pub position: VideoPosition,
}

impl Video {
    pub const fn new(name: String, pos: Position) -> Self {
        Self {
            name: VideoName(name),
            position: VideoPosition(pos),
        }
    }
}

#[derive(Debug, Component, Clone)]
pub struct VideoName(pub String);
#[derive(Debug, Component, Clone)]
pub struct VideoPosition(pub Position);
#[derive(Debug, Component, Clone)]
pub struct VideoRemove;
#[derive(Debug, Component, Clone)]
pub struct VideoTexture(pub Handle<Image>, pub TextureId);
#[derive(Debug, Default, Resource, Clone)]
pub struct VideoState(pub HashMap<Position, VideoTree>);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Position {
    Center,
    Window(ExtensionId),
}

#[derive(Debug, Clone)]
pub enum VideoTree {
    Node(Box<VideoTree>, Box<VideoTree>),
    Leaf(Entity),
    Empty,
}

impl VideoTree {
    pub fn insert(&mut self, entity: Entity) {
        match self {
            Self::Node(a, b) => {
                // TODO better balancing
                if let Self::Node(_, _) = **b {
                    a.insert(entity);
                } else {
                    b.insert(entity);
                }
            }
            Self::Leaf(cur) => {
                *self = Self::Node(Box::new(Self::Leaf(*cur)), Box::new(Self::Leaf(entity)));
            }
            Self::Empty => {
                *self = Self::Leaf(entity);
            }
        }
    }

    pub fn remove(&mut self, entity: Entity) {
        match self {
            Self::Node(a, b) => {
                a.remove(entity);
                b.remove(entity);

                if matches!(**a, Self::Empty) {
                    *self = mem::take(b);
                } else if matches!(**b, Self::Empty) {
                    *self = mem::take(a);
                }
            }
            Self::Leaf(it) => {
                if *it == entity {
                    *self = Self::Empty;
                }
            }
            Self::Empty => {}
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
        let tree = video.0.entry(pos.0).or_default();
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
