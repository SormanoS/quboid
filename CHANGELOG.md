# Changelog

What changed in each version of Quboid, for the people who use it. The Italian
version is [`CHANGELOG.it.md`](CHANGELOG.it.md); both are kept in step.

Every version has a `## [X.Y.Z]` section in both files, and the release
workflow refuses a tag whose section is missing. The section becomes the notes
of the GitHub release and the "What's new in this version" text of the
Microsoft Store listing, which accepts plain text only: write short bullets,
avoid tables and images, and keep each section under 1500 characters.

## [1.0.1] - 2026-10-28

### Added

- On first launch, Quboid now opens the settings window with the shortcut sheet
  so you can see every shortcut right away and know where to find it in the
  notification area (`Ctrl+Alt+S`).
- Screen-reader labels for dropdowns and sliders in the settings window, and
  shortcut capture support for media, browser navigation and lock keys.

### Fixed

- The default `Restore` shortcut is now `Ctrl+Alt+Backspace` (matching
  Rectangle on macOS) instead of `Ctrl+Alt+Delete`, which Windows reserves for
  the system security screen and always reported as already in use. Existing
  configurations with the old default are updated automatically.

## [1.0.0] - 2026-10-19

### Added

- Filter shortcuts by action name or key combination and restore the default
  bindings in one click from the Shortcuts page.
- Shortcuts that another program has already claimed are marked directly on the
  Shortcuts page and in the `Ctrl+Alt+S` list.

### Changed

- The snap threshold is shown and adjusted as a percentage (`1%`–`15%`) in
  General settings.

### Fixed

- Clicking an action card in Quboid's window places the last active external
  window instead of failing because Quboid itself is in the foreground.
- Moving a window to the next or previous monitor preserves the configured gap
  at the destination monitor's scaling.

## [0.1.3] - 2026-09-24

### Added

- Undo, on `Ctrl+Alt+Z`, takes back the last placement of a window one step at
  a time, including a maximize.
- `Ctrl+Alt+S` shows every action and the keys that run it, and opens even when
  Quboid is in the notification area.
- Windows are put back on the display they were on after a dock, an undock or a
  resolution change. The setting is on by default.
- Starting Quboid while it is already running brings up the settings window
  instead of starting a second copy that would own none of the shortcuts.
- The notification area icon says how many shortcuts are taken by another
  program.

### Changed

- The gap between windows keeps the same size on every monitor, whatever each
  one is scaled to.
- A configuration from an earlier version gains the default shortcuts for the
  new actions, without replacing any combination you already use.
- The installer closes a running Quboid before updating it and starts it again
  afterwards, so the new version owns every shortcut.

### Fixed

- Group headings on the actions page are readable on a light theme.

## [0.1.2] - 2026-09-11

### Changed

- First release published on GitHub, as an installer and a portable archive.
- Quboid no longer needs the Visual C++ Redistributable.

## [0.1.1]

Never distributed: its tag was withdrawn before the release completed. Its
changes ship in 0.1.2.

## [0.1.0] - 2026-09-09

### Added

- First release, on the Microsoft Store.
- Halves, thirds, quarters, centering, resizing and incremental movement from
  global shortcuts, one action per shortcut.
- Movement between monitors, including monitors at negative coordinates and
  with different scaling.
- Drag snapping to screen edges and corners, with a preview of the area.
- A configurable gap between windows.
- Italian and English interface, picked from the Windows language on first run.
- Tray mode and launch at login.
- JSON configuration with import, export and a portable mode.
