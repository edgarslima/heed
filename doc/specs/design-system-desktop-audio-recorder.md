# Design System — Desktop Audio Recorder

**Purpose:** Visual, interaction, and accessibility specification for the desktop window. **Audience:** Frontend engineers, designers, QA, and AI agents. **Status:** Proposed, revision 1.0. Last reviewed: 2026-11-13.

| Field | Definition |
| --- | --- |
| Review | 1.0 · 11/13/2026 |
| Surface | Desktop window of the Tauri 2 application |
| Initial language | American English |
| References | iPhone recorder for hierarchy and recording controls. OBS for source reading and peak retention. |

[Build specification](build-specification-desktop-audio-recorder.md) · [Software architecture](software-architecture-desktop-audio-recorder.md) · [Construction plan](../plan/desktop-audio-recorder-construction-plan.md) · [iPhone recorder](https://support.apple.com/guide/iphone/make-a-recording-iph4d2a39a3b/ios) · [OBS Audio Mixer](https://obsproject.com/kb/audio-mixer-guide)

## 1. Visual direction and layout

Recording is the center of the experience. Time is the most prominent information during the session, followed by the waveform and stop button. Source and folder configuration remain in the same window. Use the system title bar and window controls.

| Region | Composition |
| --- | --- |
| Header | Title “Heed” (the product name) in the native bar. Preference “Generate transcription copy” in the configuration area. |
| Configuration | **Source** field, device or app selector when applicable, **Recording folder** field, and **Change folder** action. |
| Preview | Duration, horizontal waveform, and meters with source names. In dual mode, show stacked meters. |
| Control | Circular central **Record** or **Stop** button, visible label, and secondary **Test level** action. |
| Messages | Region below the meters or button with current state, context-aware failures, and result actions. |

- Initial window size of **560 × 600 logical units** and minimum size of **420 × 480**. Content width max of **600 CSS px**, centered. Validate content and decoration dimensions in Tauri on each OS.
- Lateral padding of 24 px in the initial window and 16 px up to 460 px width. Configure in a single column. When width allows it, the device name and selector can share a row. Truncate long paths in the middle and provide the full path with focus, accessible description, and the native selector.
- Use a 112 px waveform height at the initial size and 72 px at minimum. If enlarged text or reduced height exceeds the visible area, allow vertical scrolling and keep **Stop** accessible. Long paths and messages wrap when needed.
- Initial theme follows the system via `prefers-color-scheme`. Use local typography and bundled assets. Essential content works without a network connection.

## 2. Implementation tokens

### Color

| Token | Light | Dark | Application |
| --- | --- | --- | --- |
| `--bg` | `#F7F7F8` | `#141416` | General background |
| `--surface` | `#FFFFFF` | `#202024` | Fields and preview area |
| `--text` | `#18181B` | `#F5F5F7` | Main text and timer |
| `--muted` | `#52525B` | `#BABAC2` | Secondary text and technical labels |
| `--border` | `#787883` | `#787883` | Functional field outlines |
| `--track` | `#DDDDE2` | `#3D3D45` | Level bar background |
| `--record` | `#C52836` | `#D13244` | Primary control, active waveform, and recording state |
| `--on-record` | `#FFFFFF` | `#FFFFFF` | Icon or text over the red button |
| `--focus` | `#1757BA` | `#8DB9FF` | Focus outline |
| `--level-normal` | `#13764B` | `#5BD596` | Below the yellow threshold |
| `--level-warn` | `#8A5A00` | `#FFD166` | Near saturation |
| `--level-clip` | `#B42331` | `#FF8290` | Critical peak and saturation |
| `--message-danger` | `#A1222E` | `#FFADB5` | Error text |

The base text, border, focus, button, and meter pairs were validated with the specified hex values. Also verify hover, disabled, and forced-colors states in the implemented UI. Target normal text contrast of at least 4.5:1 and functional indicator contrast of at least 3:1. Markers and labels follow the state colors. [W3C — WCAG 2.2](https://www.w3.org/TR/WCAG22/)

### Typography, spacing, and shape

| Token | Value | Use |
| --- | --- | --- |
| `--font-ui` | `system-ui, -apple-system, "Segoe UI", sans-serif` | System typography |
| `--type-time` | 40/48 px, weight 600, tabular numbers | Elapsed time. 32/40 px minimum |
| `--type-title` | 20/26 px, weight 600 | Internal titles when needed |
| `--type-body` | 14/20 px, weight 400 | Controls, messages, and values |
| `--type-label` | 13/18 px, weight 600 | Source name and fields |
| `--type-small` | 12/16 px, weight 400 | dBFS, state detail, and formats |
| `--space-*` | 4, 8, 12, 16, 24, 32 px | Layout rhythm and spacing |
| `--radius-*` | 8, 12, 16 px and circle | Fields, panels, and button |
| `--target-control` | 36 px minimum height. 40 px preferred | Fields, actions, and selections |
| `--target-record` | 72 px, 24 px icon | Primary circular button |
| `--focus-ring` | 2 px with 2 px offset | Keyboard-focused controls |
| `--motion-fast` | 120 ms | Control state transitions. With reduced motion, switch to no animation. |

Implement tokens in a CSS file. Semantic state colors are represented through custom properties. The waveform and meters read from the same interface tokens.

## 3. Components

| Component | Variants and implementation rules |
| --- | --- |
| **Source** | Labeled selector with Microphone, System, Application, and dual modes. Use semantic controls. If a mode is unavailable, explain the condition next to the field and keep the selection recoverable. |
| **Device or app** | Label, current selection, and list. Keep “System default” as a preference. During recording, show the active source and disable switching. |
| **Folder** | Label, path, and **Change folder** button. Initial state “No folder selected”. If invalid, keep the path visible and attach the message to the field. |
| **Recording button** | Red circular button with circle in **Record** and square in **Stop**. Visible text label and accessible name. In **Starting**, accept **Stop** to cancel the start. In **Finalizing** and **Exporting**, use progress state and message. |
| **Timer** | `MM:SS` up to one hour and `HH:MM:SS` beyond that. Use the session time supplied by Rust plus a local monotonic projection for display. Avoid announcing every second to screen readers. |
| **Waveform** | Canvas with min/max envelope from the engine. In double sessions, show two stacked bands on the same time axis with labels. Horizontal axis is roughly 8 seconds of recent data. At rest, show a static centerline. Canvas remains decorative for assistive technology, with equivalent state in text. |
| **Meter** | One bar per source. Peak, RMS energy, and hold marker. Label updates without continuous announcements. Display value or condition in text. |
| **Session state** | Short messages below the main area. Visual priority for capture failure, then export, saved state, and guidance. Failure notices include the relevant action. |
| **Result and recovery** | Display folder, duration, and status. Actions **Open folder**, **Generate copy again**, **Recover recording**, and **Later** depending on context. |
| **Close dialog** | During recording, options **Continue recording** and **Stop and close**. Focus starts on **Continue recording**. Restore focus when the dialog closes. |

### Meters and waveform

- Linear dBFS scale from −60 to 0. Visual anchors at −60, −30, −18, −6, and 0 when space allows. The bar represents RMS with an initial 100 ms window and the marker represents peak. Initial hold time is 1 second. Target level for normal speech is between −18 and −6 dBFS peak.
- The yellow band begins above −6 dBFS. The marker changes to red at −1 dBFS. When saturation is observed, show **Saturation** and keep the indication for 2 seconds. Show **No signal detected** after 3 seconds of sustained level below −60 dBFS. Disconnection uses its own message. Values are proposed for the initial calibration.
- Update visuals at up to 30 Hz. Values can decay smoothly with an envelope generated by the engine. Reduced motion preference removes smoothing and ornamental animation. Keep instant reading and focused controls.
- Scale the canvas to the WebView pixel density. Limit the visual buffer with minimum and maximum values per column. Resize without altering the audio or the temporal reference of the session. [OBS — meter guide](https://obsproject.com/kb/audio-mixer-guide)

## 4. States by screen and messages

| State | Visible message or title | Priority action |
| --- | --- | --- |
| First use | “Choose where to save your recordings.” | **Choose folder** |
| Ready | Current source and destination. Timer at zero. | **Record** |
| Testing | “Testing level” and active meters. | **Stop test** or **Record** |
| Starting | “Preparing recording…” | **Stop** to cancel |
| Recording | “Recording” and live time. | **Stop** |
| Transient source failure | “Microphone disconnected. Trying to reconnect…” or equivalent source message | **Stop** |
| Finalizing | “Saving recording…” | Wait |
| Exporting | “Recording saved. Preparing transcription copy…” | **Open folder** |
| Saved | “Recording saved.” | **Open folder** or **Record** |
| Copy failed | “Recording saved. The transcription copy failed.” | **Generate copy again** |
| Partial capture | “Partial recording saved.” and the reason if known | **Open folder** |
| Permission denied | “Allow access to the microphone to record this source.” or equivalent for system audio | **Open settings** when available |
| Folder unavailable | “The selected folder is unavailable.” | **Change folder** |
| Recovery available | “We found an interrupted recording.” | **Recover recording** or **Later** |

Use textual success state, warnings, and icons. State messages go to an `aria-live="polite"` region. Errors that block the flow may be announced as alerts. The meter, timer, and waveform do not update the live region. Labels for **Record** and **Stop** appear below the control for pointer and keyboard users.

## 5. Accessible interaction and visual validation

- Use native HTML elements for button, field, list, and checkbox whenever possible. Tab order matches the visual hierarchy. Enter and Space activate the focused control. Escape closes a dialog when allowed. The primary button keeps focus when switching from **Record** to **Stop**.
- When opening contextual errors, associate the message with the relevant control. In dialogs, trap focus and restore it when closed. Long source names and folder paths need full accessible content.
- In `forced-colors`, allow the OS to define control colors and preserve outlines. Maintain low-level and saturation labels. Test text zoom and OS scaling. [W3C — WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- Visual implementation checklist: light and dark mode, initial and minimum sizes, enlarged text, dual meters, long folder path, missing device, permission denied, finalization, export, recovery, and partial recording. Verify keyboard and VoiceOver on Mac, Narrator on Windows, and Orca on the Linux target.
- Homologation screenshots include the three product WebViews. Verify hierarchy, clipping, interaction target, contrast, and state against the tables. The design system is an implementation specification.
