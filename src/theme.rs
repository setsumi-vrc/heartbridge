//! Palette, fonts and visual style. Fonts come from the system (Segoe UI
//! and Bahnschrift on Windows, SF and DIN on macOS, whatever fontconfig
//! picks on Linux); if none are found, egui's built-in ones are used.

use std::sync::Arc;

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontTweak, Stroke};

pub mod pal {
    use egui::Color32;

    pub const GROUND: Color32 = Color32::from_rgb(0x0E, 0x0F, 0x12);
    pub const CARD: Color32 = Color32::from_rgb(0x1A, 0x1C, 0x21);
    pub const PILL: Color32 = Color32::from_rgb(0x26, 0x29, 0x30);
    pub const HAIRLINE: Color32 = Color32::from_rgb(0x26, 0x29, 0x30);
    pub const GRID: Color32 = Color32::from_rgb(0x23, 0x26, 0x2C);
    pub const FIELD: Color32 = Color32::from_rgb(0x12, 0x13, 0x17);
    pub const SWITCH_OFF: Color32 = Color32::from_rgb(0x3A, 0x3D, 0x44);

    pub const TEXT: Color32 = Color32::from_rgb(0xF5, 0xF6, 0xF8);
    pub const TEXT_2: Color32 = Color32::from_rgb(0x9A, 0xA0, 0xA8);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(0x5C, 0x61, 0x69);

    pub const ACCENT: Color32 = Color32::from_rgb(0xFF, 0x3B, 0x47);
    pub const FOCUS: Color32 = Color32::from_rgb(0xFF, 0x8A, 0x92);
    pub const OK: Color32 = Color32::from_rgb(0x34, 0xC7, 0x7B);
    pub const WARN: Color32 = Color32::from_rgb(0xF5, 0xA5, 0x24);
    pub const ERROR: Color32 = Color32::from_rgb(0xFF, 0x6B, 0x6B);
    pub const CLOSE_HOVER: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);

    /// Rest, light, moderate, hard, max.
    pub const ZONES: [Color32; 5] = [
        Color32::from_rgb(0x5B, 0x9C, 0xF5),
        Color32::from_rgb(0x34, 0xC7, 0x7B),
        Color32::from_rgb(0xF5, 0xD0, 0x24),
        Color32::from_rgb(0xF5, 0x8A, 0x24),
        Color32::from_rgb(0xFF, 0x3B, 0x47),
    ];
}

pub const NUMERAL: &str = "numeral";
pub const UI_STRONG: &str = "ui-strong";

/// The three faces the app uses.
#[derive(Clone, Copy)]
enum Face {
    /// Regular UI text.
    Ui,
    /// The big number (a DIN-style face where available).
    Numeral,
    /// Fallback for symbols like the chatbox ❤.
    Symbols,
}

/// First existing file among the candidates for this face.
fn read_font(face: Face) -> Option<Vec<u8>> {
    candidates(face)
        .into_iter()
        .find_map(|p| std::fs::read(p).ok())
}

#[cfg(windows)]
fn candidates(face: Face) -> Vec<String> {
    let dir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    let file = match face {
        Face::Ui => "SegUIVar.ttf",
        Face::Numeral => "bahnschrift.ttf",
        Face::Symbols => "seguisym.ttf",
    };
    vec![format!(r"{dir}\Fonts\{file}")]
}

#[cfg(target_os = "macos")]
fn candidates(face: Face) -> Vec<String> {
    let v: &[&str] = match face {
        Face::Ui => &[
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ],
        Face::Numeral => &[
            "/System/Library/Fonts/Supplemental/DIN Alternate Bold.ttf",
            "/System/Library/Fonts/SFNS.ttf",
        ],
        Face::Symbols => &[
            "/System/Library/Fonts/Apple Symbols.ttf",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ],
    };
    v.iter().map(|s| s.to_string()).collect()
}

