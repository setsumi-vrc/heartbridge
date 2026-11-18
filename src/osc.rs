//! Sending to VRChat over OSC (UDP). Minimal encoder: just what VRChat
//! uses (int, float, bool, string), with no extra dependencies.

use std::net::UdpSocket;
use std::time::{Duration, Instant};

use tokio::time::sleep_until;

use crate::config::Config;
use crate::hr::Zone;
use crate::state::Shared;

pub enum Arg<'a> {
    Int(i32),
    Float(f32),
    Bool(bool),
    Str(&'a str),
}

fn push_padded(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(s.as_bytes());
    buf.push(0);
    while !buf.len().is_multiple_of(4) {
        buf.push(0);
    }
}

pub fn encode(addr: &str, args: &[Arg]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);
    push_padded(&mut buf, addr);
    let mut tags = String::from(",");
    for a in args {
        tags.push(match a {
            Arg::Int(_) => 'i',
            Arg::Float(_) => 'f',
            Arg::Bool(true) => 'T',
            Arg::Bool(false) => 'F',
            Arg::Str(_) => 's',
        });
    }
    push_padded(&mut buf, &tags);
    for a in args {
        match a {
            Arg::Int(v) => buf.extend_from_slice(&v.to_be_bytes()),
            Arg::Float(v) => buf.extend_from_slice(&v.to_be_bytes()),
            Arg::Bool(_) => {}
            Arg::Str(s) => push_padded(&mut buf, s),
        }
    }
    buf
}

/// VRChat's chatbox accepts up to 144 characters.
pub fn render_chatbox(template: &str, bpm: u16, zone: Zone, battery: Option<u8>) -> String {
    let bat = battery
        .map(|b| format!("{b}%"))
        .unwrap_or_else(|| "?".into());
    let text = template
        .replace("{bpm}", &bpm.to_string())
        .replace("{zone}", zone.name())
        .replace("{zona}", zone.name())
        .replace("{bat}", &bat);
    text.chars().take(144).collect()
}

struct Sender {
    socket: UdpSocket,
}

impl Sender {
    fn new() -> std::io::Result<Self> {
        Ok(Self {
            socket: UdpSocket::bind("0.0.0.0:0")?,
        })
    }

    fn send(&self, cfg: &Config, addr: &str, args: &[Arg]) -> std::io::Result<()> {
        let packet = encode(addr, args);
        self.socket
            .send_to(&packet, (cfg.osc_host.as_str(), cfg.osc_port))?;
        Ok(())
    }

    fn param(&self, cfg: &Config, name: &str, arg: Arg) -> std::io::Result<()> {
        self.send(cfg, &format!("/avatar/parameters/{name}"), &[arg])
    }

    /// HRtoVRChat_OSC-compatible parameters.
    fn heart_params(&self, cfg: &Config, bpm: Option<u16>, connected: bool) -> std::io::Result<()> {
        let hr = bpm.unwrap_or(0);
        self.param(cfg, "HR", Arg::Int(hr as i32))?;
        self.param(cfg, "onesHR", Arg::Int((hr % 10) as i32))?;
        self.param(cfg, "tensHR", Arg::Int((hr / 10 % 10) as i32))?;
        self.param(cfg, "hundredsHR", Arg::Int((hr / 100 % 10) as i32))?;
        self.param(cfg, "floatHR", Arg::Float(cfg.float_hr(hr)))?;
        self.param(cfg, "isHRConnected", Arg::Bool(connected))?;
        self.param(cfg, "isHRActive", Arg::Bool(bpm.is_some()))?;
        Ok(())
    }

    fn chatbox(&self, cfg: &Config, text: &str) -> std::io::Result<()> {
        // [text, send immediately, play notification sound]
        self.send(
            cfg,
            "/chatbox/input",
            &[Arg::Str(text), Arg::Bool(true), Arg::Bool(false)],
        )
    }
}

const BEAT_PULSE: Duration = Duration::from_millis(100);
/// Re-sends the parameters periodically (avatar changes reset them).
const PARAM_REFRESH: Duration = Duration::from_secs(5);

fn far_future() -> tokio::time::Instant {
    tokio::time::Instant::now() + Duration::from_secs(3600)
}

