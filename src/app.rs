//! Main window (with its own title bar), compact mode and the tray popup.
//! Only redraws on input, on new data, or while a heartbeat is animating.
//! Hidden in the tray it draws nothing at all.

use std::time::{Duration, Instant};

use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, Margin, Pos2, Rect, RichText, Sense,
    Shape, Stroke, Ui, ViewportCommand, ViewportId, WidgetInfo, epaint::Mesh, epaint::PathShape,
    pos2, vec2,
};

use crate::config::Config;
use crate::hr::Zone;
use crate::osc::render_chatbox;
use crate::state::{Conn, HISTORY_SPAN, Shared, State};
use crate::theme::{self, pal};
use crate::tray::{Tray, TrayAction};

pub const FULL_SIZE: [f32; 2] = [800.0, 700.0];
pub const COMPACT_SIZE: [f32; 2] = [300.0, 150.0];
const POPUP_W: f32 = 240.0;
const DEBUG_SIZE: [f32; 2] = [560.0, 620.0];
const TITLE_H: f32 = 44.0;
const POPUP_TITLE: &str = "HeartBridge tray";

const BEAT_ANIM: Duration = Duration::from_millis(260);
const FRAME_33: Duration = Duration::from_millis(33);
const HIST_MIN: f32 = 40.0;
const HIST_MAX: f32 = 200.0;

struct Popup {
    opened: Instant,
    /// Tray click, in points; the popup opens above and to the left of it.
    anchor: Pos2,
    had_focus: bool,
    /// The window is created hidden and shown after its first frame.
    shown: bool,
}

impl Popup {
    fn new(anchor: Pos2) -> Self {
        Self {
            opened: Instant::now(),
            anchor,
            had_focus: false,
            shown: false,
        }
    }

    /// Above the click when there's room (Windows taskbar at the bottom),
    /// below it otherwise (macOS menu bar at the top).
    fn pos(&self, h: f32) -> Pos2 {
        let x = (self.anchor.x - POPUP_W + 8.0).max(0.0);
        let y = if self.anchor.y - h - 8.0 >= 0.0 {
            self.anchor.y - h - 8.0
        } else {
            self.anchor.y + 8.0
        };
        pos2(x, y)
    }
}

pub struct HeartBridgeApp {
    shared: Shared,
    cfg: Config,
    saved: Config,
    tray: Option<Tray>,
    window_visible: bool,
    popup: Option<Popup>,
    /// Measured height of the popup content (starts with an estimate).
    popup_h: f32,
    debug_open: bool,
    /// Height of the history chart, adjusted so both columns end together.
    chart_h: f32,
    quitting: bool,
}

impl HeartBridgeApp {
    pub fn new(shared: Shared, tray: Option<Tray>) -> Self {
        let cfg = shared.config();
        // Testing aids: open the popup or the debug window at startup.
        let popup =
            std::env::var_os("HEARTBRIDGE_OPEN_POPUP").map(|_| Popup::new(pos2(1100.0, 700.0)));
        Self {
            shared,
            saved: cfg.clone(),
            cfg,
            tray,
            window_visible: true,
            popup,
            popup_h: 220.0,
            debug_open: std::env::var_os("HEARTBRIDGE_OPEN_DEBUG").is_some(),
            chart_h: 72.0,
            quitting: false,
        }
    }

    fn commit_config(&mut self, ctx: &egui::Context) {
        if self.cfg == self.saved {
            return;
        }
        if self.cfg.always_on_top != self.saved.always_on_top {
            let level = if self.cfg.always_on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            };
            ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::WindowLevel(level));
        }
        if self.cfg.compact != self.saved.compact {
            let size = if self.cfg.compact {
                COMPACT_SIZE
            } else {
                FULL_SIZE
            };
            ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::InnerSize(size.into()));
        }
        *self.shared.config.lock().unwrap_or_else(|e| e.into_inner()) = self.cfg.clone();
        self.shared.state().log("Settings saved");
        self.shared.config_changed.notify_one();
        self.cfg.save();
        self.saved = self.cfg.clone();
    }

    fn set_window_visible(&mut self, ctx: &egui::Context, visible: bool) {
        self.window_visible = visible;
        ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Visible(visible));
        if visible {
            ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Focus);
        }
    }

    /// The window's X: hides to the tray and keeps sending to VRChat. Only
    /// "Quit" in the tray popup ends the app. Without a tray, it minimizes.
    fn hide_to_tray(&mut self, ctx: &egui::Context) {
        if self.tray.is_some() {
            self.set_window_visible(ctx, false);
        } else {
            ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Minimized(true));
        }
    }

    fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Close);
    }
}

/// What the window draws, copied from the shared state so the lock is not held.
struct View {
    conn: Conn,
    bpm: Option<u16>,
    contact: Option<bool>,
    battery: Option<u8>,
    beat_age: Option<Duration>,
    history: Vec<(f32, u16)>,
    osc_error: Option<String>,
    osc_recent: bool,
}

impl View {
    fn capture(s: &State) -> Self {
        let now = Instant::now();
        Self {
            conn: s.conn.clone(),
            bpm: s.live_bpm(),
            contact: s.hr.as_ref().and_then(|h| h.contact),
            battery: s.battery,
            beat_age: s.last_beat.map(|t| now.duration_since(t)),
            history: s
                .history
                .iter()
                .map(|(t, b)| (now.duration_since(*t).as_secs_f32(), *b))
                .collect(),
            osc_error: s.osc.error.clone(),
            osc_recent: s
                .osc
                .last_send
                .is_some_and(|t| t.elapsed() < Duration::from_secs(7)),
        }
    }

    fn connected(&self) -> bool {
        matches!(self.conn, Conn::Connected(_))
    }

