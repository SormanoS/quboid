# Changelog

What changed in each version of Quboid, for the people who use it. The Italian
version is [`CHANGELOG.it.md`](CHANGELOG.it.md); both are kept in step.

Every version has a `## [X.Y.Z]` section in both files, and the release
workflow refuses a tag whose section is missing. The section becomes the notes
of the GitHub release and the "What's new in this version" text of the
Microsoft Store listing, which accepts plain text only: write short bullets,
avoid tables and images, and keep each section under 1500 characters.

## [0.1.3]

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
