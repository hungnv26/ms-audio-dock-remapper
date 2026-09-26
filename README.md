# MS Audio Dock Teams Key Remapper

[简体中文](README_CN.md)

Turn the dedicated Microsoft Teams button on your Microsoft Audio Dock into a
shortcut for the application, command, or link you actually use.

## Why this app exists

The Microsoft Audio Dock includes a dedicated Teams button, but Windows does
not provide a general-purpose way to assign that button to another
application. If Teams is not your primary meeting tool, the button is largely
unused.

MS Audio Dock Teams Key Remapper makes that physical button useful without
changing the Dock firmware, installing a custom driver, or requiring
administrator privileges.

## What it can do

- Bind an action to the Teams key, and on macOS to every button on the Dock
  (Teams, play/pause, volume down, volume up, microphone mute).
- Launch any installed application: desktop or Microsoft Store apps registered
  with Windows, `.app` bundles from the Applications folders on macOS.
- Run a custom executable, open a URL, or invoke another shell-supported
  target.
- Search the installed application list by name and see each application's
  native icon.
- Try an action from the settings window; every change applies immediately.
- Play an optional confirmation sound after a successful trigger.
- Start automatically at sign-in / login, optionally hidden.
- Continue listening from the system tray (Windows) or the menu bar (macOS)
  after the settings window is closed.

The settings window follows the layout of macOS System Settings: a sidebar
with Buttons, General and About sections, and grouped rows on the right.

## How it works

The app listens to the Audio Dock through the Windows Raw Input API and watches
for the HID report produced by the Teams button. This is read-only monitoring:
the app does not write to the Dock or replace its driver.

When a button press is detected, the selected action is sent to a dedicated
worker thread. Applications selected from the list are launched through the
Windows Shell, using the same registered application catalog exposed by
`shell:AppsFolder`. This allows the remapper to open both traditional desktop
programs and packaged Microsoft Store applications while keeping the device
listener responsive.

On macOS the same read-only monitoring runs through IOKit (via `hidapi`, in
shared mode so macOS keeps handling the Dock's own volume and media keys), the
app lives in the menu bar instead of the tray, and the picker lists the `.app`
bundles from the Applications folders. The Dock sends the identical Teams
report on both systems.

## Requirements

- Windows 10 or Windows 11, x64, **or** macOS 11 or later (Apple silicon and
  Intel; build from source, see below)
- Microsoft Audio Dock

The built-in device profile targets the standard Microsoft Audio Dock HID
identity and Teams-button report.

## Download

Download the latest release from the repository's
[Releases page](../../releases/latest). Each release contains:

| Package | Use it when |
| --- | --- |
| `*-windows-x64-installer.exe` | You want a normal per-user installation, Start menu shortcut, optional desktop shortcut, and uninstall support. |
| `*-windows-x64-portable.zip` | You want to extract and run the app without installing it. |
| `SHA256SUMS-*.txt` | You want to verify the downloaded files. |

The installer does not require administrator privileges. The portable package
stores the application executable, English and Chinese documentation, and the
license together in one ZIP archive.

## Getting started

1. Connect the Microsoft Audio Dock to your computer.
2. Install the app or extract the portable package, then start
   **MS Audio Dock Remapper**.
3. In the **Buttons** section, select the button you want to change (Teams on
   Windows; any of the five buttons on macOS).
4. Under **When pressed**, choose **Open an application** and pick one from
   the searchable list, or choose **Open a URL or run a command** and enter a
   URL, a program path and optional arguments.
5. Select **Test** to confirm the action works, then press the button on the
   Dock.

Changes apply and save immediately. The sidebar shows whether the Dock is
connected; the **About** section lists the device identity and the number of
matching input collections, and the Buttons section shows the most recent
press.

## Running in the background

Closing the settings window hides it; it does not stop the remapper. Reopen it
from the tray icon (double-click on Windows) or the menu bar icon (macOS).
Quit from the tray / menu bar menu or from the **About** section.

The **General** section offers:

- **Run button actions** — pause all remapping without quitting.
- **Confirmation sound** — play a short sound after an action runs.
- **Launch at login** — register a per-user login entry.
- **Start hidden** — start with only the tray / menu bar icon; the login entry
  then carries `--minimized`, so the silent start also holds when the
  configuration file cannot be read. You can pass `--minimized` to a shortcut
  of your own for the same effect.

On macOS, giving Play/Pause or a Volume key an action makes the app take the
Dock over from the system, so the key does only your action; keys without an
action are re-posted to the system and keep working (a held volume key no
longer auto-repeats). Re-posting needs Accessibility access: macOS asks for it
the first time you bind a media key, and until it is granted the system keeps
performing the key's own function as well.

Only one instance of the application can run at a time.

## Configuration

Settings are saved locally as readable JSON at:

```text
%APPDATA%\ms-audio-dock-remapper\config.json                       (Windows)
~/Library/Application Support/ms-audio-dock-remapper/config.json   (macOS)
```

Files written by earlier versions (one action for the Teams key) are migrated
automatically. The application does not need administrator privileges and does
not modify the Audio Dock firmware or driver.

## macOS

There is no prebuilt macOS download yet. Build the app bundle from source:

```bash
brew install rustup && rustup default stable   # once
./build-macos.sh
```

This produces `target/release/MS Audio Dock Remapper.app`, signed ad hoc;
pass a Developer ID identity as the first argument to sign for distribution.
Copy the bundle to `/Applications` and open it. The app shows a menu bar icon
(no Dock icon); use its menu to open the settings window or quit. "Launch at
login" writes a per-user LaunchAgent under `~/Library/LaunchAgents/`.

No Input Monitoring or Accessibility permission is required for the default
shared mode: the Dock's Teams key lives in a vendor HID collection that macOS
leaves open to applications. Only binding a media key asks for Accessibility
access, because the app then has to re-post the system media keys it
intercepts.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for environment setup, development
builds, testing, project rules, and release packaging.

## License

This project is released under the [MIT License](LICENSE).
