# Taffy

<p align="center">
  <img src="./icon.png" alt="Taffy logo" width="300" height="300" />
</p>

Taffy is a small Rust screen capture app for the COSMIC desktop on Wayland. It aims to stay simple, approachable, and practical while still covering the everyday capture tasks: screenshots, videos, and GIFs.

## Supported Environment

Taffy should currently be treated as a COSMIC-only app.

- Supported target: Linux systems running the COSMIC desktop on Wayland
- Not supported: non-COSMIC desktops or X11 sessions
- Arch Linux is not required by the codebase itself

Taffy is built on standard Linux pieces such as XDG desktop portals, PipeWire, GStreamer, `ffmpeg`, and `slurp`. Nothing in the code currently ties it specifically to Arch Linux. In principle, it should run on other Linux distributions that provide COSMIC plus the required runtime packages.

That means the real requirement is COSMIC, not Arch. Arch is simply the environment this project has been centered around so far, and the package examples below use Arch package names.

Other distributions should be considered possible-but-not-yet-verified targets. If you are on Ubuntu, Debian, Pop!_OS, or another `apt`-based system, the main thing to check is whether your repositories actually provide the COSMIC session and `xdg-desktop-portal-cosmic`.

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

Taffy currently expects all of the following to be available:

- `xdg-desktop-portal`
- `xdg-desktop-portal-cosmic`
- `pipewire`
- `gstreamer`
- `gst-plugins-good`
- `gst-plugins-bad`
- `gst-plugins-ugly`
- `ffmpeg`
- `slurp` for Selection mode recording

On Arch Linux:

```bash
sudo pacman -S xdg-desktop-portal xdg-desktop-portal-cosmic pipewire gstreamer gst-plugins-good gst-plugins-bad gst-plugins-ugly ffmpeg slurp
```

On Debian/Ubuntu-family systems with COSMIC packages available, the equivalent package names are typically:

- `xdg-desktop-portal`
- `xdg-desktop-portal-cosmic`
- `pipewire`
- `gstreamer1.0-tools`
- `gstreamer1.0-plugins-good`
- `gstreamer1.0-plugins-bad`
- `gstreamer1.0-plugins-ugly`
- `ffmpeg`
- `slurp`

Example:

```bash
sudo apt install xdg-desktop-portal xdg-desktop-portal-cosmic pipewire gstreamer1.0-tools gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly ffmpeg slurp
```

If `apt` cannot find `xdg-desktop-portal-cosmic`, that usually means your distro or enabled repositories do not currently ship the COSMIC portal backend. In that case, Taffy should be treated as unsupported on that system for now.

## Running

For development:

```bash
cargo run
```

## Install From GitHub Releases

Most users should download a packaged release instead of building Taffy locally.

Current release page:

- [Taffy v1.0.0 release](https://github.com/pinkpixel-dev/taffy/releases/tag/v1.0.0)

Direct downloads:

- AppImage:
  `https://github.com/pinkpixel-dev/taffy/releases/download/v1.0.0/taffy.AppImage`
- Debian package:
  `https://github.com/pinkpixel-dev/taffy/releases/download/v1.0.0/taffy_1.0.0-1_amd64.deb`
- RPM package:
  `https://github.com/pinkpixel-dev/taffy/releases/download/v1.0.0/taffy-1.0.0-1.x86_64.rpm`

### AppImage

Download it, make it executable, and run it:

```bash
mkdir -p ~/Applications
curl -L https://github.com/pinkpixel-dev/taffy/releases/download/v1.0.0/taffy.AppImage -o ~/Applications/taffy.AppImage
chmod +x ~/Applications/taffy.AppImage
~/Applications/taffy.AppImage
```

### Debian And Ubuntu

Download the `.deb` package and install it with `apt`:

```bash
curl -L https://github.com/pinkpixel-dev/taffy/releases/download/v1.0.0/taffy_1.0.0-1_amd64.deb -o /tmp/taffy_1.0.0-1_amd64.deb
sudo apt install /tmp/taffy_1.0.0-1_amd64.deb
```

If your system does not support installing a local package through `apt`, use:

```bash
sudo dpkg -i /tmp/taffy_1.0.0-1_amd64.deb
sudo apt -f install
```

### RPM-Based Distributions

Download the `.rpm` package and install it with your distro package manager:

For Fedora, Nobara, or other `dnf`-based systems:

```bash
curl -L https://github.com/pinkpixel-dev/taffy/releases/download/v1.0.0/taffy-1.0.0-1.x86_64.rpm -o /tmp/taffy-1.0.0-1.x86_64.rpm
sudo dnf install /tmp/taffy-1.0.0-1.x86_64.rpm
```

For openSUSE or other `zypper`-based systems:

```bash
sudo zypper install /tmp/taffy-1.0.0-1.x86_64.rpm
```

### Notes

- These packages are intended for Linux systems running the COSMIC desktop on Wayland.
- You still need the runtime dependencies listed above, especially the COSMIC portal backend and media tools.
- RPM dependency names can vary more across distros, so some RPM-based systems may still need package metadata refinement.
- If you want to browse all published assets for future versions, use the GitHub releases page:
  `https://github.com/pinkpixel-dev/taffy/releases`

## Packaging For Maintainers

Taffy also includes package metadata for:

- AppImage through `cargo-appimage`
- `.deb` through `cargo-deb`
- `.rpm` through `cargo-generate-rpm`

This is mainly useful if you are maintaining release artifacts or testing packaging changes locally.

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

- Taffy is currently only supported on COSMIC.
- Other desktops may expose different portal behavior, and this project is not currently documenting or supporting those paths.
- COSMIC does not currently expose the Global Shortcuts portal interface used by Taffy.
- Because of that, COSMIC users do not currently get true background screenshot or recording shortcuts.
- Selection mode depends on `slurp` right now.
- The recording timer is in the app UI; there is not yet a floating in-recording overlay.
- Audio and microphone capture are not implemented yet.
- The region-selection flow is functional, but it is not yet a fully native Taffy overlay.
