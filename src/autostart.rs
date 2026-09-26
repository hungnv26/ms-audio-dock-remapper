//! Login autostart. Windows writes the HKCU Run registry value (no admin
//! needed); macOS writes a per-user LaunchAgent. Both live behind the
//! `platform` backend so this module stays OS-agnostic.

/// Switch appended to the autostart command line when "start minimized to the
/// tray" is on. It deliberately duplicates `Settings::minimize_to_tray`: the
/// sign-in launch must stay silent even when the config file is missing,
/// unreadable, or was reset to defaults.
pub const MINIMIZED_FLAG: &str = "--minimized";

pub fn is_enabled() -> bool {
    crate::platform::autostart_enabled()
}

/// Registers/unregisters the login entry. `start_minimized` decides whether the
/// registered command line carries [`MINIMIZED_FLAG`].
pub fn set_enabled(enable: bool, start_minimized: bool) {
    crate::platform::set_autostart(enable, start_minimized);
}

/// True when this process was launched with the tray-only switch. Leading `-`
/// and `/` are stripped and the `-m` shorthand is accepted, so a hand-written
/// shortcut keeps working.
pub fn started_minimized() -> bool {
    // `args_os` rather than `args`: the latter panics on an argument that is not
    // valid Unicode, and nothing here is worth aborting the launch over.
    args_request_minimized(
        std::env::args_os()
            .skip(1)
            .map(|arg| arg.to_string_lossy().into_owned()),
    )
}

fn args_request_minimized(args: impl IntoIterator<Item = String>) -> bool {
    let flag = MINIMIZED_FLAG.trim_start_matches('-');
    args.into_iter().any(|arg| {
        let name = arg.trim_start_matches(['-', '/']).to_ascii_lowercase();
        name == flag || name == "m"
    })
}

#[cfg(test)]
mod tests {
    use super::{args_request_minimized, MINIMIZED_FLAG};

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn recognizes_the_flag_written_into_the_autostart_entry() {
        assert!(args_request_minimized(args(&[MINIMIZED_FLAG])));
    }

    #[test]
    fn accepts_the_common_windows_switch_spellings() {
        assert!(args_request_minimized(args(&["/minimized"])));
        assert!(args_request_minimized(args(&["-Minimized"])));
        assert!(args_request_minimized(args(&["-m"])));
        assert!(args_request_minimized(args(&["--other", "--minimized"])));
    }

    #[test]
    fn plain_launch_is_not_minimized() {
        assert!(!args_request_minimized(args(&[])));
        assert!(!args_request_minimized(args(&["--verbose"])));
        assert!(!args_request_minimized(args(&["minimize"])));
    }
}
