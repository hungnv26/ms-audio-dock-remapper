//! Enumerates and launches installed applications.
//!
//! Windows: items registered in the AppsFolder, the same virtual shell folder
//! opened by `shell:AppsFolder`. Its children cover classic desktop shortcuts
//! as well as packaged/UWP apps, so the stored target is a Shell parsing name
//! rather than an executable path.
//!
//! macOS: `.app` bundles in the standard Applications folders; the stored
//! target is the bundle path, launched through `open`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApp {
    pub name: String,
    pub target: String,
    pub registered: bool,
    /// 32x32 premultiplied RGBA pixels extracted from the Shell item.
    pub icon_rgba: Vec<u8>,
}

#[cfg(windows)]
pub fn list() -> Result<Vec<InstalledApp>, String> {
    use windows::Win32::System::Com::{
        CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, SHGetKnownFolderItem,
        KF_FLAG_DEFAULT, SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_NORMALDISPLAY,
    };

    struct ComGuard(bool);
    impl Drop for ComGuard {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }

    unsafe fn shell_string(
        item: &IShellItem,
        kind: windows::Win32::UI::Shell::SIGDN,
    ) -> Result<String, String> {
        let raw = item.GetDisplayName(kind).map_err(|e| e.to_string())?;
        let value = raw.to_string().map_err(|e| e.to_string());
        CoTaskMemFree(Some(raw.0.cast()));
        value
    }

    unsafe {
        // S_OK and S_FALSE both mean this call owns a matching CoUninitialize.
        // If COM was initialized earlier in the same apartment, S_FALSE is
        // returned and is still considered successful by `is_ok()`.
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let _guard = ComGuard(init.is_ok());
        if init.is_err() {
            return Err(format!(
                "COM initialization failed: 0x{:08X}",
                init.0 as u32
            ));
        }

        let folder: IShellItem = SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None)
            .map_err(|e| e.to_string())?;
        let enumerator: IEnumShellItems = folder
            .BindToHandler(None, &BHID_EnumItems)
            .map_err(|e| e.to_string())?;

        let mut apps = Vec::new();
        loop {
            let mut slot = [None];
            let mut fetched = 0;
            enumerator
                .Next(&mut slot, Some(&mut fetched))
                .map_err(|e| e.to_string())?;
            if fetched == 0 {
                break;
            }
            let Some(item) = slot[0].take() else {
                continue;
            };
            let name = match shell_string(&item, SIGDN_NORMALDISPLAY) {
                Ok(value) if !value.trim().is_empty() => value,
                _ => continue,
            };
            let target = match shell_string(&item, SIGDN_DESKTOPABSOLUTEPARSING) {
                Ok(value) if !value.trim().is_empty() => value,
                _ => continue,
            };
            apps.push(InstalledApp {
                name,
                target,
                registered: true,
                icon_rgba: shell_icon(&item),
            });
        }

        apps.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.target.cmp(&b.target))
        });
        apps.dedup_by(|a, b| a.target.eq_ignore_ascii_case(&b.target));
        Ok(apps)
    }
}

#[cfg(windows)]
unsafe fn shell_icon(item: &windows::Win32::UI::Shell::IShellItem) -> Vec<u8> {
    use windows::core::Interface;
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
    };
    use windows::Win32::UI::Shell::{IShellItemImageFactory, SIIGBF_ICONONLY};

    const ICON_SIZE: u32 = 32;
    let Ok(factory) = item.cast::<IShellItemImageFactory>() else {
        return Vec::new();
    };
    let Ok(bitmap) = factory.GetImage(
        SIZE {
            cx: ICON_SIZE as i32,
            cy: ICON_SIZE as i32,
        },
        SIIGBF_ICONONLY,
    ) else {
        return Vec::new();
    };

    let dc = CreateCompatibleDC(None);
    if dc.0.is_null() {
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        return Vec::new();
    }

    // A negative height requests top-down rows, matching Slint's buffer layout.
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: ICON_SIZE as i32,
            biHeight: -(ICON_SIZE as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize];
    let rows = GetDIBits(
        dc,
        bitmap,
        0,
        ICON_SIZE,
        Some(pixels.as_mut_ptr().cast()),
        &mut info,
        DIB_RGB_COLORS,
    );
    let _ = DeleteDC(dc);
    let _ = DeleteObject(HGDIOBJ(bitmap.0));
    if rows != ICON_SIZE as i32 {
        return Vec::new();
    }

    // DIB pixels are BGRA; Slint consumes premultiplied RGBA.
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    pixels
}

