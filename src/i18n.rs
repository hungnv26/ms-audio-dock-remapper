//! Internationalization (i18n).
//!
//! The UI is fully string-driven through [Lang] + [t]. The active language is
//! resolved at startup from the system UI language (when the persisted choice
//! is empty/"auto") and can be overridden by the user via the in-UI language
//! switcher. The chosen value is persisted in `Config.language`.

#[cfg(windows)]
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

/// Supported UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    /// Parses a persisted language code ("zh" / "en"). Returns None for the
    /// empty string or anything unknown so callers fall back to system detect.
    pub fn from_code(code: &str) -> Option<Lang> {
        match code.trim().to_ascii_lowercase().as_str() {
            "zh" | "zh-cn" | "zh_cn" | "chinese" => Some(Lang::Zh),
            "en" | "en-us" | "en_us" | "english" => Some(Lang::En),
            _ => None,
        }
    }

    /// The persisted representation of this language.
    pub fn code(self) -> &'static str {
        match self {
            Lang::Zh => "zh",
            Lang::En => "en",
        }
    }

    /// Label shown inside the language switcher ComboBox.
    pub fn label(self) -> &'static str {
        match self {
            Lang::Zh => "中文",
            Lang::En => "English",
        }
    }

    /// Auto-detects the system UI language. Defaults to English when detection
    /// is unavailable or the language is neither Chinese nor English.
    pub fn detect() -> Lang {
        #[cfg(windows)]
        {
            let lcid = unsafe { GetUserDefaultUILanguage() };
            // 0x0404 (zh-TW), 0x0804 (zh-CN), 0x1004 (zh-SG), 0x0c04 (zh-HK)...
            if (lcid & 0x03ff) == 0x0004 {
                return Lang::Zh;
            }
        }
        #[cfg(target_os = "macos")]
        {
            // `AppleLanguages` lists the user's preferred languages in order,
            // e.g. `("zh-Hans-CN", "en-US")`. GUI processes launched from
            // Finder/launchd carry no LANG, so the defaults database is the
            // reliable source.
            if let Ok(out) = std::process::Command::new("defaults")
                .args(["read", "-g", "AppleLanguages"])
                .output()
            {
                let text = String::from_utf8_lossy(&out.stdout);
                let first = text
                    .lines()
                    .map(|l| l.trim().trim_matches(|c| c == '"' || c == ','))
                    .find(|l| !l.is_empty() && *l != "(" && *l != ")");
                if first.is_some_and(|l| l.to_ascii_lowercase().starts_with("zh")) {
                    return Lang::Zh;
                }
            }
        }
        // On other platforms or an unknown system language, fall back to English.
        Lang::En
    }

    /// Resolves the effective language from a persisted code: use the system
    /// default when the code is empty/unknown.
    pub fn resolve(stored: &str) -> Lang {
        Lang::from_code(stored).unwrap_or_else(Lang::detect)
    }
}

