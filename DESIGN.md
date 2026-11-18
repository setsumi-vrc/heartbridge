---
name: HS500 Pulse
description: A native Windows heart-rate bridge to VRChat; a dark heart-rate app with rounded cards and a red heart, read at a glance on the desktop, in a VR overlay and on stream.
colors:
  accent: "#FF3B47"
  focus: "#FF8A92"
  ground: "#0E0F12"
  card: "#1A1C21"
  pill: "#262930"
  hairline: "#262930"
  grid: "#23262C"
  field: "#121317"
  switch-off: "#3A3D44"
  hover-fill: "#2E3139"
  pressed-fill: "#363942"
  text: "#F5F6F8"
  text-secondary: "#9AA0A8"
  text-dim: "#5C6169"
  ok: "#34C77B"
  warn: "#F5A524"
  error: "#FF6B6B"
  close-hover: "#C42B1C"
  zone-rest: "#5B9CF5"
  zone-light: "#34C77B"
  zone-moderate: "#F5D024"
  zone-hard: "#F58A24"
  zone-max: "#FF3B47"
typography:
  numeral-display:
    fontFamily: "Bahnschrift, Segoe UI Variable, Segoe UI Symbol"
    fontSize: "88px"
    fontWeight: 700
    lineHeight: 1
  numeral-compact:
    fontFamily: "Bahnschrift, Segoe UI Variable, Segoe UI Symbol"
    fontSize: "76px"
    fontWeight: 700
    lineHeight: 1
  numeral-popup:
    fontFamily: "Bahnschrift, Segoe UI Variable, Segoe UI Symbol"
    fontSize: "52px"
    fontWeight: 700
    lineHeight: 1
  title:
    fontFamily: "Segoe UI Variable, Segoe UI Symbol"
    fontSize: "17px"
    fontWeight: 600
  title-small:
    fontFamily: "Segoe UI Variable, Segoe UI Symbol"
    fontSize: "15px"
    fontWeight: 600
  app-name:
    fontFamily: "Segoe UI Variable, Segoe UI Symbol"
    fontSize: "14px"
    fontWeight: 600
  status-compact:
    fontFamily: "Segoe UI Variable, Segoe UI Symbol"
    fontSize: "16px"
    fontWeight: 400
  body:
    fontFamily: "Segoe UI Variable, Segoe UI Symbol"
    fontSize: "15px"
    fontWeight: 400
  body-small:
    fontFamily: "Segoe UI Variable, Segoe UI Symbol"
    fontSize: "14px"
    fontWeight: 400
  label:
    fontFamily: "Segoe UI Variable, Segoe UI Symbol"
    fontSize: "13px"
    fontWeight: 400
  caption:
    fontFamily: "Segoe UI Variable, Segoe UI Symbol"
    fontSize: "12.5px"
    fontWeight: 400
  helper:
    fontFamily: "Segoe UI Variable, Segoe UI Symbol"
    fontSize: "12px"
    fontWeight: 400
rounded:
  none: "0px"
  glyph: "2px"
  menu: "8px"
  widget: "10px"
  capsule: "13px"
  card: "16px"
spacing:
  hairline: "1px"
  xs: "4px"
  sm: "8px"
  md: "12px"
  window: "16px"
  card: "18px"
components:
  card:
    backgroundColor: "{colors.card}"
    rounded: "{rounded.card}"
    padding: "18px"
  title-bar:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.text-secondary}"
    typography: "{typography.app-name}"
    height: "44px"
  caption-button:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.text-secondary}"
    rounded: "{rounded.none}"
    width: "46px"
    height: "44px"
  caption-button-hover:
    backgroundColor: "{colors.pill}"
    textColor: "{colors.text}"
  caption-button-close-hover:
    backgroundColor: "{colors.close-hover}"
    textColor: "{colors.text}"
  switch-on:
    backgroundColor: "{colors.accent}"
    rounded: "{rounded.capsule}"
    width: "44px"
    height: "26px"
  switch-off:
    backgroundColor: "{colors.switch-off}"
    rounded: "{rounded.capsule}"
    width: "44px"
    height: "26px"
  text-field:
    backgroundColor: "{colors.field}"
    textColor: "{colors.text}"
    typography: "{typography.body}"
    rounded: "{rounded.widget}"
    padding: "9px 12px"
  zone-pill:
    backgroundColor: "{colors.pill}"
    textColor: "{colors.text}"
    typography: "{typography.label}"
    rounded: "{rounded.capsule}"
    height: "26px"
  menu-item:
    textColor: "{colors.text}"
    typography: "{typography.body-small}"
    rounded: "{rounded.menu}"
    height: "34px"
  menu-item-hover:
    backgroundColor: "{colors.hover-fill}"
  button:
    backgroundColor: "{colors.pill}"
    textColor: "{colors.text}"
    rounded: "{rounded.widget}"
    padding: "7px 12px"
    height: "30px"
  button-hover:
    backgroundColor: "{colors.hover-fill}"
  button-active:
    backgroundColor: "{colors.pressed-fill}"
  tray-popup:
    backgroundColor: "{colors.card}"
    rounded: "{rounded.none}"
    padding: "16px"
    width: "300px"
    height: "330px"
