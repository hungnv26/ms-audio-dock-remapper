#![windows_subsystem = "windows"]

mod actions;
mod autostart;
mod config;
mod i18n;
mod installed_apps;
mod platform;
mod presets;
mod ui;

use std::sync::{Arc, Mutex};

fn main() {
    // Declare DPI awareness first, so every window (incl. the "already running"
    // MessageBox shown below) is crisp on high-DPI displays.
    platform::set_dpi_aware();

    let config = Arc::new(Mutex::new(config::Config::load()));
    let lang = i18n::Lang::resolve(&config.lock().unwrap().language);

    // "Start minimized to the tray": either the persisted setting or the
    // `--minimized` switch carried by the login entry. Resolved here, before the
    // UI is built, so a silent start never shows the window at all instead of
    // showing and then hiding it (which flashes on screen at sign-in).
    let start_minimized =
        config.lock().unwrap().settings.minimize_to_tray || autostart::started_minimized();

    // Single running instance (named mutex on Windows, flock on macOS).
    if !platform::ensure_single_instance() {
        platform::alert(i18n::t(lang, "single_instance"));
        return;
    }

    // Slint UI event loop (blocks until quit). The UI wires up the OS-specific
    // resident monitor (raw input + tray) itself, forwarding events into the
    // Slint event loop so the app stays event-driven (no busy polling).
    ui::run(config, lang, start_minimized);
}