/// Returns the translation for `key` in `lang`. Falls back to the key itself
/// when a string is missing, so the UI never goes blank during development.
pub fn t(lang: Lang, key: &str) -> &str {
    const ZH: &[(&str, &str)] = &[
        ("app_title", "Microsoft Audio Dock — Teams 键重映射"),
        ("subtitle", "Teams 键重映射"),
        ("status_prefix", "状态："),
        ("status_connected", "已连接 Dock 设备"),
        ("status_disconnected", "未检测到 Dock（插入后自动重连）"),
        ("collections", "已注册集合："),
        ("presses", "触发次数："),
        ("last", "上次触发："),
        ("action_label", "按下 Teams 键时执行"),
        ("apps_folder_count", "来自 shell:AppsFolder 的已注册程序："),
        ("no_registered_apps", "未找到已注册程序"),
        ("no_app_selected", "请先选择一个已注册程序。"),
        ("app_list_fail", "读取 shell:AppsFolder 失败："),
        ("app_no_longer_registered", "当前未注册"),
        ("search_apps", "搜索已注册程序…"),
        ("custom_command", "命令 / URL："),
        ("custom_args", "参数："),
        ("opt_enabled", "启用按键重映射"),
        ("opt_beep", "触发时播放确认提示音"),
        ("opt_autostart", "登录 Windows 时自动启动"),
        ("opt_minimize", "启动时最小化到托盘"),
        ("btn_save", "保存"),
        ("btn_test", "测试动作"),
        (
            "hint_tray",
            "关闭窗口只是最小化到托盘；双击托盘图标重新打开。",
        ),
        ("lang_label", "界面语言："),
        ("lang_button", "语言"),
        ("menu_exit", "退出"),
        ("tray_open", "打开设置…"),
        ("saved", "已保存配置"),
        ("test_ok", "测试动作已执行"),
        ("single_instance", "MsAudioDock Remapper 已在运行。"),
        ("init_fail", "无法创建监听窗口（Raw Input 初始化失败）。"),
        ("render_fail", "无法初始化图形后端，应用无法显示界面："),
        ("save_fail", "保存配置失败："),
        ("action_fail", "动作执行失败："),
        ("test_fail", "测试动作失败："),
        ("unknown_preset", "未知动作预设"),
        ("empty_command", "未配置启动命令"),
        ("preset_teams", "Microsoft Teams"),
        ("preset_zoom", "Zoom"),
        ("preset_webex", "Cisco Webex"),
        ("preset_googlemeet", "Google Meet"),
        ("preset_skype", "Skype"),
        ("preset_chatgpt_codex", "ChatGPT Codex"),
        ("preset_claude", "Claude"),
        ("preset_beep", "响提示音 (Demo)"),
        ("preset_custom", "自定义程序…"),
    ];
    const EN: &[(&str, &str)] = &[
        ("app_title", "Microsoft Audio Dock — Teams Key Remapper"),
        ("subtitle", "Teams key remapping"),
        ("status_prefix", "Status: "),
        ("status_connected", "Dock device connected"),
        (
            "status_disconnected",
            "No Dock detected (reconnects when plugged in)",
        ),
        ("collections", "Registered collections: "),
        ("presses", "Presses: "),
        ("last", "Last trigger: "),
        ("action_label", "When the Teams key is pressed"),
        (
            "apps_folder_count",
            "Registered apps from shell:AppsFolder: ",
        ),
        ("no_registered_apps", "No registered applications found"),
        ("no_app_selected", "Select a registered application first."),
        ("app_list_fail", "Failed to read shell:AppsFolder: "),
        ("app_no_longer_registered", "no longer registered"),
        ("search_apps", "Search registered apps…"),
        ("custom_command", "Command / URL:"),
        ("custom_args", "Arguments:"),
        ("opt_enabled", "Enable key remapping"),
        ("opt_beep", "Play confirmation sound on trigger"),
        ("opt_autostart", "Launch at Windows sign-in"),
        ("opt_minimize", "Start minimized to the tray"),
        ("btn_save", "Save"),
        ("btn_test", "Test action"),
        (
            "hint_tray",
            "Closing the window minimizes to the tray; double-click the tray icon to reopen.",
        ),
        ("lang_label", "Language:"),
        ("lang_button", "Language"),
        ("menu_exit", "Exit"),
        ("tray_open", "Open Settings…"),
        ("saved", "Settings saved"),
        ("test_ok", "Test action executed"),
        (
            "single_instance",
            "MsAudioDock Remapper is already running.",
        ),
        (
            "init_fail",
            "Failed to create the listener window (Raw Input init failed).",
        ),
        (
            "render_fail",
            "Failed to initialize the graphics backend, the user interface cannot be shown:",
        ),
        ("save_fail", "Failed to save config: "),
        ("action_fail", "Action failed: "),
        ("test_fail", "Test action failed: "),
        ("unknown_preset", "Unknown action preset"),
        ("empty_command", "No launch command configured"),
        ("preset_teams", "Microsoft Teams"),
        ("preset_zoom", "Zoom"),
        ("preset_webex", "Cisco Webex"),
        ("preset_googlemeet", "Google Meet"),
        ("preset_skype", "Skype"),
        ("preset_chatgpt_codex", "ChatGPT Codex"),
        ("preset_claude", "Claude"),
        ("preset_beep", "Play a sound (Demo)"),
        ("preset_custom", "Custom program…"),
    ];

    // macOS wording: no registry, no shell:AppsFolder, no tray — a menu bar
    // item, LaunchAgents and the Applications folders instead. Anything not
    // listed here falls through to the shared table.
    #[cfg(target_os = "macos")]
    {
        const ZH_MAC: &[(&str, &str)] = &[
            ("opt_autostart", "登录 macOS 时自动启动"),
            ("opt_minimize", "启动时仅显示菜单栏图标"),
            ("hint_tray", "关闭窗口只是隐藏；通过菜单栏图标重新打开。"),
            ("apps_folder_count", "已找到的应用程序："),
            ("app_list_fail", "扫描应用程序文件夹失败："),
            ("search_apps", "搜索应用程序…"),
            ("no_registered_apps", "未找到应用程序"),
            ("no_app_selected", "请先选择一个应用程序。"),
            ("app_no_longer_registered", "已不存在"),
            ("init_fail", "无法初始化 HID 监听："),
        ];
        const EN_MAC: &[(&str, &str)] = &[
            ("opt_autostart", "Launch at login"),
            ("opt_minimize", "Start hidden (menu bar icon only)"),
            (
                "hint_tray",
                "Closing the window hides it; use the menu bar icon to reopen.",
            ),
            ("apps_folder_count", "Applications found: "),
            ("app_list_fail", "Failed to scan the Applications folders: "),
            ("search_apps", "Search applications…"),
            ("no_registered_apps", "No applications found"),
            ("no_app_selected", "Select an application first."),
            ("app_no_longer_registered", "no longer installed"),
            ("init_fail", "Failed to initialize HID monitoring: "),
        ];
        let overrides = match lang {
            Lang::Zh => ZH_MAC,
            Lang::En => EN_MAC,
        };
        for (k, v) in overrides {
            if *k == key {
                return v;
            }
        }
    }

    let table = match lang {
        Lang::Zh => ZH,
        Lang::En => EN,
    };
    for (k, v) in table {
        if *k == key {
            return v;
        }
    }
    key
}