/// Asks fontconfig, which every Linux desktop has.
#[cfg(not(any(windows, target_os = "macos")))]
fn candidates(face: Face) -> Vec<String> {
    let pattern = match face {
        Face::Ui => "sans-serif",
        Face::Numeral => "sans-serif:bold",
        Face::Symbols => "sans-serif:charset=2764",
    };
    std::process::Command::new("fc-match")
        .args(["-f", "%{file}", pattern])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .filter(|p| !p.is_empty())
        .into_iter()
        .collect()
}

fn with_weight(bytes: &[u8], wght: f32) -> Arc<FontData> {
    let tweak = FontTweak {
        coords: egui::epaint::text::VariationCoords::new([(b"wght", wght)]),
        ..Default::default()
    };
    Arc::new(FontData::from_owned(bytes.to_vec()).tweak(tweak))
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut add = |key: &str, data: Arc<FontData>, family: FontFamily| {
        fonts.font_data.insert(key.into(), data);
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, key.into());
    };

    if let Some(b) = read_font(Face::Ui) {
        add(
            "segoe-400",
            with_weight(&b, 400.0),
            FontFamily::Proportional,
        );
        add(
            "segoe-600",
            with_weight(&b, 600.0),
            FontFamily::Name(UI_STRONG.into()),
        );
    }
    if let Some(b) = read_font(Face::Numeral) {
        add(
            "din-700",
            with_weight(&b, 700.0),
            FontFamily::Name(NUMERAL.into()),
        );
    }
    // Covers ❤ and other chatbox symbols that the UI face may lack.
    if let Some(b) = read_font(Face::Symbols) {
        fonts
            .font_data
            .insert("symbols".into(), Arc::new(FontData::from_owned(b)));
        let list = fonts.families.entry(FontFamily::Proportional).or_default();
        let at = list
            .iter()
            .position(|k| k == "segoe-400")
            .map_or(0, |i| i + 1);
        list.insert(at, "symbols".into());
    }

    // The named families fall back to the proportional ones (symbols, emoji).
    let fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for name in [NUMERAL, UI_STRONG] {
        let list = fonts
            .families
            .entry(FontFamily::Name(name.into()))
            .or_default();
        for f in &fallbacks {
            if !list.contains(f) {
                list.push(f.clone());
            }
        }
    }
    ctx.set_fonts(fonts);

    let mut v = egui::Visuals::dark();
    v.panel_fill = pal::GROUND;
    v.window_fill = pal::CARD;
    v.extreme_bg_color = pal::FIELD;
    v.faint_bg_color = pal::CARD;
    v.override_text_color = None;
    v.hyperlink_color = pal::ACCENT;
    v.selection.bg_fill = pal::ACCENT.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, pal::TEXT);
    v.text_cursor.stroke = Stroke::new(2.0, pal::ACCENT);

    let radius = CornerRadius::same(10);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = radius;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, pal::HAIRLINE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, pal::TEXT_2);
    v.widgets.inactive.bg_fill = pal::PILL;
    v.widgets.inactive.weak_bg_fill = pal::PILL;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, pal::HAIRLINE);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, pal::TEXT);
    v.widgets.hovered.bg_fill = Color32::from_rgb(0x2E, 0x31, 0x39);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x2E, 0x31, 0x39);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, pal::SWITCH_OFF);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, pal::TEXT);
    v.widgets.active.bg_fill = Color32::from_rgb(0x36, 0x39, 0x42);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(0x36, 0x39, 0x42);
    v.widgets.active.bg_stroke = Stroke::new(1.0, pal::FOCUS);
    v.widgets.active.fg_stroke = Stroke::new(1.0, pal::TEXT);
    // Always dark: the scene is a dim room, a VR overlay or OBS.
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, v);
    ctx.style_mut_of(egui::Theme::Dark, |s| {
        // Labels are text, not documents: no selection and no text cursor.
        s.interaction.selectable_labels = false;
        s.spacing.button_padding = egui::vec2(12.0, 7.0);
        s.spacing.interact_size.y = 30.0;
    });
}

pub fn numeral(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(NUMERAL.into()))
}

pub fn ui_strong(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(UI_STRONG.into()))
}
