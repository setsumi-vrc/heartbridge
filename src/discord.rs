//! Discord Rich Presence: your pulse on your Discord profile.
//!
//! Talks to the Discord app running on this computer over its local IPC
//! socket (a named pipe on Windows, a Unix socket elsewhere). No bot, no
//! token, nothing leaves the PC except through Discord itself. Off by
//! default: everyone who can see your profile sees it.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::hr::Zone;
use crate::state::Shared;

/// The HeartBridge application in the Discord developer portal. Its name is
/// what Discord shows ("Playing HeartBridge"). `discord_app_id` in the
/// settings file overrides it.
pub const APP_ID: &str = "1572701918751494165";

const ICON_URL: &str =
    "https://raw.githubusercontent.com/setsumi-vrc/heartbridge/main/assets/icon.png";
const REPO_URL: &str = "https://github.com/setsumi-vrc/heartbridge";

/// Discord accepts about one presence update every 15 seconds.
const MIN_UPDATE: Duration = Duration::from_secs(15);
/// How often to try again while Discord isn't running.
const RETRY: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Default, PartialEq)]
pub enum DiscordStatus {
    #[default]
    Off,
    /// Discord isn't running (or not reachable).
    NotRunning,
    /// Connected; the user's display name.
    Connected(String),
    Error(String),
}

trait Pipe: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Pipe for T {}

#[cfg(windows)]
async fn connect() -> std::io::Result<Box<dyn Pipe>> {
    use tokio::net::windows::named_pipe::ClientOptions;
    for i in 0..10 {
        if let Ok(p) = ClientOptions::new().open(format!(r"\\.\pipe\discord-ipc-{i}")) {
            return Ok(Box::new(p));
        }
    }
    Err(std::io::ErrorKind::NotFound.into())
}

#[cfg(unix)]
async fn connect() -> std::io::Result<Box<dyn Pipe>> {
    use std::path::PathBuf;
    let mut bases: Vec<PathBuf> = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect();
    bases.push("/tmp".into());
    // Plain installs, then Flatpak and Snap sandboxes.
    let subdirs = ["", "app/com.discordapp.Discord", "snap.discord"];
    for base in &bases {
        for sub in subdirs {
            for i in 0..10 {
                let path = base.join(sub).join(format!("discord-ipc-{i}"));
                if let Ok(s) = tokio::net::UnixStream::connect(&path).await {
                    return Ok(Box::new(s));
                }
            }
        }
    }
    Err(std::io::ErrorKind::NotFound.into())
}

// ---------------------------------------------------------------- frames

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const OP_PING: u32 = 3;
const OP_PONG: u32 = 4;

async fn write_frame(pipe: &mut dyn Pipe, op: u32, body: &Value) -> std::io::Result<()> {
    let body = body.to_string();
    let mut buf = Vec::with_capacity(8 + body.len());
    buf.extend_from_slice(&op.to_le_bytes());
    buf.extend_from_slice(&(body.len() as u32).to_le_bytes());
    buf.extend_from_slice(body.as_bytes());
    pipe.write_all(&buf).await?;
    pipe.flush().await
}

