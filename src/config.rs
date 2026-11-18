//! Settings persisted to `heartbridge.json` next to the executable.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Only connect to devices whose name contains this (empty = any heart-rate sensor).
    pub name_filter: String,

    pub osc_host: String,
    pub osc_port: u16,

    pub send_params: bool,
    /// Off at startup: the chatbox shows the pulse to everyone in the instance.
    pub send_chatbox: bool,
    /// Supports {bpm}, {zone} and {bat} ({zona} still works).
    pub chatbox_template: String,
    /// VRChat throttles the chatbox; never less than 2 s.
    pub chatbox_interval_secs: f32,

    /// floatHR = (HR - float_min) / (float_max - float_min), clamped to 0..1.
    pub float_min: u16,
    pub float_max: u16,

    /// Max heart rate, used for the zones.
    pub max_hr: u16,

    pub always_on_top: bool,
    pub compact: bool,
    pub animate_beat: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name_filter: String::new(),
            osc_host: "127.0.0.1".into(),
            osc_port: 9000,
            send_params: true,
            send_chatbox: false,
            chatbox_template: "❤ {bpm} bpm".into(),
            chatbox_interval_secs: 2.0,
            float_min: 0,
            float_max: 255,
            max_hr: 190,
            always_on_top: false,
            compact: false,
            animate_beat: true,
        }
    }
}

impl Config {
    fn dir() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn path() -> PathBuf {
        Self::dir().join("heartbridge.json")
    }

    /// Reads `heartbridge.json`; if it doesn't exist yet, falls back to the
    /// file from before the rename (`hs500-pulso.json`) so settings carry over.
    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .or_else(|_| std::fs::read_to_string(Self::dir().join("hs500-pulso.json")))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::path(), json);
        }
    }

    pub fn chatbox_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f32(self.chatbox_interval_secs.max(2.0))
    }

    pub fn float_hr(&self, bpm: u16) -> f32 {
        let lo = self.float_min as f32;
        let hi = (self.float_max as f32).max(lo + 1.0);
        ((bpm as f32 - lo) / (hi - lo)).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_parcial_usa_defaults() {
        let c: Config = serde_json::from_str(r#"{"osc_port": 9001}"#).unwrap();
        assert_eq!(c.osc_port, 9001);
        assert!(!c.send_chatbox);
        assert_eq!(c.osc_host, "127.0.0.1");
    }

    #[test]
    fn float_hr_y_intervalo_minimo() {
        let mut c = Config::default();
        assert_eq!(c.float_hr(0), 0.0);
        assert_eq!(c.float_hr(255), 1.0);
        c.chatbox_interval_secs = 0.5;
        assert_eq!(c.chatbox_interval().as_secs_f32(), 2.0);
    }
}
