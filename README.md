# Taffy

<p align="center">
  <img src="./icon.png" alt="Taffy logo" width="300" height="300" />
</p>

Taffy is a small Rust screen capture app for modern Wayland desktops, with a focus on Arch Linux and COSMIC. It aims to stay simple, approachable, and practical while still covering the everyday capture tasks: screenshots, videos, and GIFs.

## Current Features

- Screenshot capture through the XDG Screenshot portal
- Video recording through the ScreenCast portal and GStreamer
- GIF capture by recording to video first and converting with `ffmpeg`
- Whole-screen recording
- Region recording through a selection flow
- Configurable frame rate
- Configurable start and stop delay
- Pointer visibility toggle
- Per-type output folders for screenshots, GIFs, and videos
- Embedded window icon and Linux application id
- Keyboard shortcut preferences for start, stop, and screenshot
- Focused-window shortcut handling when global shortcuts are unavailable

## Current Behavior On COSMIC

- Taffy works through the portal stack first so it behaves better on Wayland and newer desktops.
- On the current COSMIC portal backend, `ScreenCast` and `Screenshot` are available.
- On the current COSMIC portal backend, `GlobalShortcuts` is not exposed, so compositor-wide shortcuts are not currently available through the standard portal path.
- Taffy therefore only receives shortcuts while its own window is focused, and it surfaces that limitation in the UI.
- In practice on COSMIC, this means screenshot shortcuts are not very useful when the Taffy window needs to stay out of the shot.
- In practice on COSMIC, video and GIF capture should currently be treated as a start-and-stop-from-the-Taffy-window workflow.

## Selection Mode

Selection mode currently works like this:

1. Taffy asks the ScreenCast portal for a monitor source.
2. After permission is granted, Taffy uses `slurp` to collect the region rectangle.
3. Taffy records the full granted monitor stream.
4. After recording stops, Taffy crops the finished file with `ffmpeg` using the actual recorded video dimensions.

This approach is intentional for now because it is more reliable than doing live cropping inside the recording pipeline on scaled displays.

## Runtime Requirements

On Arch Linux, install at least:

- `xdg-desktop-portal`
- `xdg-desktop-portal-cosmic` or another portal backend appropriate for your session
- `pipewire`
- `gstreamer`
- `gst-plugins-good`
- `gst-plugins-bad`
- `gst-plugins-ugly`
- `ffmpeg`
- `slurp` for Selection mode recording

Example:

```bash
sudo pacman -S xdg-desktop-portal xdg-desktop-portal-cosmic pipewire gstreamer gst-plugins-good gst-plugins-bad gst-plugins-ugly ffmpeg slurp
```

## Running

For development:

```bash
cargo run
```

## Output Folders

By default, Taffy saves:

- Screenshots to `~/Pictures/Taffy`
- Videos to `~/Videos/Taffy`
- GIFs to `~/Videos/Taffy`

These can be changed in the app preferences.

## Shortcuts

Taffy currently supports three configurable shortcuts:

- Start recording
- Stop recording
- Take screenshot

Supported shortcut strings currently include:

- Letters and digits, such as `Ctrl+I` or `Ctrl+Shift+R`
- `Print`
- `Space`
- `Tab`
- `Enter`
- `Escape`

If the desktop ignores global shortcut binding, Taffy only sees shortcuts while the Taffy window is focused.

For current COSMIC users, the practical guidance is:

- screenshots should usually be taken another way if the Taffy window would be visible
- videos and GIFs should generally be started and stopped from the Taffy UI, with any extra beginning or ending trimmed afterward if needed

## Taskbar Icon Notes

Taffy embeds `icon.png` into the app window and also sets its Linux application id to `taffy`.

If your desktop still does not show the taskbar icon when launching with `cargo run`, the desktop may be preferring desktop-entry integration over the raw window icon. In that case, install the desktop file and icon into your local application directories.

## Desktop Integration

The project includes a desktop entry at [assets/taffy.desktop](/home/sizzlebop/PINKPIXEL/PROJECTS/CURRENT/taffy/assets/taffy.desktop).

For launcher integration, copy it to:

```bash
~/.local/share/applications/taffy.desktop
```

Then make sure an icon named `taffy` is available to your icon theme, or adapt the desktop entry to point at an explicit icon path.

## Project Docs

- [OVERVIEW.md](/home/sizzlebop/PINKPIXEL/PROJECTS/CURRENT/taffy/OVERVIEW.md): technical overview and development guide
- [CHANGELOG.md](/home/sizzlebop/PINKPIXEL/PROJECTS/CURRENT/taffy/CHANGELOG.md): project history so far
- [LICENSE](/home/sizzlebop/PINKPIXEL/PROJECTS/CURRENT/taffy/LICENSE): Apache 2.0 license

## Current Limitations

- COSMIC does not currently expose the Global Shortcuts portal interface used by Taffy.
- Because of that, COSMIC users do not currently get true background screenshot or recording shortcuts.
- Selection mode depends on `slurp` right now.
- The recording timer is in the app UI; there is not yet a floating in-recording overlay.
- Audio and microphone capture are not implemented yet.
- The region-selection flow is functional, but it is not yet a fully native Taffy overlay like Kooha’s custom selector window.
