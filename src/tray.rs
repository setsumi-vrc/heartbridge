//! Tray icon: shows the live BPM; left click shows or hides the window.
//!
//! Right click opens the app's own popup on Windows and macOS. Linux tray
//! hosts (StatusNotifierItem) don't report where a right click happened and
//! Wayland doesn't let apps place windows, so there it opens a native menu
//! with the same items instead.

use std::sync::mpsc::{Receiver, Sender, channel};

use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::icon;
use crate::theme::pal;

pub enum TrayAction {
    ToggleWindow,
    /// Physical screen position of the click.
    OpenPopup { x: f64, y: f64 },
    /// From the Linux tray menu.
    #[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
    ShowWindow,
    #[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
    ToggleParams,
    #[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
    ToggleChatbox,
    #[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    pub events: Receiver<TrayAction>,
    shown: Option<Option<u16>>,
    #[cfg(not(any(windows, target_os = "macos")))]
    menu: linux::Menu,
}

fn rgb(c: egui::Color32) -> [u8; 3] {
    [c.r(), c.g(), c.b()]
}

impl Tray {
    /// Must be created on the main thread (the one running the window).
    pub fn new(ctx: &egui::Context) -> Option<Self> {
        let (tx, rx) = channel();
        let send = {
            let ctx = ctx.clone();
            move |tx: &Sender<TrayAction>, a: TrayAction| {
                let _ = tx.send(a);
                ctx.request_repaint();
            }
        };

        {
            let tx = tx.clone();
            let send = send.clone();
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
                    send(&tx, a);
                }
            }));
        }

        let builder = TrayIconBuilder::new()
            .with_tooltip("HeartBridge")
            .with_icon(idle_icon()?)
            .with_menu_on_left_click(false);

        #[cfg(any(windows, target_os = "macos"))]
        {
            let _ = (&send, &tx);
            let icon = builder.with_menu_on_right_click(false).build().ok()?;
            Some(Self { icon, events: rx, shown: None })
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            let menu = linux::Menu::new(move |a| send(&tx, a));
            let icon = builder.with_menu(Box::new(menu.menu.clone())).build().ok()?;
            Some(Self { icon, events: rx, shown: None, menu })
        }
    }

    /// Updates the icon and tooltip only when the BPM changes. On Linux it
    /// also keeps the menu's check marks in step with the settings.
    pub fn update(&mut self, bpm: Option<u16>, status: &str, params: bool, chatbox: bool) {
        #[cfg(not(any(windows, target_os = "macos")))]
        self.menu.sync(params, chatbox);
        #[cfg(any(windows, target_os = "macos"))]
        let _ = (params, chatbox);

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
            Some(b) => format!("HeartBridge · {b} bpm · {status}"),
            None => format!("HeartBridge · {status}"),
        };
        let _ = self.icon.set_tooltip(Some(tip));
    }
}

fn idle_icon() -> Option<Icon> {
    Icon::from_rgba(icon::heart_rgba(32, rgb(pal::TEXT_2)), 32, 32).ok()
}

#[cfg(not(any(windows, target_os = "macos")))]
mod linux {
    use tray_icon::menu::{CheckMenuItem, Menu as NativeMenu, MenuEvent, MenuItem, PredefinedMenuItem};

    use super::TrayAction;

    pub struct Menu {
        pub menu: NativeMenu,
        params: CheckMenuItem,
        chatbox: CheckMenuItem,
    }

    impl Menu {
        pub fn new(send: impl Fn(TrayAction) + Send + Sync + 'static) -> Self {
            let open = MenuItem::new("Open window", true, None);
            let params = CheckMenuItem::new("Avatar parameters", true, true, None);
            let chatbox = CheckMenuItem::new("Chatbox", true, false, None);
            let quit = MenuItem::new("Quit HeartBridge", true, None);
            let menu = NativeMenu::new();
            let _ = menu.append_items(&[
                &open,
                &PredefinedMenuItem::separator(),
                &params,
                &chatbox,
                &PredefinedMenuItem::separator(),
                &quit,
            ]);

            let ids = (open.id().clone(), params.id().clone(), chatbox.id().clone(), quit.id().clone());
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                let a = if e.id == ids.0 {
                    TrayAction::ShowWindow
                } else if e.id == ids.1 {
                    TrayAction::ToggleParams
                } else if e.id == ids.2 {
                    TrayAction::ToggleChatbox
                } else if e.id == ids.3 {
                    TrayAction::Quit
                } else {
                    return;
                };
                send(a);
            }));
            Self { menu, params, chatbox }
        }

        pub fn sync(&self, params: bool, chatbox: bool) {
            if self.params.is_checked() != params {
                self.params.set_checked(params);
            }
            if self.chatbox.is_checked() != chatbox {
                self.chatbox.set_checked(chatbox);
            }
        }
    }
}