    /// One short line for the compact mode, the popup and the tray tooltip.
    fn summary(&self, cfg: &Config) -> (Color32, &'static str) {
        if self.bpm.is_some() {
            if self.osc_error.is_some() {
                (pal::ERROR, "Connected · OSC error")
            } else if cfg.send_params || cfg.send_chatbox {
                (pal::OK, "Connected · sending to VRChat")
            } else {
                (pal::OK, "Connected · sending off")
            }
        } else if self.connected() {
            (pal::WARN, "No skin contact")
        } else if matches!(self.conn, Conn::NoAdapter(_)) {
            (pal::WARN, "No Bluetooth")
        } else {
            (pal::WARN, "Looking for the strap…")
        }
    }
}

impl eframe::App for HeartBridgeApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        pal::GROUND.to_normalized_gamma_f32()
    }

    /// Runs even when the window is hidden in the tray: handles tray events
    /// and keeps the icon up to date.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let view = View::capture(&self.shared.state());
        if let Some(tray) = &mut self.tray {
            tray.update(
                view.bpm,
                view.summary(&self.cfg).1,
                self.cfg.send_params,
                self.cfg.send_chatbox,
            );
            let actions: Vec<TrayAction> = tray.events.try_iter().collect();
            for a in actions {
                match a {
                    TrayAction::ToggleWindow => {
                        self.popup = None;
                        let show = !self.window_visible
                            || ctx.input(|i| i.viewport().minimized).unwrap_or(false);
                        self.set_window_visible(ctx, show);
                    }
                    TrayAction::OpenPopup { x, y } => {
                        let ppp = ctx
                            .input(|i| i.viewport().native_pixels_per_point)
                            .unwrap_or(1.0);
                        self.popup = Some(Popup::new(pos2(x as f32 / ppp, y as f32 / ppp)));
                    }
                    TrayAction::ShowWindow => self.set_window_visible(ctx, true),
                    TrayAction::ToggleParams => self.cfg.send_params = !self.cfg.send_params,
                    TrayAction::ToggleChatbox => self.cfg.send_chatbox = !self.cfg.send_chatbox,
                    TrayAction::Quit => self.quit(ctx),
                }
            }
        }

        // Closing from Windows (Alt+F4, taskbar) also goes to the tray.
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting && self.tray.is_some() {
            ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::CancelClose);
            self.set_window_visible(ctx, false);
        }

        // The popup and the debug window are children of the main window:
        // while they are open the pass has to run even if it is hidden.
        if (self.popup.is_some() || self.debug_open) && !self.window_visible {
            ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::RequestPaintWhileHidden);
        }

        // The tray menu can change settings while the window is hidden.
        self.commit_config(ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let view = View::capture(&self.shared.state());
        let ctx = ui.ctx().clone();

        let mut animating = false;
        if self.window_visible {
            let full = ui.max_rect();
            if self.cfg.compact {
                animating |= self.compact_ui(ui, &view);
            } else {
                self.title_bar(ui);
                animating |= self.columns(ui, &view);
            }
            // Thin border: without the system frame, the window would blend
            // into dark backgrounds.
            ui.painter().rect_stroke(
                full,
                CornerRadius::ZERO,
                Stroke::new(1.0, pal::HAIRLINE),
                egui::StrokeKind::Inside,
            );
        }

        if self.popup.is_some() {
            animating |= self.show_popup(&ctx, &view);
        }
        if self.debug_open {
            self.show_debug(&ctx);
        }

        self.commit_config(&ctx);

        // Frugal repaint: 30 fps only while the heart beats; otherwise once a
        // second so the "no reading" states go stale on time.
        if animating {
            ctx.request_repaint_after(FRAME_33);
        } else {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }
}

// ---------------------------------------------------------------- title bar

#[derive(Clone, Copy)]
enum Glyph {
    Minimize,
    Close,
    Compact,
    Expand,
    Debug,
}

impl HeartBridgeApp {
    fn title_bar(&mut self, ui: &mut Ui) {
        let (rect, resp) =
            ui.allocate_exact_size(vec2(ui.available_width(), TITLE_H), Sense::click_and_drag());
        if resp.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        let p = ui.painter();
        paint_heart(ui, pos2(rect.left() + 28.0, rect.center().y), 18.0, true);
        p.text(
            pos2(rect.left() + 46.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            "HeartBridge",
            theme::ui_strong(14.0),
            pal::TEXT_2,
        );

        let mut x = rect.right();
        let mut button = |ui: &mut Ui, glyph: Glyph, tip: &str| {
            let r = Rect::from_min_size(pos2(x - 46.0, rect.top()), vec2(46.0, TITLE_H));
            x -= 46.0;
            title_button(ui, r, glyph, tip)
        };
        if button(ui, Glyph::Close, "Close to tray").clicked() {
            self.hide_to_tray(ui.ctx());
        }
        if button(ui, Glyph::Minimize, "Minimize").clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
        }
        if button(ui, Glyph::Compact, "Compact mode").clicked() {
            self.cfg.compact = true;
        }
        if button(ui, Glyph::Debug, "Debug window").clicked() {
            self.debug_open = !self.debug_open;
        }
    }
}

