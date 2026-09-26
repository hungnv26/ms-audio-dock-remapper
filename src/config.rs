use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persistent configuration for the resident remapper, serialized as pretty
/// JSON under the user's config dir. Version 3 maps every Dock button to its
/// own action; version 2 files (single Teams action) are migrated on load.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub device: DeviceFilter,
    pub buttons: Buttons,
    pub settings: Settings,
    /// Version-2 field, read for migration only and never written back.
    #[serde(rename = "action", skip_serializing)]
    legacy_action: Option<LegacyAction>,
}

/// Which physical Dock collection carries the Teams key. Hex strings (no 0x
/// prefix) so the file stays human-readable. Defaults match the observed real
/// Dock (VID 045E / PID 084D, UsagePage FF99 / Usage 0001).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceFilter {
    pub vendor_id: String,
    pub product_id: String,
    pub usage_page: String,
    pub usage: String,
}

/// The physical buttons on the Dock, in left-to-right order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Button {
    Teams,
    PlayPause,
    VolumeDown,
    VolumeUp,
    MicMute,
}

impl Button {
    pub const ALL: [Button; 5] = [
        Button::Teams,
        Button::PlayPause,
        Button::VolumeDown,
        Button::VolumeUp,
        Button::MicMute,
    ];

    /// Keys the OS also acts on. Binding one of them makes the macOS backend
    /// take the Dock over so only the bound action runs.
    pub const MEDIA: [Button; 3] = [Button::PlayPause, Button::VolumeUp, Button::VolumeDown];

    pub fn is_media(self) -> bool {
        Self::MEDIA.contains(&self)
    }

    /// User-facing name.
    pub fn label(self) -> &'static str {
        match self {
            Button::Teams => "Teams",
            Button::PlayPause => "Play / Pause",
            Button::VolumeDown => "Volume Down",
            Button::VolumeUp => "Volume Up",
            Button::MicMute => "Microphone Mute",
        }
    }

    /// What the OS does with the key on its own, shown as a hint.
    pub fn builtin_behavior(self) -> &'static str {
        match self {
            Button::Teams => "Ignored by the system",
            Button::PlayPause => "Also toggles playback",
            Button::VolumeDown => "Also lowers the volume",
            Button::VolumeUp => "Also raises the volume",
            Button::MicMute => "Also toggles the Dock's mic mute",
        }
    }
}

/// What a button press does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    #[default]
    None,
    /// Launch (or activate) the application at `app_target`.
    App,
    /// Open a URL, or run `command` with `arguments`.
    Command,
    /// Play the confirmation sound only.
    Sound,
}

impl ActionKind {
    pub const ALL: [ActionKind; 4] = [
        ActionKind::None,
        ActionKind::App,
        ActionKind::Command,
        ActionKind::Sound,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|k| *k == self).unwrap_or(0)
    }

    pub fn from_index(index: usize) -> ActionKind {
        Self::ALL.get(index).copied().unwrap_or_default()
    }
}

/// The action bound to one button. `app_target` is the launch handle of an
/// installed application (bundle path on macOS, Shell parsing name on
/// Windows); `app_name` is retained for display and recovery.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ButtonAction {
    pub kind: ActionKind,
    pub app_name: String,
    pub app_target: String,
    pub command: String,
    pub arguments: String,
}

