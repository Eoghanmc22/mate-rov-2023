use std::fs;

use anyhow::Context;
use common::types::Camera;
use opencv::videoio::{self, VideoCapture, VideoCaptureTrait};

use super::{MatId, Source};

pub fn camera_source(camera: Camera) -> anyhow::Result<Source> {
    let path = format!("/tmp/{}.sdp", camera.name.replace(" ", "_"));

    fs::write(&path, gen_sdp(&camera)).context("Write sdp")?;

    let mut src =
        VideoCapture::from_file(&path, videoio::CAP_FFMPEG).context("Open video capture")?;

    Ok(Source::new(camera.name, move |mats| {
        let mat = mats.entry(MatId::RAW).or_default();
        src.read(mat).context("Read stream")?;
        Ok(())
    }))
}

fn gen_sdp(camera: &Camera) -> String {
    let port = camera.location.port();

    format!("m=video {port} RTP/AVP 96\nc=IN IP4 127.0.0.1\na=rtpmap:96 H264/90000")
}