fn title_button(ui: &mut Ui, rect: Rect, glyph: Glyph, tip: &str) -> egui::Response {
    let id = ui.scope_id().with(tip);
    let resp = ui.interact(rect, id, Sense::click()).on_hover_text(tip);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tip));
    let p = ui.painter();
    if resp.hovered() {
        let fill = if matches!(glyph, Glyph::Close) {
            pal::CLOSE_HOVER
        } else {
            pal::PILL
        };
        p.rect_filled(rect, CornerRadius::ZERO, fill);
    }
    let color = if resp.hovered() {
        pal::TEXT
    } else {
        pal::TEXT_2
    };
    let s = Stroke::new(1.3, color);
    let c = rect.center();
    match glyph {
        Glyph::Minimize => {
            p.line_segment([pos2(c.x - 5.0, c.y), pos2(c.x + 5.0, c.y)], s);
        }
        Glyph::Close => {
            p.line_segment([pos2(c.x - 5.0, c.y - 5.0), pos2(c.x + 5.0, c.y + 5.0)], s);
            p.line_segment([pos2(c.x + 5.0, c.y - 5.0), pos2(c.x - 5.0, c.y + 5.0)], s);
        }
        Glyph::Compact => {
            // Outer frame with a small window in the corner (picture in picture).
            let outer = Rect::from_center_size(c, vec2(14.0, 11.0));
            p.rect_stroke(outer, CornerRadius::same(2), s, egui::StrokeKind::Middle);
            let inner = Rect::from_min_max(pos2(c.x + 0.5, c.y + 0.5), pos2(c.x + 5.0, c.y + 3.5));
            p.rect_filled(inner, CornerRadius::same(1), color);
        }
        Glyph::Expand => {
            let outer = Rect::from_center_size(c, vec2(12.0, 12.0));
            p.rect_stroke(outer, CornerRadius::same(2), s, egui::StrokeKind::Middle);
        }
        Glyph::Debug => {
            // A small bug: body, head, three legs a side and two antennae.
            let body = Rect::from_center_size(pos2(c.x, c.y + 1.5), vec2(8.0, 10.0));
            p.rect_stroke(body, CornerRadius::same(4), s, egui::StrokeKind::Middle);
            p.line_segment([pos2(c.x, c.y - 1.0), pos2(c.x, c.y + 6.5)], s);
            p.line_segment([pos2(c.x - 2.5, c.y - 3.5), pos2(c.x + 2.5, c.y - 3.5)], s);
            for (dy, dx) in [(-1.0, 2.5), (2.0, 3.0), (5.0, 2.5)] {
                p.line_segment(
                    [
                        pos2(c.x - 4.0, c.y + dy),
                        pos2(c.x - 4.0 - dx, c.y + dy - 0.5),
                    ],
                    s,
                );
                p.line_segment(
                    [
                        pos2(c.x + 4.0, c.y + dy),
                        pos2(c.x + 4.0 + dx, c.y + dy - 0.5),
                    ],
                    s,
                );
            }
            p.line_segment([pos2(c.x - 1.5, c.y - 3.5), pos2(c.x - 3.5, c.y - 6.0)], s);
            p.line_segment([pos2(c.x + 1.5, c.y - 3.5), pos2(c.x + 3.5, c.y - 6.0)], s);
        }
    }
    if resp.has_focus() {
        p.rect_stroke(
            rect.shrink(2.0),
            CornerRadius::same(4),
            Stroke::new(2.0, pal::FOCUS),
            egui::StrokeKind::Inside,
        );
    }
    resp
}

// ---------------------------------------------------------------- cards

fn card(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(pal::CARD)
        .stroke(Stroke::new(1.0, pal::HAIRLINE))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
            add(ui);
        });
}

