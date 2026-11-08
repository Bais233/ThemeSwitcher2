// settings.rs — INI 配置读写, 存 %APPDATA%\ThemeSwitcher2\settings.ini
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::SystemTime;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub switch_apps: bool,
    pub switch_system: bool,
    pub sunrise_offset_min: i64,
    pub sunset_offset_min: i64,
    pub switch_cursor: bool,
    pub cursor_light_scheme: String,
    pub cursor_dark_scheme: String,
    pub location: String,
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            switch_apps: true,
            switch_system: true,
            sunrise_offset_min: 0,
            sunset_offset_min: 0,
            switch_cursor: false,
            cursor_light_scheme: "(不修改)".into(),
            cursor_dark_scheme: "(不修改)".into(),
            location: String::new(),
            autostart: false,
        }
    }
}

static SETTINGS: RwLock<Option<Settings>> = RwLock::new(None);

pub fn config_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(appdata).join("ThemeSwitcher2");
    let _ = fs::create_dir_all(&dir);
    dir.join("settings.ini")
}

pub fn load() -> Settings {
    let mut s = Settings::default();
    let Ok(text) = fs::read_to_string(config_path()) else {
        return s;
    };
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let k = k.trim();
        let v = v.trim();
        match k {
            "SwitchApps" => {
                if let Some(b) = parse_bool(v) {
                    s.switch_apps = b;
                }
            }
            "SwitchSystem" => {
                if let Some(b) = parse_bool(v) {
                    s.switch_system = b;
                }
            }
            "SunriseOffsetMin" => s.sunrise_offset_min = v.parse().unwrap_or(0),
            "SunsetOffsetMin" => s.sunset_offset_min = v.parse().unwrap_or(0),
            "SwitchCursor" => {
                if let Some(b) = parse_bool(v) {
                    s.switch_cursor = b;
                }
            }
            "CursorLightScheme" => {
                if !v.is_empty() {
                    s.cursor_light_scheme = v.to_string();
                }
            }
            "CursorDarkScheme" => {
                if !v.is_empty() {
                    s.cursor_dark_scheme = v.to_string();
                }
            }
            "Location" => s.location = v.to_string(),
            "Autostart" => {
                if let Some(b) = parse_bool(v) {
                    s.autostart = b;
                }
            }
            _ => {}
        }
    }
    s
}

pub fn save(s: &Settings) {
    let body = format!(
        "SwitchApps={}\nSwitchSystem={}\nSunriseOffsetMin={}\nSunsetOffsetMin={}\nSwitchCursor={}\nCursorLightScheme={}\nCursorDarkScheme={}\nLocation={}\nAutostart={}\n",
        s.switch_apps,
        s.switch_system,
        s.sunrise_offset_min,
        s.sunset_offset_min,
        s.switch_cursor,
        s.cursor_light_scheme,
        s.cursor_dark_scheme,
        s.location,
        s.autostart
    );
    let _ = fs::write(config_path(), body);
}

pub fn timestamp() -> SystemTime {
    fs::metadata(config_path())
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

pub fn init() {
    *SETTINGS.write().unwrap() = Some(load());
}

pub fn get() -> Settings {
    SETTINGS
        .read()
        .unwrap()
        .clone()
        .unwrap_or_default()
}

pub fn set(s: Settings) {
    *SETTINGS.write().unwrap() = Some(s);
}

fn parse_bool(v: &str) -> Option<bool> {
    if v.eq_ignore_ascii_case("true") {
        Some(true)
    } else if v.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}
