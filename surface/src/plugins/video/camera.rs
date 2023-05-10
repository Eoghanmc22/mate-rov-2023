use anyhow::Context;
use common::types::Camera;
use opencv::videoio::{self, VideoCapture, VideoCaptureTrait};

use super::pipeline::{MatId, Mats, SourceFn};

/// Returns a function that retreives camera frames from `camera`
pub fn camera_source(camera: Camera) -> anyhow::Result<SourceFn> {
    let mut src = VideoCapture::from_file(&gen_src(&camera), videoio::CAP_GSTREAMER)
        .context("Open video capture")?;

    Ok(Box::new(move |mats: &mut Mats| {
        let mat = mats.entry(MatId::Raw).or_default();
        src.read(&mut *mat.borrow_mut()).context("Read stream")
    }))
}

/// Generates the gstreamer pipeline to recieve data from `camera`
fn gen_src(camera: &Camera) -> String {
    let ip = camera.location.ip();
    let port = camera.location.port();

    format!("udpsrc address={ip} port={port} ! gdpdepay ! rtpjpegdepay ! jpegparse ! avdec_mjpeg ! video/x-raw,format=BGR ! appsink drop=1")
}
