# Quboid

Quboid is a native Windows window manager written in Rust. It provides global
shortcuts, multi-monitor placement, edge and corner drag snapping and a native
settings window.

The project currently targets Windows 10 22H2 and Windows 11 on x86_64.

Quboid starts in the notification area instead of opening its settings window.
Windows controls whether its icon is shown directly on the taskbar or under the
hidden-icons menu.

## Development

```powershell
cargo run -p quboid-app
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

Quboid runs with the current user's privileges. Windows intentionally prevents
it from controlling elevated windows.

## Features

- Halves, thirds, quarters, centering, resizing and incremental movement
- Movement between monitors with negative-coordinate and mixed-DPI support
- Configurable global shortcuts, one action per shortcut
- Edge and corner drag snapping with a non-interactive overlay
- Italian and English interface, picked from the Windows user locale on first run, tray mode and launch at login
- Versioned JSON configuration with import/export and portable mode

## Architecture

```text
crates/quboid-core      actions, geometry and the configuration document
crates/quboid-windows   the Win32 runtime: hooks, monitors, DPI, overlay
crates/quboid-ui        the settings window: shell, theme and pages
crates/quboid-app       the composition root, as a library and a binary
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
`quboid_app::run` accepts a `Profile` (storage, runtime adapters and optional
extra pages), so a host can reuse the whole application shell.

## Configuration

The document is stored at `%APPDATA%\Quboid\config.json`, or next to the
executable when a `portable.flag` file is present. Documents written by earlier
versions are migrated on load; settings that this build does not support are
reported and left out instead of being rewritten.

Before such settings disappear from the document, Quboid copies them once to
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
| WiX Toolset v4 or later | The `.msi` | `dotnet tool install --global wix` |
| A code-signing certificate and `signtool.exe` | Signed releases. Optional: unsigned artifacts build fine, but Windows SmartScreen warns about them | Your certificate authority |

