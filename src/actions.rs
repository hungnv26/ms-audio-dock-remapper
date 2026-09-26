use crate::config::Config;
use crate::installed_apps;
use crate::presets;

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
    if command.is_empty() {
        return Err("未配置启动命令".into());
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

/// Resolves the first installed executable among the hints, or None.
pub fn resolve_executable(preset: &presets::Preset) -> Option<String> {
    for hint in preset.exe_hints {
        let expanded = expand_env(hint);
        if std::path::Path::new(&expanded).exists() {
            return Some(expanded);
        }
    }
    None
}

fn expand_env(path: &str) -> String {
    // Minimal %VAR% expansion (Windows-style); harmless elsewhere.
    let mut result = path.to_string();
    for (key, val) in [("LOCALAPPDATA", "LOCALAPPDATA"), ("APPDATA", "APPDATA")] {
        if let Ok(v) = std::env::var(val) {
            result = result.replace(&format!("%{}%", key), &v);
        }
    }
    result
}

/// Executes the configured action. Read-only with respect to the Dock.
pub fn execute(config: &Config) -> Result<(), String> {
    if !config.action.app_target.trim().is_empty() {
        installed_apps::launch(&config.action.app_target)?;
        if config.settings.play_confirmation_beep {
            beep();
        }
        return Ok(());
    }

    // Version-1 compatibility: configs saved before AppsFolder support keep
    // working until the user saves a registered application from the new list.
    let preset = presets::find(&config.action.preset_id).ok_or("未知动作预设")?;

    if preset.kind == "beep" {
        beep();
        return Ok(());
    }

    let (command, args) = if preset.id == "custom" {
        (
            config.action.command.clone(),
            config.action.arguments.clone(),
        )
    } else {
        (
            resolve_executable(preset).unwrap_or_else(|| preset.command.to_string()),
            String::new(),
        )
    };

    launch(&command, &args)?;

    if config.settings.play_confirmation_beep {
        beep();
    }
    Ok(())
}