impl ButtonAction {
    /// One-line description for the button list.
    pub fn summary(&self) -> String {
        match self.kind {
            ActionKind::None => "No action".to_string(),
            ActionKind::App if !self.app_name.is_empty() => format!("Open {}", self.app_name),
            ActionKind::App => "Open an application".to_string(),
            ActionKind::Command if !self.command.trim().is_empty() => {
                self.command.trim().to_string()
            }
            ActionKind::Command => "Open a URL or run a command".to_string(),
            ActionKind::Sound => "Play a sound".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Buttons {
    pub teams: ButtonAction,
    pub play_pause: ButtonAction,
    pub volume_down: ButtonAction,
    pub volume_up: ButtonAction,
    pub mic_mute: ButtonAction,
}

impl Buttons {
    pub fn get(&self, button: Button) -> &ButtonAction {
        match button {
            Button::Teams => &self.teams,
            Button::PlayPause => &self.play_pause,
            Button::VolumeDown => &self.volume_down,
            Button::VolumeUp => &self.volume_up,
            Button::MicMute => &self.mic_mute,
        }
    }

    pub fn get_mut(&mut self, button: Button) -> &mut ButtonAction {
        match button {
            Button::Teams => &mut self.teams,
            Button::PlayPause => &mut self.play_pause,
            Button::VolumeDown => &mut self.volume_down,
            Button::VolumeUp => &mut self.volume_up,
            Button::MicMute => &mut self.mic_mute,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(alias = "start_with_windows")]
    pub launch_at_login: bool,
    #[serde(alias = "minimize_to_tray")]
    pub start_hidden: bool,
    pub play_confirmation_beep: bool,
    pub enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            launch_at_login: false,
            start_hidden: false,
            play_confirmation_beep: true,
            enabled: true,
        }
    }
}

/// Version-2 single-action layout.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct LegacyAction {
    app_name: String,
    app_target: String,
    preset_id: String,
    command: String,
    arguments: String,
}

impl LegacyAction {
    fn into_button_action(self) -> ButtonAction {
        if !self.app_target.trim().is_empty() {
            return ButtonAction {
                kind: ActionKind::App,
                app_name: self.app_name,
                app_target: self.app_target,
                ..Default::default()
            };
        }
        match self.preset_id.as_str() {
            "custom" if !self.command.trim().is_empty() => ButtonAction {
                kind: ActionKind::Command,
                command: self.command,
                arguments: self.arguments,
                ..Default::default()
            },
            "beep" => ButtonAction {
                kind: ActionKind::Sound,
                ..Default::default()
            },
            _ => ButtonAction::default(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            version: 3,
            device: DeviceFilter {
                vendor_id: "045E".into(),
                product_id: "084D".into(),
                usage_page: "FF99".into(),
                usage: "0001".into(),
            },
            buttons: Buttons::default(),
            settings: Settings::default(),
            legacy_action: None,
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        dir.push("ms-audio-dock-remapper");
        dir.push("config.json");
        dir
    }

    pub fn load() -> Config {
        let path = Config::path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Config::default();
        };
        match serde_json::from_str::<Config>(&text) {
            Ok(parsed) => parsed.migrated(),
            Err(_) => Config::default(),
        }
    }

    /// Lifts a version-2 file into the per-button layout. Idempotent.
    fn migrated(mut self) -> Config {
        if let Some(legacy) = self.legacy_action.take() {
            if self.version < 3 && self.buttons.teams == ButtonAction::default() {
                self.buttons.teams = legacy.into_button_action();
            }
        }
        self.version = 3;
        self
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Config::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&path, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_a_version_2_app_target_to_the_teams_button() {
        let v2 = r#"{
            "version": 2,
            "action": { "app_name": "Calculator", "app_target": "/System/Applications/Calculator.app",
                        "preset_id": "registered_app", "command": "", "arguments": "" },
            "settings": { "start_with_windows": true, "minimize_to_tray": true,
                          "play_confirmation_beep": false, "enabled": true },
            "language": "en"
        }"#;
        let cfg = serde_json::from_str::<Config>(v2).unwrap().migrated();
        assert_eq!(cfg.version, 3);
        assert_eq!(cfg.buttons.teams.kind, ActionKind::App);
        assert_eq!(cfg.buttons.teams.app_name, "Calculator");
        assert_eq!(cfg.buttons.play_pause.kind, ActionKind::None);
        assert!(cfg.settings.launch_at_login);
        assert!(cfg.settings.start_hidden);
        assert!(!cfg.settings.play_confirmation_beep);
    }

    #[test]
    fn migrates_version_2_custom_commands_and_beeps() {
        let custom =
            r#"{"version":2,"action":{"preset_id":"custom","command":"https://example.com"}}"#;
        let cfg = serde_json::from_str::<Config>(custom).unwrap().migrated();
        assert_eq!(cfg.buttons.teams.kind, ActionKind::Command);
        assert_eq!(cfg.buttons.teams.command, "https://example.com");

        let beep = r#"{"version":2,"action":{"preset_id":"beep"}}"#;
        let cfg = serde_json::from_str::<Config>(beep).unwrap().migrated();
        assert_eq!(cfg.buttons.teams.kind, ActionKind::Sound);
    }

    #[test]
    fn version_3_round_trips_without_the_legacy_field() {
        let mut cfg = Config::default();
        cfg.buttons.volume_up.kind = ActionKind::Sound;
        let text = serde_json::to_string(&cfg).unwrap();
        assert!(!text.contains("\"action\""));
        let back: Config = serde_json::from_str(&text).unwrap();
        assert_eq!(back.buttons.volume_up.kind, ActionKind::Sound);
        assert_eq!(back.version, 3);
    }

    #[test]
    fn summaries_describe_the_bound_action() {
        let mut a = ButtonAction::default();
        assert_eq!(a.summary(), "No action");
        a.kind = ActionKind::App;
        a.app_name = "Zoom".into();
        assert_eq!(a.summary(), "Open Zoom");
    }
}