async fn read_frame(pipe: &mut dyn Pipe) -> std::io::Result<(u32, Value)> {
    let mut head = [0u8; 8];
    pipe.read_exact(&mut head).await?;
    let op = u32::from_le_bytes(head[..4].try_into().unwrap_or_default());
    let len = u32::from_le_bytes(head[4..].try_into().unwrap_or_default()) as usize;
    if len > 64 * 1024 {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    let mut body = vec![0u8; len];
    pipe.read_exact(&mut body).await?;
    Ok((op, serde_json::from_slice(&body).unwrap_or(Value::Null)))
}

/// Reads until a reply arrives, answering pings on the way.
async fn read_reply(pipe: &mut dyn Pipe) -> anyhow::Result<Value> {
    let read = async {
        loop {
            let (op, body) = read_frame(pipe).await?;
            match op {
                OP_PING => write_frame(pipe, OP_PONG, &body).await?,
                OP_CLOSE => {
                    let msg = body["message"].as_str().unwrap_or("closed the connection");
                    anyhow::bail!("{msg}");
                }
                _ => return Ok(body),
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(5), read)
        .await
        .map_err(|_| anyhow::anyhow!("Discord didn't answer"))?
}

/// Says hello; returns the name of the signed-in user.
async fn handshake(pipe: &mut dyn Pipe, app_id: &str) -> anyhow::Result<String> {
    write_frame(pipe, OP_HANDSHAKE, &json!({ "v": 1, "client_id": app_id })).await?;
    let ready = read_reply(pipe).await?;
    if ready["evt"] != "READY" {
        anyhow::bail!("unexpected answer from Discord");
    }
    let user = &ready["data"]["user"];
    let name = user["global_name"]
        .as_str()
        .or(user["username"].as_str())
        .unwrap_or("Discord");
    Ok(name.to_string())
}

/// Sets (or with `None`, clears) the presence.
async fn set_activity(
    pipe: &mut dyn Pipe,
    nonce: u64,
    activity: Option<&Value>,
) -> anyhow::Result<()> {
    let msg = json!({
        "cmd": "SET_ACTIVITY",
        "args": { "pid": std::process::id(), "activity": activity },
        "nonce": format!("heartbridge-{nonce}"),
    });
    write_frame(pipe, OP_FRAME, &msg).await?;
    let reply = read_reply(pipe).await?;
    if reply["evt"] == "ERROR" {
        let msg = reply["data"]["message"].as_str().unwrap_or("unknown error");
        anyhow::bail!("Discord refused the status: {msg}");
    }
    Ok(())
}

// ---------------------------------------------------------------- presence

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// What the profile shows: "❤ 82 bpm", the zone, and how long it has been live.
pub fn activity(bpm: u16, zone: Zone, since: u64) -> Value {
    json!({
        "details": format!("❤ {bpm} bpm"),
        "state": format!("{} zone", zone.name()),
        "timestamps": { "start": since },
        "assets": { "large_image": ICON_URL, "large_text": "HeartBridge" },
        "buttons": [{ "label": "Get HeartBridge", "url": REPO_URL }],
        // Show the details ("❤ 82 bpm") in the member list instead of the app name.
        "status_display_type": 2,
    })
}

fn set_status(shared: &Shared, status: DiscordStatus) {
    let mut s = shared.state();
    if s.discord != status {
        let line = match &status {
            DiscordStatus::Off => "Discord: off".to_string(),
            DiscordStatus::NotRunning => "Discord: waiting for the Discord app".to_string(),
            DiscordStatus::Connected(u) => format!("Discord: connected as {u}"),
            DiscordStatus::Error(e) => format!("Discord: {e}"),
        };
        s.log(line);
        s.discord = status;
        drop(s);
        shared.ctx.request_repaint();
    }
}

/// Waits `dur` or until the settings change, whichever comes first.
async fn pause(shared: &Shared, dur: Duration) {
    let _ = tokio::time::timeout(dur, shared.discord_config.notified()).await;
}

pub async fn run(shared: Shared) {
    loop {
        let cfg = shared.config();
        if !cfg.discord_enabled {
            set_status(&shared, DiscordStatus::Off);
            shared.discord_config.notified().await;
            continue;
        }
        let app_id = match cfg.discord_app_id.trim() {
            "" => APP_ID,
            id => id,
        }
        .to_string();
        if app_id.is_empty() {
            set_status(&shared, DiscordStatus::Error("no application ID".into()));
            shared.discord_config.notified().await;
            continue;
        }

        let Ok(mut pipe) = connect().await else {
            set_status(&shared, DiscordStatus::NotRunning);
            pause(&shared, RETRY).await;
            continue;
        };
        match handshake(pipe.as_mut(), &app_id).await {
            Ok(user) => set_status(&shared, DiscordStatus::Connected(user)),
            Err(e) => {
                set_status(&shared, DiscordStatus::Error(format!("{e:#}")));
                pause(&shared, RETRY * 2).await;
                continue;
            }
        }

        if let Err(e) = session(&shared, pipe.as_mut()).await {
            shared
                .state()
                .log(format!("Discord: connection lost ({e:#})"));
            set_status(&shared, DiscordStatus::NotRunning);
            pause(&shared, Duration::from_secs(3)).await;
        }
    }
}

/// Keeps the presence in step with the pulse until Discord is turned off
/// (then clears it) or the connection drops.
async fn session(shared: &Shared, pipe: &mut dyn Pipe) -> anyhow::Result<()> {
    let mut sent: Option<Value> = None;
    let mut first = true;
    let mut next_allowed = Instant::now();
    let mut live_since: Option<u64> = None;
    let mut nonce = 0u64;
    let mut tick = tokio::time::interval(Duration::from_secs(3));

    loop {
        tokio::select! {
            _ = tick.tick() => {}
            _ = shared.discord_config.notified() => {
                if !shared.config().discord_enabled {
                    nonce += 1;
                    let _ = set_activity(pipe, nonce, None).await;
                    let _ = write_frame(pipe, OP_CLOSE, &json!({})).await;
                    return Ok(());
                }
            }
        }

        let max_hr = shared.config().max_hr;
        let bpm = shared.state().live_bpm();
        live_since = match (bpm, live_since) {
            (Some(_), None) => Some(unix_secs()),
            (Some(_), since) => since,
            (None, _) => None,
        };
        // No reading: no presence, rather than a stale or empty number.
        let wanted = bpm
            .zip(live_since)
            .map(|(b, since)| activity(b, Zone::from_bpm(b, max_hr), since));

        if (first || wanted != sent) && Instant::now() >= next_allowed {
            nonce += 1;
            set_activity(pipe, nonce, wanted.as_ref()).await?;
            first = false;
            sent = wanted;
            next_allowed = Instant::now() + MIN_UPDATE;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_text() {
        let a = activity(82, Zone::Moderate, 1_700_000_000);
        assert_eq!(a["details"], "❤ 82 bpm");
        assert_eq!(a["state"], "Moderate zone");
        assert_eq!(a["timestamps"]["start"], 1_700_000_000u64);
    }
}
