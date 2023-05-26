//! Handles video display, video io, and video processing

use std::ffi::c_void;
use std::mem;
use std::time::Instant;
use std::{cell::RefCell, thread, time::Duration};

use anyhow::Context;
use bevy::render::texture::Volume;
use bevy::{prelude::*, render::render_resource::Extent3d};
use bevy_egui::EguiContexts;
use common::store::tokens;
use common::{
    error::LogErrorExt,
    types::{Camera, Movement},
};
use crossbeam::channel::{self, Receiver, Sender};
use egui::TextureId;
use fxhash::FxHashMap as HashMap;
use fxhash::FxHashSet as HashSet;
use opencv::core::{Vector, CV_8UC4};
use opencv::platform_types::size_t;
use opencv::{
    imgproc,
    prelude::{Mat, MatTraitConstManual},
};
use tracing::{error, span, Level};

use self::pipeline::{MatId, Mats, PipelineProto, ProcessorFn, SourceFn};

use super::robot::Updater;

pub mod camera;
pub mod pipeline;

pub const MAX_UPDATE_AGE: Duration = Duration::from_millis(250);

pub struct VideoPlugin;

impl Plugin for VideoPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(video_sink);
        app.add_system(spawn_video_captures);
        app.add_system(update_pipelines);
        app.add_system(video_frames);
        app.add_system(video_movement.in_schedule(CoreSchedule::FixedUpdate));
        app.add_system(
            video_movement_emitter
                .in_schedule(CoreSchedule::FixedUpdate)
                .after(video_movement),
        );
    }
}

// ECS Types

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct VideoCamera(pub Camera);

#[derive(Component, Clone, Debug, PartialEq, Eq)]
struct VideoCameraLast(Camera);

#[derive(Debug, Component, Clone)]
pub struct VideoSinkMat(pub MatId);

#[derive(Debug, Component, Clone)]
pub struct VideoSinkTexture(pub TextureId);

#[derive(Debug, Component, Clone)]
pub struct VideoSinkPeer(pub Entity);

#[derive(Debug, Component, Clone)]
pub struct VideoSinkRemove;

#[derive(Debug, Component, Clone)]
pub struct VideoSinkMarker;

#[derive(Bundle)]
pub struct VideoSink {
    pub camera: VideoCamera,
    pub mat: VideoSinkMat,
    pub marker: VideoSinkMarker,
}

#[derive(Component, Clone, Debug)]
pub struct VideoCaptureThread(
    pub Sender<VideoMessage>,
    pub Receiver<(HashMap<MatId, Image>, HashSet<MatId>)>,
    pub Receiver<(Movement, Instant)>,
);

#[derive(Component, Clone, Debug)]
pub struct VideoCapturePipeline(pub PipelineProto, HashSet<MatId>);

#[derive(Component, Clone, Debug)]
pub struct VideoCaptureFrames(pub HashMap<MatId, Handle<Image>>, pub HashSet<MatId>);

#[derive(Component, Clone, Debug)]
pub struct VideoCaptureMovement(pub Movement, pub Instant);

#[derive(Component, Clone, Debug)]
pub struct VideoCaptureMovementEnabled;

#[derive(Debug, Component, Clone)]
pub struct VideoCaptureMarker;

#[derive(Bundle)]
pub struct VideoCapture {
    pub camera: VideoCamera,
    pub pipeline: VideoCapturePipeline,
    pub marker: VideoCaptureMarker,
}

/// Video layout primitive
#[derive(Default, Debug, Clone)]
pub enum VideoTree {
    Node(Box<VideoTree>, Box<VideoTree>),
    Leaf(Entity),
    #[default]
    Empty,
}

impl VideoTree {
    pub fn entities(&self) -> Vec<Entity> {
        let mut entities = Vec::new();

        self.entities_internal(&mut entities);

        entities
    }

    fn entities_internal(&self, entities: &mut Vec<Entity>) {
        match self {
            VideoTree::Node(a, b) => {
                a.entities_internal(entities);
                b.entities_internal(entities);
            }
            VideoTree::Leaf(entity) => {
                entities.push(*entity);
            }
            VideoTree::Empty => {}
        }
    }
}

