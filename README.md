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

- Launch any desktop or Microsoft Store application registered with Windows.
- Run a custom executable, open a URL, or invoke another shell-supported
  target.
- Search the installed application list by name and see the native Windows
  icon for each application.
- Test an action before saving it.
- Play an optional confirmation sound after a successful trigger.
- Start automatically when you sign in to Windows.
- Continue listening from the system tray after the settings window is closed.
- Display its interface in English or Chinese.

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
3. Open the action list under **When the Teams key is pressed**.
4. Search for and select any application registered with Windows.
5. Alternatively, select **Custom program** at the top of the list and enter a
   command, executable path, URL, and optional arguments.
6. Select **Test action** to confirm that the target opens correctly.
7. Select **Save**.
8. Press the Teams button on the Dock.

The status area shows whether the Dock is detected, how many matching input
collections are registered, and the most recent trigger.

## Running in the background

Closing the settings window hides it in the system tray; it does not stop the
remapper. Double-click the tray icon to reopen the window. Use **Exit** in the
application menu when you want to stop the remapper completely.

You can also enable:

- **Launch at Windows sign-in**
- **Start minimized to the tray**
- **Play confirmation sound on trigger**
- **Enable key remapping**

With **Start minimized to the tray** enabled the application starts tray-only:
the settings window is not shown at all, while key monitoring runs as usual.
Double-click the tray icon to open the window. Combined with **Launch at Windows
sign-in** this gives a silent background start after boot — the sign-in entry is
registered as `"…\ms-audio-dock-remapper.exe" --minimized`, so the silent start
also holds when the configuration file cannot be read. You can pass
`--minimized` to a shortcut of your own for the same effect.

Only one instance of the application can run at a time.

## Configuration

Settings are saved locally as readable JSON at:

```text
%APPDATA%\ms-audio-dock-remapper\config.json
```

The application does not need administrator privileges and does not modify the
Audio Dock firmware or driver.

## Language

The interface follows the Windows UI language on first launch. English and
Chinese can also be selected directly from the application menu, and the
selection is remembered.

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

No Input Monitoring or Accessibility permission is required: the Dock's Teams
key lives in a vendor HID collection that macOS leaves open to applications.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for environment setup, development
builds, testing, project rules, and release packaging.

## License

This project is released under the [MIT License](LICENSE).
