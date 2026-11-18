//! Tray icon: shows the live BPM; left click shows or hides the window,
//! right click opens the custom popup (drawn by the app, not a native menu).

use std::sync::mpsc::{Receiver, channel};

use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::icon;
use crate::theme::pal;

pub enum TrayAction {
    ToggleWindow,
    /// Physical screen position of the click.
    OpenPopup { x: f64, y: f64 },
}

pub struct Tray {
    icon: TrayIcon,
    pub events: Receiver<TrayAction>,
    shown: Option<Option<u16>>,
}

fn rgb(c: egui::Color32) -> [u8; 3] {
    [c.r(), c.g(), c.b()]
}

impl Tray {
    /// Must be created on the main thread (the one running the window).
    pub fn new(ctx: &egui::Context) -> Option<Self> {
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
            let action = match e {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => Some(TrayAction::ToggleWindow),
                TrayIconEvent::Click {
                    button: MouseButton::Right,
                    button_state: MouseButtonState::Up,
                    position,
                    ..
                } => Some(TrayAction::OpenPopup { x: position.x, y: position.y }),
                _ => None,
            };
            if let Some(a) = action {
                let _ = tx.send(a);
                ctx.request_repaint();
            }
        }));

        let icon = TrayIconBuilder::new()
            .with_tooltip("HS500 Pulse")
            .with_icon(idle_icon()?)
            .with_menu_on_left_click(false)
            .with_menu_on_right_click(false)
            .build()
            .ok()?;
        Some(Self { icon, events: rx, shown: None })
    }

    /// Updates the icon and tooltip only when the BPM changes.
    pub fn update(&mut self, bpm: Option<u16>, status: &str) {
        if self.shown == Some(bpm) {
            return;
        }
        self.shown = Some(bpm);
        let icon = match bpm {
            Some(b) => Icon::from_rgba(icon::tray_bpm_rgba(b, rgb(pal::ACCENT)), 32, 32).ok(),
            None => idle_icon(),
        };
        let _ = self.icon.set_icon(icon);
        let tip = match bpm {
            Some(b) => format!("HS500 Pulse · {b} bpm · {status}"),
            None => format!("HS500 Pulse · {status}"),
        };
        let _ = self.icon.set_tooltip(Some(tip));
    }
}

fn idle_icon() -> Option<Icon> {
    Icon::from_rgba(icon::heart_rgba(32, rgb(pal::TEXT_2)), 32, 32).ok()
}
