# Cuboid

Cuboid is a native Windows window manager written in Rust. It provides global
shortcuts, multi-monitor placement, edge and corner drag snapping and a native
settings window.

The project currently targets Windows 10 22H2 and Windows 11 on x86_64.

Cuboid starts in the notification area instead of opening its settings window.
Windows controls whether its icon is shown directly on the taskbar or under the
hidden-icons menu.

## Development

```powershell
cargo run -p cuboid-app
cargo test --workspace
```

`cargo test` also runs a guard that fails if separately licensed types, pages or
runtime logic reappear in this tree.

The complete suite also includes Windows-native and packaging tests:

```powershell
.\scripts\test-all.ps1 -WixPath C:\path\to\wix.exe
```

Use `-SkipPackaging` when only source and Windows behavior tests are needed.
The Windows E2E test requires an interactive desktop and an executable allowed
by the local application-control policy; pass a signed build with
`-ExecutablePath` when unsigned binaries are blocked.

Cuboid runs with the current user's privileges. Windows intentionally prevents
it from controlling elevated windows.

## Features

- Halves, thirds, quarters, centering, resizing and incremental movement
- Movement between monitors with negative-coordinate and mixed-DPI support
- Configurable global shortcuts, one action per shortcut
- Edge and corner drag snapping with a non-interactive overlay
- Italian and English interface, tray mode and launch at login
- Versioned JSON configuration with import/export and portable mode

## Architecture

```text
crates/cuboid-core      actions, geometry and the configuration document
crates/cuboid-windows   the Win32 runtime: hooks, monitors, DPI, overlay
crates/cuboid-ui        the settings window: shell, theme and pages
crates/cuboid-app       the composition root, as a library and a binary
```

The Windows runtime is a deep module behind a command/event interface. Three
small interfaces let a host change *what* it decides without reimplementing
*how* it talks to Windows:

- `SnapResolver`: which rectangle a drag snaps to (`EdgeSnapResolver` is the
  standard adapter);
- `ShortcutResolver`: which action a shortcut triggers
  (`SingleActionShortcuts` is the standard adapter);
- `WindowFilter`: which windows may be managed (`AllWindows` by default).

The settings window exposes `UiExtension`, so a host can contribute pages while
reusing the navigation rail, theme and shared controls. A hosted page reports
whether its change is still in progress or ready to be written, so continuous
edits stay visible to the runtime without rewriting the document on every frame.
`cuboid_app::run` accepts a `Profile` (storage, runtime adapters and optional
extra pages), so a host can reuse the whole application shell.

## Configuration

The document is stored at `%APPDATA%\Cuboid\config.json`, or next to the
executable when a `portable.flag` file is present. Documents written by earlier
versions are migrated on load; settings that this build does not support are
reported and left out instead of being rewritten.

Before such settings disappear from the document, Cuboid copies them once to
`config.json.unsupported.json` next to it. That archive is written a single time
and never rewritten, so a configuration written by another distribution survives
the first save of this build. A document that keeps the Base settings in a
`base` section, the shape another distribution writes, is imported by reading
that section and reporting everything stored beside it as discarded.

## Building and packaging

### What you need

| Tool | Needed for | How to get it |
| --- | --- | --- |
| Rust, stable MSVC toolchain | Everything. `rust-toolchain.toml` pins the channel and the `x86_64-pc-windows-msvc` target | [rustup](https://rustup.rs) |
| Visual Studio Build Tools with the C++ workload | The MSVC linker that Rust invokes | Visual Studio Installer, "Desktop development with C++" |
| Windows SDK | `mt.exe`, which packaging uses to verify the manifest embedded in the executable, and `rc.exe` if the manifest changes | Included in the workload above |
| WiX Toolset v4, v5 or v6 | The `.msi` | `dotnet tool install --global wix --version 4.*` |
| A code-signing certificate and `signtool.exe` | Signed releases. Optional: unsigned artifacts build fine, but Windows SmartScreen warns about them | Your certificate authority |

WiX v7 refuses to build until its Open Source Maintenance Fee licence is
accepted (`error WIX7015`). Either accept it, following the instructions the
error links to, or keep an earlier WiX and point the packaging script at it with
`-WixPath`, which is what the command below does when `wix.exe` on `PATH` is v7.

### The executable

```powershell
cargo build --locked --release --target x86_64-pc-windows-msvc
```

The binary is written to `target\x86_64-pc-windows-msvc\release\cuboid-app.exe`
and needs no installation: it runs from wherever it is.

`crates\cuboid-app\build.rs` compiles `cuboid.rc`, the resource script that embeds
`cuboid.exe.manifest`. That manifest is what asks Windows for per-monitor DPI
awareness and for the privileges of the current user, so window geometry stays
correct on mixed-DPI setups. The build locates `rc.exe` in the installed Windows
SDK on its own, so editing the manifest needs no extra step: a plain
`cargo build` picks the change up.

### The installer and the portable archive

```powershell
.\scripts\package.ps1 -Version 0.1.0
```

The script builds the release executable, checks that the manifest embedded in
it is the expected one, and writes three files to `dist`:

- `Cuboid-<version>-windows-x64.msi`: a per-user installer. It installs into
  `%LOCALAPPDATA%\Cuboid`, adds a Start Menu shortcut, upgrades an older version
  in place and needs no administrator rights;
- `Cuboid-<version>-windows-x64-portable.zip`: the executable, this README and
  the licence, plus a `portable.flag` file. That flag is what makes Cuboid keep
  its configuration and logs next to the executable instead of under `%APPDATA%`,
  so the archive can be unpacked onto a USB stick;
- `SHA256SUMS.txt`: the checksums of both artifacts.

WiX also leaves a `Cuboid-<version>-windows-x64.wixpdb` next to them. It holds
the debug symbols of the installer and is not part of a release.

```powershell
# Package what is already built, without compiling again.
.\scripts\package.ps1 -Version 0.1.0 -SkipBuild

# Use a WiX CLI that is not on PATH, or an older one than the PATH provides.
.\scripts\package.ps1 -Version 0.1.0 -WixPath $env:USERPROFILE\.dotnet\tools\wix.exe

# Sign the executable and the installer with a certificate in your store.
.\scripts\package.ps1 -Version 0.1.0 -CertificateThumbprint <thumbprint>
```

### Checking the artifacts

```powershell
.\scripts\test-packaging.ps1 -Version 0.1.0 -InstallSmoke
```

This verifies the checksums, the contents of the portable archive and the
properties of the installer, and reads back the manifest embedded in the shipped
executable. With `-InstallSmoke` it also installs the MSI silently, checks that
the executable landed in `%LOCALAPPDATA%\Cuboid` and uninstalls it again; it
refuses to run that step when Cuboid is already installed, so it never replaces
your own installation.

`.\scripts\test-all.ps1` runs the whole chain: formatting, tests, lints, the
release build, the Windows end-to-end test, packaging and these checks.

## License

Cuboid is distributed under the Mozilla Public License, Version 2.0. See
[`LICENSE`](LICENSE). Each source file carries the MPL-2.0 notice.