WiX v7 refuses to build until its [Open Source Maintenance Fee](https://wixtoolset.org/osmf/)
licence is accepted (`error WIX7015`). `scripts\package.ps1` accepts it for you
when the WiX CLI it uses is v7 or later, so read that licence before packaging:
it asks organizations above a revenue threshold to sponsor the toolset.

### The executable

```powershell
cargo build --locked --release --target x86_64-pc-windows-msvc
```

The binary is written to `target\x86_64-pc-windows-msvc\release\quboid-app.exe`
and needs no installation: it runs from wherever it is.

`crates\quboid-app\build.rs` compiles `quboid.rc`, the resource script that embeds
`quboid.exe.manifest`. That manifest is what asks Windows for per-monitor DPI
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

- `Quboid-<version>-windows-x64.msi`: a per-user installer. It installs into
  `%LOCALAPPDATA%\Quboid`, adds a Start Menu shortcut, upgrades an older version
  in place and needs no administrator rights;
- `Quboid-<version>-windows-x64-portable.zip`: the executable, this README and
  the licence, plus a `portable.flag` file. That flag is what makes Quboid keep
  its configuration and logs next to the executable instead of under `%APPDATA%`,
  so the archive can be unpacked onto a USB stick;
- `SHA256SUMS.txt`: the checksums of both artifacts.

WiX also leaves a `Quboid-<version>-windows-x64.wixpdb` next to them. It holds
the debug symbols of the installer and is not part of a release.

```powershell
# Package what is already built, without compiling again.
.\scripts\package.ps1 -Version 0.1.0 -SkipBuild

# Use a WiX CLI that is not on PATH, or another one than the PATH provides.
.\scripts\package.ps1 -Version 0.1.0 -WixPath $env:USERPROFILE\.dotnet\tools\wix.exe

# Sign the executable and the installer with a certificate in your store.
.\scripts\package.ps1 -Version 0.1.0 -CertificateThumbprint <thumbprint>
```

### The Store package

```powershell
.\scripts\package-msix.ps1
```

This writes `dist\Quboid-<version>-windows-x64.msix`, a full-trust packaged
build for the Microsoft Store. The package is deliberately left unsigned: the
Store re-signs submitted MSIX packages itself, and the `Publisher` in
`packaging\msix\AppxManifest.xml` is the identifier Partner Center assigned to
the reserved name rather than a certificate anyone holds.

```powershell
# Register the staged layout instead of packing it. The app then runs with
# real package identity, which is the only way to exercise the packaged
# behaviour, and it needs Developer Mode but no certificate and no admin.
.\scripts\package-msix.ps1 -Register
```

Launch-at-login differs between the two builds. A packaged build has no say
over the Run key, so it drives the `StartupTask` the manifest declares; every
other build writes `HKCU\...\CurrentVersion\Run` as before. Both paths live
behind `set_launch_at_login`.

### Icons

```powershell
.\scripts\generate-icons.ps1
```

Every icon is drawn from one description, so the executable, the tray and the
Store package cannot drift apart. The script writes `quboid.ico`, which
`quboid.rc` embeds into the executable, and the scale- and target-size
variants under `packaging\msix\assets`. Those variants are named for Windows
to resolve, which it only does through the resource index `package-msix.ps1`
builds with `makepri`; the index is what lets the manifest name
`Square44x44Logo.png` even though no file has that exact name.

Pass `-PreviewPath <file.png>` to render the mark at the sizes that decide
whether it still reads, on both a light and a dark surface, without touching
the assets.

### Checking the artifacts

```powershell
.\scripts\test-packaging.ps1 -Version 0.1.0 -InstallSmoke
```

This verifies the checksums, the contents of the portable archive and the
properties of the installer, and reads back the manifest embedded in the shipped
executable. With `-InstallSmoke` it also installs the MSI silently, checks that
the executable landed in `%LOCALAPPDATA%\Quboid` and uninstalls it again; it
refuses to run that step when Quboid is already installed, so it never replaces
your own installation.

`.\scripts\test-all.ps1` runs the whole chain: formatting, tests, lints, the
release build, the Windows end-to-end test, packaging and these checks.

### Releasing

Pushing an `X.Y.Z` tag runs `.github\workflows\release.yml`, which refuses to go
on unless the tag matches the version in `Cargo.toml`, then publishes the MSI,
the portable archive and the checksums as a GitHub release and submits the MSIX
to the Microsoft Store. Tags carry no `v` prefix, so the tag and the version in
`Cargo.toml` are the same string.

Running the workflow by hand from the Actions tab rehearses all of that without
publishing anything: it builds, packages and verifies, and stops short of the
release and the submission. Nothing creates a tag, so there is no way to spend a
version by accident.

`Cargo.toml` is the only place a version is written. The MSIX identity derives
from it as `X.Y.Z.0`: the fourth part belongs to the Store and stays zero, which
is why a prerelease tag such as `1.0.0-rc.1` produces a GitHub release but no
Store submission. The Store also rejects a package that does not outrank the
published one, so a submission that fails certification cannot be retried under
the same version; the fix ships as a new patch release.

The submission needs five repository secrets — `STORE_PRODUCT_ID`,
`STORE_TENANT_ID`, `STORE_SELLER_ID`, `STORE_CLIENT_ID` and
`STORE_CLIENT_SECRET` — obtained from Partner Center and a Microsoft Entra ID
application holding the Manager role on the account. With none of them set the
job reports a skip instead of failing, so a fork releases without them. Setting
only some of them fails the release instead: a secret that exists but is empty
counts as missing, which is the likeliest way to break a release that worked
before. Partner Center still owns what no API can create: the reserved name, the
listing and the first submission are done by hand once, and every tag after that
ships on its own. Certification itself is asynchronous, so the job waits on the
outcome and fails if Microsoft rejects the package.

`STORE_CLIENT_SECRET` expires on the date chosen when it was created in Entra,
and releases fail to authenticate from that day on. Issuing a new secret on the
same application and updating the repository secret is the whole fix; nothing
else needs to change.

Releases are immutable, so deleting one does not free its tag name: the name is
burned and that version can never be released again. A release published by
mistake is superseded by the next patch rather than replaced.

## License

Copyright (C) 2026 Samuele Sormano.

Quboid is free software, distributed under the GNU General Public License,
version 3. See [`LICENSE`](LICENSE).

You may use, study, modify and redistribute Quboid, including commercially. If
you distribute it, modified or not, you must do so under the GPL and make the
complete corresponding source available to whoever receives it.

Contributions are accepted under the agreement described in
[`CONTRIBUTING.md`](CONTRIBUTING.md), which also lets the author license the
project under other terms.

## Privacy

Quboid collects nothing and transmits nothing; it has no networking code at
all. What it stores locally, and where, is described in
[`PRIVACY.md`](PRIVACY.md).
