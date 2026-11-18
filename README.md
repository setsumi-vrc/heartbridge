<p align="center">
  <img src="assets/icon.png" width="128" alt="HeartBridge icon">
</p>

<h1 align="center">HeartBridge</h1>

<p align="center">
  <b>Your heart rate in VRChat.</b><br>
  One small app for Windows, Linux and macOS: a Bluetooth chest strap in, your pulse on your avatar, in the chatbox, on stream and in any app you write.
</p>

<p align="center">
  <a href="https://github.com/setsumi-vrc/heartbridge/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/setsumi-vrc/heartbridge?style=flat-square&color=ff3b47"></a>
  <a href="https://github.com/setsumi-vrc/heartbridge/actions/workflows/build.yml"><img alt="Build" src="https://img.shields.io/github/actions/workflow/status/setsumi-vrc/heartbridge/build.yml?branch=main&style=flat-square&label=build"></a>
  <img alt="Windows, Linux and macOS" src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-5aaaff?style=flat-square">
  <img alt="Rust" src="https://img.shields.io/badge/made%20with-Rust-dea584?style=flat-square">
  <a href="LICENSE"><img alt="MIT license" src="https://img.shields.io/badge/license-MIT-b28cff?style=flat-square"></a>
</p>

<p align="center">
  <a href="#-quick-start">Quick start</a> ·
  <a href="#-features">Features</a> ·
  <a href="#-open-channel">Open channel</a> ·
  <a href="#-troubleshooting">Troubleshooting</a> ·
  <a href="#-building-from-source">Building</a> ·
  <a href="CHANGELOG.md">Changelog</a>
</p>

---

