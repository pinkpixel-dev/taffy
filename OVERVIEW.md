# Overview

## Purpose

Taffy is a lightweight Wayland screen capture utility built in Rust. The project targets modern Linux desktops, especially COSMIC on Arch Linux, where older recorder stacks can be unreliable or incomplete.

The design goal is a small, friendly capture app with practical defaults and a backend strategy that works with desktop portals instead of fighting them.

## Stack

- Language: Rust
- UI: `iced`
- Wayland desktop integration: `ashpd`
- Video pipeline: `gstreamer`
- Video and GIF post-processing: `ffmpeg` and `ffprobe`
- Region selection helper: `slurp`
- Config persistence: `serde` + JSON

## Architecture

### UI Layer

The main UI lives in [src/main.rs](/home/sizzlebop/PINKPIXEL/PROJECTS/CURRENT/taffy/src/main.rs).

Responsibilities:

- render the compact app interface
- manage preferences and shortcut editing state
- start and stop capture tasks
- show status text and shortcut binding feedback
- maintain the in-app recording timer
- configure Linux window metadata such as icon and application id

### Configuration Layer

The config model lives in [src/config.rs](/home/sizzlebop/PINKPIXEL/PROJECTS/CURRENT/taffy/src/config.rs).

Responsibilities:

- define `CaptureKind` and `CaptureSource`
- provide defaults for output folders and shortcuts
- load and save the JSON config file
- ensure output directories exist

### Shortcut Layer

Shortcut handling lives in [src/shortcuts.rs](/home/sizzlebop/PINKPIXEL/PROJECTS/CURRENT/taffy/src/shortcuts.rs).

Responsibilities:

- attempt portal global shortcut registration
- surface portal shortcut status messages
- parse user-entered shortcut strings
- match in-window `iced` keyboard events as a reliable fallback

Important current behavior:

- Taffy tries the Global Shortcuts portal when available.
- On COSMIC, the installed portal backend currently exposes `ScreenCast` and `Screenshot`, but not `GlobalShortcuts`.
- Because of that, focused-window shortcuts are currently the reliable shortcut path on COSMIC.

### Capture Layer

Capture behavior lives in [src/capture.rs](/home/sizzlebop/PINKPIXEL/PROJECTS/CURRENT/taffy/src/capture.rs).

Responsibilities:

- screenshot portal flow
- screencast portal session setup
- GStreamer recording pipeline
- GIF conversion
- selection-region handling
- post-process crop for selection recordings

## Capture Flows

### Screenshot

1. Taffy calls the Screenshot portal.
2. The portal returns a temporary file URI.
3. Taffy copies the screenshot into the configured screenshot directory.

### Whole-Screen Video

1. Taffy opens a ScreenCast portal session.
2. The user grants a monitor source.
3. Taffy records the PipeWire stream with GStreamer.
4. On stop, the finalized MP4 is saved to the configured video directory.

### Selection Video

1. Taffy opens a ScreenCast portal session for a monitor.
2. The user grants that source.
3. Taffy runs `slurp` to collect the desired rectangle.
4. Taffy records the full monitor stream.
5. On stop, Taffy inspects the recorded video dimensions with `ffprobe`.
6. Taffy crops the saved video with `ffmpeg` using scaled coordinates derived from the original selection rectangle.

This post-process crop approach is deliberate. A previous live-crop pipeline experiment caused empty output files and was removed in favor of the more stable post-stop crop flow.

### GIF

1. Taffy records to a temporary MP4.
2. Taffy optionally crops for selection mode.
3. Taffy converts the result to GIF with `ffmpeg`.
4. The temporary MP4 is removed.

## Why Portals First

Portal-first design is the core architectural decision in Taffy.

Benefits:

- better compatibility with Wayland security constraints
- better fit for COSMIC and newer desktops
- less dependence on compositor-specific direct capture hacks

Tradeoff:

- feature availability depends on what the active portal backend exposes
- global shortcuts are limited by desktop support
- some flows require helper tools until Taffy has a richer native implementation

## Linux Integration Notes

Taffy currently sets:

- an embedded window icon from `icon.png`
- a Linux application id of `taffy`

Why this matters:

- some desktops use the raw window icon
- some desktops prefer matching a window app id to a desktop entry
- launcher/taskbar behavior can differ between `cargo run` and an installed desktop entry

If taskbar identity is still inconsistent, the next likely step is fuller desktop integration by installing:

- `~/.local/share/applications/taffy.desktop`
- a matching icon under the local icon theme path

## Known Constraints

- COSMIC does not currently expose the Global Shortcuts portal interface needed for true compositor-wide shortcut registration through the standard portal API.
- Selection mode currently depends on `slurp`.
- There is no native floating recording overlay yet.
- Audio and microphone capture are not implemented yet.

## Suggested Next Work

### UX

- add a floating recording overlay with timer and stop affordance
- add audio and microphone toggles
- add format options beyond the current basic flows

### Capture

- replace `slurp` with a native Taffy region selector
- add better desktop-specific diagnostics around portal capabilities

### Packaging

- install desktop file and icon properly for smoother taskbar integration
- prepare an Arch-friendly packaging flow
