use std::fs;

use anyhow::Context;
use common::types::Camera;
use opencv::{
    core::Vector,
    videoio::{self, VideoCapture, VideoCaptureTrait},
};

use super::pipeline::{MatId, Mats, SourceFn};

pub fn camera_source(camera: Camera) -> anyhow::Result<SourceFn> {
    let mut src = VideoCapture::from_file(&gen_src(&camera), videoio::CAP_GSTREAMER)
        .context("Open video capture")?;

    Ok(Box::new(move |mats: &mut Mats| {
        let mat = mats.entry(MatId::Raw).or_default();
        src.read(mat).context("Read stream")
    }))
}

fn gen_src(camera: &Camera) -> String {
    let ip = camera.location.ip();
    let port = camera.location.port();

    format!("udpsrc address={ip} port={port} caps=application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,a-framerate=30,payload=96 ! rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! video/x-raw,format=BGR ! appsink drop=1")
}
