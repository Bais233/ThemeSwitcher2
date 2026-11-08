// theme.rs — 深浅色主题切换(注册表 + ImmersiveColorSet 广播)与开机自启(HKCU Run)
use crate::cursor;
use crate::settings;
use winreg::enums::*;
use winreg::RegKey;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
};

const BROADCAST: HWND = 0xFFFF as HWND;
const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

pub fn set_theme(light: bool) {
    let s = settings::get();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(PERSONALIZE, KEY_SET_VALUE) {
        if s.switch_apps {
            let _ = key.set_value("AppsUseLightTheme", if light { 1u32 } else { 0u32 });
        }
        if s.switch_system {
            let _ = key.set_value("SystemUsesLightTheme", if light { 1u32 } else { 0u32 });
        }
    }
    if s.switch_cursor {
        let scheme = if light {
            s.cursor_light_scheme.clone()
        } else {
            s.cursor_dark_scheme.clone()
        };
        cursor::apply_cursor_scheme(&scheme);
    }
    broadcast();
}

// 读注册表里的真实深浅色状态: 用户在系统设置里手动改也能跟上
pub fn sync_current_theme() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(PERSONALIZE) {
        let apps: u32 = key.get_value("AppsUseLightTheme").unwrap_or(1);
        return apps != 0;
    }
    true
}

pub fn get_autostart() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    hkcu.open_subkey(RUN_KEY)
        .ok()
        .and_then(|k| k.get_value::<String, _>("ThemeSwitcher2").ok())
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

pub fn set_autostart(enable: bool) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE) {
        if enable {
            if let Ok(exe) = std::env::current_exe() {
                let _ = key.set_value("ThemeSwitcher2", format!("\"{}\"", exe.display()).as_str());
            }
        } else {
            let _ = key.delete_value("ThemeSwitcher2");
        }
    }
}

fn broadcast() {
    let mut w: Vec<u16> = "ImmersiveColorSet"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut result: usize = 0;
    unsafe {
        SendMessageTimeoutW(
            BROADCAST,
            WM_SETTINGCHANGE,
            0,
            w.as_mut_ptr() as isize,
            SMTO_ABORTIFHUNG,
            1000,
            &mut result,
        );
    }
}
