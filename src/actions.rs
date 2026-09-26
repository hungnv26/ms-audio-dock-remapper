use crate::config::{ActionKind, ButtonAction, Settings};
use crate::installed_apps;

/// Plays the platform's default notification sound (Windows `MessageBeep`, a
/// system sound through `afplay` on macOS). Elsewhere this is a no-op.
pub fn beep() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Diagnostics::Debug::MessageBeep;
        use windows_sys::Win32::UI::WindowsAndMessaging::MB_OK;
        unsafe {
            let _ = MessageBeep(MB_OK);
        }
    }
    #[cfg(target_os = "macos")]
    {
        // Reap the player on a helper thread so it never lingers as a zombie.
        if let Ok(mut child) = std::process::Command::new("afplay")
            .arg("/System/Library/Sounds/Tink.aiff")
            .spawn()
        {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        // Best-effort: hook up platform sound later (paplay).
    }
}

/// Launches `command`, optionally with `args`. URIs / URLs / executables are all
/// handled: `open` (cross-platform) opens them via the OS shell, while custom
/// commands with arguments go through `std::process::Command`.
pub fn launch(command: &str, args: &str) -> Result<(), String> {
    let command = command.trim();
    if command.is_empty() {
        return Err("No command or URL is configured".into());
    }

    if args.trim().is_empty() {
        open::that_detached(command).map_err(|e| e.to_string())
    } else {
        std::process::Command::new(command)
            .args(args.split_whitespace())
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

/// Executes one button's action. Read-only with respect to the Dock.
pub fn execute(action: &ButtonAction) -> Result<(), String> {
    match action.kind {
        ActionKind::None => Ok(()),
        ActionKind::App => installed_apps::launch(&action.app_target),
        ActionKind::Command => launch(&action.command, &action.arguments),
        ActionKind::Sound => {
            beep();
            Ok(())
        }
    }
}

/// Executes the action and, when the setting is on, confirms it with a sound
/// (unless the action itself was the sound). What the backends call on press.
pub fn run(action: &ButtonAction, settings: &Settings) -> Result<(), String> {
    execute(action)?;
    if settings.play_confirmation_beep
        && !matches!(action.kind, ActionKind::None | ActionKind::Sound)
    {
        beep();
    }
    Ok(())
}
