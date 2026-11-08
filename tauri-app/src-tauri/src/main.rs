// ThemeSwitcher2 玻璃仪表盘 — Tauri 2 + WebView2
// 复用引擎 crate(themeswitcher2)的核心逻辑; 窗口玻璃 = CSS + Win10 亚克力(window-vibrancy)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use themeswitcher2::{controller, cursor, location, settings, sun, theme, RESOLVED_LATLON};

#[tauri::command]
fn get_status() -> serde_json::Value {
    let st = controller::ENGINE.read().unwrap().clone();
    let s = settings::get();
    let lat = RESOLVED_LATLON.lock().unwrap().clone();
    let (sunrise, sunset) = match sun::get_sun_event(&lat, true, 0) {
        Ok(t) => match sun::get_sun_event(&lat, false, 0) {
            Ok(t2) => (t.format("%H:%M").to_string(), t2.format("%H:%M").to_string()),
            Err(_) => (String::new(), String::new()),
        },
        Err(_) => (String::new(), String::new()),
    };
    serde_json::json!({
        "light": st.light,
        "nextWhen": st.next_when,
        "nextLight": st.next_light,
        "locationResolved": lat,
        "sunrise": sunrise,
        "sunset": sunset,
        "engine": "running",
    })
}

#[tauri::command]
fn get_settings() -> serde_json::Value {
    serde_json::to_value(settings::get()).unwrap_or_default()
}

#[tauri::command]
fn save_settings(new_settings: settings::Settings) {
    let old = settings::get();
    if old.autostart != new_settings.autostart {
        theme::set_autostart(new_settings.autostart);
    }
    settings::set(new_settings.clone());
    settings::save(&new_settings);
    controller::request_reload();
}

#[tauri::command]
fn list_schemes() -> Vec<String> {
    cursor::get_cursor_schemes()
}

#[tauri::command]
fn open_config() {
    let _ = std::process::Command::new("notepad.exe")
        .arg(settings::config_path())
        .spawn();
}

#[tauri::command]
fn reload_cfg() {
    controller::request_reload();
}

#[tauri::command]
fn set_theme(light: bool) {
    theme::set_theme(light);
    controller::request_reload();
}

fn main() {
    settings::init();
    cursor::capture_active_cursor();

    // 定位(引擎与仪表盘共享同一份解析结果)
    let s = settings::get();
    let lat = if s.location.trim().is_empty() {
        location::detect_location().unwrap_or_default()
    } else if s.location.trim().split(',').count() == 2 {
        s.location.trim().to_string()
    } else {
        location::geocode(s.location.trim()).unwrap_or_default()
    };
    *RESOLVED_LATLON.lock().unwrap() = lat;
    drop(s);

    tauri::Builder::default()
        .setup(|app| {
            let win = app
                .get_webview_window("main")
                .expect("main window not found");
            // Win10: SetWindowCompositionAttribute 亚克力(Win11 会自动用更稳的路径)
            #[cfg(target_os = "windows")]
            {
                let _ = window_vibrancy::apply_acrylic(&win, Some((14, 18, 28, 125)));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_settings,
            save_settings,
            list_schemes,
            open_config,
            reload_cfg,
            set_theme
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
