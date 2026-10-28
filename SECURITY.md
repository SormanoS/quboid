# Security Policy

## Supported Versions

Security fixes are released for the latest `1.x` version of Quboid.

| Version | Supported |
| --- | --- |
| 1.0.x | Yes |
| < 1.0 | No |

## Security Model

Quboid is a local desktop utility for Windows 10 (22H2+) and Windows 11:

- **No network code.** The executable links no networking library (`ws2_32.dll`, `winhttp.dll`, `wininet.dll`, etc.) and `Cargo.lock` contains no networking crates. Both properties are checked automatically in CI.
- **Runs as the current user (`asInvoker`).** Quboid never asks for administrator privileges and respects Windows User Interface Privilege Isolation (UIPI).
- **Local configuration with bounded size.** `config.json` is capped at 1 MiB and validated before any setting is applied.

Even without network access, a window manager registers global shortcuts and observes window movement, so security bugs (such as unsafe Win32 handle handling, path traversal in configuration import/export, or local privilege escalation) are taken seriously.

## Reporting a Vulnerability

Please **do not** open a public issue for a security vulnerability.

Instead, report it privately through one of these channels:

1. **GitHub Private Vulnerability Reporting (preferred):** open a private advisory at <https://github.com/SormanoS/quboid/security/advisories/new>.
2. **Email:** write to Samuele Sormano at <samuele.sormano@outlook.it> with the subject `Quboid security report`.

Include the Quboid version, Windows version, and steps to reproduce the issue. You can expect an initial response within 48 hours. Once a fix is released, credit will be given in the release notes unless you prefer to remain anonymous.
