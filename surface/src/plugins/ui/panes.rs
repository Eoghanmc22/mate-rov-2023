use common::error::LogErrorExt;
use crossbeam::channel::Sender;
use egui::{vec2, Align2, Frame, Id};

use crate::plugins::video::Position;

use super::{components, ExtensionId, Pane, PaneId, UiMessage};

pub fn menu_bar() -> Pane {
    let mut pane = Pane::new(|ctx, add_contents| {
        egui::TopBottomPanel::top("menu_bar").show(ctx, add_contents);
    });

    pane.add(components::MenuBar::default());

    pane
}

pub fn status_bar() -> Pane {
    let mut pane = Pane::new(|ctx, add_contents| {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, add_contents);
    });

    pane.add(components::StatusBar::default());

    pane
}

pub fn data_panel() -> Pane {
    let mut pane = Pane::new(|ctx, add_contents| {
        egui::SidePanel::left("data_pane").show(ctx, add_contents);
    });

    pane.add(components::InputUi::default());
    pane.add(components::OrientationUi::default());
    pane.add(components::MovementUi::default());
    pane.add(components::RawSensorDataUi::default());
    pane.add(components::MotorsUi::default());
    pane.add(components::CamerasUi::default());
    pane.add(components::RemoteSystemUi::default());
    pane.add(components::PreserveSize::default());

    pane
}

pub fn video_panel() -> Pane {
    let mut pane = Pane::new(|ctx, add_contents| {
        egui::CentralPanel::default().show(ctx, add_contents);
    });

    pane.add(components::VideoUi::new(Position::Center));

    pane
}

pub fn camera_bar() -> Pane {
    let mut pane = Pane::new(|ctx, add_contents| {
        egui::TopBottomPanel::top("camera_bar").show(ctx, add_contents);
    });

    pane.add(components::CameraBar::default());

    pane
}

pub fn connect_window(id: ExtensionId, ui: Sender<UiMessage>) -> Pane {
    let mut pane = {
        Pane::new(move |ctx, add_contents| {
            let mut open = true;

            egui::Window::new("Connect")
                .id(Id::new(id))
                .open(&mut open)
                .show(ctx, add_contents);

            if !open {
                ui.try_send(UiMessage::ClosePanel(PaneId::Extension(id)))
                    .log_error("Close connetion window");
            }
        })
    };

    pane.add(components::ConnectUi::new(id));

    pane
}

pub fn notification_popup() -> Pane {
    let mut pane = Pane::new(|ctx, add_contents| {
        egui::Window::new("Notifs")
            .frame(Frame::none())
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .resizable(false)
            .anchor(Align2::RIGHT_TOP, vec2(0.0, 0.0))
            .hscroll(false)
            .vscroll(false)
            .show(ctx, add_contents);
    });

    pane.add(components::NotificationUi::default());

    pane
}

pub fn orientation_display_window(id: ExtensionId, ui: Sender<UiMessage>) -> Pane {
    let mut pane = {
        Pane::new(move |ctx, add_contents| {
            let mut open = true;

            egui::Window::new("Orientation")
                .id(Id::new(id))
                .open(&mut open)
                .show(ctx, add_contents);

            if !open {
                ui.try_send(UiMessage::ClosePanel(PaneId::Extension(id)))
                    .log_error("Close connetion window");
            }
        })
    };

    pane.add(components::OrientationDisplayUi::default());

    pane
}

pub fn debug_egui_window(id: ExtensionId, ui: Sender<UiMessage>) -> Pane {
    let mut pane = {
        Pane::new(move |ctx, add_contents| {
            let mut open = true;

            egui::Window::new("Debug Egui")
                .id(Id::new(id))
                .open(&mut open)
                .show(ctx, add_contents);

            if !open {
                ui.try_send(UiMessage::ClosePanel(PaneId::Extension(id)))
                    .log_error("Close connetion window");
            }
        })
    };

    pane.add(components::DebugEguiUi::default());
    pane.add(components::PreserveSize::default());

    pane
}