pub async fn run(shared: Shared) {
    let sender = match Sender::new() {
        Ok(s) => s,
        Err(e) => {
            let mut s = shared.state();
            s.osc.error = Some(format!("could not open the UDP socket: {e}"));
            s.log(format!("OSC: could not open the UDP socket: {e}"));
            return;
        }
    };

    let mut cfg = shared.config();
    let mut next_beat: Option<tokio::time::Instant> = None;
    let mut beat_off: Option<tokio::time::Instant> = None;
    let mut next_chat = tokio::time::Instant::now();
    let mut next_refresh = tokio::time::Instant::now();

    loop {
        let (bpm, connected, zone_bpm, battery) = {
            let s = shared.state();
            let bpm = s.live_bpm();
            (
                bpm,
                matches!(s.conn, crate::state::Conn::Connected(_)),
                bpm,
                s.battery,
            )
        };

        // Schedule the beat from the current BPM.
        match bpm {
            Some(b) if next_beat.is_none() => {
                next_beat = Some(tokio::time::Instant::now() + beat_interval(b) / 4)
            }
            None => next_beat = None,
            _ => {}
        }

        let mut result = Ok(());
        tokio::select! {
            _ = shared.hr_updated.notified() => {
                if cfg.send_params {
                    let bpm = shared.state().live_bpm();
                    result = sender.heart_params(&cfg, bpm, connected);
                    next_refresh = tokio::time::Instant::now() + PARAM_REFRESH;
                }
            }
            _ = sleep_until(next_refresh) => {
                if cfg.send_params {
                    result = sender.heart_params(&cfg, bpm, connected);
                }
                next_refresh = tokio::time::Instant::now() + PARAM_REFRESH;
            }
            _ = sleep_until(next_beat.unwrap_or_else(far_future)) => {
                let now = tokio::time::Instant::now();
                shared.state().last_beat = Some(Instant::now());
                shared.ctx.request_repaint();
                if cfg.send_params {
                    result = sender.param(&cfg, "isHRBeat", Arg::Bool(true));
                }
                beat_off = Some(now + BEAT_PULSE);
                next_beat = bpm.map(|b| now + beat_interval(b));
            }
            _ = sleep_until(beat_off.unwrap_or_else(far_future)) => {
                beat_off = None;
                if cfg.send_params {
                    result = sender.param(&cfg, "isHRBeat", Arg::Bool(false));
                }
            }
            _ = sleep_until(next_chat) => {
                next_chat = tokio::time::Instant::now() + cfg.chatbox_interval();
                if cfg.send_chatbox && let Some(b) = zone_bpm {
                    let text = render_chatbox(&cfg.chatbox_template, b, Zone::from_bpm(b, cfg.max_hr), battery);
                    result = sender.chatbox(&cfg, &text);
                    if result.is_ok() {
                        let mut s = shared.state();
                        s.osc.last_chatbox = Some(Instant::now());
                        s.osc.last_chatbox_text = text;
                    }
                }
            }
            _ = shared.config_changed.notified() => {
                let new = shared.config();
                if cfg.send_chatbox && !new.send_chatbox {
                    // Clear the bubble right away instead of waiting for it to expire.
                    result = sender.chatbox(&cfg, "");
                    shared.state().osc.last_chatbox_text.clear();
                }
                if !cfg.send_chatbox && new.send_chatbox {
                    next_chat = tokio::time::Instant::now();
                }
                if cfg.send_params && !new.send_params {
                    result = sender.heart_params(&cfg, None, false);
                }
                cfg = new;
            }
        }

        let mut s = shared.state();
        match result {
            Ok(()) => {
                if cfg.send_params || cfg.send_chatbox {
                    s.osc.last_send = Some(Instant::now());
                }
                if s.osc.error.take().is_some() {
                    s.log("OSC: sending again");
                    shared.ctx.request_repaint();
                }
            }
            Err(e) => {
                if s.osc.error.is_none() {
                    s.log(format!("OSC error: {e}"));
                }
                s.osc.error = Some(format!("{e}"));
                shared.ctx.request_repaint();
            }
        }
    }
}

fn beat_interval(bpm: u16) -> Duration {
    Duration::from_secs_f32(60.0 / bpm.clamp(20, 250) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_int() {
        assert_eq!(
            encode("/a", &[Arg::Int(1)]),
            b"/a\0\0,i\0\0\0\0\0\x01".to_vec()
        );
    }

    #[test]
    fn encode_float_y_bool() {
        let p = encode("/avatar/parameters/floatHR", &[Arg::Float(0.5)]);
        assert_eq!(p.len() % 4, 0);
        assert_eq!(&p[p.len() - 8..p.len() - 4], b",f\0\0");
        assert_eq!(&p[p.len() - 4..], &0.5f32.to_be_bytes());

        let b = encode("/x", &[Arg::Bool(true)]);
        assert_eq!(b, b"/x\0\0,T\0\0".to_vec());
    }

    #[test]
    fn encode_chatbox() {
        let p = encode(
            "/chatbox/input",
            &[Arg::Str("hola"), Arg::Bool(true), Arg::Bool(false)],
        );
        // "/chatbox/input\0\0" (16) + ",sTF\0\0\0\0" (8) + "hola\0\0\0\0" (8)
        assert_eq!(p.len(), 32);
        assert_eq!(&p[16..24], b",sTF\0\0\0\0");
        assert_eq!(&p[24..], b"hola\0\0\0\0");
    }

    #[test]
    fn plantilla_chatbox() {
        let t = render_chatbox(
            "❤ {bpm} bpm · {zone} · {bat}",
            128,
            Zone::Moderate,
            Some(90),
        );
        assert_eq!(t, "❤ 128 bpm · Moderate · 90%");
        assert_eq!(render_chatbox("{zona}", 60, Zone::Rest, None), "Rest");
        let largo = render_chatbox(&"x".repeat(300), 60, Zone::Rest, None);
        assert_eq!(largo.chars().count(), 144);
    }
}
