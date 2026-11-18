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

/// What the open channel (local HTTP/WebSocket server) is doing.
#[derive(Debug, Default, Clone)]
pub struct ChannelStatus {
    /// Address it is listening on, when it is.
    pub listening: Option<String>,
    pub error: Option<String>,
    /// Apps connected to the live stream (WebSocket or SSE).
    pub clients: usize,
}

/// Published to everything listening on the open channel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// The pulse, the connection or the battery changed.
    Update,
    /// One heartbeat, timed from the current BPM.
    Beat,
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
    pub channel: ChannelStatus,
    pub discord: crate::discord::DiscordStatus,
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
            channel: ChannelStatus::default(),
            discord: Default::default(),
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
    /// Same as `config_changed`, for the open channel (a `Notify` wakes one task).
    pub channel_config: Arc<tokio::sync::Notify>,
    pub discord_config: Arc<tokio::sync::Notify>,
    pub events: tokio::sync::broadcast::Sender<Event>,
    pub ctx: egui::Context,
}

impl Shared {
    pub fn new(config: Config, ctx: egui::Context) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
            config: Arc::new(Mutex::new(config)),
            hr_updated: Arc::new(tokio::sync::Notify::new()),
            config_changed: Arc::new(tokio::sync::Notify::new()),
            channel_config: Arc::new(tokio::sync::Notify::new()),
            discord_config: Arc::new(tokio::sync::Notify::new()),
            events: tokio::sync::broadcast::channel(64).0,
            ctx,
        }
    }

    pub fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Tells everything listening that the pulse, connection or battery
    /// changed: OSC, the open channel and the window.
    pub fn changed(&self) {
        self.hr_updated.notify_one();
        let _ = self.events.send(Event::Update);
        self.ctx.request_repaint();
    }

    pub fn config(&self) -> Config {
        self.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}
