# Changelog

All notable changes to HeartBridge are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-11-18

First release.

### Added

- Connects to any Bluetooth LE strap with the standard Heart Rate service (GEOID HS500,
  Polar, Coospo, Wahoo and similar), and reconnects on its own when the connection drops.
- Avatar parameters with the HRtoVRChat_OSC names: `HR`, `onesHR`, `tensHR`, `hundredsHR`,
  `floatHR`, `isHRConnected`, `isHRActive` and `isHRBeat`.
- Chatbox message from a template with `{bpm}`, `{zone}` and `{bat}`, sent every 2 seconds.
  Off by default.
- Main window with the live BPM, heart-rate zone, a two-minute history, strap and battery
  status, and whether VRChat is receiving.
- Compact 300×150 mode for OBS window capture and VR overlays.
- Tray icon that shows the live BPM. Right click opens a menu with the parameter and
  chatbox switches. Closing the window keeps it running in the tray.
- Debug window with the raw strap packets, the OSC state and a copyable event log.
- Windows, Linux (X11 and Wayland) and macOS builds.

[0.1.0]: https://github.com/setsumi-vrc/heartbridge/releases/tag/v0.1.0
