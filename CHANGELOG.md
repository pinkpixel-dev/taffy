# Changelog

All notable project work so far is documented here.

## Unreleased

### Added

- Initial Rust implementation of Taffy using `iced`
- Portal-first screenshot capture
- Portal and GStreamer based video recording
- GIF conversion flow using `ffmpeg`
- Persisted configuration for:
  - capture type
  - capture source
  - frame rate
  - start delay
  - stop delay
  - output folders
  - shortcut preferences
- Compact launcher-style UI with menu, preferences, and shortcut sections
- Pointer visibility toggle
- Embedded window icon support from `icon.png`
- Linux application id set to `taffy`
- Desktop entry asset in [assets/taffy.desktop](/home/sizzlebop/PINKPIXEL/PROJECTS/CURRENT/taffy/assets/taffy.desktop)
- Recording timer in the UI
- Focused-window shortcut fallback using `iced` keyboard events
- Selection recording flow using:
  - ScreenCast portal for monitor access
  - `slurp` for region selection
  - post-record crop with `ffmpeg`
- Screenshot save path handling to move portal screenshots into the configured output directory
- Per-type save directory preferences for screenshots, GIFs, and videos

### Changed

- Shifted the project away from a `wf-recorder`-first design toward a portal-first Wayland/COSMIC-safe design
- Updated the source selector terminology from an interactive picker to explicit `Selection` and `Whole Screen`
- Removed the earlier minimize-first behavior because it left the app unrecoverable in practice
- Updated selection button styling so the active source is visually obvious
- Improved runtime messaging around shortcut support on COSMIC
- Updated README content to reflect current runtime behavior and requirements

### Fixed

- Screenshots now default to `~/Pictures/Taffy` instead of remaining in the portal temp location
- Selection recording no longer relies on the unstable live-crop pipeline that produced empty video files
- Selection crop now uses actual recorded video dimensions during the post-process crop step
- Shortcut parsing now supports common combinations like `Ctrl+I`, `Ctrl+Shift+R`, and `Print`
- Shortcut validation feedback now appears in the UI

### Known Gaps

- COSMIC currently does not expose the Global Shortcuts portal interface needed for true compositor-wide shortcuts through the standard portal path
- Audio capture and microphone capture are still pending
- A native floating recording overlay is still pending
- Native in-app region selection is still pending; current selection uses `slurp`