---

# Design System: HS500 Pulse

## Overview

**Creative North Star: "The Bedside Monitor, Finished"**

HS500 Pulse is the category standard for heart-rate apps executed with full finish: a near-black ground, rounded charcoal cards, one red heart, and a number large enough to read from across a dim room, through a VR overlay or in an OBS capture. Everything is native egui, painted in the app's own palette; no control is left in egui's stock grey. The window has no system frame; its own title bar sits on the ground colour so the whole window reads as one surface.

The hierarchy is fixed by the job: the pulse first (drawn heart, Bahnschrift numeral, zone pill, two-minute trace), then whether the strap is connected and VRChat is receiving (status rows with coloured dots), then the two send switches, then the settings, all visible at once in two columns with no scrolling. Density is calm; cards are separated by tone and a hairline, never by shadow. Motion is spent only on the heartbeat and on state changes, and the app redraws only when it has to, because it runs beside VRChat under heavy GPU load.

The same world collapses into three expressions: the full window (800 x 700), the compact window (300 x 150) for overlays and stream, and the tray popup (240 wide, height fitted to its content, about 248), plus a 32 px tray icon that carries the live BPM in pixel digits.

**Key Characteristics:**
- Always dark: ground, then card, then pill, in three tonal steps.
- One red, used for the heart, the switches when on, the history line and the tray icon.
- Numerals in Bahnschrift 700; every other word in Segoe UI Variable, all loaded from C:\Windows\Fonts.
- Every glyph is painted in code: heart, caption buttons (including the debug bug), dots, switches, tray digits. No icon font, no image files.
- Frugal rendering: 30 fps only while a beat animates, otherwise once a second.

## Colors

A near-black, slightly cool neutral ramp carrying a single saturated red, with green, amber and soft red reserved for state.

### Primary
- **Pulse Red** (accent): the heart fill, a switch when on, the history line (with a 14% fade fill beneath it), the text cursor, hyperlinks, text selection at 35%, and the tray icon square. The max zone dot reuses it.
- **Soft Pulse** (focus): keyboard focus rings (2 px) and the pressed widget stroke. It is the only tint of the accent.

### Neutral
- **Night Ground** (ground): the window, the title bar and the compact window. Also the GL clear colour, so no frame ever flashes another tone.
- **Charcoal Card** (card): cards and the tray popup surface.
- **Slate Pill** (pill): zone pill, default widget fill, caption-button hover.
- **Hairline** (hairline): 1 px card borders, the 1 px window frame, popup separators. Same value as the pill by design of the build.
- **Chart Grid** (grid): the three history gridlines at 80, 120 and 160 bpm.
- **Well** (field): text-field interiors, one step below the ground.
- **Switch Off** (switch-off): an off switch track and the hovered widget stroke.
- **Hover / Pressed** (hover-fill, pressed-fill): egui widget fills on hover and press.
- **Paper White** (text): numerals, values, setting titles.
- **Fog** (text-secondary): labels, descriptions, units ("bpm"), placeholders, the app name, idle heart outline, idle tray heart.
- **Ash** (text-dim): status dots only, for "sending off" and "waiting".

### Secondary (state)
- **Connected Green** (ok), **Amber Warning** (warn), **Error Coral** (error): status dots and the compact/popup summary dot. Never fills, never text colour.
- **Caption Close Red** (close-hover): the Windows close-button hover, on the close caption only.