HeartBridge reads the standard Bluetooth LE Heart Rate service from a chest strap and sends it
to VRChat over OSC. It drives avatar parameters with the same names as
[HRtoVRChat_OSC](https://github.com/200Tigersbloxed/HRtoVRChat_OSC), so existing heart-rate
prefabs work as they are, and it can put your BPM in the chatbox. It was made for the GEOID HS500
and works with any strap that uses the standard service.

<p align="center">
  <img src="assets/window.png" width="720" alt="HeartBridge main window">
</p>

## ✨ Highlights

|   |   |
|---|---|
| 💓 **Any standard strap** | GEOID HS500, Polar, Coospo, Wahoo and others with the Bluetooth Heart Rate service. Connects and reconnects on its own. |
| 🧩 **HRtoVRChat parameters** | `HR`, the three digits, `floatHR` and the connected, active and beat flags. |
| 💬 **Chatbox** | Your own text with `{bpm}`, `{zone}` and `{bat}`. Off until you turn it on. |
| 📈 **Live view** | Large BPM, heart-rate zone, a two-minute history, strap battery and OSC status. |
| 🎥 **OBS overlay** | A browser source with a transparent background, three layouts and your own colors. No window capture. |
| 📡 **Open channel** | Your pulse over HTTP, WebSocket and SSE. Bots, games, lights, your own overlay: anything can read it. |
| 🪟 **Compact mode** | A 300×150 window for VR overlays and window capture. |
| 🪶 **Light** | Redraws only when new data arrives, so it stays out of VRChat's way. |

## 🚀 Quick start

### 1. Install

Download the latest release from
[**Releases**](https://github.com/setsumi-vrc/heartbridge/releases/latest), then:

<details open>
<summary><b>Windows</b></summary>

1. Download `HeartBridge-vX.Y.Z-windows-x64.zip`, extract it anywhere and run `heartbridge.exe`.
2. Settings are saved next to the executable, so the folder can live on a USB stick.

</details>

<details open>
<summary><b>Linux</b></summary>

1. Download `HeartBridge-vX.Y.Z-linux-x86_64.tar.gz` and extract it:

   ```sh
   tar xzf HeartBridge-*-linux-x86_64.tar.gz
   ./HeartBridge-*/heartbridge
   ```

- Works on X11 and Wayland, on distributions with glibc 2.39 or newer (Ubuntu 24.04+,
  Debian 13+, Fedora 40+, Arch and the distributions based on them).
- Bluetooth goes through BlueZ: the `bluetooth` service has to be running.
- The tray icon needs a panel with StatusNotifierItem support. KDE has it built in; on GNOME
  install the AppIndicator extension. Without a tray, closing the window minimizes it.

</details>

<details open>
<summary><b>macOS</b></summary>

1. Download `HeartBridge-vX.Y.Z-macos-arm64.tar.gz` (Apple Silicon) and extract it.
2. The build is not notarized, so clear the download flag once and start it from Terminal:

   ```sh
   xattr -cr HeartBridge-*
   ./HeartBridge-*/heartbridge
   ```

3. Allow Bluetooth access when macOS asks (the request comes from Terminal).

On an Intel Mac, [build from source](#-building-from-source).

</details>

### 2. Put the strap on

> [!IMPORTANT]
> A strap accepts **one connection at a time**. If it's paired with a phone app (Polar Flow,
> Zwift, a fitness app) or a watch, close that app or turn off its Bluetooth first, or
> HeartBridge won't find it.

Wet the electrodes and put the strap on. Most straps sleep until they feel skin contact. Within a
few seconds the status card shows **Connected** and the number starts moving.

### 3. Connect to VRChat

1. **Enable OSC in VRChat**: Action menu → Options → OSC → **Enabled**. On Linux this works the
   same with VRChat running through Steam's Proton.
2. **Turn on what you want** in *Send to VRChat*: avatar parameters are on by default, the
   chatbox is off.
3. **Load an avatar** that reads the HRtoVRChat parameters, or just watch the chatbox.

That's it. Closing the window keeps HeartBridge sending from the tray; **Quit** is in the tray
menu.

## 🧰 Features

<details open>
<summary><b>Avatar parameters</b></summary>

| Parameter | Type | Value |
|---|---|---|
| `HR` | int | Beats per minute |
| `onesHR`, `tensHR`, `hundredsHR` | int | Each digit of the BPM |
| `floatHR` | float | BPM scaled to 0–1 (0–255 bpm) |
| `isHRConnected` | bool | The strap is connected |
| `isHRActive` | bool | Readings are coming in |
| `isHRBeat` | bool | Turns on briefly with each beat |

- Sent whenever the strap reports, and refreshed every 5 seconds so a newly loaded avatar picks
  them up.
- `isHRBeat` follows the measured rate, so a beating-heart animation stays in time.

</details>

<details open>
<summary><b>Chatbox</b></summary>

- A template where `{bpm}`, `{zone}` (Rest, Light, Moderate, Hard, Max) and `{bat}` (strap
  battery) are filled in, for example `❤ {bpm} bpm`. A live preview shows the result.
- Sent every 2 seconds, which is as fast as VRChat allows. Turning it off clears the chatbox.

</details>

<details open>
<summary><b>Interface</b></summary>

- A two-column window with no scrolling: the pulse and connection status on the left, the
  VRChat switches and settings on the right.
- Heart-rate zones based on your max heart rate (190 by default, set it in Settings).
- **Compact mode** from the title bar: heart, number and status in 300×150, draggable from
  anywhere, with an always-on-top option.
- **Tray icon** that shows the live BPM. Left click shows or hides the window; right click opens
  the parameter and chatbox switches, Open window and Quit.
- **Open channel** card: turn it on or off, choose the address and port, copy the OBS link and
  see how many apps are connected.
- **Debug window** (the bug icon) with the raw packets from the strap, the parsed values, what's
  going out over OSC and an event log you can copy.

</details>

## 📡 Open channel

HeartBridge runs a small web server with your live pulse, on `http://127.0.0.1:7171` by default.
The OBS overlay is built on it, and anything else can use it the same way.

### OBS overlay

1. Press **Open page** in the *Open channel* card. Your browser opens the overlay builder.
2. Pick a layout, colors and size, and press **Copy link**.
3. In OBS: **Sources → + → Browser**, paste the link into **URL**, width 600, height 300.

The background is transparent and the heart beats with your pulse. The options live in the link,
so you can also write them by hand:

| Option | Values | Default |
|---|---|---|
| `layout` | `row`, `stack` (number under the heart), `inside` (number inside the heart) | `row` |
| `color`, `text` | Hex color without `#`, for the heart and the text | `ff3b47`, `ffffff` |
| `size` | Number height in pixels, 12–400 | `64` |
| `zone` | `1` shows the heart-rate zone under the number | `0` |
| `zonecolor` | `1` colors the heart by zone | `0` |
| `unit` | `0` hides "bpm" | `1` |
| `beat` | `0` stops the heartbeat animation | `1` |
| `shadow` | `0` removes the shadow behind the text | `1` |
| `hide` | `1` hides the overlay while there's no reading | `0` |

For example, `http://127.0.0.1:7171/overlay?layout=inside&zonecolor=1&size=48`.

### Your own apps

| Address | What it gives you |
|---|---|
| `ws://127.0.0.1:7171/ws` | Live stream over WebSocket |
| `http://127.0.0.1:7171/events` | The same stream as Server-Sent Events |
| `http://127.0.0.1:7171/api/state` | The current state, once |

The stream sends a `state` message whenever something changes (and every 2 seconds anyway),
and a `beat` message on each heartbeat:

```json
{"type":"state","bpm":82,"zone":"Moderate","max_hr":190,"status":"reading","connected":true,
 "device":"HS500","contact":true,"battery":90,"rr_ms":[731],"time":1795029065430}
{"type":"beat","bpm":82,"time":1795029066160}
```

- `bpm` and `zone` are `null` while there's no reading.
- `status` is `reading`, `no_contact`, `connecting`, `searching` or `no_bluetooth`.
- `rr_ms` holds the beat-to-beat intervals from the last packet, when the strap sends them.
- `time` is Unix time in milliseconds.

Ten lines in a browser or Node:

```js
const ws = new WebSocket("ws://127.0.0.1:7171/ws");
ws.onmessage = (e) => {
  const msg = JSON.parse(e.data);
  if (msg.type === "state") console.log("BPM", msg.bpm, msg.zone);
  if (msg.type === "beat") console.log("beat");
};
```

Or from a terminal: `curl -N http://127.0.0.1:7171/events`.

Every address accepts requests from any origin, so a web page hosted anywhere can read it.

### Address and port

`127.0.0.1` means only this PC can connect. Set the address to `0.0.0.0` to let other devices on
your network in, or to a single interface's IP. Past that, HeartBridge stays out of the way:
put it behind Tailscale, a tunnel, a reverse proxy or a relay of your own, and send your pulse
wherever you like.

## 🩺 Troubleshooting

<details>
<summary><b>Stuck on "Looking for the strap…"</b></summary>

- Wet the electrodes again and check that the strap sits snug against your skin.
- Make sure no phone, watch or other app is connected to it.
- Check that Bluetooth is on. On Linux, check that the `bluetooth` service is running.
- If your strap doesn't advertise the heart-rate service, type part of its name in
  **Settings → Strap name**.
</details>

<details>
<summary><b>Connected, but no number</b></summary>

The strap is awake but has no skin contact. Tighten it and wet the electrodes; the status card
says <i>no skin contact</i> until readings come in.
</details>

<details>
<summary><b>The avatar or chatbox doesn't react</b></summary>

- Check that OSC is enabled in VRChat (Action menu → Options → OSC).
- Check that the port in **Settings** matches VRChat's (9000 by default). The status card
  shows *Sending to 127.0.0.1:9000* when data is going out.
- If you just added the parameters to an avatar, reset its OSC config in VRChat (Action menu →
  Options → OSC → Reset Config). VRChat caches it per avatar.
</details>

<details>
<summary><b>The overlay shows "--" or nothing</b></summary>

- Check that the *Open channel* card says it's on and not *Can't open the port*. If another app
  uses port 7171, pick a different port and copy the link again.
- `--` means HeartBridge is running but has no reading: look at the strap.
- In OBS, press **Refresh cache of current page** in the browser source's properties after
  changing the link.
</details>

<details>
<summary><b>Reporting a problem</b></summary>

Open the debug window (bug icon in the title bar), press **Copy log** and paste it into an
[issue](https://github.com/setsumi-vrc/heartbridge/issues).
</details>

## 📁 Your data

Settings live in `heartbridge.json`:

| System | Location |
|---|---|
| Windows | Next to `heartbridge.exe` |
| Linux | `~/.config/heartbridge/` |
| macOS | `~/Library/Application Support/HeartBridge/` |

Deleting the file resets everything. HeartBridge sends your heart rate to the OSC address you set
and serves it on the open channel's address, both on your own PC by default. It doesn't send
anything anywhere else on its own.

## 🔧 Building from source

Requires **Rust 1.95** or newer.

```sh
git clone https://github.com/setsumi-vrc/heartbridge
cd heartbridge
cargo build --release
```

The binary is `target/release/heartbridge` (`heartbridge.exe` on Windows). No system packages
are needed on any platform: on Linux, libdbus for BlueZ is compiled from source, and on Windows
the icon is generated and embedded at build time.

<details>
<summary><b>Development</b></summary>

```sh
cargo test
cargo clippy --all-targets
HEARTBRIDGE_DEMO=1 cargo run --release          # simulated pulse, no strap needed
HEARTBRIDGE_OPEN_DEBUG=1 cargo run --release    # open the debug window at startup
```

Pushing a `v*` tag builds Windows, Linux and macOS and publishes a release with the three
archives.
</details>

## 📜 License

[MIT](LICENSE)