#[cfg(target_os = "macos")]
pub fn list() -> Result<Vec<InstalledApp>, String> {
    use std::path::PathBuf;

    let mut roots: Vec<PathBuf> = [
        "/Applications",
        "/Applications/Utilities",
        "/System/Applications",
        "/System/Applications/Utilities",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join("Applications"));
    }

    let mut apps = Vec::new();
    let mut readable = 0;
    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        readable += 1;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("app") {
                continue;
            }
            if let Some(app) = macos::read_bundle(&path) {
                apps.push(app);
            }
        }
    }
    if readable == 0 {
        return Err("no Applications folder is readable".into());
    }

    apps.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.target.cmp(&b.target))
    });
    apps.dedup_by(|a, b| a.target.eq_ignore_ascii_case(&b.target));
    Ok(apps)
}

#[cfg(target_os = "macos")]
mod macos {
    use super::InstalledApp;
    use std::path::Path;

    const ICON_SIZE: u32 = 32;

    /// Reads one `.app` bundle. `None` for bundles that are not launchable
    /// targets (unreadable Info.plist, background-only helpers).
    pub fn read_bundle(path: &Path) -> Option<InstalledApp> {
        let info = plist::Value::from_file(path.join("Contents/Info.plist")).ok()?;
        let dict = info.into_dictionary()?;
        if dict
            .get("LSBackgroundOnly")
            .and_then(plist::Value::as_boolean)
            == Some(true)
        {
            return None;
        }
        // The bundle's file name is what Finder shows (modulo localization)
        // and is more recognisable than CFBundleName, which is often short.
        let name = path.file_stem()?.to_string_lossy().into_owned();
        let icon_file = dict
            .get("CFBundleIconFile")
            .and_then(plist::Value::as_string)
            .map(str::to_owned)
            .unwrap_or_else(|| "AppIcon".into());
        Some(InstalledApp {
            name,
            target: path.to_string_lossy().into_owned(),
            registered: true,
            icon_rgba: bundle_icon(path, &icon_file),
        })
    }

    /// 32x32 premultiplied RGBA from the bundle's `.icns`, or empty when the
    /// bundle keeps its icon in an asset catalog or uses an undecodable
    /// (JPEG 2000) payload for every usable size.
    fn bundle_icon(bundle: &Path, icon_file: &str) -> Vec<u8> {
        let mut file = icon_file.to_owned();
        if !file.to_ascii_lowercase().ends_with(".icns") {
            file.push_str(".icns");
        }
        let Ok(f) = std::fs::File::open(bundle.join("Contents/Resources").join(file)) else {
            return Vec::new();
        };
        let Ok(family) = icns::IconFamily::read(std::io::BufReader::new(f)) else {
            return Vec::new();
        };
        // The smallest square variant of at least 32px decodes fastest and
        // downsamples cleanly.
        let mut types = family.available_icons();
        types.retain(|t| {
            !t.is_mask() && t.pixel_width() >= ICON_SIZE && t.pixel_width() == t.pixel_height()
        });
        types.sort_by_key(|t| t.pixel_width());
        for icon_type in types {
            let Ok(image) = family.get_icon_with_type(icon_type) else {
                continue;
            };
            let image = image.convert_to(icns::PixelFormat::RGBA);
            return downsample_premultiply(image.data(), image.width(), image.height());
        }
        Vec::new()
    }

