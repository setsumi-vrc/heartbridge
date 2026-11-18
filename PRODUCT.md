# Product

<!-- impeccable:product-schema 1 -->

## Platform

windows

## Stack
Native Rust app for Windows: `eframe`/`egui` (glow renderer), `btleplug` for BLE, OSC over UDP. No Tauri, no webview, no web server. Optimized: redraws only when data arrives or an animation is running.

## Users
- A VRChat player and avatar creator who wears the GEOID HS500 chest strap while playing.
- Sets it up on the desktop before putting on the headset, then watches it inside VR through a window overlay (XSOverlay, OVR Toolkit, Desktop+) and shows it on stream through an OBS window capture.
- Job: get the live heart rate into VRChat (chatbox and avatar parameters) and know in one glance that it is connected and sending.

## Product Purpose
Bridges a BLE heart-rate strap to VRChat over OSC. Success: from launch, with the strap on, connected and sending with no configuration; the chatbox toggles with one click; a lost connection is visible right away.

## Positioning
A single native executable, light enough to leave running alongside VRChat, that understands the standard Bluetooth Heart Rate service and the community HRtoVRChat parameter names.

## Operating Context
- Runs next to VRChat (high GPU load): the app must not compete for frames.
- Read from a distance and at low resolution in a VR overlay, and captured by OBS: the number and state have to read at small sizes.
- The strap accepts one BLE connection; a phone holding it blocks the PC.
- VRChat: OSC on port 9000, chatbox throttled (one message every ~2 s), float parameters synced at 8 bits.

## Capabilities and Constraints
- Reads BPM, skin contact, RR intervals and battery (services 0x180D / 0x2A37, 0x180F / 0x2A19).
- Sends `/chatbox/input` with an editable template, plus avatar parameters: HR, onesHR, tensHR, hundredsHR, floatHR, isHRConnected, isHRActive, isHRBeat.
- Settings persist in `heartbridge.json` next to the executable.
- UI in English.
- Borderless 800×700 window with its own title bar (drag, debug, compact, minimize, close), two columns, no scrolling.
- A debug window shows the raw strap and OSC state and an event log that can be copied.
- Tray icon that shows the live BPM; left click shows or hides the window, right click opens a custom popup (pulse, the two VRChat switches, open, quit). The X (and Alt+F4) always hides the window to the tray so sending to VRChat continues; only Quit in the popup exits.

## Brand Commitments
- Visual: the category standard for heart-rate apps (dark ground, rounded cards, red heart), executed with full finish. No external product reference.

## Evidence on Hand
- Verified on hardware: the strap advertises as "24660-1", reports battery 100%, and sends 0 bpm with no contact when not worn.
- No real BPM readings captured while worn yet.

## Product Principles
- Glanceable first: the number and the state come before any control.
- Opt-in for anything public: the chatbox starts off.
- Never get in VRChat's way: minimal CPU/GPU, no continuous redraw.
- Recovers on its own: reconnects without user action.