impl HeartBridgeApp {
    /// Two columns, no scrolling: pulse and status on the left, VRChat and
    /// settings on the right. The history chart grows or shrinks so both
    /// columns end at the same line.
    fn columns(&mut self, ui: &mut Ui, v: &View) -> bool {
        let mut animating = false;
        let mut slack = 0.0;
        egui::Frame::new()
            .inner_margin(Margin {
                left: 16,
                right: 16,
                top: 4,
                bottom: 16,
            })
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                ui.columns_const(|[left, right]| {
                    left.spacing_mut().item_spacing = vec2(0.0, 12.0);
                    right.spacing_mut().item_spacing = vec2(0.0, 12.0);
                    let bottom = left.max_rect().bottom();
                    animating = self.pulse_card(left, v);
                    self.status_card(left, v);
                    slack = bottom - left.min_rect().bottom();
                    self.vrchat_card(right, v);
                    self.settings_card(right);
                });
            });
        let target = (self.chart_h + slack).clamp(56.0, 320.0);
        if (target - self.chart_h).abs() > 0.5 {
            self.chart_h = target;
            ui.ctx().request_repaint();
        }
        animating
    }

    /// Returns true while the heartbeat animation is running.
    fn pulse_card(&mut self, ui: &mut Ui, v: &View) -> bool {
        let mut animating = false;
        card(ui, |ui| {
            ui.horizontal(|ui| {
                let (scale, anim) = beat_scale(v, self.cfg.animate_beat);
                animating = anim;
                let (rect, _) = ui.allocate_exact_size(vec2(60.0, 72.0), Sense::hover());
                paint_heart(ui, rect.center(), 52.0 * scale, v.bpm.is_some());
                ui.add_space(6.0);
                bpm_label(ui, v.bpm, 88.0);
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if let Some(b) = v.bpm {
                        zone_pill(ui, Zone::from_bpm(b, self.cfg.max_hr));
                    }
                });
            });
            history_chart(ui, v, self.chart_h);
        });
        animating
    }

    fn status_card(&mut self, ui: &mut Ui, v: &View) {
        card(ui, |ui| {
            let (dot, text) = match &v.conn {
                Conn::NoAdapter(_) => (pal::WARN, "No Bluetooth".to_string()),
                Conn::Searching => (pal::WARN, "Looking for the strap…".to_string()),
                Conn::Connecting(n) => (pal::WARN, format!("Connecting to {n}…")),
                Conn::Connected(n) if v.contact == Some(false) || v.bpm.is_none() => {
                    (pal::WARN, format!("{n} · no skin contact"))
                }
                Conn::Connected(n) => (pal::OK, format!("Connected · {n}")),
            };
            status_row(ui, "Strap", Some(dot), &text);

            let bat = v
                .battery
                .map(|b| format!("{b}%"))
                .unwrap_or_else(|| "—".into());
            let bat_dot = match v.battery {
                Some(b) if b <= 15 => Some(pal::WARN),
                _ => None,
            };
            status_row(ui, "Battery", bat_dot, &bat);

            let dest = format!("{}:{}", self.cfg.osc_host, self.cfg.osc_port);
            let (dot, text) = if let Some(e) = &v.osc_error {
                (pal::ERROR, format!("Error: {e}"))
            } else if !self.cfg.send_params && !self.cfg.send_chatbox {
                (pal::TEXT_DIM, "Sending off".to_string())
            } else if v.osc_recent && v.connected() {
                (pal::OK, format!("Sending to {dest}"))
            } else {
                (pal::TEXT_DIM, format!("Waiting · {dest}"))
            };
            status_row(ui, "VRChat", Some(dot), &text);

            if let Some(hint) = connection_hint(v) {
                ui.add_space(2.0);
                ui.label(RichText::new(hint).size(13.0).color(pal::TEXT_2));
            }
        });
    }

    fn vrchat_card(&mut self, ui: &mut Ui, v: &View) {
        card(ui, |ui| {
            ui.label(
                RichText::new("Send to VRChat")
                    .font(theme::ui_strong(17.0))
                    .color(pal::TEXT),
            );
            setting_switch(
                ui,
                "Avatar parameters",
                "HRtoVRChat set: HR, floatHR, isHRBeat…",
                &mut self.cfg.send_params,
            );
            setting_switch(
                ui,
                "Chatbox",
                "Everyone in the instance can see it",
                &mut self.cfg.send_chatbox,
            );

            ui.add_space(2.0);
            ui.label(RichText::new("Chatbox text").size(13.0).color(pal::TEXT_2));
            ui.add(
                egui::TextEdit::singleline(&mut self.cfg.chatbox_template)
                    .font(FontId::proportional(15.0))
                    .margin(Margin::symmetric(12, 9))
                    .desired_width(f32::INFINITY)
                    .char_limit(144)
                    .hint_text(RichText::new("❤ {bpm} bpm").color(pal::TEXT_2)),
            );
            let sample = v.bpm.unwrap_or(72);
            let preview = render_chatbox(
                &self.cfg.chatbox_template,
                sample,
                Zone::from_bpm(sample, self.cfg.max_hr),
                v.battery,
            );
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(RichText::new("Preview:").size(13.0).color(pal::TEXT_2));
                ui.label(RichText::new(preview).size(13.0).color(pal::TEXT));
            });
            ui.label(
                RichText::new("Use {bpm}, {zone} and {bat}. Sent every 2 seconds.")
                    .size(12.0)
                    .color(pal::TEXT_2),
            );
        });
    }

    fn settings_card(&mut self, ui: &mut Ui) {
        card(ui, |ui| {
            // Fills down to the bottom of the window.
            ui.set_min_height(ui.available_height() - 19.0);
            ui.label(
                RichText::new("Settings")
                    .font(theme::ui_strong(17.0))
                    .color(pal::TEXT),
            );

            field_label(ui, "VRChat OSC address");
            ui.horizontal(|ui| {
                let host_w = ui.available_width() - 80.0 - 40.0 - 2.0 * ui.spacing().item_spacing.x;
                ui.add(
                    egui::TextEdit::singleline(&mut self.cfg.osc_host)
                        .font(FontId::proportional(15.0))
                        .margin(Margin::symmetric(12, 9))
                        .desired_width(host_w),
                );
                ui.label(RichText::new("port").size(13.0).color(pal::TEXT_2));
                number_field(ui, "osc-port", &mut self.cfg.osc_port, 1..=65535, 80.0);
            });

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 10.0;
                    field_label(ui, "Max heart rate (zones)");
                    ui.horizontal(|ui| {
                        number_field(ui, "max-hr", &mut self.cfg.max_hr, 120..=230, 64.0);
                        ui.label(RichText::new("bpm").size(13.0).color(pal::TEXT_2));
                    });
                });
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 10.0;
                    field_label(ui, "Strap name");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cfg.name_filter)
                            .font(FontId::proportional(15.0))
                            .margin(Margin::symmetric(12, 9))
                            .desired_width(f32::INFINITY)
                            .hint_text(RichText::new("Any heart-rate sensor").color(pal::TEXT_2)),
                    );
                });
            });
            ui.add_space(2.0);
            setting_switch(
                ui,
                "Always on top",
                "Stays above other windows",
                &mut self.cfg.always_on_top,
            );
            setting_switch(
                ui,
                "Animate the heartbeat",
                "Turn off to use even less",
                &mut self.cfg.animate_beat,
            );
        });
    }

    /// Small window for OBS or a VR overlay: heart, number and status.
    /// The whole window drags; the controls appear on hover.
    fn compact_ui(&mut self, ui: &mut Ui, v: &View) -> bool {
        let full = ui.max_rect();
        let drag = ui.interact(
            full,
            ui.scope_id().with("compact-drag"),
            Sense::click_and_drag(),
        );
        if drag.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        let hovering = ui.rect_contains_pointer(full);

        let mut animating = false;
        egui::Frame::new()
            .inner_margin(Margin {
                left: 16,
                right: 16,
                top: 14,
                bottom: 12,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (scale, anim) = beat_scale(v, self.cfg.animate_beat);
                    animating = anim;
                    let (rect, _) = ui.allocate_exact_size(vec2(56.0, 76.0), Sense::hover());
                    paint_heart(ui, rect.center(), 48.0 * scale, v.bpm.is_some());
                    ui.add_space(4.0);
                    bpm_label(ui, v.bpm, 76.0);
                });
                // Read from a distance in VR and on stream: larger dot and text.
                let (dot, text) = v.summary(&self.cfg);
                ui.horizontal(|ui| {
                    dot_widget(ui, dot, 12.0);
                    ui.label(RichText::new(text).size(16.0).color(pal::TEXT));
                });
            });

        if hovering {
            let r1 = Rect::from_min_size(pos2(full.right() - 36.0, full.top()), vec2(36.0, 32.0));
            let r2 = r1.translate(vec2(-36.0, 0.0));
            if title_button(ui, r1, Glyph::Close, "Close to tray").clicked() {
                self.hide_to_tray(ui.ctx());
            }
            if title_button(ui, r2, Glyph::Expand, "Full window").clicked() {
                self.cfg.compact = false;
            }
        }
        animating
    }

    /// Custom tray popup: pulse, status, quick toggles and actions.
    /// Created hidden, measured and placed on its first frame, then shown
    /// without the Windows open animation so it appears at once.
    fn show_popup(&mut self, ctx: &egui::Context, v: &View) -> bool {
        let Some(popup) = &self.popup else {
            return false;
        };
        let builder = egui::ViewportBuilder::default()
            .with_title(POPUP_TITLE)
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_always_on_top()
            .with_visible(false)
            .with_position(popup.pos(self.popup_h))
            .with_inner_size([POPUP_W, self.popup_h]);

        let mut close = false;
        let mut open_window = false;
        let mut quit = false;
        let mut animating = false;
        let mut had_focus = popup.had_focus;
        let shown = popup.shown;
        let young = popup.opened.elapsed() < Duration::from_millis(600);
        let mut measured = self.popup_h;
        let cfg = &mut self.cfg;
        let popup_id = ViewportId::from_hash_of("tray-popup");

        ctx.show_viewport_immediate(popup_id, builder, |ui, _| {
            // Closes when it loses focus (click outside), like a native menu.
            // In the first moments, Windows may hand focus to another window:
            // take it back instead of closing.
            if shown {
                let focused = ui.input(|i| i.viewport().focused).unwrap_or(false);
                if focused {
                    had_focus = true;
                } else if young {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Focus);
                } else if had_focus {
                    close = true;
                }
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape) || i.viewport().close_requested()) {
                close = true;
            }

            let full = ui.max_rect();
            ui.painter()
                .rect_filled(full, CornerRadius::ZERO, pal::CARD);
            ui.painter().rect_stroke(
                full,
                CornerRadius::ZERO,
                Stroke::new(1.0, pal::HAIRLINE),
                egui::StrokeKind::Inside,
            );

            let inner = egui::Frame::new()
                .inner_margin(Margin::symmetric(8, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
                    egui::Frame::new()
                        .inner_margin(Margin::symmetric(8, 2))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (scale, anim) = beat_scale(v, cfg.animate_beat);
                                animating = anim;
                                let (rect, _) =
                                    ui.allocate_exact_size(vec2(30.0, 34.0), Sense::hover());
                                paint_heart(ui, rect.center(), 24.0 * scale, v.bpm.is_some());
                                bpm_label(ui, v.bpm, 36.0);
                            });
                            let (dot, text) = v.summary(cfg);
                            ui.horizontal(|ui| {
                                dot_widget(ui, dot, 7.0);
                                ui.label(RichText::new(text).size(12.5).color(pal::TEXT_2));
                            });
                        });
                    ui.add_space(4.0);
                    separator(ui);
                    ui.add_space(2.0);
                    menu_switch(ui, "Avatar parameters", &mut cfg.send_params);
                    menu_switch(ui, "Chatbox", &mut cfg.send_chatbox);
                    ui.add_space(2.0);
                    separator(ui);
                    ui.add_space(2.0);
                    if menu_item(ui, "Open window").clicked() {
                        open_window = true;
                    }
                    if menu_item(ui, "Quit HeartBridge").clicked() {
                        quit = true;
                    }
                });
            measured = inner.response.rect.height().ceil();

            if !shown {
                // First frame, still hidden: fit the height, turn off the
                // fade-in, then show it.
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::InnerSize(vec2(POPUP_W, measured)));
                disable_open_animation(POPUP_TITLE);
                ui.ctx().send_viewport_cmd(ViewportCommand::Visible(true));
                ui.ctx().send_viewport_cmd(ViewportCommand::Focus);
            }
        });

        let first = !shown;
        if let Some(p) = &mut self.popup {
            p.had_focus = had_focus;
            if first {
                let pos = p.pos(measured);
                ctx.send_viewport_cmd_to(popup_id, ViewportCommand::OuterPosition(pos));
                p.shown = true;
                let ms = p.opened.elapsed().as_millis();
                self.shared
                    .state()
                    .log(format!("Tray popup ready in {ms} ms"));
            }
        }
        self.popup_h = measured;
        if open_window {
            self.set_window_visible(ctx, true);
            close = true;
        }
        if quit {
            self.quit(ctx);
            close = true;
        }
        if close {
            self.popup = None;
        }
        if first {
            ctx.request_repaint();
        }
        animating
    }

    /// Debug window: raw state of the strap and OSC, and the event log.
    fn show_debug(&mut self, ctx: &egui::Context) {
        let builder = egui::ViewportBuilder::default()
            .with_title("HeartBridge debug")
            .with_icon(std::sync::Arc::new(egui::IconData {
                rgba: crate::icon::heart_rgba(32, [0xFF, 0x3B, 0x47]),
                width: 32,
                height: 32,
            }))
            .with_inner_size(DEBUG_SIZE)
            .with_min_inner_size([420.0, 360.0]);
        let shared = self.shared.clone();
        let cfg = self.cfg.clone();
        let mut close = false;
        ctx.show_viewport_immediate(ViewportId::from_hash_of("debug"), builder, |ui, _| {
            if ui.input(|i| i.viewport().close_requested()) {
                close = true;
            }
            let all = ui.ctx().content_rect();
            ui.ctx()
                .layer_painter(egui::LayerId::background())
                .rect_filled(all, CornerRadius::ZERO, pal::GROUND);
            egui::Frame::new()
                .inner_margin(Margin::same(16))
                .show(ui, |ui| {
                    debug_ui(ui, &shared, &cfg);
                });
        });
        if close {
            self.debug_open = false;
        }
    }
}