    /// Box-filters straight-alpha RGBA down to ICON_SIZE and premultiplies,
    /// which is the layout `ui::app_icon` hands to Slint.
    fn downsample_premultiply(src: &[u8], width: u32, height: u32) -> Vec<u8> {
        let mut out = Vec::with_capacity((ICON_SIZE * ICON_SIZE * 4) as usize);
        for oy in 0..ICON_SIZE {
            let y0 = oy * height / ICON_SIZE;
            let y1 = ((oy + 1) * height / ICON_SIZE).max(y0 + 1);
            for ox in 0..ICON_SIZE {
                let x0 = ox * width / ICON_SIZE;
                let x1 = ((ox + 1) * width / ICON_SIZE).max(x0 + 1);
                let mut acc = [0u64; 4];
                let mut samples = 0u64;
                for y in y0..y1 {
                    for x in x0..x1 {
                        let i = ((y * width + x) * 4) as usize;
                        let alpha = u64::from(src[i + 3]);
                        acc[0] += u64::from(src[i]) * alpha;
                        acc[1] += u64::from(src[i + 1]) * alpha;
                        acc[2] += u64::from(src[i + 2]) * alpha;
                        acc[3] += alpha;
                        samples += 1;
                    }
                }
                // Colour sums are already alpha-weighted: dividing by 255 per
                // sample yields the premultiplied average directly.
                out.push((acc[0] / (255 * samples)) as u8);
                out.push((acc[1] / (255 * samples)) as u8);
                out.push((acc[2] / (255 * samples)) as u8);
                out.push((acc[3] / samples) as u8);
            }
        }
        out
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn list() -> Result<Vec<InstalledApp>, String> {
    Ok(Vec::new())
}

/// Invokes the registered item's default Shell verb. Unlike spawning an EXE,
/// this also activates packaged applications whose target is an AppsFolder PIDL
/// parsing name.
#[cfg(windows)]
pub fn launch(target: &str) -> Result<(), String> {
    use windows::Win32::System::Com::{
        CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, SHGetIDListFromObject,
        SHGetKnownFolderItem, ShellExecuteExW, KF_FLAG_DEFAULT, SEE_MASK_INVOKEIDLIST,
        SHELLEXECUTEINFOW, SIGDN_DESKTOPABSOLUTEPARSING,
    };

    struct ComGuard(bool);
    impl Drop for ComGuard {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }

    if target.trim().is_empty() {
        return Err("No registered application is selected".into());
    }

    unsafe {
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let _guard = ComGuard(init.is_ok());
        if init.is_err() {
            return Err(format!(
                "COM initialization failed: 0x{:08X}",
                init.0 as u32
            ));
        }

        // The parsing names returned by AppsFolder are relative to that virtual
        // folder, not global filesystem paths. Re-find the original Shell item
        // and obtain its absolute PIDL before invoking the default verb.
        let folder: IShellItem = SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None)
            .map_err(|e| e.to_string())?;
        let enumerator: IEnumShellItems = folder
            .BindToHandler(None, &BHID_EnumItems)
            .map_err(|e| e.to_string())?;
        let mut matched = None;
        loop {
            let mut slot = [None];
            let mut fetched = 0;
            enumerator
                .Next(&mut slot, Some(&mut fetched))
                .map_err(|e| e.to_string())?;
            if fetched == 0 {
                break;
            }
            let Some(item) = slot[0].take() else {
                continue;
            };
            let raw = item
                .GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING)
                .map_err(|e| e.to_string())?;
            let candidate = raw.to_string().map_err(|e| e.to_string());
            CoTaskMemFree(Some(raw.0.cast()));
            if candidate?.eq_ignore_ascii_case(target) {
                matched = Some(item);
                break;
            }
        }
        let item = matched.ok_or_else(|| "The selected app is no longer registered".to_string())?;
        let pidl = SHGetIDListFromObject(&item).map_err(|e| e.to_string())?;

        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_INVOKEIDLIST,
            lpIDList: pidl.cast(),
            nShow: 1, // SW_SHOWNORMAL
            ..Default::default()
        };
        let result = ShellExecuteExW(&mut info).map_err(|e| e.to_string());
        CoTaskMemFree(Some(pidl.cast()));
        result
    }
}