/// Handle the addition and modification of video sinks
fn video_sink(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut egui_ctx: EguiContexts,

    changed_sinks: Query<
        (
            Entity,
            &VideoCamera,
            Option<&VideoCameraLast>,
            &VideoSinkMat,
            Option<&VideoSinkRemove>,
        ),
        (
            With<VideoSinkMarker>,
            Or<(
                Changed<VideoCamera>,
                Changed<VideoSinkMat>,
                Added<VideoSinkRemove>,
            )>,
        ),
    >,
    mut sources: Query<
        (
            Entity,
            &VideoCamera,
            &mut VideoCapturePipeline,
            &mut VideoCaptureFrames,
        ),
        With<VideoCaptureMarker>,
    >,
    sinks: Query<(Entity, &VideoCamera, &VideoSinkMat), With<VideoSinkMarker>>,
) {
    for (entity, sink_camera, sink_camera_old, sink_mat, remove) in &changed_sinks {
        let should_remove = remove.is_some();

        // If the source changes, recalculate its targets
        if let Some(sink_camera_old) = sink_camera_old {
            if sink_camera_old.0 != sink_camera.0 {
                // Determine which mats are being requested
                let mats: HashSet<MatId> = sinks
                    .iter()
                    .filter(|(_, camera, _)| {
                        // Does this sink need the old camera?
                        camera.0 == sink_camera_old.0
                    })
                    .map(|(_, _, mat)| mat.0)
                    .collect();
                // Determine sources could need updating
                let mut old_sources: Vec<_> = sources
                    .iter_mut()
                    .filter(|(_, camera, _, _)| camera.0 == sink_camera_old.0)
                    .collect();

                // Set the mats to be sourced
                for (_, _, ref mut pipeline, _) in &mut old_sources {
                    pipeline.1 = mats.clone();
                }
            }
        }

        // Determine which mats are being requested
        let mats: HashSet<MatId> = sinks
            .iter()
            .filter(|(current_entity, camera, _)| {
                // Does this sink need the current camera?
                // Excludes current sink if we are about to remove it
                camera == &sink_camera && (!should_remove || *current_entity != entity)
            })
            .map(|(_, _, mat)| mat.0)
            .collect();
        // Determine sources which provide the camera we need
        let mut filtered_sources: Vec<_> = sources
            .iter_mut()
            .filter(|(_, camera, _, _)| camera == &sink_camera)
            .collect();

        // Set the mats to be sourced
        for (_, _, ref mut pipeline, _) in &mut filtered_sources {
            pipeline.1 = mats.clone();
        }

        // Edit state according to request
        if !should_remove {
            let (source, image) = if let Some((source, _, _, frames)) = filtered_sources.first_mut()
            {
                // The sourse already exists, we just need to grab the texture for the texture we need
                (
                    *source,
                    frames
                        .0
                        .entry(sink_mat.0)
                        .or_insert_with(|| images.add(Default::default()))
                        .clone_weak(),
                )
            } else {
                // The sourse does not exists, we need to create it and pre-initalize the texture we need
                let image = images.add(Default::default());
                let image_weak = image.clone_weak();

                let mut frames = HashMap::default();
                frames.insert(sink_mat.0, image);

                let source = commands
                    .spawn(VideoCapture {
                        camera: sink_camera.clone(),
                        marker: VideoCaptureMarker,
                        pipeline: VideoCapturePipeline(PipelineProto::default(), mats),
                    })
                    .insert(VideoCaptureFrames(frames, Default::default()))
                    .id();

                (source, image_weak)
            };

            let texture = egui_ctx.add_image(image.clone_weak());
            commands.entity(entity).insert((
                VideoSinkTexture(texture),
                VideoSinkPeer(source),
                VideoCameraLast(sink_camera.0.clone()),
            ));
        } else {
            // The removal of the sink has been requested, remove it
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Spawn video capture for each video entity
fn spawn_video_captures(
    mut cmds: Commands,
    query: Query<
        (
            Entity,
            &VideoCamera,
            &VideoCapturePipeline,
            Option<&VideoCaptureThread>,
        ),
        (With<VideoCaptureMarker>, Changed<VideoCamera>),
    >,
) {
    for (entity, peer, pipeline, thread) in query.iter() {
        // Retrieve or create video thread
        let thread = if let Some(thread) = thread {
            info!("Update video thread");
            thread.to_owned()
        } else {
            info!("Spawn video thread");
            let (msg_sender, msg_receiver) = channel::bounded(10);
            let (image_sender, image_receiver) = channel::bounded(10);
            let (move_sender, move_receiver) = channel::bounded(10);

            let thread = VideoCaptureThread(msg_sender, image_receiver, move_receiver);

            thread::spawn(|| video_capture_thread(msg_receiver, image_sender, move_sender));

            thread
        };

        // Tell the video thread which camera to use
        thread
            .0
            .try_send(VideoMessage::ConnectTo(peer.0.clone()))
            .log_error("Send tx message");

        // Tell the video the pipeline necessary
        thread
            .0
            .try_send(VideoMessage::Pipeline(
                pipeline.0.clone(),
                pipeline.1.clone(),
            ))
            .log_error("Send pipeline");

        cmds.entity(entity).insert(thread);
    }
}

/// Tells video capture thread about pipeline changes
fn update_pipelines(
    mut commands: Commands,
    query: Query<
        (Entity, &VideoCaptureThread, &VideoCapturePipeline),
        Changed<VideoCapturePipeline>,
    >,
) {
    for (source, thread, pipeline) in query.iter() {
        thread
            .0
            .try_send(VideoMessage::Pipeline(
                pipeline.0.clone(),
                pipeline.1.clone(),
            ))
            .log_error("Send pipeline");

        // No cameras are listining and the thread is stopping
        // Delete the ecs data
        if pipeline.1.is_empty() {
            commands.entity(source).despawn_recursive();
        }
    }
}

/// Process new frames from opencv
fn video_frames(
    mut images: ResMut<Assets<Image>>,

    mut sources: Query<
        (
            Entity,
            &VideoCamera,
            &VideoCaptureThread,
            &mut VideoCaptureFrames,
        ),
        With<VideoCaptureMarker>,
    >,
) {
    for (_, _, thread, mut frames) in sources.iter_mut() {
        let mut new_image_data = None;

        for image_data in thread.1.try_iter() {
            // Recycle last image set
            if let Some((reuse_images, _)) = new_image_data {
                thread
                    .0
                    .try_send(VideoMessage::ReuseImages(reuse_images))
                    .log_error("Reuse images");
            }

            new_image_data = Some(image_data);
        }

        if let Some((new_images, available_mats)) = new_image_data {
            let mut to_recycle = HashMap::default();

            frames.0.drain_filter(|id, _| !available_mats.contains(id));

            for (id, new_image) in new_images {
                let handle = frames
                    .0
                    .entry(id)
                    .or_insert_with(|| images.add(Default::default()));
                let texture = images.get_mut(handle).expect("Lookup image handle");
                let old_image = mem::replace(texture, new_image);

                to_recycle.insert(id, old_image);
            }

            frames.1 = available_mats;

            thread
                .0
                .try_send(VideoMessage::ReuseImages(to_recycle))
                .log_error("Reuse images");
        }
    }
}

/// Process new movements from opencv
fn video_movement(
    mut commands: Commands,
    mut sources: Query<(Entity, &VideoCaptureThread), With<VideoCaptureMarker>>,
) {
    for (entity, thread) in &mut sources {
        let mut new_movement = None;

        for movement in thread.2.try_iter() {
            new_movement = Some(movement);
        }

        if let Some((movement, instant)) = new_movement {
            commands
                .entity(entity)
                .insert(VideoCaptureMovement(movement, instant));
        }
    }
}

fn video_movement_emitter(
    updater: Local<Updater>,
    sources: Query<
        (&VideoCaptureMovement, Option<&VideoCaptureMovementEnabled>),
        With<VideoCaptureMarker>,
    >,
) {
    let mut movements = Vec::new();

    for (VideoCaptureMovement(movement, instant), enabled) in &sources {
        if enabled.is_some() && instant.elapsed() < MAX_UPDATE_AGE {
            movements.push(movement);
        }
    }

    if !movements.is_empty() {
        let movement = movements.into_iter().copied().sum();
        updater.emit_update(&tokens::MOVEMENT_OPENCV, movement);
    } else {
        updater.emit_delete(&tokens::MOVEMENT_OPENCV);
    }
}

pub enum VideoMessage {
    ReuseImages(HashMap<MatId, Image>),
    ConnectTo(Camera),
    Pipeline(PipelineProto, HashSet<MatId>),
    SaveFrame(String, MatId),
}

/// The video capture thread
fn video_capture_thread(
    msg_receiver: Receiver<VideoMessage>,
    image_sender: Sender<(HashMap<MatId, Image>, HashSet<MatId>)>,
    move_sender: Sender<(Movement, Instant)>,
) {
    span!(Level::INFO, "Video capture thread");
    let mut mats = Mats::default();
    let mut to_reuse: HashMap<MatId, Vec<Image>> = HashMap::default();

    let src: RefCell<Option<SourceFn>> = RefCell::new(None);
    let mut pipeline: Vec<ProcessorFn> = Vec::new();
    let mut target_mats: HashSet<MatId> = HashSet::default();

    'main_loop: loop {
        let mut handle = |message| match message {
            VideoMessage::ReuseImages(images) => {
                for (id, image) in images {
                    to_reuse.entry(id).or_default().push(image);
                }
            }
            VideoMessage::ConnectTo(camera) => {
                *src.borrow_mut() = Some(camera::camera_source(camera).unwrap());
            }
            VideoMessage::Pipeline(proto_pipeline, new_target_mats) => {
                if new_target_mats.is_empty() {
                    // No sinks are listening, stop
                    info!("Stopping video thread");
                    return;
                }

                pipeline.clear();
                mats.clear();
                to_reuse.clear();
                target_mats = new_target_mats;

                for proto_stage in proto_pipeline {
                    pipeline.push(proto_stage.construct());
                }
            }
            VideoMessage::SaveFrame(name, mat) => {
                let mat = mats.get(&mat);

                if let Some(mat) = mat {
                    opencv::imgcodecs::imwrite(&name, &*mat.borrow(), &Vector::new())
                        .log_error("Write screenshot");
                } else {
                    warn!("A screen shot requested bad mat id");
                }
            }
        };

        let dont_block = src.borrow().is_some();

        // Avoid spinning when no source is set
        if dont_block {
            for message in msg_receiver.try_iter() {
                (handle)(message);
            }
        } else {
            let message = msg_receiver.recv().unwrap();
            (handle)(message);
        }

        if let Some(src_fn) = &mut *src.borrow_mut() {
            let mut movement_total = Movement::default();

            // Source frame
            let rst = (src_fn)(&mut mats);

            match rst {
                Ok(true) => {
                    // Apply processors
                    for stage in &mut pipeline {
                        let rst = (stage)(&mut mats);

                        match rst {
                            Ok(movement) => {
                                movement_total += movement;
                            }
                            Err(err) => {
                                error!("Could not process frame: {:?}", err);
                            }
                        }
                    }

                    // Convert target mats to bevy images
                    let mut images = HashMap::default();
                    for mat_id in &target_mats {
                        if let Some(mat) = mats.get(mat_id) {
                            let mut image: Image = to_reuse
                                .entry(*mat_id)
                                .or_default()
                                .pop()
                                .unwrap_or_default();

                            let rst = mats_to_image(&*mat.borrow(), *mat_id, &mut image);
                            if let Err(err) = rst {
                                error!(
                                    "Could not convert mat to bevy image: {:?}. Dropping frame!",
                                    err
                                );

                                continue 'main_loop;
                            }

                            images.insert(*mat_id, image);
                        } else {
                            error!("Target mats included {mat_id:?} which is not in `mats`");
                        }
                    }

                    let available_mats = mats.keys().copied().collect();

                    // Return processed mats
                    let rst = image_sender.send((images, available_mats));
                    move_sender
                        .try_send((movement_total, Instant::now()))
                        .log_error("Send move");

                    if rst.is_err() {
                        info!("Image receiver disconnected, stoping video capture thread");

                        return;
                    }
                }
                Ok(false) => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(err) => {
                    error!("Could not retreve frame: {:?}", err);
                    error!("Dropping camera source");

                    *src.borrow_mut() = None;
                    continue 'main_loop;
                }
            }
        }
    }
}

/// Efficiently converts opencv `Mat`s to bevy `Image`s
fn mats_to_image(mat: &Mat, mat_id: MatId, image: &mut Image) -> anyhow::Result<()> {
    // Convert opencv size to bevy size
    let size = mat.size().context("Get size")?;
    let extent = Extent3d {
        width: size.width as u32,
        height: size.height as u32,
        depth_or_array_layers: 1,
    };
    image.texture_descriptor.size = extent;

    // Allocate bevy image if needed
    let cap = extent.volume() * 4;
    image.data.clear();
    image.data.reserve(cap);

    // Make the bevy image into a opencv mat
    // SAFETY: The vector outlives the returned mat and we dont do anything that could cause the
    // vec to re allocate until after the mat gets dropped
    let mut out_mat = unsafe {
        let dst_ptr = image.data.as_mut_ptr() as *mut c_void;
        let dst_step = size.width as size_t * 4;

        let out_mat =
            Mat::new_rows_cols_with_data(size.height, size.width, CV_8UC4, dst_ptr, dst_step)
                .context("Convert colors")?;
        image.data.set_len(cap);

        out_mat
    };

    // Convert opencv mat to bevy image, out_mat must go out of scope before we touch `image.data`
    imgproc::cvt_color(mat, &mut out_mat, mat_id.conversion_code(), 4).context("Convert colors")?;
    mem::drop(out_mat);

    Ok(())
}