fn debug_ui(ui: &mut Ui, shared: &Shared, cfg: &Config) {
    let mut s = shared.state();
    let now = Instant::now();
    let ago = |t: Option<Instant>| match t {
        Some(t) => format!("{:.1} s ago", now.duration_since(t).as_secs_f32()),
        None => "never".into(),
    };
    let mono = |t: String| {
        RichText::new(t)
            .font(FontId::monospace(12.5))
            .color(pal::TEXT)
    };

    let rows: Vec<(&str, String)> = vec![
        (
            "Uptime",
            format!("{:.0} s", s.started.elapsed().as_secs_f32()),
        ),
        ("Connection", format!("{:?}", s.conn)),
        ("Battery", s.battery.map_or("—".into(), |b| format!("{b}%"))),
        ("Packets", s.packets.to_string()),
        (
            "Last packet",
            format!("{} · {:02X?}", ago(s.last_hr_at), s.last_packet),
        ),
        (
            "Parsed",
            match &s.hr {
                Some(h) => {
                    let contact = match h.contact {
                        Some(true) => "yes",
                        Some(false) => "no",
                        None => "not reported",
                    };
                    let rr: Vec<String> = h.rr_ms.iter().map(|r| format!("{r:.0}")).collect();
                    let rr = if rr.is_empty() {
                        "—".to_string()
                    } else {
                        format!("{} ms", rr.join(", "))
                    };
                    let energy = h.energy_kj.map_or("—".into(), |e| format!("{e} kJ"));
                    format!(
                        "{} bpm · contact {contact} · RR {rr} · energy {energy}",
                        h.bpm
                    )
                }
                None => "—".into(),
            },
        ),
        (
            "Live BPM",
            s.live_bpm()
                .map_or("— (none or stale)".into(), |b| b.to_string()),
        ),
        ("OSC target", format!("{}:{}", cfg.osc_host, cfg.osc_port)),
        (
            "OSC sends",
            format!("params {} · chatbox {}", cfg.send_params, cfg.send_chatbox),
        ),
        ("Last send", ago(s.osc.last_send)),
        (
            "Last chatbox",
            format!(
                "{} · \"{}\"",
                ago(s.osc.last_chatbox),
                s.osc.last_chatbox_text
            ),
        ),
        (
            "OSC error",
            s.osc.error.clone().unwrap_or_else(|| "none".into()),
        ),
    ];

    ui.label(
        RichText::new("State")
            .font(theme::ui_strong(15.0))
            .color(pal::TEXT),
    );
    egui::Grid::new("debug-state")
        .num_columns(2)
        .spacing(vec2(16.0, 6.0))
        .show(ui, |ui| {
            for (k, v) in rows {
                ui.label(RichText::new(k).size(13.0).color(pal::TEXT_2));
                ui.add(egui::Label::new(mono(v)).wrap());
                ui.end_row();
            }
        });

    ui.add_space(12.0);
    let mut copy = false;
    let mut clear = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Log")
                .font(theme::ui_strong(15.0))
                .color(pal::TEXT),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            clear = ui.button(RichText::new("Clear").size(13.0)).clicked();
            copy = ui.button(RichText::new("Copy log").size(13.0)).clicked();
        });
    });
    let fmt = |(at, msg): &(Duration, String)| format!("{:>7.1}s  {msg}", at.as_secs_f32());
    if copy {
        let text: Vec<String> = s.log.iter().map(fmt).collect();
        ui.ctx().copy_text(text.join("\n"));
    }
    if clear {
        s.log.clear();
    }
    egui::Frame::new()
        .fill(pal::FIELD)
        .stroke(Stroke::new(1.0, pal::HAIRLINE))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if s.log.is_empty() {
                        ui.label(RichText::new("No events yet").size(13.0).color(pal::TEXT_2));
                    }
                    for line in &s.log {
                        ui.label(mono(fmt(line)));
                    }
                });
        });
}

