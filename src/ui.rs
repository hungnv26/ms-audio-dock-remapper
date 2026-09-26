use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use slint::{
    invoke_from_event_loop, CloseRequestResponse, Rgba8Pixel, SharedPixelBuffer, Timer, TimerMode,
    VecModel, Weak,
};

use crate::actions;
use crate::autostart;
use crate::config::Config;
use crate::i18n::{self, Lang};
use crate::platform::{self, MonitorEvent};

slint::slint! {
    #[style = "fluent"]
    import { CheckBox, LineEdit, ScrollView }
        from "std-widgets.slint";

    export struct AppChoice {
        name: string,
        icon: image,
        visible: bool,
    }

    // Single source of truth for the visual language: colors, radii, motion.
    // Every component below reads from here so retheming stays in one place.
    global Theme {
        in-out property <color> bg: #f4f6f9;
        in-out property <color> surface: #ffffff;
        in-out property <color> border: #e7e9ee;
        in-out property <color> divider: #eef0f4;
        in-out property <color> text: #0f172a;
        in-out property <color> text2: #64748b;
        in-out property <color> text3: #9aa3b2;
        in-out property <color> accent: #3b66f5;
        in-out property <color> accent-press: #2f54d6;
        in-out property <color> accent-soft: #eaf0ff;
        in-out property <color> ok: #0f9d6b;
        in-out property <length> radius: 14px;
    }

    // A rounded card surface used to group related controls.
    component Card inherits Rectangle {
        property <length> pad: 16px;
        background: Theme.surface;
        border-radius: Theme.radius;
        border-width: 1px;
        border-color: Theme.border;
        VerticalLayout {
            padding: root.pad;
            spacing: 8px;
            @children
        }
    }

    // Filled, primary action button (accent).
    component PrimaryButton inherits Rectangle {
        in-out property <string> text: "";
        in-out property <bool> enabled: true;
        in-out property <string> font: "Microsoft YaHei";
        callback clicked;
        height: 42px;
        border-radius: 11px;
        background: root.enabled
            ? (touch-area.has-hover ? Theme.accent-press : Theme.accent)
            : #c7d0e6;
        touch-area := TouchArea {
            enabled: root.enabled;
            width: 100%;
            height: 100%;
            clicked => { if root.enabled { root.clicked(); } }
        }
        Text {
            text: root.text;
            color: #ffffff;
            font-size: 14px;
            font-weight: 600;
            font-family: root.font;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }

    // Outlined, secondary action button.
    component SecondaryButton inherits Rectangle {
        in-out property <string> text: "";
        in-out property <bool> enabled: true;
        in-out property <string> font: "Microsoft YaHei";
        callback clicked;
        height: 42px;
        border-radius: 11px;
        background: touch-area.has-hover ? #eef1f6 : Theme.surface;
        border-width: 1px;
        border-color: Theme.border;
        touch-area := TouchArea {
            enabled: root.enabled;
            width: 100%;
            height: 100%;
            clicked => { if root.enabled { root.clicked(); } }
        }
        Text {
            text: root.text;
            color: Theme.text;
            font-size: 14px;
            font-weight: 600;
            font-family: root.font;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }

    // Flat WinForms/MenuStrip-like command: transparent until hover/press.
    component MenuItem inherits Rectangle {
        in-out property <string> text: "";
        in-out property <string> font: "Microsoft YaHei";
        in property <image> icon;
        in property <bool> show-icon: false;
        callback clicked;
        height: 28px;
        border-radius: 2px;
        background: touch-area.pressed ? #dcdfe4
                    : touch-area.has-hover ? #e8eaed : #00000000;
        accessible-role: button;
        accessible-label: root.text;
        accessible-action-default => { root.clicked(); }

        HorizontalLayout {
            padding-left: 8px;
            padding-right: 8px;
            spacing: 5px;
            alignment: center;
            if root.show-icon : Image {
                y: (parent.height - self.height) / 2;
                source: root.icon;
                width: 16px;
                height: 16px;
                image-fit: contain;
                colorize: Theme.text2;
            }
            Text {
                text: root.text;
                color: Theme.text;
                font-size: 13px;
                font-family: root.font;
                vertical-alignment: center;
            }
        }
        touch-area := TouchArea {
            width: 100%;
            height: 100%;
            clicked => { root.clicked(); }
        }
    }

    // Icon-aware application picker. The standard ComboBox only accepts
    // strings, so AppsFolder icons require a small custom popup list.
    component AppSelector inherits Rectangle {
        in property <[AppChoice]> model;
        in-out property <int> current-index: 0;
        in-out property <string> search-text: "";
        in property <string> search-placeholder: "";
        in property <int> visible-count: 1;
        in property <string> font: "Microsoft YaHei";
        callback filter-changed(string);

        height: 42px;
        border-radius: 8px;
        border-width: 1px;
        border-color: #b8bec8;
        background: selector-touch.has-hover ? #f8faff : Theme.surface;
        clip: true;
        accessible-role: combobox;
        accessible-label: root.current-index >= 0 && root.current-index < root.model.length
            ? root.model[root.current-index].name : "";

        if root.current-index >= 0 && root.current-index < root.model.length : Image {
            x: 10px;
            y: (root.height - self.height) / 2;
            source: root.model[root.current-index].icon;
            width: 24px;
            height: 24px;
            image-fit: contain;
        }
        Text {
            x: 43px;
            y: 0px;
            width: root.width - 78px;
            height: root.height;
            text: root.current-index >= 0 && root.current-index < root.model.length
                ? root.model[root.current-index].name : "";
            color: Theme.text;
            font-size: 13px;
            font-family: root.font;
            horizontal-alignment: left;
            vertical-alignment: center;
            overflow: elide;
        }
        // Draw the chevron as a path instead of a font glyph. Some selected UI
        // fonts rendered the old glyph as a tofu square.
        Path {
            x: root.width - 25px;
            y: (root.height - self.height) / 2;
            width: 12px;
            height: 7px;
            commands: "M 1 1 L 6 6 L 11 1";
            fill: #00000000;
            stroke: Theme.text2;
            stroke-width: 1.5px;
        }
        selector-touch := TouchArea {
            width: 100%;
            height: 100%;
            clicked => { app-popup.show(); }
        }

        app-popup := PopupWindow {
            x: 0px;
            y: root.height + 4px;
            width: root.width;
            height: min(400px, root.visible-count * 40px + 56px);
            close-policy: close-on-click-outside;
            forward-focus: search-input;

            Rectangle {
                background: Theme.surface;
                border-width: 1px;
                border-color: Theme.border;
                border-radius: 8px;
                clip: true;
                Rectangle {
                    x: 0px;
                    y: 0px;
                    width: parent.width;
                    height: 52px;
                    background: Theme.surface;
                    search-input := LineEdit {
                        x: 8px;
                        y: 8px;
                        width: parent.width - 16px;
                        height: 36px;
                        text <=> root.search-text;
                        placeholder-text: root.search-placeholder;
                        font-family: root.font;
                        edited(text) => { root.filter-changed(text); }
                    }
                    Rectangle {
                        y: parent.height - 1px;
                        width: parent.width;
                        height: 1px;
                        background: Theme.divider;
                    }
                }
                ScrollView {
                    x: 0px;
                    y: 52px;
                    width: parent.width;
                    height: parent.height - 52px;
                    VerticalLayout {
                        padding-top: 4px;
                        padding-bottom: 4px;
                        spacing: 0px;
                        for choice[index] in root.model : Rectangle {
                            visible: choice.visible;
                            height: choice.visible ? 40px : 0px;
                            background: row-touch.has-hover ? Theme.accent-soft
                                : index == root.current-index ? #f2f5fb : Theme.surface;
                            Image {
                                x: 10px;
                                y: (parent.height - self.height) / 2;
                                source: choice.icon;
                                width: 24px;
                                height: 24px;
                                image-fit: contain;
                            }
                            Text {
                                x: 43px;
                                y: 0px;
                                width: parent.width - 53px;
                                height: parent.height;
                                text: choice.name;
                                color: Theme.text;
                                font-size: 13px;
                                font-family: root.font;
                                horizontal-alignment: left;
                                vertical-alignment: center;
                                overflow: elide;
                            }
                            row-touch := TouchArea {
                                width: 100%;
                                height: 100%;
                                clicked => {
                                    root.current-index = index;
                                    app-popup.close();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    export component AppWindow inherits Window {
        title: root.window-title;
        width: 460px;
        height: 660px;
        background: Theme.bg;
        // Brand the title bar / taskbar with the same icon shown in the header.
        icon: root.header-icon;

        in-out property <string> ui-font: "Microsoft YaHei";
        in-out property <string> window-title: "";
        in-out property <string> subtitle-text: "";
        in-out property <string> status-text: "—";
        in-out property <string> collections-text: "—";
        in-out property <string> presses-text: "0";
        in-out property <string> last-text: "—";
        // Raw running counters, kept on the component so event handling stays
        // event-driven (no Rust-side shared state captured into a `Send`
        // closure). `presses-text`/`status-text`/etc. stay the display strings.
        in-out property <int> presses-count: 0;
        in-out property <int> status-count: 0;
        in-out property <int> action-index: 0;
        in-out property <[AppChoice]> action-model;
        in-out property <string> action-filter: "";
        in-out property <int> action-visible-count: 1;
        in-out property <string> custom-command: "";
        in-out property <string> custom-args: "";
        in-out property <bool> enabled: true;
        in-out property <bool> confirm-beep: true;
        in-out property <bool> start-windows: false;
        in-out property <bool> minimize: false;
        in-out property <int> lang-index: 0;
        in-out property <[string]> lang-model;
        in-out property <string> lang-button-label: "";
        in-out property <string> exit-button-label: "";
        in-out property <bool> feedback-visible: false;
        in-out property <string> feedback-text: "";
        in-out property <string> hint-text: "";
        in-out property <image> header-icon;

        in-out property <string> action-label: "";
        in-out property <string> app-list-hint: "";
        in-out property <string> search-placeholder: "";
        in-out property <string> custom-command-label: "";
        in-out property <string> custom-args-label: "";
        in-out property <string> opt-enabled: "";
        in-out property <string> opt-beep: "";
        in-out property <string> opt-autostart: "";
        in-out property <string> opt-minimize: "";
        in-out property <string> btn-save: "";
        in-out property <string> btn-test: "";

        callback save();
        callback test();
        callback filter-changed(string);
        callback lang-chosen(int);
        callback quit-app();

        VerticalLayout {
            spacing: 0px;

            // --- native-style menu strip directly below the title bar ---
            Rectangle {
                height: 31px;
                background: #f8f8f8;
                HorizontalLayout {
                    padding-left: 4px;
                    padding-right: 4px;
                    padding-top: 1px;
                    padding-bottom: 2px;
                    spacing: 2px;
                    MenuItem {
                        text: root.exit-button-label;
                        font: root.ui-font;
                        width: 62px;
                        horizontal-stretch: 0;
                        clicked => { root.quit-app(); }
                    }
                    MenuItem {
                        text: root.lang-button-label;
                        font: root.ui-font;
                        icon: @image-url("../public/language-icon.svg");
                        show-icon: true;
                        width: 88px;
                        horizontal-stretch: 0;
                        clicked => { lang_popup.show(); }
                    }
                    Rectangle { horizontal-stretch: 1; }
                }
                Rectangle {
                    y: parent.height - 1px;
                    height: 1px;
                    width: parent.width;
                    background: #d9dce1;
                }
            }

            VerticalLayout {
                padding: 18px;
                spacing: 14px;

                // --- header: brand icon + title ---
                HorizontalLayout {
                spacing: 12px;
                Image {
                    source: root.header-icon;
                    width: 38px;
                    height: 38px;
                    image-fit: contain;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                }
                VerticalLayout {
                    spacing: 2px;
                    horizontal-stretch: 1;
                    Text {
                        text: "Microsoft Audio Dock";
                        font-size: 18px;
                        font-weight: 700;
                        color: Theme.text;
                        font-family: root.ui-font;
                        overflow: elide;
                    }
                    Text {
                        text: root.subtitle-text;
                        font-size: 12px;
                        color: Theme.text2;
                        font-family: root.ui-font;
                        overflow: elide;
                    }
                }
                }

                // --- live status ---
                Card {
                VerticalLayout {
                    spacing: 6px;
                    Text { text: root.status-text; color: Theme.text; font-weight: 600; font-family: root.ui-font; }
                    Text { text: root.collections-text; color: Theme.text2; font-size: 12px; font-family: root.ui-font; }
                    Text { text: root.presses-text; color: Theme.text; font-size: 12px; font-family: root.ui-font; }
                    Text { text: root.last-text; color: Theme.text2; font-size: 12px; font-family: root.ui-font; }
                }
                }

                // --- action ---
                Card {
                VerticalLayout {
                    spacing: 8px;
                    Text { text: root.action-label; font-weight: 700; color: Theme.text; font-family: root.ui-font; }
                    AppSelector {
                        model: root.action-model;
                        current-index <=> root.action-index;
                        search-text <=> root.action-filter;
                        search-placeholder: root.search-placeholder;
                        visible-count: root.action-visible-count;
                        font: root.ui-font;
                        filter-changed(text) => { root.filter-changed(text); }
                    }
                    if root.action-index == 0 : VerticalLayout {
                        spacing: 6px;
                        Text { text: root.custom-command-label; color: Theme.text2; font-size: 12px; font-family: root.ui-font; }
                        LineEdit { text <=> root.custom-command; }
                        Text { text: root.custom-args-label; color: Theme.text2; font-size: 12px; font-family: root.ui-font; }
                        LineEdit { text <=> root.custom-args; }
                    }
                    Text { text: root.app-list-hint; color: Theme.text2; font-size: 11px; font-family: root.ui-font; }
                }
                }

                // --- options ---
                Card {
                VerticalLayout {
                    spacing: 4px;
                    CheckBox { text: root.opt-enabled; checked <=> root.enabled; }
                    CheckBox { text: root.opt-beep; checked <=> root.confirm-beep; }
                    CheckBox { text: root.opt-autostart; checked <=> root.start-windows; }
                    CheckBox { text: root.opt-minimize; checked <=> root.minimize; }
                }
                }

                // --- actions ---
                HorizontalLayout {
                spacing: 10px;
                PrimaryButton {
                    text: root.btn-save;
                    font: root.ui-font;
                    horizontal-stretch: 1;
                    clicked => { root.save(); }
                }
                SecondaryButton {
                    text: root.btn-test;
                    font: root.ui-font;
                    horizontal-stretch: 1;
                    clicked => { root.test(); }
                }
                }

                // --- feedback / hint ---
                Rectangle {
                height: 44px;
                border-radius: 10px;
                background: root.feedback-visible ? Theme.accent-soft : #f1f3f7;
                Text {
                    text: root.feedback-visible ? root.feedback-text : root.hint-text;
                    color: root.feedback-visible ? Theme.accent : Theme.text3;
                    font-size: 11px;
                    wrap: word-wrap;
                    vertical-alignment: center;
                    x: 12px;
                    width: parent.width - 24px;
                    font-family: root.ui-font;
                }
                }
            }
        }

        // Language dropdown, anchored below the menu-bar language button.
        lang_popup := PopupWindow {
            x: 68px;
            y: 31px;
            width: 150px;
            height: 76px;
            close-policy: close-on-click-outside;

            Rectangle {
                background: Theme.surface;
                border-radius: 12px;
                border-width: 1px;
                border-color: Theme.border;
                VerticalLayout {
                    padding: 6px;
                    spacing: 4px;

                    for name[index] in root.lang-model : Rectangle {
                        height: 32px;
                        border-radius: 8px;
                        background: index == root.lang-index ? Theme.accent-soft
                                    : touch-area.has-hover ? Theme.bg : #00000000;
                        Text {
                            text: name;
                            vertical-alignment: center;
                            horizontal-alignment: left;
                            x: 12px;
                            width: parent.width - 16px;
                            color: Theme.text;
                            font-family: root.ui-font;
                        }
                        touch-area := TouchArea {
                            width: 100%;
                            height: 100%;
                            clicked => {
                                root.lang-chosen(index);
                                lang_popup.close();
                            }
                        }
                    }
                }
            }
        }
    }

}

/// Running counters (press count, device status, last trigger time) live in
/// Slint `int`/`string` properties so the event handler can stay fully
/// event-driven and avoid capturing any `!Send` Rust state into the `Send`
/// closure passed to `slint::invoke_from_event_loop`.
///
/// `start_minimized` comes from `main` (persisted setting or the `--minimized`
/// switch on the login entry) and decides whether the settings window is shown
/// at all; the resident monitor and the tray icon start either way.
pub fn run(config: Arc<Mutex<Config>>, initial_lang: Lang, start_minimized: bool) {
    // Fix CJK tofu globally (must happen before the first Slint window, which
    // is when the font collection is created).
    set_default_cjk_font();

    let mut apps = match crate::installed_apps::list() {
        Ok(apps) => apps,
        Err(error) => {
            platform::alert(&format!(
                "{} {error}",
                i18n::t(initial_lang, "app_list_fail")
            ));
            Vec::new()
        }
    };

    // Keep a previously saved item visible if it temporarily disappeared from
    // AppsFolder. This avoids silently changing the configured target merely by
    // opening and saving the settings window.
    {
        let c = config.lock().unwrap();
        if !c.action.app_target.is_empty()
            && !apps
                .iter()
                .any(|app| app.target.eq_ignore_ascii_case(&c.action.app_target))
        {
            apps.push(crate::installed_apps::InstalledApp {
                name: if c.action.app_name.is_empty() {
                    c.action.app_target.clone()
                } else {
                    c.action.app_name.clone()
                },
                target: c.action.app_target.clone(),
                registered: false,
                icon_rgba: Vec::new(),
            });
        }
    }
    let apps = Rc::new(apps);

    let ui = AppWindow::new().unwrap();

    // Fix CJK tofu: point the default font family at a CJK-capable system font.
    ui.set_ui_font(default_ui_font().into());

    // Branding: set the in-app header icon from the embedded asset. The window
    // (title-bar / taskbar) icon is bound to the same image via `icon:` in the
    // .slint definition. Use the small, pre-anti-aliased asset so the 26x26
    // header (and 16x16 menu) render without jaggies. No-op if unavailable.
    if let Some(icon_path) = crate::platform::header_icon_path() {
        if let Ok(img) = slint::Image::load_from_path(&icon_path) {
            ui.set_header_icon(img);
        }
    }

    let lang = Rc::new(Cell::new(initial_lang));
    let feedback_timer = Rc::new(Timer::default());

    // Seed control values from the persisted config.
    {
        let c = config.lock().unwrap();
        let selected = if c.action.preset_id == "custom" {
            0
        } else {
            crate::installed_apps::selected_index(
                &apps,
                &c.action.app_target,
                &c.action.app_name,
                &c.action.preset_id,
            )
            .map(|index| index as i32 + 1)
            .unwrap_or(0)
        };
        ui.set_action_index(selected);
        ui.set_custom_command(c.action.command.clone().into());
        ui.set_custom_args(c.action.arguments.clone().into());
        ui.set_enabled(c.settings.enabled);
        ui.set_confirm_beep(c.settings.play_confirmation_beep);
        ui.set_start_windows(autostart::is_enabled());
        ui.set_minimize(c.settings.minimize_to_tray);
    }

    select_language(&ui, &lang, &config, &apps, initial_lang);

    // --- live application-name filtering -----------------------------------
    {
        let uiw = ui.as_weak();
        let lang2 = lang.clone();
        let apps2 = apps.clone();
        ui.on_filter_changed(move |query| {
            if let Some(ui) = uiw.upgrade() {
                apply_action_filter(&ui, lang2.get(), &apps2, query.as_str());
            }
        });
    }

    // The title-bar X always hides the window. Full process exit is deliberately
    // available only from the menu bar, so closing can never stop monitoring by
    // accident or because of an obsolete remembered close choice.
    {
        let uiw = ui.as_weak();
        ui.window().on_close_requested(move || {
            if let Some(ui) = uiw.upgrade() {
                ui.window().hide().ok();
            }
            CloseRequestResponse::KeepWindowShown
        });
    }

    // --- menu-bar exit: stop the resident monitor and the Slint event loop ---
    {
        ui.on_quit_app(move || {
            platform::request_quit();
            slint::quit_event_loop().ok();
        });
    }

    // --- language dropdown selection ----------------------------------------
    {
        let uiw = ui.as_weak();
        let lang2 = lang.clone();
        let cfg = config.clone();
        let apps2 = apps.clone();
        ui.on_lang_chosen(move |idx| {
            let new_lang = if idx == 0 { Lang::Zh } else { Lang::En };
            if let Some(ui) = uiw.upgrade() {
                select_language(&ui, &lang2, &cfg, &apps2, new_lang);
            }
        });
    }

    // --- save ---
    {
        let cfg = config.clone();
        let uiw = ui.as_weak();
        let lang2 = lang.clone();
        let ft = feedback_timer.clone();
        let apps2 = apps.clone();
        ui.on_save(move || {
            let ui = uiw.upgrade().unwrap();
            let mut c = cfg.lock().unwrap();
            c.version = 2;
            if ui.get_action_index() == 0 {
                c.action.app_name.clear();
                c.action.app_target.clear();
                c.action.preset_id = "custom".to_string();
                c.action.command = ui.get_custom_command().to_string();
                c.action.arguments = ui.get_custom_args().to_string();
            } else {
                let Some(app) = selected_app(&apps2, ui.get_action_index()) else {
                    platform::alert(i18n::t(lang2.get(), "no_app_selected"));
                    return;
                };
                c.action.app_name = app.name.clone();
                c.action.app_target = app.target.clone();
                c.action.preset_id = "registered_app".to_string();
                c.action.command.clear();
                c.action.arguments.clear();
            }
            let start_win = ui.get_start_windows();
            let start_minimized = ui.get_minimize();
            c.settings.enabled = ui.get_enabled();
            c.settings.play_confirmation_beep = ui.get_confirm_beep();
            c.settings.minimize_to_tray = start_minimized;
            c.settings.start_with_windows = start_win;
            c.language = lang2.get().code().to_string();
            drop(c);

            // The login entry carries `--minimized` too, so a silent start does
            // not depend on the config file being readable at sign-in.
            autostart::set_enabled(start_win, start_minimized);
            match cfg.lock().unwrap().save() {
                Ok(_) => show_feedback(&ui, &ft, i18n::t(lang2.get(), "saved")),
                Err(e) => platform::alert(&format!("{} {e}", i18n::t(lang2.get(), "save_fail"))),
            }
        });
    }

    // --- test ---
    {
        let cfg = config.clone();
        let uiw = ui.as_weak();
        let lang2 = lang.clone();
        let ft = feedback_timer.clone();
        let apps2 = apps.clone();
        ui.on_test(move || {
            let ui = uiw.upgrade().unwrap();
            let c = cfg.lock().unwrap();
            let mut temp = c.clone();
            if ui.get_action_index() == 0 {
                temp.action.app_name.clear();
                temp.action.app_target.clear();
                temp.action.preset_id = "custom".to_string();
                temp.action.command = ui.get_custom_command().to_string();
                temp.action.arguments = ui.get_custom_args().to_string();
            } else {
                let Some(app) = selected_app(&apps2, ui.get_action_index()) else {
                    platform::alert(i18n::t(lang2.get(), "no_app_selected"));
                    return;
                };
                temp.action.app_name = app.name.clone();
                temp.action.app_target = app.target.clone();
                temp.action.preset_id = "registered_app".to_string();
            }
            drop(c);

            match actions::execute(&temp) {
                Ok(_) => show_feedback(&ui, &ft, i18n::t(lang2.get(), "test_ok")),
                Err(e) => platform::alert(&format!("{} {e}", i18n::t(lang2.get(), "test_fail"))),
            }
        });
    }

    // --- deliver monitor events to the UI thread, event-driven ---------------
    // The resident monitor invokes `on_event` on its own thread for every
    // `MonitorEvent`. We buffer the event in an `mpsc` channel (so nothing is
    // lost if the Slint event loop is not ready yet — `invoke_from_event_loop`
    // returns `Err` before the loop starts) and then ask the Slint event loop
    // to drain the channel on the UI thread via `invoke_from_event_loop`.
    //
    // This replaces the old 200 ms `Timer::Repeated` poll: with no active timer,
    // the winit event loop can block in its message wait and the UI thread idles
    // at ~0% CPU when nothing happens.
    let (tx, rx) = mpsc::channel::<MonitorEvent>();
    let rx = Arc::new(Mutex::new(rx));
    let ui_weak = ui.as_weak();
    let on_event = {
        let tx = tx;
        let rx = rx.clone();
        let ui_weak = ui_weak.clone();
        let config = config.clone();
        move |ev: MonitorEvent| {
            // Buffer first so the event survives a not-yet-running loop.
            let _ = tx.send(ev);
            let rx = rx.clone();
            let ui_weak = ui_weak.clone();
            let config = config.clone();
            let _ = invoke_from_event_loop(move || {
                pump_events(&rx, &ui_weak, &config);
            });
        }
    };
    platform::start_monitor(on_event, config.clone());

    // Flush any events that arrived before the event loop was ready (notably the
    // monitor's initial device-status broadcast sent during startup). A one-shot
    // timer fires exactly once after the loop starts and then disarms, so it does
    // not keep the loop awake the way the old repeating timer did.
    {
        let rx = rx.clone();
        let ui_weak = ui_weak.clone();
        let config = config.clone();
        let flush = Timer::default();
        flush.start(TimerMode::SingleShot, Duration::from_millis(0), move || {
            pump_events(&rx, &ui_weak, &config);
        });
    }

    // Run the event loop "until quit": hiding the window (minimize-to-tray)
    // must NOT terminate the app — our Win32 tray keeps it alive. Only an
    // explicit menu-bar Exit ends the loop.
    //
    // With "start minimized to the tray" the window is never shown in the first
    // place. The monitor and the tray icon are already up by now, so the app
    // comes up resident-only and the user opens the settings window by
    // double-clicking the tray icon. `run_event_loop_until_quit` is documented
    // for exactly this daemon-style case (keeps running with nothing visible).
    let outcome = if start_minimized {
        slint::run_event_loop_until_quit()
    } else {
        ui.window()
            .show()
            .and_then(|()| slint::run_event_loop_until_quit())
    };
    if let Err(error) = outcome {
        handle_backend_failure(&error, ui_lang(&config));
    }
}

/// Slint's default renderer needs a working OpenGL driver. Machines without one
/// (remote-desktop sessions, VMs, very old GPUs) fail *inside* `show()` /
/// `run_event_loop_until_quit`, which used to be swallowed with `.ok()` — the
/// process then returned from `main` with exit code 0 and, having no console,
/// simply vanished with no explanation at all.
///
/// The software renderer is compiled in, so try to recover by re-running
/// ourselves with it forced on, and only report the failure when that is not an
/// option any more.
fn handle_backend_failure(error: &slint::PlatformError, lang: Lang) {
    if relaunch_with_software_renderer() {
        return;
    }
    platform::alert(&format!("{} {error}", i18n::t(lang, "render_fail")));
}

/// The Slint backend/renderer combination that needs no GPU driver.
const SOFTWARE_BACKEND: &str = "winit-software";

/// Re-executes this process with the software renderer forced on, handing over
/// the tray icon and the single-instance slot first.
///
/// Returns false — meaning "report the error instead" — when a backend was
/// already pinned (so a still-broken relaunch can never loop) or when the
/// replacement process could not be started.
fn relaunch_with_software_renderer() -> bool {
    if std::env::var_os("SLINT_BACKEND").is_some() {
        return false;
    }
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    // Stop the monitor thread first: it removes its tray icon and destroys its
    // window before returning, so the replacement process does not end up
    // sitting next to a dead icon. Then drop the single-instance mutex, or the
    // replacement would report itself as "already running".
    platform::request_quit();
    platform::release_single_instance();
    std::process::Command::new(exe)
        .args(std::env::args_os().skip(1))
        .env("SLINT_BACKEND", SOFTWARE_BACKEND)
        .spawn()
        .is_ok()
}

/// Applies `new_lang` everywhere: persists the choice and re-localizes all
/// strings plus the application picker model without changing its selection.
fn select_language(
    ui: &AppWindow,
    lang: &Rc<Cell<Lang>>,
    config: &Arc<Mutex<Config>>,
    apps: &[crate::installed_apps::InstalledApp],
    new_lang: Lang,
) {
    lang.set(new_lang);
    {
        let mut c = config.lock().unwrap();
        c.language = new_lang.code().to_string();
        let _ = c.save();
    }
    relocalize(ui, new_lang, apps);
    ui.set_lang_index(if new_lang == Lang::Zh { 0 } else { 1 });
}

/// Re-applies every localized string + the application picker and language menu
/// models for `lang`. Running counters are read back from the Slint properties
/// (`presses-count`, `status-count`, `last-text`) so a language switch repaints
/// them without needing any Rust-side shared state.
fn relocalize(ui: &AppWindow, lang: Lang, apps: &[crate::installed_apps::InstalledApp]) {
    ui.set_window_title(i18n::t(lang, "app_title").into());
    ui.set_subtitle_text(i18n::t(lang, "subtitle").into());

    let status = if ui.get_status_count() > 0 {
        format!(
            "{}{}",
            i18n::t(lang, "status_prefix"),
            i18n::t(lang, "status_connected")
        )
    } else {
        format!(
            "{}{}",
            i18n::t(lang, "status_prefix"),
            i18n::t(lang, "status_disconnected")
        )
    };
    ui.set_status_text(status.into());
    ui.set_collections_text(
        format!("{}{}", i18n::t(lang, "collections"), ui.get_status_count()).into(),
    );
    ui.set_presses_text(format!("{}{}", i18n::t(lang, "presses"), ui.get_presses_count()).into());
    ui.set_last_text(ui.get_last_text());

    ui.set_action_label(i18n::t(lang, "action_label").into());
    ui.set_action_filter("".into());
    apply_action_filter(ui, lang, apps, "");
    let registered_count = apps.iter().filter(|app| app.registered).count();
    ui.set_app_list_hint(
        format!("{}{}", i18n::t(lang, "apps_folder_count"), registered_count).into(),
    );
    ui.set_custom_command_label(i18n::t(lang, "custom_command").into());
    ui.set_custom_args_label(i18n::t(lang, "custom_args").into());
    ui.set_search_placeholder(i18n::t(lang, "search_apps").into());
    ui.set_opt_enabled(i18n::t(lang, "opt_enabled").into());
    ui.set_opt_beep(i18n::t(lang, "opt_beep").into());
    ui.set_opt_autostart(i18n::t(lang, "opt_autostart").into());
    ui.set_opt_minimize(i18n::t(lang, "opt_minimize").into());
    ui.set_lang_button_label(i18n::t(lang, "lang_button").into());
    ui.set_exit_button_label(i18n::t(lang, "menu_exit").into());

    // Language menu lists native names so it is readable in either language.
    let lang_model = Rc::new(VecModel::from(vec![
        Lang::Zh.label().to_string().into(),
        Lang::En.label().to_string().into(),
    ]));
    ui.set_lang_model(lang_model.into());

    ui.set_btn_save(i18n::t(lang, "btn_save").into());
    ui.set_btn_test(i18n::t(lang, "btn_test").into());
    ui.set_hint_text(i18n::t(lang, "hint_tray").into());
}

fn apply_action_filter(
    ui: &AppWindow,
    lang: Lang,
    apps: &[crate::installed_apps::InstalledApp],
    query: &str,
) {
    let mut visible_count = 1;
    let mut action_model = vec![AppChoice {
        name: i18n::t(lang, "preset_custom").into(),
        icon: ui.get_header_icon(),
        visible: true,
    }];
    action_model.extend(apps.iter().map(|app| {
        let visible = app_matches_filter(&app.name, query);
        if visible {
            visible_count += 1;
        }
        AppChoice {
            name: if app.registered {
                app.name.clone().into()
            } else {
                format!(
                    "{} ({})",
                    app.name,
                    i18n::t(lang, "app_no_longer_registered")
                )
                .into()
            },
            icon: app_icon(app),
            visible,
        }
    }));
    ui.set_action_visible_count(visible_count);
    ui.set_action_model(Rc::new(VecModel::from(action_model)).into());
}

fn app_matches_filter(name: &str, query: &str) -> bool {
    let normalized_query = query.trim().to_lowercase();
    normalized_query.is_empty() || name.to_lowercase().contains(&normalized_query)
}

fn selected_app(
    apps: &[crate::installed_apps::InstalledApp],
    index: i32,
) -> Option<&crate::installed_apps::InstalledApp> {
    usize::try_from(index - 1)
        .ok()
        .and_then(|index| apps.get(index))
}

fn app_icon(app: &crate::installed_apps::InstalledApp) -> slint::Image {
    const ICON_SIZE: u32 = 32;
    if app.icon_rgba.len() != (ICON_SIZE * ICON_SIZE * 4) as usize {
        return slint::Image::default();
    }
    let buffer =
        SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&app.icon_rgba, ICON_SIZE, ICON_SIZE);
    slint::Image::from_rgba8_premultiplied(buffer)
}

/// Shows a transient confirmation message that auto-clears after ~2s.
fn show_feedback(ui: &AppWindow, timer: &Rc<Timer>, msg: &str) {
    let uiw = ui.as_weak();
    ui.set_feedback_text(msg.into());
    ui.set_feedback_visible(true);
    let timer = timer.clone();
    timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(2200),
        move || {
            if let Some(ui) = uiw.upgrade() {
                ui.set_feedback_visible(false);
                ui.set_feedback_text("".into());
            }
        },
    );
}

/// Candidate CJK-capable fonts: a single-face `.ttf`/`.otf` is preferred over a
/// `.ttc` collection (more reliably parsed by the font loader). First existing
/// file wins.
const FONT_CANDIDATES: &[(&str, &str)] = &[
    ("C:\\Windows\\Fonts\\simhei.ttf", "SimHei"),
    (
        "C:\\Windows\\Fonts\\NotoSansCJKsc-Regular.otf",
        "Noto Sans CJK SC",
    ),
    ("C:\\Windows\\Fonts\\msyh.ttc", "Microsoft YaHei"),
    ("C:\\Windows\\Fonts\\simsun.ttc", "SimSun"),
];

/// Picks a CJK-capable system font *family name* so Chinese renders instead of
/// tofu. Used as the per-`Text` `font-family` binding (a belt-and-suspenders
/// measure alongside the global `SLINT_DEFAULT_FONT` set in `set_default_cjk_font`).
fn default_ui_font() -> String {
    // macOS: the system font stack (San Francisco + PingFang fallback) already
    // renders CJK, so leave the family at Slint's default.
    if cfg!(target_os = "macos") {
        return String::new();
    }
    for (file, family) in FONT_CANDIDATES {
        if std::path::Path::new(file).exists() {
            return family.to_string();
        }
    }
    "Microsoft YaHei".to_string()
}

/// First existing CJK font *file path*, used to set the `SLINT_DEFAULT_FONT`
/// env var (read once when the Slint font collection is created). Returns None
/// when no candidate is present, letting Slint fall back to system fonts.
fn default_ui_font_path() -> Option<String> {
    for (file, _) in FONT_CANDIDATES {
        if std::path::Path::new(file).exists() {
            return Some(file.to_string());
        }
    }
    None
}

/// Makes the global default UI font a CJK-capable one so *all* text — including
/// `CheckBox`/`ComboBox` widget labels and dropdown items — renders Chinese
/// instead of tofu. Must run before the first Slint window is created.
fn set_default_cjk_font() {
    if cfg!(target_os = "macos") {
        return;
    }
    if let Some(path) = default_ui_font_path() {
        std::env::set_var("SLINT_DEFAULT_FONT", path);
    }
}

fn now_hms() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

/// Drains every buffered `MonitorEvent` and applies it on the UI thread. Called
/// from `slint::invoke_from_event_loop` (per event) and once from a one-shot
/// timer (to flush events that arrived before the loop was ready). All state it
/// touches is `Send` (a `Weak<AppWindow>`, an `Arc<Mutex<Receiver>>` buffer, and
/// `Arc<Mutex<Config>>`), so this is safe to run inside the `Send` closure.
fn pump_events(
    rx: &Arc<Mutex<Receiver<MonitorEvent>>>,
    ui_weak: &Weak<AppWindow>,
    config: &Arc<Mutex<Config>>,
) {
    let Some(ui) = ui_weak.upgrade() else {
        return;
    };
    loop {
        let ev = match rx.lock().unwrap().try_recv() {
            Ok(ev) => ev,
            Err(_) => break,
        };
        apply_event(&ui, config, ev);
    }
}

/// Applies a single `MonitorEvent` to the UI. Running counters live in Slint
/// `int`/`string` properties so this function needs no `!Send` Rust state.
fn apply_event(ui: &AppWindow, config: &Arc<Mutex<Config>>, ev: MonitorEvent) {
    let lang = ui_lang(config);
    match ev {
        MonitorEvent::Press => {
            let n = ui.get_presses_count() + 1;
            ui.set_presses_count(n);
            ui.set_presses_text(format!("{}{}", i18n::t(lang, "presses"), n).into());
            let last = now_hms();
            ui.set_last_text(format!("{}{}", i18n::t(lang, "last"), last).into());
        }
        MonitorEvent::Status(n) => {
            ui.set_status_count(n as i32);
            let status = if n > 0 {
                format!(
                    "{}{}",
                    i18n::t(lang, "status_prefix"),
                    i18n::t(lang, "status_connected")
                )
            } else {
                format!(
                    "{}{}",
                    i18n::t(lang, "status_prefix"),
                    i18n::t(lang, "status_disconnected")
                )
            };
            ui.set_status_text(status.into());
            ui.set_collections_text(format!("{}{}", i18n::t(lang, "collections"), n).into());
        }
        MonitorEvent::TrayShow => {
            let _ = ui.show();
        }
        MonitorEvent::Quit => {
            platform::request_quit();
            let _ = slint::quit_event_loop();
        }
    }
}

/// Resolves the active UI language from the persisted config (single source of
/// truth), used by the event handler so it does not need to capture the
/// UI-thread-only `Rc<Cell<Lang>>`.
fn ui_lang(config: &Arc<Mutex<Config>>) -> Lang {
    Lang::resolve(&config.lock().unwrap().language)
}

#[cfg(test)]
mod tests {
    use super::app_matches_filter;

    #[test]
    fn app_filter_is_case_insensitive_and_trims_input() {
        assert!(app_matches_filter("Microsoft Teams", "  TEAMS "));
        assert!(!app_matches_filter("Microsoft Teams", "Zoom"));
    }

    #[test]
    fn empty_filter_keeps_every_app_visible() {
        assert!(app_matches_filter("计算器", "  "));
    }
}