/// Launches the bundle through `open`, which activates an already-running
/// instance instead of starting a second one, exactly like a Dock click.
#[cfg(target_os = "macos")]
pub fn launch(target: &str) -> Result<(), String> {
    if target.trim().is_empty() {
        return Err("No application is selected".into());
    }
    if !std::path::Path::new(target).exists() {
        return Err("The selected application is no longer installed".into());
    }
    let status = std::process::Command::new("open")
        .arg(target)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("open failed with {status}"))
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn launch(_target: &str) -> Result<(), String> {
    Err("Registered application launching is only available on Windows and macOS".into())
}

/// Finds the best initial selection without silently losing an existing target.
/// Version-1 preset ids are used only as a one-time migration hint.
pub fn selected_index(
    apps: &[InstalledApp],
    app_target: &str,
    app_name: &str,
    legacy_preset_id: &str,
) -> Option<usize> {
    if !app_target.is_empty() {
        if let Some(index) = apps
            .iter()
            .position(|app| app.target.eq_ignore_ascii_case(app_target))
        {
            return Some(index);
        }
    }
    if !app_name.is_empty() {
        if let Some(index) = apps
            .iter()
            .position(|app| app.name.eq_ignore_ascii_case(app_name))
        {
            return Some(index);
        }
    }

    let legacy_name = match legacy_preset_id {
        "teams" => "Microsoft Teams",
        "zoom" => "Zoom",
        "webex" => "Webex",
        "skype" => "Skype",
        "chatgpt_codex" => "ChatGPT",
        "claude" => "Claude",
        _ => "",
    };
    if !legacy_name.is_empty() {
        if let Some(index) = apps.iter().position(|app| {
            app.name
                .to_lowercase()
                .contains(&legacy_name.to_lowercase())
        }) {
            return Some(index);
        }
    }
    (!apps.is_empty()).then_some(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, target: &str) -> InstalledApp {
        InstalledApp {
            name: name.into(),
            target: target.into(),
            registered: true,
            icon_rgba: Vec::new(),
        }
    }

    #[test]
    fn exact_target_wins_over_name_and_legacy_hint() {
        let apps = vec![app("Microsoft Teams", "teams"), app("Zoom", "zoom")];
        assert_eq!(
            selected_index(&apps, "zoom", "Microsoft Teams", "teams"),
            Some(1)
        );
    }

    #[test]
    fn migrates_a_legacy_preset_by_display_name() {
        let apps = vec![
            app("Calculator", "calc"),
            app("Microsoft Teams (work or school)", "teams"),
        ];
        assert_eq!(selected_index(&apps, "", "", "teams"), Some(1));
    }

    #[test]
    fn empty_list_has_no_selection() {
        assert_eq!(selected_index(&[], "", "", "teams"), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn application_bundles_have_names_icons_and_paths() {
        let apps = list().expect("the Applications folders should be readable");
        assert!(!apps.is_empty(), "at least one .app bundle should be found");
        assert!(apps
            .iter()
            .all(|app| !app.name.is_empty() && app.target.ends_with(".app")));
        let icons = apps
            .iter()
            .filter(|app| {
                app.icon_rgba.len() == 32 * 32 * 4
                    && app
                        .icon_rgba
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .any(|pixel| pixel[3] != 0)
            })
            .count();
        assert!(
            icons * 2 > apps.len(),
            "most bundles should expose a decodable .icns ({icons}/{})",
            apps.len()
        );
    }

    #[cfg(windows)]
    #[test]
    fn apps_folder_entries_have_launch_targets() {
        let apps = list().expect("shell:AppsFolder should be enumerable");
        assert!(!apps.is_empty(), "shell:AppsFolder should not be empty");
        assert!(apps
            .iter()
            .all(|app| !app.name.is_empty() && !app.target.is_empty()));
        let icons = apps
            .iter()
            .filter(|app| {
                app.icon_rgba.len() == 32 * 32 * 4
                    && app.icon_rgba.chunks_exact(4).any(|pixel| pixel[3] != 0)
            })
            .count();
        assert!(
            icons * 2 > apps.len(),
            "most AppsFolder items should expose a Shell icon ({icons}/{})",
            apps.len()
        );
    }
}