/// Turns off the DWM open/close animation of a window of this process,
/// found by its title. Without it Windows fades the popup in.
#[cfg(windows)]
fn disable_open_animation(title: &str) {
    use windows_sys::Win32::Graphics::Dwm::{
        DWMWA_TRANSITIONS_FORCEDISABLED, DwmSetWindowAttribute,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: valid null-terminated UTF-16 title; the attribute is a BOOL (i32).
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), wide.as_ptr());
        if !hwnd.is_null() {
            let on: i32 = 1;
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_TRANSITIONS_FORCEDISABLED as u32,
                (&on as *const i32).cast(),
                std::mem::size_of::<i32>() as u32,
            );
        }
    }
}

#[cfg(not(windows))]
fn disable_open_animation(_title: &str) {}

fn connection_hint(v: &View) -> Option<&'static str> {
    match &v.conn {
        Conn::NoAdapter(_) => Some("Turn on the PC's Bluetooth or plug in an adapter."),
        Conn::Searching => Some(
            "Put the strap on and wet the electrodes to wake it. If it's connected to your phone, close that app: it only accepts one connection.",
        ),
        Conn::Connected(_) if v.bpm.is_none() => Some("Tighten the strap and wet the electrodes."),
        _ => None,
    }
}

// ---------------------------------------------------------------- widgets

fn status_row(ui: &mut Ui, label: &str, dot: Option<Color32>, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(14.0).color(pal::TEXT_2));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).size(14.0).color(pal::TEXT));
            if let Some(c) = dot {
                dot_widget(ui, c, 8.0);
            }
        });
    });
}

