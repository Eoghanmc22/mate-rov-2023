use std::time::{Duration, Instant};

use bevy::prelude::*;

const TIMEOUT: Duration = Duration::from_secs(15);

pub struct NotificationPlugin;

impl Plugin for NotificationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Notification>();
        app.init_resource::<NotificationResource>();
        app.add_system(handle_notification.in_base_set(CoreSet::Last));
        app.add_system(expire_notifications);
    }
}

#[derive(Debug, Default, Resource, Clone)]
pub struct NotificationResource(pub Vec<(InternalNotification, Instant)>);

#[derive(Debug)]
pub enum Notification {
    Info(String, String),
    Error(String, anyhow::Error),
    SimpleError(String),
}

#[derive(Debug, Clone)]
pub struct InternalNotification {
    pub title: String,
    pub description: String,
    pub level: Level,
}

#[derive(Debug, Clone)]
pub enum Level {
    Info,
    Error,
}

/// Returns a function that converts errors to notification
/// For use with system piping
pub fn create_error_handler(
    name: impl Into<String>,
) -> impl Fn(In<anyhow::Result<()>>, EventWriter<Notification>) {
    let name = name.into();

    move |In(result), mut notifs| {
        if let Err(err) = result {
            notifs.send(Notification::Error(name.clone(), err));
        }
    }
}

/// Adds `NotificationEvents` to the `NotificationResource`
fn handle_notification(
    mut notifications: EventReader<Notification>,
    mut res: ResMut<NotificationResource>,
) {
    for notification in notifications.iter() {
        let notif = match notification {
            Notification::Info(message, details) => {
                info!("Notif msg: {}, details: {}", message, details);

                InternalNotification {
                    title: message.clone(),
                    description: details.clone(),
                    level: Level::Info,
                }
            }
            Notification::Error(message, error) => {
                error!("An error occurred: {}, {:?}", message, error);

                InternalNotification {
                    title: message.clone(),
                    description: format!("{}", error),
                    level: Level::Error,
                }
            }
            Notification::SimpleError(message) => {
                error!("An error occurred: {}", message);

                InternalNotification {
                    title: message.clone(),
                    description: "".to_owned(),
                    level: Level::Error,
                }
            }
        };

        res.0.push((notif, Instant::now()));
    }
}

fn expire_notifications(mut res: ResMut<NotificationResource>) {
    res.0.retain(|item| item.1.elapsed() < TIMEOUT);
}
