//! Settings persisted to `heartbridge.json` (location per platform, see `Config::dir`).

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

    /// Open channel: HTTP, WebSocket and SSE with the live pulse, for OBS
    /// and any other app. 127.0.0.1 = this PC only; 0.0.0.0 = the network.
    pub channel_enabled: bool,
    pub channel_host: String,
    pub channel_port: u16,

    /// Discord Rich Presence. Off at startup: everyone who can see your
    /// profile sees it.
    pub discord_enabled: bool,
    /// Empty = HeartBridge's own Discord application.
    pub discord_app_id: String,
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
            channel_enabled: true,
            channel_host: "127.0.0.1".into(),
            channel_port: 7171,
            discord_enabled: false,
            discord_app_id: String::new(),
        }
    }
}

#[cfg(not(windows))]
fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

impl Config {
    /// Windows: next to the executable (portable). macOS:
    /// ~/Library/Application Support/HeartBridge. Linux: ~/.config/heartbridge.
    fn dir() -> PathBuf {
        #[cfg(windows)]
        {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."))
        }
        #[cfg(target_os = "macos")]
        {
            home().join("Library/Application Support/HeartBridge")
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| home().join(".config"))
                .join("heartbridge")
        }
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
            let _ = std::fs::create_dir_all(Self::dir());
            let _ = std::fs::write(Self::path(), json);
        }
    }

    /// Address to show and link to (a wildcard bind is reachable locally at 127.0.0.1).
    pub fn channel_url(&self) -> String {
        let host = match self.channel_host.trim() {
            "" | "0.0.0.0" | "::" | "[::]" => "127.0.0.1",
            h => h,
        };
        if host.contains(':') && !host.starts_with('[') {
            format!("http://[{host}]:{}", self.channel_port)
        } else {
            format!("http://{host}:{}", self.channel_port)
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
    fn channel_url_for_every_bind() {
        let mut c = Config::default();
        assert_eq!(c.channel_url(), "http://127.0.0.1:7171");
        c.channel_host = "0.0.0.0".into();
        assert_eq!(c.channel_url(), "http://127.0.0.1:7171");
        c.channel_host = "192.168.1.20".into();
        c.channel_port = 8080;
        assert_eq!(c.channel_url(), "http://192.168.1.20:8080");
        c.channel_host = "::1".into();
        assert_eq!(c.channel_url(), "http://[::1]:8080");
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
