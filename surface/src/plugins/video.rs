//! Handles video display, video io, and video processing

use std::mem;
use std::{cell::RefCell, thread, time::Duration};

use anyhow::Context;
use bevy::{prelude::*, render::render_resource::Extent3d};
use bevy_egui::EguiContexts;
use common::{
    error::LogErrorExt,
    types::{Camera, Movement},
};
use crossbeam::channel::{self, Receiver, Sender};
use egui::TextureId;
use fxhash::FxHashMap as HashMap;
use fxhash::FxHashSet as HashSet;
use opencv::{
    imgproc,
    prelude::{Mat, MatTraitConstManual},
};
use tracing::{error, span, Level};

use self::pipeline::{MatId, Mats, PipelineProto, ProcessorFn, SourceFn};

pub mod camera;
pub mod pipeline;

pub struct VideoPlugin;

impl Plugin for VideoPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(video_sink);
        app.add_system(spawn_video_captures);
        app.add_system(update_pipelines);
        app.add_system(video_frames);
    }
}

// ECS Types

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct VideoCamera(pub Camera);

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
struct VideoCaptureThread(
    Sender<VideoMessage>,
    Receiver<HashMap<MatId, Image>>,
    Receiver<Movement>,
);

#[derive(Component, Clone, Debug)]
pub struct VideoCapturePipeline(pub PipelineProto, HashSet<MatId>);

#[derive(Component, Clone, Debug)]
pub struct VideoCaptureFrames(pub HashMap<MatId, Handle<Image>>);

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
    for (entity, sink_camera, sink_mat, remove) in &changed_sinks {
        let should_remove = remove.is_some();

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
                    .insert(VideoCaptureFrames(frames))
                    .id();

                (source, image_weak)
            };

            let texture = egui_ctx.add_image(image.clone_weak());
            commands
                .entity(entity)
                .insert((VideoSinkTexture(texture), VideoSinkPeer(source)));
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
            .send(VideoMessage::ConnectTo(peer.0.clone()))
            .log_error("Send tx message");

        // Tell the video the pipeline necessary
        thread
            .0
            .send(VideoMessage::Pipeline(
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
            .send(VideoMessage::Pipeline(
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
        let mut new_images = None;

        for images in thread.1.try_iter() {
            if let Some(reuse_images) = new_images {
                thread
                    .0
                    .send(VideoMessage::ReuseImages(reuse_images))
                    .log_error("Reuse images");
            }

            new_images = Some(images);
        }

        if let Some(new_images) = new_images {
            let mut to_recycle = HashMap::default();

            frames.0.drain_filter(|id, _| !new_images.contains_key(id));

            for (id, new_image) in new_images {
                let handle = frames
                    .0
                    .entry(id)
                    .or_insert_with(|| images.add(Default::default()));
                let texture = images.get_mut(handle).expect("Lookup image handle");
                let old_image = mem::replace(texture, new_image);

                to_recycle.insert(id, old_image);
            }

            thread
                .0
                .send(VideoMessage::ReuseImages(to_recycle))
                .log_error("Reuse images");
        }
    }
}

pub enum VideoMessage {
    ReuseImages(HashMap<MatId, Image>),
    ConnectTo(Camera),
    Pipeline(PipelineProto, HashSet<MatId>),
}

/// The video capture thread
fn video_capture_thread(
    msg_receiver: Receiver<VideoMessage>,
    image_sender: Sender<HashMap<MatId, Image>>,
    _move_sender: Sender<Movement>,
) {
    span!(Level::INFO, "Video capture thread");
    let mut mats = Mats::default();
    let mut mat_tmp = Mat::default();
    let mut to_reuse: HashMap<MatId, Vec<Image>> = HashMap::default();

    let src: RefCell<Option<SourceFn>> = RefCell::new(None);
    let mut pipeline: Vec<ProcessorFn> = Vec::new();
    let mut target_mats: HashSet<MatId> = HashSet::default();

    'main_loop: loop {
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
                                error!("Dropping frame");

                                continue 'main_loop;
                            }
                        }
                    }

                    // Convert target mats to bevy images
                    let mut images = HashMap::default();
                    for mat in &target_mats {
                        let mut image: Image =
                            to_reuse.entry(*mat).or_default().pop().unwrap_or_default();
                        let rst = mats_to_image(&mats, *mat, &mut mat_tmp, &mut image);
                        if let Err(err) = rst {
                            error!("Could not convert mat to bevy image: {:?}", err);
                            error!("Dropping frame");

                            continue 'main_loop;
                        }

                        images.insert(*mat, image);
                    }

                    // Return processed mats
                    let rst = image_sender.send(images);
                    // move_sender.try_send(movement_total).log_error("Send move");

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

        let mut handle = |message| match message {
            VideoMessage::ReuseImages(images) => {
                for (id, image) in images {
                    to_reuse.entry(id).or_default().push(image);
                }
            }
            VideoMessage::ConnectTo(camera) => {
                *src.borrow_mut() = Some(camera::camera_source(camera).unwrap());
            }
            VideoMessage::Pipeline(proto_pipeline, mats) => {
                if mats.is_empty() {
                    // No sinks are listening, stop
                    return;
                }

                pipeline.clear();
                target_mats = mats;

                for proto_stage in proto_pipeline {
                    pipeline.push(proto_stage.construct());
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
    }
}

/// Converts opencv `Mat`s to bevy `Image`s
fn mats_to_image(
    mats: &Mats,
    mat_id: MatId,
    mat_tmp: &mut Mat,
    image: &mut Image,
) -> anyhow::Result<()> {
    let mat = mats.get(&mat_id).context("Get mat")?;
    imgproc::cvt_color(&mat, mat_tmp, imgproc::COLOR_BGR2RGBA, 4).context("Convert colors")?;
    let mat = &mat_tmp;

    let size = mat.size().context("Get size")?;
    let data = mat.data_bytes().context("Get data")?;

    image.resize(Extent3d {
        width: size.width as u32,
        height: size.height as u32,
        depth_or_array_layers: 1,
    });
    image.data.copy_from_slice(data);

    Ok(())
}
