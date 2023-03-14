//! Handles video io and processing

use std::{cell::RefCell, thread, time::Duration};

use anyhow::Context;
use bevy::{prelude::*, render::render_resource::Extent3d};
use common::{
    error::LogErrorExt,
    types::{Camera, Movement},
};
use crossbeam::channel::{self, Receiver, Sender};
use opencv::{
    imgproc,
    prelude::{Mat, MatTraitConstManual},
};
use tracing::{error, span, Level};

use self::pipeline::{MatId, Mats, PipelineProto, ProcessorFn, SourceFn};

pub mod camera;
pub mod pipeline;

pub struct OpenCvPlugin;

impl Plugin for OpenCvPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_video_captures);
        app.add_system(update_pipelines);
    }
}

#[derive(Component, Clone, Debug)]
pub struct VideoCapturePeer(pub Camera);
#[derive(Component, Clone, Debug)]
pub struct VideoCaptureThread(
    pub Sender<VideoMessage>,
    pub Receiver<Image>,
    pub Receiver<Movement>,
);
#[derive(Component, Clone, Debug)]
pub struct VideoCapturePipeline(pub PipelineProto, pub MatId);

pub enum VideoMessage {
    ReuseImage(Image),
    ConnectTo(Camera),
    Pipeline(PipelineProto, MatId),
}

/// Spawn video capture for each video entity
fn spawn_video_captures(
    mut cmds: Commands,
    query: Query<
        (Entity, &VideoCapturePeer, Option<&VideoCaptureThread>),
        Changed<VideoCapturePeer>,
    >,
) {
    for (entity, peer, thread) in query.iter() {
        info!("Spawn vid thread");
        let thread = if let Some(thread) = thread {
            thread.to_owned()
        } else {
            let (msg_sender, msg_receiver) = channel::bounded(10);
            let (image_sender, image_receiver) = channel::bounded(10);
            let (move_sender, move_receiver) = channel::bounded(10);

            let thread = VideoCaptureThread(msg_sender, image_receiver, move_receiver);

            thread::spawn(|| video_capture_thread(msg_receiver, image_sender, move_sender));

            thread
        };

        thread
            .0
            .send(VideoMessage::ConnectTo(peer.0.clone()))
            .log_error("Send tx message");

        cmds.entity(entity).insert(thread);
    }
}

/// Tells video capture thread about pipeline changes
fn update_pipelines(
    query: Query<(&VideoCaptureThread, &VideoCapturePipeline), Changed<VideoCapturePipeline>>,
) {
    for (thread, pipeline) in query.iter() {
        thread
            .0
            .send(VideoMessage::Pipeline(pipeline.0.to_owned(), pipeline.1))
            .log_error("Send tx message");
    }
}

/// The video capture thread
fn video_capture_thread(
    msg_receiver: Receiver<VideoMessage>,
    image_sender: Sender<Image>,
    move_sender: Sender<Movement>,
) {
    span!(Level::INFO, "Video capture thread");
    let mut mats = Mats::default();
    let mut mat_tmp = Mat::default();
    let mut to_reuse = Vec::new();

    let src: RefCell<Option<SourceFn>> = RefCell::new(None);
    let mut pipeline: Vec<ProcessorFn> = Vec::new();
    let mut target_mat = MatId::Raw;

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

                    // Convert target mat to bevy image
                    let mut image: Image = to_reuse.pop().unwrap_or_default();
                    let rst = mats_to_image(&mats, target_mat, &mut mat_tmp, &mut image);
                    if let Err(err) = rst {
                        error!("Could not convert mat to bevy image: {:?}", err);
                        error!("Dropping frame");

                        continue 'main_loop;
                    }

                    // Return processed mats
                    let rst = image_sender.send(image);
                    // move_sender.try_send(movement_total).log_error("Send move");

                    if let Err(_) = rst {
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
            VideoMessage::ReuseImage(mats) => {
                to_reuse.push(mats);
            }
            VideoMessage::ConnectTo(camera) => {
                *src.borrow_mut() = Some(camera::camera_source(camera).unwrap())
            }
            VideoMessage::Pipeline(proto_pipeline, mat) => {
                pipeline.clear();
                target_mat = mat;

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