### Tertiary (zones)
- **Rest Blue, Light Green, Moderate Yellow, Hard Orange, Max Red** (zone-rest to zone-max): the 8 px dot inside the zone pill. Nowhere else.

### Named Rules
**The One Red Rule.** The accent marks the pulse and an "on" state; apart from the 35% text selection and the tray icon square, it is never a surface behind text, a border or a decoration.

**The Zone Dot Rule.** Zone colours appear only on the zone pill's dot; the pill itself stays slate and its text white.

**The Ash Is Not Text Rule.** text-dim is too faint for words on card (about 2.6:1); it colours dots only. Placeholders and hints use text-secondary.

## Typography

**Display Font:** Bahnschrift at weight 700 (falls back to Segoe UI Variable, then Segoe UI Symbol)
**Body Font:** Segoe UI Variable at 400, with a 600 instance for titles (Segoe UI Symbol as fallback for ❤ and other chatbox symbols)

**Character:** A DIN numeral that stays legible at overlay resolution, paired with the Windows system face so the chrome feels native while the number feels instrument-like. Both ship with Windows; nothing is bundled or downloaded.

### Hierarchy
- **Numeral** (700, 88 px full / 76 px compact / 52 px popup): the BPM only. "bpm" sits beside it in Fog on the same baseline, at 21% of the numeral size with a 15 px floor, 6 px gap. No reading shows an em dash in Fog.
- **Title** (600, 17 px): card titles ("Send to VRChat").
- **Title Small** (600, 15 px): section titles in the debug window. **App Name** (600, 14 px, Fog): the title bar.
- **Status Compact** (400, 16 px, Paper White): the compact-mode status line, sized for VR and stream.
- **Body** (400, 15 px): setting titles and text-field contents. **Body Small** (400, 14 px): status rows and popup menu items.
- **Label** (400, 13 px, Fog): field labels, hints, preview, zone pill text, popup status line.
- **Caption** (400, 12.5 px, Fog): setting descriptions, chart caption and max/min. **Helper** (400, 12 px, Fog): template help line.

### Named Rules
**The Instrument Numeral Rule.** Bahnschrift is for numbers that are read as measurements (the BPM). Labels, values like "87%" and addresses stay in Segoe UI Variable.

**The Shared Baseline Rule.** A unit set beside a numeral shares its baseline; it never centres against the numeral's box.

## Layout

Two equal columns, no scroll. The full window is 800 x 700: a 44 px title bar, then 16 px side and bottom margins (4 px top), 12 px between the columns and between cards. Left: pulse card, then status card; the history chart takes whatever height makes the left column end on the same line as the right. Right: "Send to VRChat", then "Settings", which stretches to the bottom. Inside a card, 18 px padding, 10 px vertical and 8 px horizontal item spacing; label-value rows push values to the right edge. Setting rows put title and description stacked on the left (2 px apart) and the switch right-aligned and centred; the whole row is the hit target.

Compact mode (300 x 150) drops the title bar and cards: 16 px sides, 14 px top, 12 px bottom, heart and numeral on one row, a status row (12 px dot, 16 px text) beneath. The whole window drags; close and expand captions (36 x 32) appear only on hover.

The tray popup (300 x 330) is a card-coloured borderless window, 16 px padding, 8 px item spacing: pulse row, status line, separator, two switches, separator, two full-width menu items.

Base control height is 30 px; buttons pad 12 x 7; text fields pad 12 x 9.

## Elevation & Depth

Flat. Depth is tonal: Night Ground, then Charcoal Card with a 1 px hairline, then Slate Pill for controls, with the Well recessed below the ground for text entry. There are no shadows anywhere. Because the window has no system frame, a 1 px hairline is drawn inside its edge so it does not dissolve into dark desktops, overlays or stream scenes.

### Named Rules
**The Tone Not Shadow Rule.** A surface is raised by one tonal step and a hairline, never by a drop shadow or glow.

## Shapes

Generous, soft corners inside a square window. Cards round at 16 px; widgets (fields, buttons) at 10 px; capsules (zone pill, switch track) are fully rounded at 13 px on a 26 px height; popup menu items at 8 px; focus rings follow their shape plus 3 px (16 px on a switch, 4 px inside a caption button). The window, the title bar captions and the tray popup are square, matching Windows chrome. The tray icon is a 32 px red square rounded at 7 px.

