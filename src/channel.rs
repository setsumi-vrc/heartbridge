//! Open channel: a small HTTP server with the live pulse, for OBS and for
//! any app that wants it.
//!
//! - `GET /`: page with the overlay builder and the API
//! - `GET /overlay`: browser source for OBS (transparent background)
//! - `GET /api/state`: current state as JSON
//! - `GET /ws`: WebSocket with a `state` message on every change and a
//!   `beat` message on every heartbeat
//! - `GET /events`: the same stream as Server-Sent Events
//!
//! Every response allows any origin, so a web page anywhere can read it.
//! Where it listens is up to the user (127.0.0.1 by default).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, watch};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::Role;

use crate::hr::Zone;
use crate::state::{Conn, Event, Shared};

const INDEX_HTML: &str = include_str!("web/index.html");
const OVERLAY_HTML: &str = include_str!("web/overlay.html");

/// A state message also goes out this often without changes, so a reading
/// that goes stale turns into `"bpm": null` and dead connections are noticed.
const KEEPALIVE: Duration = Duration::from_secs(2);

/// How often to try again when the port can't be opened.
const RETRY: Duration = Duration::from_secs(3);

/// (enabled, host, port): the listener restarts when this changes.
type Bind = (bool, String, u16);

fn wanted(shared: &Shared) -> Bind {
    let c = shared.config();
    (
        c.channel_enabled,
        c.channel_host.trim().to_string(),
        c.channel_port,
    )
}

pub async fn run(shared: Shared) {
    loop {
        let want = wanted(&shared);
        // Dropping `stop` ends every client of this listener.
        let (stop, stop_rx) = watch::channel(());
        if want.0 {
            let host = match want.1.trim_matches(['[', ']']) {
                "" => "127.0.0.1",
                h => h,
            };
            match TcpListener::bind((host, want.2)).await {
                Ok(listener) => {
                    let addr = listener
                        .local_addr()
                        .map(|a| a.to_string())
                        .unwrap_or_else(|_| format!("{host}:{}", want.2));
                    set_status(&shared, Some(addr.clone()), None);
                    shared
                        .state()
                        .log(format!("Open channel: listening on {addr}"));
                    loop {
                        tokio::select! {
                            r = listener.accept() => {
                                if let Ok((stream, _)) = r {
                                    let _ = stream.set_nodelay(true);
                                    tokio::spawn(handle(stream, shared.clone(), stop_rx.clone()));
                                }
                            }
                            _ = shared.channel_config.notified() => {
                                if wanted(&shared) != want { break; }
                            }
                        }
                    }
                    shared.state().log("Open channel: stopped");
                }
                Err(e) => {
                    // Another app may have the port: try again every few
                    // seconds, logging only when the error changes.
                    let msg = format!("can't listen on {host}:{}: {e}", want.2);
                    if shared.state().channel.error.as_ref() != Some(&msg) {
                        shared.state().log(format!("Open channel: {msg}"));
                        set_status(&shared, None, Some(msg));
                    }
                    let _ = tokio::time::timeout(RETRY, wait_for_change(&shared, &want)).await;
                    continue;
                }
            }
        } else {
            set_status(&shared, None, None);
            wait_for_change(&shared, &want).await;
        }
        drop(stop);
        // Let typing in the address settle before binding again.
        tokio::time::sleep(Duration::from_millis(600)).await;
    }
}

async fn wait_for_change(shared: &Shared, want: &Bind) {
    loop {
        shared.channel_config.notified().await;
        if wanted(shared) != *want {
            return;
        }
    }
}

fn set_status(shared: &Shared, listening: Option<String>, error: Option<String>) {
    let mut s = shared.state();
    s.channel.listening = listening;
    s.channel.error = error;
    drop(s);
    shared.ctx.request_repaint();
}

/// Counts a live-stream client for as long as it is connected.
struct Client(Shared);

impl Client {
    fn new(shared: &Shared) -> Self {
        shared.state().channel.clients += 1;
        shared.ctx.request_repaint();
        Self(shared.clone())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let mut s = self.0.state();
        s.channel.clients = s.channel.clients.saturating_sub(1);
        drop(s);
        self.0.ctx.request_repaint();
    }
}

