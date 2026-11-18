//! State shared between the BLE task, the OSC task and the window.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::hr::HeartRate;

/// How much history the window keeps.
pub const HISTORY_SPAN: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, PartialEq)]
pub enum Conn {
    NoAdapter(String),
    Searching,
    Connecting(String),
    Connected(String),
}

#[derive(Debug, Default, Clone)]
pub struct OscStatus {
    pub last_send: Option<Instant>,
    pub last_chatbox: Option<Instant>,
    pub last_chatbox_text: String,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct State {
    pub conn: Conn,
    pub battery: Option<u8>,
    pub hr: Option<HeartRate>,
    pub last_hr_at: Option<Instant>,
    /// (instant, bpm) of the last two minutes; 0 = no reading.
    pub history: VecDeque<(Instant, u16)>,
    pub last_beat: Option<Instant>,
    pub osc: OscStatus,
    /// For the debug window.
    pub started: Instant,
    pub log: VecDeque<(Duration, String)>,
    pub packets: u64,
    pub last_packet: Vec<u8>,
}

const LOG_LINES: usize = 400;

impl Default for State {
    fn default() -> Self {
        Self {
            conn: Conn::Searching,
            battery: None,
            hr: None,
            last_hr_at: None,
            history: VecDeque::new(),
            last_beat: None,
            osc: OscStatus::default(),
            started: Instant::now(),
            log: VecDeque::new(),
            packets: 0,
            last_packet: Vec::new(),
        }
    }
}

impl State {
    /// Adds a line to the debug log (keeps the last 400).
    pub fn log(&mut self, msg: impl Into<String>) {
        let at = self.started.elapsed();
        self.log.push_back((at, msg.into()));
        while self.log.len() > LOG_LINES {
            self.log.pop_front();
        }
    }

    pub fn push_hr(&mut self, hr: HeartRate) {
        let now = Instant::now();
        let bpm = if hr.is_reading() { hr.bpm } else { 0 };
        self.history.push_back((now, bpm));
        while self
            .history
            .front()
            .is_some_and(|(t, _)| now.duration_since(*t) > HISTORY_SPAN)
        {
            self.history.pop_front();
        }
        self.hr = Some(hr);
        self.last_hr_at = Some(now);
    }

    /// Current BPM only if connected and with a recent valid reading.
    pub fn live_bpm(&self) -> Option<u16> {
        if !matches!(self.conn, Conn::Connected(_)) {
            return None;
        }
        let fresh = self
            .last_hr_at
            .is_some_and(|t| t.elapsed() < Duration::from_secs(5));
        self.hr
            .as_ref()
            .filter(|h| fresh && h.is_reading())
            .map(|h| h.bpm)
    }

    pub fn disconnect(&mut self) {
        self.hr = None;
        self.last_hr_at = None;
        self.battery = None;
    }
}

/// Everything the threads share. The UI changes the config and signals
/// `config_changed`; the tasks wake the window with `ctx.request_repaint()`.
#[derive(Clone)]
pub struct Shared {
    pub state: Arc<Mutex<State>>,
    pub config: Arc<Mutex<Config>>,
    pub hr_updated: Arc<tokio::sync::Notify>,
    pub config_changed: Arc<tokio::sync::Notify>,
    pub ctx: egui::Context,
}

impl Shared {
    pub fn new(config: Config, ctx: egui::Context) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
            config: Arc::new(Mutex::new(config)),
            hr_updated: Arc::new(tokio::sync::Notify::new()),
            config_changed: Arc::new(tokio::sync::Notify::new()),
            ctx,
        }
    }

    pub fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn config(&self) -> Config {
        self.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}
