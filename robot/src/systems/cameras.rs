use core::str;
use std::{
    borrow::ToOwned,
    io,
    net::{IpAddr, SocketAddr},
    process::{Child, Command},
    thread::{self, Scope},
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Error};
use common::{
    error::LogErrorExt,
    store::{self, tokens},
    types::Camera,
};
use crossbeam::channel::bounded;
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
use tracing::{error, info, span, Level};

use crate::{
    event::Event,
    events::EventHandle,
    systems::{stop, System},
    SystemId,
};

/// Handles camera detection, starting and stopping gstreamer, and notifying the suface about
/// available cameras
pub struct CameraSystem;

impl System for CameraSystem {
    const ID: SystemId = SystemId::Camera;

    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        let (tx, rx) = bounded(30);

        spawner.spawn(move || {
            span!(Level::INFO, "Event filterer");

            for event in listner {
                if stop::world_stopped() | matches!(&*event, Event::Exit) {
                    tx.try_send(Event::Exit.into())
                        .log_error("Forward exit to camera manager");

                    return;
                }

                match &*event {
                    Event::PeerConnected(_) | Event::SyncStore => {
                        tx.try_send(event)
                            .log_error("Forward event to camera manager");
                    }
                    _ => {}
                }
            }
        });

        spawner.spawn(move || {
            span!(Level::INFO, "Camera manager");

            let mut last_cameras: HashSet<String> = HashSet::default();
            let mut cameras: HashMap<String, (Child, SocketAddr)> = HashMap::default();
            let mut target_ip = None;
            let mut port = 1024u16;

            for event in rx {
                match &*event {
                    // Respawns all instances of gstreamer and points the new ones towards the new peer
                    Event::PeerConnected(addrs) => {
                        target_ip = Some(addrs.ip());

                        for (camera, (mut child, _)) in cameras.drain() {
                            let rst = child.kill();

                            if let Err(err) = rst {
                                events.send(Event::Error(
                                    Error::new(err).context(format!("Kill gstreamer for {camera}")),
                                ));
                            }
                        }

                        thread::sleep(Duration::from_millis(500));

                        for camera in &last_cameras {
                            let rst = add_camera(camera, addrs.ip(), &mut cameras, &mut port);

                            if let Err(err) = rst {
                                events.send(Event::Error(
                                    err.context(format!("Start gstreamer for {camera}")),
                                ));
                            }
                        }

                        let camera_list = camera_list(&cameras);
                        let update = store::create_update(&tokens::CAMERAS, camera_list);
                        events.send(Event::Store(update));
                    }
                    // Reruns detect cameras script and start or kill instances of gstreamer as needed
                    Event::SyncStore => {
                        info!("Checking for new cameras");

                        let camera_detect =
                            Command::new("/home/pi/mate/detect_cameras.sh").output();

                        match camera_detect {
                            Ok(output) => {
                                if !output.status.success() {
                                    events.send(Event::Error(anyhow!(
                                        "Collect cameras: {}",
                                        output.status
                                    )));
                                    continue;
                                }

                                match str::from_utf8(&output.stdout) {
                                    Ok(data) => {
                                        let next_cameras: HashSet<String> =
                                            data.lines().map(ToOwned::to_owned).collect();

                                        for old_camera in last_cameras.difference(&next_cameras) {
                                            if let Some(mut child) = cameras.remove(old_camera) {
                                                let rst = child.0.kill();

                                                if let Err(err) = rst {
                                                    events.send(Event::Error(
                                                        Error::new(err).context(format!(
                                                            "Kill gstreamer for {old_camera}"
                                                        )),
                                                    ));
                                                }
                                            } else {
                                                error!("Attempted to remove a nonexistant camera");
                                            }
                                        }

                                        for new_camera in next_cameras.difference(&last_cameras) {
                                            if let Some(ip) = target_ip {
                                                let rst = add_camera(
                                                    new_camera,
                                                    ip,
                                                    &mut cameras,
                                                    &mut port,
                                                );

                                                if let Err(err) = rst {
                                                    events.send(Event::Error(err.context(
                                                        format!("Start gstreamer for {new_camera}"),
                                                    )));
                                                }
                                            } else {
                                                error!("Tried to update cameras without a peer");
                                            }
                                        }

                                        last_cameras = next_cameras;

                                        let camera_list = camera_list(&cameras);
                                        let update =
                                            store::create_update(&tokens::CAMERAS, camera_list);
                                        events.send(Event::Store(update));
                                    }
                                    Err(err) => {
                                        events.send(Event::Error(
                                            Error::new(err).context("Collect cameras"),
                                        ));
                                    }
                                }
                            }
                            Err(err) => {
                                events
                                    .send(Event::Error(Error::new(err).context("Collect cameras")));
                            }
                        }
                    }
                    Event::Exit => {
                        for (camera, (mut child, _)) in cameras.drain() {
                            let rst = child.kill();

                            if let Err(err) = rst {
                                events.send(Event::Error(
                                    Error::new(err).context(format!("Kill gstreamer for {camera}")),
                                ));
                            }
                        }

                        return;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }
}

/// Spawns a gstreamer with the args necessary
fn start_gstreamer(camera: &str, addrs: SocketAddr) -> io::Result<Child> {
    Command::new("gst-launch-1.0")
        .arg("v4l2src")
        .arg(format!("device={camera}"))
        .arg("!")
        .arg("video/x-h264,width=1920,height=1080,framerate=30/1")
        .arg("!")
        .arg("rtph264pay")
        .arg("!")
        .arg("udpsink")
        .arg(format!("host={}", addrs.ip()))
        .arg(format!("port={}", addrs.port()))
        .spawn()
}

/// Starts a gstreamer and updates state
fn add_camera(
    camera: &str,
    ip: IpAddr,
    cameras: &mut HashMap<String, (Child, SocketAddr)>,
    port: &mut u16,
) -> anyhow::Result<()> {
    let setup_exit = Command::new("/home/pi/mate/setup_camera.sh")
        .arg(camera)
        .spawn()
        .context("Setup cameras")?
        .wait()
        .context("wait on setup")?;
    if !setup_exit.success() {
        bail!("Could not setup cameras");
    }

    let bind = (ip, *port).into();
    let child =
        start_gstreamer(camera, bind).with_context(|| format!("Spawn gstreamer for {camera}"))?;
    *port += 1;

    cameras.insert((*camera).to_owned(), (child, bind));

    Ok(())
}

/// Converts internal repersentation of cameras to what the protocol calls for
fn camera_list(cameras: &HashMap<String, (Child, SocketAddr)>) -> Vec<Camera> {
    let mut list = Vec::new();

    for (name, (_, location)) in cameras {
        list.push(Camera {
            name: name.clone(),
            location: *location,
        });
    }

    list
}