// ---------------------------------------------------------------- messages

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The `state` message. Also what `/api/state` returns.
pub fn state_message(shared: &Shared) -> Value {
    let max_hr = shared.config().max_hr;
    let s = shared.state();
    let bpm = s.live_bpm();
    let status = match &s.conn {
        Conn::NoAdapter(_) => "no_bluetooth",
        Conn::Searching => "searching",
        Conn::Connecting(_) => "connecting",
        Conn::Connected(_) if bpm.is_some() => "reading",
        Conn::Connected(_) => "no_contact",
    };
    let device = match &s.conn {
        Conn::Connecting(n) | Conn::Connected(n) => Some(n.clone()),
        _ => None,
    };
    let rr: Vec<u32> = match (&s.hr, bpm) {
        (Some(h), Some(_)) => h.rr_ms.iter().map(|r| r.round() as u32).collect(),
        _ => Vec::new(),
    };
    json!({
        "type": "state",
        "bpm": bpm,
        "zone": bpm.map(|b| Zone::from_bpm(b, max_hr).name()),
        "max_hr": max_hr,
        "status": status,
        "connected": matches!(s.conn, Conn::Connected(_)),
        "device": device,
        "contact": s.hr.as_ref().and_then(|h| h.contact),
        "battery": s.battery,
        "rr_ms": rr,
        "time": unix_ms(),
    })
}

fn beat_message(shared: &Shared) -> Value {
    json!({ "type": "beat", "bpm": shared.state().live_bpm(), "time": unix_ms() })
}

/// Next message for a live-stream client: on a change, a beat or the keepalive.
async fn next_message(
    shared: &Shared,
    events: &mut broadcast::Receiver<Event>,
    keepalive: &mut tokio::time::Interval,
) -> Option<String> {
    let msg = tokio::select! {
        ev = events.recv() => match ev {
            Ok(Event::Beat) => beat_message(shared),
            Ok(Event::Update) | Err(broadcast::error::RecvError::Lagged(_)) => state_message(shared),
            Err(broadcast::error::RecvError::Closed) => return None,
        },
        _ = keepalive.tick() => state_message(shared),
    };
    Some(msg.to_string())
}

// ---------------------------------------------------------------- http

struct Request {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn wants_websocket(&self) -> bool {
        self.header("upgrade")
            .is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
    }
}

