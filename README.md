# Taffy

Taffy is a small Rust screen capture app aimed at modern Wayland desktops, especially COSMIC on Arch Linux.

## What it does right now

- Takes screenshots through the XDG desktop portal
- Records videos through the ScreenCast portal and GStreamer
- Converts recordings to GIFs after capture stops
- Stores a few simple preferences:
  - interactive vs whole-screen source selection
  - frame rate
  - start delay
  - stop delay
  - start, stop, and screenshot shortcut preferences

## Current design choices

- Taffy intentionally uses the portal stack first so it can behave better on newer desktops like COSMIC.
- Screenshot capture relies on the compositor's portal UI.
- Video and GIF capture use the ScreenCast portal, then record the granted PipeWire stream with GStreamer.
- Taffy asks the Global Shortcuts portal for start, stop, and screenshot shortcuts when the desktop supports it.
- On the current COSMIC portal backend, global shortcuts are not exposed, so Taffy falls back to focused-window shortcuts.
- Selection recording currently uses `slurp` after the portal share flow so Taffy can crop the granted monitor stream.

## Run

```bash
cargo run
```

To install it as a launcher later, copy `assets/taffy.desktop` into `~/.local/share/applications/` after you have a built `taffy` binary somewhere on your `PATH`.

## Runtime requirements

On Arch Linux, make sure these pieces are available:

- `xdg-desktop-portal`
- a portal backend for your session, such as `xdg-desktop-portal-cosmic`
- `pipewire`
- `gstreamer`
- `gst-plugins-good`
- `gst-plugins-bad`
- `gst-plugins-ugly`
- `ffmpeg`
- `slurp` for Selection mode recording

## Notes

- Selection recording asks the portal for a monitor first, then uses `slurp` to choose the crop rectangle.
- Whole-screen recording asks the portal for monitor-only sources.
- GIF capture records to a temporary MP4 first, then converts it with `ffmpeg`.