fn dot_widget(ui: &mut Ui, color: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    ui.painter().circle_filled(rect.center(), size / 2.0, color);
}

/// Number entry drawn like the text fields. Applies valid values as you
/// type and clamps to the range when focus leaves.
fn number_field(
    ui: &mut Ui,
    key: &str,
    value: &mut u16,
    range: std::ops::RangeInclusive<u16>,
    width: f32,
) {
    let id = ui.make_persistent_id(key);
    let mut text = ui
        .data_mut(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| value.to_string());
    let resp = ui.add(
        egui::TextEdit::singleline(&mut text)
            .id(id.with("edit"))
            .font(FontId::proportional(15.0))
            .margin(Margin::symmetric(12, 9))
            .desired_width(width)
            .char_limit(5),
    );
    if resp.changed() {
        text.retain(|c| c.is_ascii_digit());
        if let Ok(n) = text.parse::<u16>()
            && range.contains(&n)
        {
            *value = n;
        }
    }
    if resp.lost_focus()
        && let Ok(n) = text.parse::<u32>()
    {
        *value = n.clamp(*range.start() as u32, *range.end() as u32) as u16;
    }
    if resp.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, text));
    } else {
        ui.data_mut(|d| d.remove::<String>(id));
    }
}

fn separator(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, pal::HAIRLINE);
}

/// Menu row: full width, text on the left, highlight on hover.
fn menu_item(ui: &mut Ui, text: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, text));
    let p = ui.painter();
    if resp.hovered() || resp.has_focus() {
        p.rect_filled(rect, CornerRadius::same(6), pal::PILL);
    }
    p.text(
        pos2(rect.left() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(13.5),
        pal::TEXT,
    );
    resp
}

/// Menu row with a switch on the right. The whole row toggles.
fn menu_switch(ui: &mut Ui, title: &str, on: &mut bool) {
    let row = ui.allocate_ui_with_layout(
        vec2(ui.available_width(), 30.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.add_space(8.0);
            ui.label(RichText::new(title).size(13.5).color(pal::TEXT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0);
                small_switch(ui, on, title);
            });
        },
    );
    let resp = ui.interact(
        row.response.rect,
        row.response.id.with("row"),
        Sense::click(),
    );
    if resp.clicked() {
        *on = !*on;
    }
}

fn field_label(ui: &mut Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(text).size(13.0).color(pal::TEXT_2));
}

fn zone_pill(ui: &mut Ui, zone: Zone) {
    let color = pal::ZONES[zone as usize];
    let galley =
        ui.painter()
            .layout_no_wrap(zone.name().into(), FontId::proportional(13.0), pal::TEXT);
    let size = vec2(galley.size().x + 30.0, 26.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(13), pal::PILL);
    p.circle_filled(pos2(rect.left() + 14.0, rect.center().y), 4.0, color);
    p.galley(
        pos2(rect.left() + 24.0, rect.center().y - galley.size().y / 2.0),
        galley,
        pal::TEXT,
    );
}

/// Title + description on the left, switch on the right. The whole row toggles.
fn setting_switch(ui: &mut Ui, title: &str, desc: &str, on: &mut bool) {
    let row = ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_max_width(ui.available_width() - 60.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new(title).size(15.0).color(pal::TEXT));
            ui.label(RichText::new(desc).size(12.5).color(pal::TEXT_2));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            switch(ui, on, title)
        });
    });
    let resp = ui.interact(
        row.response.rect,
        row.response.id.with("row"),
        Sense::click(),
    );
    if resp.clicked() {
        *on = !*on;
    }
}

fn switch(ui: &mut Ui, on: &mut bool, label: &str) -> egui::Response {
    switch_sized(ui, on, label, 26.0)
}

fn small_switch(ui: &mut Ui, on: &mut bool, label: &str) -> egui::Response {
    switch_sized(ui, on, label, 20.0)
}

/// Pill switch; `h` is its height, the width is 1.7×.
fn switch_sized(ui: &mut Ui, on: &mut bool, label: &str, h: f32) -> egui::Response {
    let size = vec2((h * 1.7).round(), h);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::CheckBox, ui.is_enabled(), *on, label)
    });
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let p = ui.painter();
    let mut bg = lerp_color(pal::SWITCH_OFF, pal::ACCENT, t);
    if resp.hovered() {
        bg = bg.gamma_multiply(1.12);
    }
    let r = h / 2.0;
    p.rect_filled(rect, CornerRadius::same(r as u8), bg);
    let x = egui::lerp((rect.left() + r)..=(rect.right() - r), t);
    p.circle_filled(pos2(x, rect.center().y), r - 3.0, Color32::WHITE);
    if resp.has_focus() {
        p.rect_stroke(
            rect.expand(3.0),
            CornerRadius::same(r as u8 + 3),
            Stroke::new(2.0, pal::FOCUS),
            egui::StrokeKind::Outside,
        );
    }
    resp
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

// ---------------------------------------------------------------- drawing

/// Heartbeat scale: a quick pulse that decays with an ease-out.
fn beat_scale(v: &View, animate: bool) -> (f32, bool) {
    match (animate, v.bpm, v.beat_age) {
        (true, Some(_), Some(age)) if age < BEAT_ANIM => {
            let t = 1.0 - age.as_secs_f32() / BEAT_ANIM.as_secs_f32();
            (1.0 + 0.14 * t * t, true)
        }
        _ => (1.0, false),
    }
}

/// Parametric heart: x = 16 sin³t, y = 13cos t − 5cos 2t − 2cos 3t − cos 4t.
fn heart_points(center: Pos2, size: f32) -> Vec<Pos2> {
    let s = size / 34.0;
    (0..72)
        .map(|i| {
            let t = i as f32 / 72.0 * std::f32::consts::TAU;
            let x = 16.0 * t.sin().powi(3);
            let y =
                13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            pos2(center.x + x * s, center.y - (y + 2.5) * s)
        })
        .collect()
}

