---
version: 1
slug: "src-app-rs"
primary_target: "src/app.rs"
related_targets: []
---

## Scope
The HeartBridge main window (native egui) with its merged title bar, its compact mode, and the tray icon with its popup. Mode: Operate. UI in English.

## Task
- Read the live pulse at a glance (desktop, VR overlay, OBS).
- Know whether the strap is connected and whether data is reaching VRChat.
- Turn avatar parameters and chatbox on or off; edit the chatbox template.
- Adjust OSC host/port, max HR, name filter, always on top, close to tray, beat animation.
- From the tray: see the pulse, toggle the two VRChat sends, open the window, quit.

## Direction contract
THESIS: The category standard, executed at its best: a dark heart-rate app with rounded cards and a red heart. It refuses the generic grey panel with stock controls; every control is drawn in the app's palette.
OWN-WORLD: Ground #0E0F12, cards #1A1C21 at 16 px radius with a 1 px #24272E hairline, text #F5F6F8, secondary #9AA0A8, single accent #FF3B47 (heart, toggles, history line), connected #34C77B, warning #F5A524. Zone colours only on the zone dot. Numbers in Bahnschrift 700, UI in Segoe UI Variable.
STORY: The player looks and in one second knows their pulse, that the strap is connected, and that VRChat is receiving; they turn on the chatbox with one click and edit the text with a live preview.
FIRST VIEWPORT: Top card: a drawn red heart (52 px) that beats with the pulse, the BPM at 88 px with "bpm" beside it, zone pill on the right, two-minute history (fixed 40–200 scale) with max/min underneath. Above everything, a 44 px title bar on the ground colour: small heart and "HeartBridge", then compact, minimize and close glyphs (46 px each, close hover #C42B1C). Two columns at 800×700 with no scrolling: on the left the pulse card and below it the status card (Strap, Battery, VRChat with coloured dots); on the right the "Send to VRChat" card (two switches, template field, preview) and the "Settings" card. Title-bar glyphs from the right: close (to tray), minimize, compact, debug. Tray: a 32 px red square with the BPM in pixel digits (grey heart when idle); right click opens a 240 px card-coloured popup with heart, BPM, status line, the two switches, "Open window" and "Quit HeartBridge", shown at once with no fade.
FORM: canon (the user's choice in the direction round); seed: d6de5dbad9cd6660
FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
