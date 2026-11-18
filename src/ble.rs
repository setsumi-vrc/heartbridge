//! Connection to the strap over Bluetooth LE (standard Heart Rate service).

use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, bleuuid::uuid_from_u16};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use uuid::Uuid;

use crate::hr::parse_heart_rate;
use crate::state::{Conn, Shared};

const HR_SERVICE: Uuid = uuid_from_u16(0x180D);
const HR_MEASUREMENT: Uuid = uuid_from_u16(0x2A37);
const BATTERY_LEVEL: Uuid = uuid_from_u16(0x2A19);

/// If no notification arrives within this long, assume the connection dropped.
const SILENCE_TIMEOUT: Duration = Duration::from_secs(8);

pub async fn run(shared: Shared) {
    if std::env::var_os("HEARTBRIDGE_DEMO").is_some() {
        return demo(shared).await;
    }
    loop {
        let adapter = match first_adapter().await {
            Ok(a) => a,
            Err(e) => {
                shared.state().log(format!("Bluetooth: {e:#}"));
                set_conn(&shared, Conn::NoAdapter(format!("{e:#}")));
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        set_conn(&shared, Conn::Searching);
        let sensor = match find_sensor(&adapter, &shared).await {
            Ok(p) => p,
            Err(e) => {
                shared.state().log(format!("Scan: {e:#}, restarting"));
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        match run_session(&sensor, &shared).await {
            Ok(()) => shared
                .state()
                .log("Session ended: no data for 8 s or the strap disconnected"),
            Err(e) => shared.state().log(format!("Session error: {e:#}")),
        }
        let _ = sensor.disconnect().await;
        shared.state().disconnect();
        set_conn(&shared, Conn::Searching);
        shared.hr_updated.notify_one();
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Simulated pulse (HEARTBRIDGE_DEMO=1), for testing the window and OSC without the strap.
async fn demo(shared: Shared) {
    set_conn(&shared, Conn::Connected("Demo (simulated pulse)".into()));
    shared.state().battery = Some(87);
    let start = Instant::now();
    loop {
        let t = start.elapsed().as_secs_f32();
        let bpm = (118.0 + 22.0 * (t / 9.0).sin() + 4.0 * (t * 1.7).sin()) as u16;
        let rr = (60_000.0 / bpm as f32 * 1.024) as u16;
        let packet = [0x16, bpm as u8, rr.to_le_bytes()[0], rr.to_le_bytes()[1]];
        if let Some(m) = parse_heart_rate(&packet) {
            let mut s = shared.state();
            s.packets += 1;
            s.last_packet = packet.to_vec();
            s.push_hr(m);
            drop(s);
            shared.changed();
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

fn set_conn(shared: &Shared, conn: Conn) {
    let mut s = shared.state();
    if s.conn != conn {
        let line = match &conn {
            Conn::NoAdapter(e) => format!("State: no Bluetooth ({e})"),
            Conn::Searching => "State: scanning for a heart-rate sensor".to_string(),
            Conn::Connecting(n) => format!("State: connecting to {n}"),
            Conn::Connected(n) => format!("State: connected to {n}"),
        };
        s.log(line);
        s.conn = conn;
        drop(s);
        let _ = shared.events.send(crate::state::Event::Update);
        shared.ctx.request_repaint();
    }
}

async fn first_adapter() -> Result<Adapter> {
    let manager = Manager::new().await.context("could not start Bluetooth")?;
    manager
        .adapters()
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no Bluetooth adapter found"))
}

async fn find_sensor(adapter: &Adapter, shared: &Shared) -> Result<Peripheral> {
    adapter.start_scan(ScanFilter::default()).await?;
    let started = Instant::now();
    loop {
        let filter = shared.config().name_filter.trim().to_lowercase();
        for p in adapter.peripherals().await? {
            let Some(props) = p.properties().await? else {
                continue;
            };
            let name = props.local_name.clone().unwrap_or_default();
            let lname = name.to_lowercase();
            let matches = if filter.is_empty() {
                props.services.contains(&HR_SERVICE)
                    || lname.contains("hs500")
                    || lname.contains("geoid")
            } else {
                lname.contains(&filter)
            };
            if matches {
                adapter.stop_scan().await.ok();
                let label = if name.is_empty() {
                    "unnamed sensor".into()
                } else {
                    name
                };
                shared.state().log(format!(
                    "Found {label} [{}], RSSI {}",
                    props.address,
                    props.rssi.map_or("?".into(), |r| format!("{r} dBm"))
                ));
                set_conn(shared, Conn::Connecting(label));
                return Ok(p);
            }
        }
        // Restart the scan every so often: WinRT sometimes goes deaf.
        if started.elapsed() > Duration::from_secs(60) {
            adapter.stop_scan().await.ok();
            return Err(anyhow!("timeout"));
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

async fn run_session(sensor: &Peripheral, shared: &Shared) -> Result<()> {
    sensor.connect().await.context("could not connect")?;
    sensor.discover_services().await?;

    let name = sensor
        .properties()
        .await?
        .and_then(|p| p.local_name)
        .unwrap_or_else(|| "unnamed sensor".into());

    let chars = sensor.characteristics();
    if let Some(batt) = chars.iter().find(|c| c.uuid == BATTERY_LEVEL)
        && let Ok(v) = sensor.read(batt).await
    {
        let mut s = shared.state();
        s.battery = v.first().copied();
        let line = format!(
            "Battery: {}",
            s.battery.map_or("unknown".into(), |b| format!("{b}%"))
        );
        s.log(line);
    }

    let hr_char = chars
        .iter()
        .find(|c| c.uuid == HR_MEASUREMENT)
        .ok_or_else(|| anyhow!("the device has no heart-rate characteristic"))?;

    let mut notifications = sensor.notifications().await?;
    sensor.subscribe(hr_char).await?;
    set_conn(shared, Conn::Connected(name));

    loop {
        let next = tokio::time::timeout(SILENCE_TIMEOUT, notifications.next()).await;
        let Ok(Some(n)) = next else { return Ok(()) };
        if n.uuid != HR_MEASUREMENT {
            continue;
        }
        let parsed = parse_heart_rate(&n.value);
        {
            let mut s = shared.state();
            s.packets += 1;
            s.last_packet = n.value.clone();
            if parsed.is_none() {
                s.log(format!("Unreadable packet: {:02X?}", n.value));
            }
        }
        if let Some(m) = parsed {
            shared.state().push_hr(m);
            shared.changed();
        }
    }
}