fn paint_heart(ui: &Ui, center: Pos2, size: f32, live: bool) {
    let pts = heart_points(center, size);
    let p = ui.painter();
    // The outline is drawn as two open halves that end at the notch and the
    // tip: a closed line would produce miter spikes at those two points.
    let half = pts.len() / 2;
    let right: Vec<Pos2> = pts[..=half].to_vec();
    let left: Vec<Pos2> = pts[half..]
        .iter()
        .chain(std::iter::once(&pts[0]))
        .copied()
        .collect();
    if live {
        // Fan fill from the centre (the heart is star-shaped from there)
        // plus an anti-aliased edge.
        let mut mesh = Mesh::default();
        mesh.colored_vertex(center, pal::ACCENT);
        for pt in &pts {
            mesh.colored_vertex(*pt, pal::ACCENT);
        }
        let n = pts.len() as u32;
        for i in 0..n {
            mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
        }
        p.add(Shape::mesh(mesh));
        p.add(PathShape::line(right, Stroke::new(1.0, pal::ACCENT)));
        p.add(PathShape::line(left, Stroke::new(1.0, pal::ACCENT)));
    } else {
        p.add(PathShape::line(right, Stroke::new(2.0, pal::TEXT_2)));
        p.add(PathShape::line(left, Stroke::new(2.0, pal::TEXT_2)));
    }
}

/// Baseline of the first glyph, relative to the top of the galley.
fn baseline(g: &egui::Galley) -> f32 {
    g.rows
        .first()
        .and_then(|r| r.row.glyphs.first().map(|gl| r.pos.y + gl.pos.y))
        .unwrap_or(g.size().y)
}

/// BPM number with "bpm" beside it, both on the same baseline. The
/// number's box is trimmed to its digits (DIN has no descenders).
fn bpm_label(ui: &mut Ui, bpm: Option<u16>, size: f32) {
    let (text, color) = match bpm {
        Some(b) => (b.to_string(), pal::TEXT),
        None => ("--".to_string(), pal::TEXT_2),
    };
    let num = ui
        .painter()
        .layout_no_wrap(text, theme::numeral(size), color);
    let unit = ui.painter().layout_no_wrap(
        "bpm".into(),
        FontId::proportional((size * 0.21).max(15.0)),
        pal::TEXT_2,
    );
    let num_base = baseline(&num);
    let gap = 6.0;
    let w = num.size().x + gap + unit.size().x;
    let (rect, _) = ui.allocate_exact_size(vec2(w, num_base + 2.0), Sense::hover());
    let p = ui.painter();
    p.galley(rect.min, num.clone(), color);
    let unit_top = rect.top() + num_base - baseline(&unit);
    p.galley(
        pos2(rect.left() + num.size().x + gap, unit_top),
        unit,
        pal::TEXT_2,
    );
}

/// History of the last two minutes on a fixed scale (40-200 bpm).
fn history_chart(ui: &mut Ui, v: &View, height: f32) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let p = ui.painter_at(rect.expand(2.0));

    for bpm in [80.0, 120.0, 160.0] {
        let y = y_for(rect, bpm);
        p.line_segment(
            [pos2(rect.left(), y), pos2(rect.right(), y)],
            Stroke::new(1.0, pal::GRID),
        );
    }

    let span = HISTORY_SPAN.as_secs_f32();
    let to_pos = |age: f32, bpm: u16| {
        pos2(
            rect.right() - age / span * rect.width(),
            y_for(rect, bpm as f32),
        )
    };

    // Contiguous segments (a 0 or a gap > 3 s breaks the line).
    let mut segments: Vec<Vec<Pos2>> = vec![];
    let mut last_age: Option<f32> = None;
    for &(age, bpm) in &v.history {
        let gap = last_age.is_some_and(|a| a - age > 3.0);
        if bpm == 0 || gap || segments.is_empty() {
            segments.push(vec![]);
        }
        if bpm > 0 {
            segments.last_mut().unwrap().push(to_pos(age, bpm));
        }
        last_age = Some(age);
    }

    let fill = pal::ACCENT.gamma_multiply(0.14);
    for seg in segments.iter().filter(|s| s.len() >= 2) {
        let mut mesh = Mesh::default();
        for w in seg.windows(2) {
            let base = mesh.vertices.len() as u32;
            mesh.colored_vertex(w[0], fill);
            mesh.colored_vertex(w[1], fill);
            mesh.colored_vertex(pos2(w[1].x, rect.bottom()), Color32::TRANSPARENT);
            mesh.colored_vertex(pos2(w[0].x, rect.bottom()), Color32::TRANSPARENT);
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base, base + 2, base + 3);
        }
        p.add(Shape::mesh(mesh));
        p.add(PathShape::line(seg.clone(), Stroke::new(2.0, pal::ACCENT)));
    }

    let readings: Vec<u16> = v
        .history
        .iter()
        .map(|(_, b)| *b)
        .filter(|b| *b > 0)
        .collect();
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Last 2 minutes")
                .size(12.5)
                .color(pal::TEXT_2),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let text = match (readings.iter().max(), readings.iter().min()) {
                (Some(max), Some(min)) => format!("max {max} · min {min}"),
                _ => "No readings yet".into(),
            };
            ui.label(RichText::new(text).size(12.5).color(pal::TEXT_2));
        });
    });
}

fn y_for(rect: Rect, bpm: f32) -> f32 {
    let t = ((bpm - HIST_MIN) / (HIST_MAX - HIST_MIN)).clamp(0.0, 1.0);
    rect.bottom() - t * rect.height()
}