/// Reads the request line and headers (up to 16 KiB).
async fn read_request(stream: &mut BufReader<TcpStream>) -> Option<Request> {
    let mut line = String::new();
    let mut total = 0;
    let mut first: Option<String> = None;
    let mut headers = Vec::new();
    loop {
        line.clear();
        let n = stream.read_line(&mut line).await.ok()?;
        total += n;
        if n == 0 || total > 16 * 1024 {
            return None;
        }
        let l = line.trim_end();
        if l.is_empty() {
            break;
        }
        if first.is_none() {
            first = Some(l.to_string());
        } else if let Some((k, v)) = l.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    let first = first?;
    let mut parts = first.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?;
    let path = target.split(['?', '#']).next().unwrap_or("/");
    let path = path.trim_end_matches('/');
    Some(Request {
        method,
        path: if path.is_empty() {
            "/".into()
        } else {
            path.into()
        },
        headers,
    })
}

const CORS: &str = "Access-Control-Allow-Origin: *\r\n\
                    Access-Control-Allow-Private-Network: true\r\n";

async fn respond(
    stream: &mut BufReader<TcpStream>,
    status: &str,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\n{CORS}Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    if !head_only {
        stream.write_all(body).await?;
    }
    stream.flush().await
}

async fn handle(stream: TcpStream, shared: Shared, stop: watch::Receiver<()>) {
    let mut stream = BufReader::new(stream);
    let Ok(Some(req)) =
        tokio::time::timeout(Duration::from_secs(10), read_request(&mut stream)).await
    else {
        return;
    };
    let head_only = req.method == "HEAD";
    let _ = match (req.method.as_str(), req.path.as_str()) {
        ("OPTIONS", _) => {
            let head = format!(
                "HTTP/1.1 204 No Content\r\n{CORS}Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n\
                 Access-Control-Allow-Headers: *\r\nAccess-Control-Max-Age: 86400\r\n\
                 Content-Length: 0\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(head.as_bytes()).await
        }
        ("GET", "/ws") if req.wants_websocket() => {
            websocket(stream, &req, shared, stop).await;
            Ok(())
        }
        ("GET", "/events") => {
            sse(stream, shared, stop).await;
            Ok(())
        }
        ("GET" | "HEAD", "/") => {
            let html = INDEX_HTML.replace("{{VERSION}}", env!("CARGO_PKG_VERSION"));
            respond(
                &mut stream,
                "200 OK",
                "text/html; charset=utf-8",
                html.as_bytes(),
                head_only,
            )
            .await
        }
        ("GET" | "HEAD", "/overlay") => {
            let body = OVERLAY_HTML.as_bytes();
            respond(
                &mut stream,
                "200 OK",
                "text/html; charset=utf-8",
                body,
                head_only,
            )
            .await
        }
        ("GET" | "HEAD", "/api/state") => {
            let body = state_message(&shared).to_string();
            respond(
                &mut stream,
                "200 OK",
                "application/json",
                body.as_bytes(),
                head_only,
            )
            .await
        }
        ("GET" | "HEAD", "/ws") => {
            let body = b"This address is a WebSocket: connect with ws://";
            respond(
                &mut stream,
                "426 Upgrade Required",
                "text/plain",
                body,
                head_only,
            )
            .await
        }
        ("GET" | "HEAD", _) => {
            respond(
                &mut stream,
                "404 Not Found",
                "text/plain",
                b"Not found",
                head_only,
            )
            .await
        }
        _ => {
            let body = b"Only GET";
            respond(
                &mut stream,
                "405 Method Not Allowed",
                "text/plain",
                body,
                head_only,
            )
            .await
        }
    };
}

async fn websocket(
    mut stream: BufReader<TcpStream>,
    req: &Request,
    shared: Shared,
    mut stop: watch::Receiver<()>,
) {
    let Some(key) = req.header("sec-websocket-key") else {
        let _ = respond(
            &mut stream,
            "400 Bad Request",
            "text/plain",
            b"Missing key",
            false,
        )
        .await;
        return;
    };
    let head = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\r\n",
        derive_accept_key(key.as_bytes())
    );
    if stream.write_all(head.as_bytes()).await.is_err() || stream.flush().await.is_err() {
        return;
    }

    let ws = WebSocketStream::from_raw_socket(stream, Role::Server, None).await;
    let (mut tx, mut rx) = ws.split();
    let _client = Client::new(&shared);
    let mut events = shared.events.subscribe();
    let mut keepalive = tokio::time::interval(KEEPALIVE);

    // The first keepalive tick fires at once: that is the initial state.
    loop {
        tokio::select! {
            msg = next_message(&shared, &mut events, &mut keepalive) => {
                let Some(msg) = msg else { break };
                if tx.send(Message::text(msg)).await.is_err() { return; }
            }
            incoming = rx.next() => match incoming {
                // Pings are answered by the library on the next write.
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return,
                Some(Ok(_)) => {}
            },
            _ = stop.changed() => break,
        }
    }
    let _ = tx.send(Message::Close(None)).await;
}

async fn sse(mut stream: BufReader<TcpStream>, shared: Shared, mut stop: watch::Receiver<()>) {
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\n\
         {CORS}Connection: keep-alive\r\n\r\nretry: 1000\n\n"
    );
    if stream.write_all(head.as_bytes()).await.is_err() {
        return;
    }
    let _client = Client::new(&shared);
    let mut events = shared.events.subscribe();
    let mut keepalive = tokio::time::interval(KEEPALIVE);
    loop {
        tokio::select! {
            msg = next_message(&shared, &mut events, &mut keepalive) => {
                let Some(msg) = msg else { break };
                let frame = format!("data: {msg}\n\n");
                if stream.write_all(frame.as_bytes()).await.is_err()
                    || stream.flush().await.is_err()
                {
                    return;
                }
            }
            _ = stop.changed() => break,
        }
    }
}
