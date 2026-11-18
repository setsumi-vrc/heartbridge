## 2026-11-18 · The frontend is a native eframe/egui window
- Decided by: the user ("rust native sin tauri, optimizado")
- Why: a single native executable; VR overlays and OBS capture it as a window.
- Rejected: a local web page served by the binary, Tauri.
- Rests on: the intake answer from 2026-11-18.

## 2026-11-18 · Typefaces: Bahnschrift for the number, Segoe UI Variable for the UI, loaded from C:\Windows\Fonts
- Decided by: Claude, default
- Why: they ship with Windows 10/11, so nothing is downloaded or bundled; Bahnschrift (DIN) keeps figures legible at a distance and at overlay resolution.
- Rejected: embedding Google Fonts (adds weight and a download step).
- Rests on: Operate mode, the VR overlay and OBS use cases.

## 2026-11-18 · Chatbox off at startup, avatar parameters on
- Decided by: Claude, default
- Why: the chatbox publishes heart rate to everyone in the instance; parameters only matter when the avatar uses them.
- Rejected: everything on at startup.
- Rests on: Ethical defaults (health data, opt-in).

## 2026-11-18 · Avatar parameter names follow the community HRtoVRChat_OSC convention
- Decided by: the user (intake answer "Estándar HRtoVRChat")
- Why: existing avatar prefabs already read HR, onesHR/tensHR/hundredsHR, floatHR, isHRConnected, isHRActive and isHRBeat, so they work with no avatar changes.
- Rejected: our own names (would force rebuilding animators).
- Rests on: the user being a VRChat creator who has avatars with existing heart-rate prefabs.

## 2026-11-18 · Visual: the category standard (dark heart-rate app, rounded cards, red heart)
- Decided by: the user (decision round, standard option)
- Why: chosen over the zone-coloured pharmaceutical band, the race bib and airport signage; the user named no reference products.
- Rejected: Swiss pharmaceutical packaging (assigned by the roll), race bib, airport signage.
- Rests on: the direction round of 2026-11-18.

## 2026-11-18 · Zones as a percentage of max HR 190: <60% rest, 60-70 light, 70-80 moderate, 80-90 hard, 90+ max
- Decided by: Claude, default
- Why: the standard five-zone scheme; max HR can be edited under "More settings".
- Rejected: fixed BPM thresholds (don't adapt to the person).
- Rests on: common practice in fitness apps.

## 2026-11-18 · UI in English
- Decided by: the user ("change language to english")
- Why: the user asked for it; replaces the Spanish (Argentina) UI.
- Rejected: Spanish UI, a language switch.
- Rests on: the user's request of 2026-11-18.

## 2026-11-18 · Own title bar merged into the window, no system frame
- Decided by: the user ("merge title bar")
- Why: the title bar shares the ground colour, so the window reads as one surface in overlays and OBS; buttons are 46 px wide like the Windows caption buttons, close turns #C42B1C on hover.
- Rejected: the native Windows frame.
- Rests on: the user's request of 2026-11-18.

## 2026-11-18 · Tray icon with a custom popup instead of a native menu
- Decided by: the user ("add tray icon with custom layout")
- Why: the popup carries the pulse and the two VRChat switches, so the window is not needed while playing; the icon itself draws the BPM in pixel digits on a red square.
- Rejected: a native context menu (text only, no pulse).
- Rests on: the user's request of 2026-11-18.

## 2026-11-18 · Closing the window always hides it to the tray
- Decided by: the user ("when i click X should go to tray not close the app")
- Why: closing during play would otherwise stop sending to VRChat; "Quit HS500 Pulse" in the popup is the only way out, so the "Close to tray" setting was removed.
- Rejected: close exits the app.
- Rests on: the principle that the app never gets in VRChat's way.

## 2026-11-18 · Tray popup closes when it loses focus, with a 600 ms grace at opening
- Decided by: Claude, default
- Why: it behaves like a native menu (click outside closes it); at opening Windows can hand focus to the main window, so the popup takes focus back instead of closing.
- Rejected: a close button only.
- Rests on: the focus trace captured on 2026-11-18.

## 2026-11-18 · Finish review: fix first, six fixes applied
- Decided by: Claude, default
- Why: the review found the window ground rendered #010101 instead of #0E0F12 (colour converted twice), compact-mode status too faint for VR and OBS, placeholders at 3.3:1, heart and number below the contract sizes, stock number spinners, and a settings header that did not look clickable; all six were fixed and the ground re-measured at #0E0F12.
- Rejected: shipping with the stock spinners and the dim compact status.
- Rests on: .impeccable/review/desktop.png, desktop-collapsed.png, compact.png and popup.png captured after the fixes.

## 2026-11-18 · Two columns at 800×700, no scrolling
- Decided by: the user ("i dont want to scroll down use two columns")
- Why: everything fits at once; pulse and status on the left, VRChat and settings on the right, and the history chart stretches so both columns end on the same line.
- Rejected: one scrolling column; a collapsible settings section.
- Rests on: the user's request of 2026-11-18.

## 2026-11-18 · Tray popup 240 px wide, shown at once with no fade
- Decided by: the user ("the tray menu is too big", "the transition to appear is too slow")
- Why: the popup is created hidden, sized to its content and shown after its first frame with the DWM open animation turned off; measured ready in about 40 ms.
- Rejected: the 300×330 popup with the default Windows fade-in.
- Rests on: the user's request and the "Tray popup ready in N ms" line in the debug log.

## 2026-11-18 · Debug window opened from a bug glyph in the title bar
- Decided by: the user asked for a debug button; the placement and contents are Claude's default
- Why: the raw packet bytes, parsed fields, OSC state and an event log make a strap or VRChat problem diagnosable without a console.
- Rejected: a text button inside a card (competes with the pulse).
- Rests on: the user's request of 2026-11-18.
