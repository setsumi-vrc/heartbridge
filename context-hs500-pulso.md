# HS500 Pulso → VRChat

## Users
- A VRChat player wearing the GEOID HS500 chest strap during a session, often with the headset on. The app is set up before entering VR; mid-session the window is glanced at on the desktop or through an overlay.
- Goals: show the live heart rate in the chatbox, drive avatar parameters (animated heart, number display), and know at a glance that the strap is connected and sending.
- Constraints: the headset hides the desktop, so status has to be legible from far away and in a single glance. Setup happens once and then the app has to run untouched.

## Product
- A Rust app for Windows. It connects to the strap over Bluetooth LE using the standard Heart Rate service (0x180D / 0x2A37) and sends OSC over UDP to VRChat at 127.0.0.1:9000.
- Frontend: a native eframe/egui window that redraws only on new data. It is captured as a window by OBS and VR overlays.
- Settings persist in `hs500-pulso.json` next to the executable.

## Constraints
- VRChat throttles the chatbox, so messages go out at most once every 2 s.
- VRChat syncs float parameters at 8-bit precision, so normalized values lose resolution.
- The strap accepts one BLE connection at a time, and a phone holding it blocks the PC.
- OSC has to be enabled in VRChat: Action Menu → Options → OSC → Enabled.

## Ethical stance
- Heart rate is health data. The chatbox shows it to everyone in the instance, so it starts off and is turned on with one explicit click.
- Avatar parameters start on. They are visible to others only if the avatar uses them.
- OSC goes only to the configured host (127.0.0.1 by default). There is no telemetry and no history is stored on disk.

## Success
- From a fresh launch with the strap on: connected and sending in under 30 s with no configuration.
- Turning the chatbox on or off is a single click and takes effect on the next send.
- Disconnecting the strap shows up in the UI and in `isHRConnected` within 5 s.

## Open questions
- None.
