use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy_egui::EguiContexts;
use egui::{Align2, Frame, Id, Vec2};

const TIMEOUT: Duration = Duration::from_secs(15);
const WINDOW_SIZE: Vec2 = Vec2::new(400.0, 50.0);

pub struct NotificationPlugin;

impl Plugin for NotificationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Notification>();
        app.init_resource::<NotificationResource>();
        app.add_system(handle_notification.in_base_set(CoreSet::Last));
        app.add_system(render_notifications);
    }
}

#[derive(Default, Resource)]
struct NotificationResource(Vec<(InternalNotification, Instant)>);

#[derive(Debug)]
pub enum Notification {
    Info(String, String),
    Error(String, anyhow::Error),
    SimpleError(String),
}

struct InternalNotification {
    title: String,
    description: String,
    level: Level,
}

enum Level {
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

/// Renders `NotificationResource` to the screen
fn render_notifications(mut res: ResMut<NotificationResource>, mut egui_context: EguiContexts) {
    let ctx = egui_context.ctx_mut();

    // TODO this could be its own system?
    res.0.retain(|item| item.1.elapsed() < TIMEOUT);

    // TODO change font size
    for (idx, (notif, time)) in res.0.iter().enumerate() {
        let offset = (WINDOW_SIZE.y + 5.0) * idx as f32 + 5.0;

        // let visuals = match notif.level {
        //     Level::Info => {
        //         let widget_visuals_normal = WidgetVisuals {
        //             bg_fill: Color32::from_rgb(0, 152, 229),
        //             bg_stroke: Stroke::new(2.0, Color32::from_rgb(0, 101, 153)),
        //             fg_stroke: Stroke::new(2.0, Color32::from_rgb(0, 135, 200)),
        //             rounding: 15.0.into(),
        //             expansion: 0.0,
        //         };
        //         let widget_visuals_hover = WidgetVisuals {
        //             bg_fill: Color32::from_rgb(0, 169, 255),
        //             bg_stroke: Stroke::new(2.0, Color32::from_rgb(0, 118, 178)),
        //             fg_stroke: Stroke::new(2.0, Color32::from_rgb(0, 152, 229)),
        //             ..widget_visuals_normal
        //         };
        //
        //         Visuals::
        //         (widget_visuals_normal, widget_visuals_hover)
        //     }
        //     Level::Error => {
        //         let widget_visuals_normal = WidgetVisuals {
        //             bg_fill: Color32::from_rgb(229, 95, 0),
        //             bg_stroke: Stroke::new(2.0, Color32::from_rgb(153, 63, 0)),
        //             fg_stroke: Stroke::new(2.0, Color32::from_rgb(204, 85, 0)),
        //             rounding: 15.0.into(),
        //             expansion: 0.0,
        //         };
        //         let widget_visuals_hover = WidgetVisuals {
        //             bg_fill: Color32::from_rgb(255, 106, 0),
        //             bg_stroke: Stroke::new(2.0, Color32::from_rgb(178, 74, 0)),
        //             fg_stroke: Stroke::new(2.0, Color32::from_rgb(229, 95, 0)),
        //             ..widget_visuals_normal
        //         };
        //
        //         (widget_visuals_normal, widget_visuals_hover)
        //     }
        // };
        //
        // let style = Style {
        //     visuals,
        //     ..*ctx.style()
        // };

        let style = &ctx.style();

        egui::Window::new("Notif")
            .id(Id::new(time))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .frame(Frame::popup(style))
            .anchor(Align2::RIGHT_TOP, Vec2::new(0.0, offset))
            .hscroll(false)
            .vscroll(false)
            .resize(|it| it.fixed_size(WINDOW_SIZE))
            .show(ctx, |ui| {
                ui.heading(&notif.title);
                ui.label(&notif.description);
            });
    }
}
