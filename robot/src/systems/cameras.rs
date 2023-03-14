use core::str;
use std::{
    io,
    net::{IpAddr, SocketAddr},
    process::{Child, Command},
    thread::Scope,
};

use anyhow::{anyhow, Context, Error};
use common::{
    store::{tokens, Store},
    types::Camera,
};
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
use tracing::{info, span, Level};

use crate::{event::Event, events::EventHandle, systems::System};

/// Handles camera detection, starting and stopping gstreamer, and notifying the suface about
/// available cameras
pub struct CameraSystem;

impl System for CameraSystem {
    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        let mut store = {
            let mut events = events.clone();
            Store::new(move |update| {
                events.send(Event::Store(update));
            })
        };

        spawner.spawn(move || {
            span!(Level::INFO, "Camera manager");

            let mut last_cameras: HashSet<String> = HashSet::default();
            let mut cameras: HashMap<String, (Child, SocketAddr)> = HashMap::default();
            let mut target_ip = None;
            let mut port = 1024u16;

            for event in listner.into_iter() {
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

                        for camera in &last_cameras {
                            let rst = add_camera(camera, addrs.ip(), &mut cameras, &mut port);

                            if let Err(err) = rst {
                                events.send(Event::Error(
                                    err.context(format!("Start gstreamer for {camera}")),
                                ));
                            }
                        }

                        let camera_list = camera_list(&cameras);
                        store.insert(&tokens::CAMERAS, camera_list);
                    }
                    Event::Store(update) => {
                        store.handle_update_shared(update);
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
                                            data.lines().map(|it| it.to_owned()).collect();

                                        for old_camera in next_cameras.difference(&last_cameras) {
                                            if let Some(mut child) = cameras.remove(old_camera) {
                                                let rst = child.0.kill();

                                                if let Err(err) = rst {
                                                    events.send(Event::Error(
                                                        Error::new(err).context(format!(
                                                            "Kill gstreamer for {old_camera}"
                                                        )),
                                                    ));
                                                }
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
                                            }
                                        }

                                        last_cameras = next_cameras;

                                        let camera_list = camera_list(&cameras);
                                        store.insert(&tokens::CAMERAS, camera_list);
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
            name: name.to_owned(),
            location: location.to_owned(),
        })
    }

    list
}