The heart is the signature form: a parametric curve (16 sin³t; 13 cos t - 5 cos 2t - 2 cos 3t - cos 4t), filled solid red when live and outlined 2 px in Fog when idle; the same equation, as an implicit field, draws the window and idle tray icons.

## Components

### Buttons
- **Caption buttons:** 46 x 44, square, painted glyphs (1.3 px strokes, 10 px marks) in Fog; hover fills Slate Pill and turns the glyph white; the close caption hovers Caption Close Red. Order from the right: close, minimize, compact.
- **Default button:** Slate Pill fill, hairline stroke, 10 px radius, 12 x 7 padding; hover #2E3139 with a Switch Off stroke; pressed #363942 with a Soft Pulse stroke.
- **Menu item (popup):** full width, 34 px tall, transparent at rest, 14 px Paper White, 8 px radius.

### Chips
- **Zone pill:** 26 px capsule in Slate Pill, an 8 px zone dot 14 px from the left, 13 px white zone name; top-right of the pulse row.

### Cards / Containers
- **Corner Style:** 16 px.
- **Background:** Charcoal Card.
- **Shadow Strategy:** none (see Elevation & Depth).
- **Border:** 1 px hairline.
- **Internal Padding:** 18 px.

### Inputs / Fields
- **Style:** Well interior, 10 px radius, 15 px text, 12 x 9 padding; placeholders in Fog.
- **Number fields:** the same text field, digits only, applying valid values as typed and clamping to range on blur. No spinners.
- **Focus:** Pulse Red 2 px text cursor; selection in Pulse Red at 35%.

### Switches
- 44 x 26 capsule; track blends Switch Off to Pulse Red as it turns on, a 20 px white knob slides with egui's responsive animation; hover brightens the track 12%; focus draws a 2 px Soft Pulse ring 3 px outside.

### Navigation
- **Title bar:** 44 px on Night Ground; an 18 px live heart at 28 px from the left, "HS500 Pulse" at 46 px; the bar drags the window.
- **Settings card:** 17 px semibold title; OSC host and port on one row; max heart rate and strap name side by side; then the always-on-top and heartbeat switches.
- **Tray popup:** 240 px wide, square, card colour with a hairline; 36 px numeral with a 24 px heart, a 12.5 px status line, two 30 px menu rows with 20 px switches, and two 30 px left-aligned menu items with a pill-colour hover. It is created hidden, sized to its content, and shown with the Windows open animation turned off.
- **Debug window:** native frame, 560 x 620, ground colour; a two-column grid of raw state in 12.5 px monospace and the event log in a field-coloured box with Copy log and Clear.

### Pulse Card (signature)
A 60 x 72 slot holding the 52 px heart, the 88 px numeral with its unit, the zone pill, then a 72 px tall two-minute history on a fixed 40 to 200 bpm scale: three grid lines, a 2 px red line over a 14% red fade, breaks on zero or gaps over 3 s; "Last 2 minutes" left and "max · min" right beneath. On each beat the heart scales up to 1.14 and eases back out over 260 ms.

### Status Rows
Fog label left; Paper White value right with an 8 px state dot before it. Battery shows a dot only at 15% or below. A Fog hint line follows when the strap needs action.

### Tray Icon
32 px: the live BPM in white 3 x 5 pixel digits (2 px cells on even coordinates, so it stays crisp at 16 px) on the Pulse Red rounded square; an idle Fog heart with no reading.

## Do's and Don'ts

### Do:
- **Do** paint every control in the palette: switches, captions, dots and numeric fields are drawn, not stock egui.
- **Do** keep the BPM in Bahnschrift 700 with "bpm" in Fog on the same baseline.
- **Do** separate surfaces by tone plus a 1 px hairline (card on ground, pill on card).
- **Do** use text-secondary (#9AA0A8) for placeholders, hints and descriptions.
- **Do** request a repaint at 33 ms only while a beat animates; otherwise once a second.
- **Do** load fonts from C:\Windows\Fonts and fall back to egui's built-ins if missing.

### Don't:
- **Don't** add shadows or glows to cards, popups or the window.
- **Don't** use zone colours anywhere but the zone pill's dot.
- **Don't** set text in text-dim (#5C6169); it is for dots.
- **Don't** use the accent as a surface behind text, except the tray icon's digit square.
- **Don't** introduce a light theme; the app is always dark.
- **Don't** use icon fonts or bitmap assets for UI glyphs.
