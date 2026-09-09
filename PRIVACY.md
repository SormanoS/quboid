# Privacy Policy

**Last updated: 9 September 2026**

Quboid collects no personal information, and it transmits nothing anywhere.

## No data is collected or shared

Quboid contains no telemetry, no analytics, no advertising, no crash
reporting and no update checks. It sends nothing to the author, Samuele
Sormano, or to any third party, because it cannot: the program has no
networking code and links no networking library, so it is unable to open a
network connection at all.

## What Quboid stores on your device

Two things, both local to your computer and never transmitted:

- **Your settings** — your chosen language, keyboard shortcuts, gap and
  snapping preferences, in `config.json`.
- **A diagnostic log** — the interface language picked from your Windows
  locale, and error messages when something fails, such as a configuration
  file that could not be read. The log records no window titles, no keystrokes
  and no document contents.

Where these live depends on how you installed Quboid:

| Installation | Location |
| --- | --- |
| Microsoft Store | `%LOCALAPPDATA%\Packages\<package>\LocalCache\` |
| Installer | `%APPDATA%\Quboid\` and `%LOCALAPPDATA%\Quboid\logs\` |
| Portable | Next to the executable |

You may read, edit or delete these files at any time. Uninstalling Quboid
from the Microsoft Store removes them.

## What Quboid reads from your system

To do its job Quboid asks Windows about the windows on your desktop — their
position and size — and registers the keyboard shortcuts you configured. This
information is used to move and resize windows as you ask, is held in memory
only, and is never written down or sent anywhere.

## Children

Quboid is a desktop utility. It has no accounts, no content and no way to
communicate with anyone, so it neither collects nor can reveal anything about
a user of any age.

## Changes

If this policy ever changes, the revision will be published in the project
repository along with the change that prompted it, and the date above will be
updated.

## Contact

This policy is published by Samuele Sormano, the author of Quboid. Questions
about it can be raised at <https://github.com/SormanoS/quboid/issues>.
