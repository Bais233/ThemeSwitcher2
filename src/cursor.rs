// cursor.rs — 鼠标指针方案切换: 注册表方案(HKCU/HKLM) + 未注册指针包(C:\Windows\Cursors 文件夹)
// 原理: 写 HKCU\Control Panel\Cursors 的 14 个角色值, SystemParametersInfo(SPI_SETCURSORS) 立即生效
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use winreg::enums::*;
use winreg::RegKey;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_SETCURSORS,
};

pub const KEEP: &str = "(不修改)";
pub const DEFAULT_SCHEME: &str = "Windows 默认（系统方案）";
const CURSORS_KEY: &str = "Control Panel\\Cursors";
const SCHEMES_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Control Panel\\Cursors\\Schemes";
const CURSORS_DIR: &str = "C:\\Windows\\Cursors";
const THEMES_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes";

pub const VALUE_NAMES: [&str; 14] = [
    "Arrow", "Help", "AppStarting", "Wait", "Crosshair", "IBeam", "NWPen", "No", "SizeNS",
    "SizeWE", "SizeNWSE", "SizeNESW", "UpArrow", "Hand",
];

// 角色识别别名(社区指针包只拷文件不注册方案, 靠文件名认领)
// 匹配规则: 文件名(去扩展名/去空格/小写) == 别名, 或别名长度>=4 且被文件名包含
static ROLE_ALIASES: [&[&str]; 14] = [
    &["pointer", "default", "normal-select", "normal", "正常选择", "arrow"],
    &["help-select", "help", "帮助选择", "帮助"],
    &["working", "appstarting", "后台运行", "后台"],
    &["busy", "wait", "忙", "beachball"],
    &["crosshair", "cross", "精确选择"],
    &["text-select", "text", "ibeam", "文本选择", "beam"],
    &["handwriting", "手写", "pen"],
    &["unavailable", "不可用", "no"],
    &["vertical", "vert", "垂直调整大小", "垂直", "size-ns", "sizens", "_n-s"],
    &["horizontal", "horz", "水平调整大小", "水平", "size-ew", "sizewe", "_w-e"],
    &["diagonal-resize-1", "对角线调整大小1", "对角线调整大小 1", "dgn1", "nw-se", "sizenwse"],
    &["diagonal-resize-2", "对角线调整大小2", "对角线调整大小 2", "dgn2", "ne-sw", "sizenesw"],
    &["alternate", "up-arrow", "uparrow", "up"],
    &["link-select", "链接选择", "alt-select", "link", "hand", "候选"],
];

#[derive(Clone)]
pub struct ActiveSnapshot {
    pub files: [String; 14],
    pub name: String,
    pub source: u32,
}

static ACTIVE: OnceLock<Option<ActiveSnapshot>> = OnceLock::new();
static PACK_DIRS: OnceLock<HashMap<String, String>> = OnceLock::new();

pub fn capture_active_cursor() {
    if ACTIVE.get().is_some() {
        return;
    }
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let snapshot = hkcu.open_subkey(CURSORS_KEY).map(|key| {
        let mut files = [String::new(); 14];
        for (i, n) in VALUE_NAMES.iter().enumerate() {
            files[i] = key.get_value::<String, _>(n).unwrap_or_default();
        }
        let name: String = key.get_value("").unwrap_or_default();
        let source: u32 = key.get_value("Scheme Source").unwrap_or(0u32);
        ActiveSnapshot { files, name, source }
    });
    let _ = ACTIVE.set(snapshot.ok());
}

pub fn active_snapshot() -> Option<ActiveSnapshot> {
    ACTIVE.get().cloned().flatten()
}

pub fn get_current_cursor_scheme() -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(CURSORS_KEY).ok()?;
    key.get_value::<String, _>("").ok()
}

pub fn get_cursor_schemes() -> Vec<String> {
    let mut list = vec![DEFAULT_SCHEME.to_string()];
    let mut seen: HashSet<String> = HashSet::new();
    seen.insert(DEFAULT_SCHEME.to_string());
    // 指针包放最前: 用户自己装的包才是高频选项
    for name in scan_cursor_pack_names() {
        if seen.insert(name.clone()) {
            list.push(name);
        }
    }
    if let Some(cur) = get_current_cursor_scheme() {
        if !cur.is_empty() && seen.insert(cur.clone()) {
            list.push(cur);
        }
    }
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        if let Ok(key) = RegKey::predef(root).open_subkey(SCHEMES_KEY) {
            for (name, _) in key.enum_values().flatten() {
                if !name.is_empty() && seen.insert(name.clone()) {
                    list.push(name);
                }
            }
        }
    }
    list
}

fn pack_dirs() -> &'static HashMap<String, String> {
    PACK_DIRS.get_or_init(|| {
        let mut map = HashMap::new();
        if let Ok(rd) = std::fs::read_dir(CURSORS_DIR) {
            let mut dirs: Vec<_> = rd.flatten().collect();
            dirs.sort_by_key(|e| e.file_name());
            for d in dirs {
                let path = d.path();
                if path.is_dir() {
                    try_add_pack(&path, None, &mut map);
                    if let Ok(subs) = std::fs::read_dir(&path) {
                        let mut subs: Vec<_> = subs.flatten().collect();
                        subs.sort_by_key(|e| e.file_name());
                        for s in subs {
                            let spath = s.path();
                            if spath.is_dir() {
                                let parent = d.file_name().to_string_lossy().to_string();
                                try_add_pack(&spath, Some(&parent), &mut map);
                            }
                        }
                    }
                }
            }
        }
        map
    })
}

fn try_add_pack(path: &std::path::Path, parent: Option<&str>, map: &mut HashMap<String, String>) {
    let mut count = 0;
    if let Ok(rd) = std::fs::read_dir(path) {
        for e in rd.flatten() {
            let ext = e
                .path()
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext == "ani" || ext == "cur" {
                count += 1;
            }
        }
    }
    if count < 3 {
        return; // 至少 3 个指针文件才算可用的包
    }
    let base = path
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or("?")
        .to_string();
    let name = match parent {
        Some(p) => format!("{} {}", p, base),
        None => base,
    };
    map.entry(name).or_insert_with(|| path.to_string_lossy().to_string());
}

pub fn scan_cursor_pack_names() -> Vec<String> {
    let mut names: Vec<String> = pack_dirs().keys().cloned().collect();
    names.sort();
    names
}

fn match_pack_files(dir: &str) -> Option<[Option<String>; 14]> {
    let mut result: [Option<String>; 14] = Default::default();
    let mut claimed: HashSet<String> = HashSet::new();
    let rd = std::fs::read_dir(dir).ok()?;
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for e in rd.flatten() {
        let p = e.path();
        let ext = p
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "ani" || ext == "cur" {
            files.push(p);
        }
    }
    if files.is_empty() {
        return None;
    }
    // 认领顺序: 专属别名多的角色先挑, 防止 Arrow 的泛化别名抢走 up-arrow 的文件
    let order: [usize; 14] = [12, 13, 4, 5, 6, 7, 3, 2, 8, 9, 10, 11, 1, 0];
    for role in order {
        for alias in ROLE_ALIASES[role] {
            let a = alias.to_lowercase().replace(' ', "");
            if a.is_empty() {
                continue;
            }
            let mut hit: Option<usize> = None;
            for (fi, f) in files.iter().enumerate() {
                let key = f.to_string_lossy().to_string();
                if claimed.contains(&key) {
                    continue;
                }
                let stem = f
                    .file_stem()
                    .and_then(|x| x.to_str())
                    .unwrap_or("")
                    .to_lowercase()
                    .replace(' ', "");
                if stem == a || (a.chars().count() >= 4 && stem.contains(a.as_str())) {
                    hit = Some(fi);
                    break;
                }
            }
            if let Some(fi) = hit {
                result[role] = Some(files[fi].to_string_lossy().to_string());
                claimed.insert(files[fi].to_string_lossy().to_string());
                break;
            }
        }
    }
    Some(result)
}

fn read_scheme_data(name: &str) -> Option<String> {
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        if let Ok(key) = RegKey::predef(root).open_subkey(SCHEMES_KEY) {
            if let Ok(v) = key.get_value::<String, _>(name) {
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

// 系统策略 ThemeChangesMousePointers=0 时指针替换会静默失效(组策略/优化工具会关它)。
// 防御来自 Auto Dark Mode 的 RegistryHandler.EnableThemeChangesMousePointers
fn enable_theme_changes_mouse_pointers() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(THEMES_KEY, KEY_SET_VALUE | KEY_QUERY_VALUE) {
        if let Ok(v) = key.get_value::<u32, _>("ThemeChangesMousePointers") {
            if v == 0 {
                let _ = key.set_value("ThemeChangesMousePointers", 1u32);
            }
        }
    }
}

pub fn apply_cursor_scheme(scheme: &str) {
    enable_theme_changes_mouse_pointers();
    if scheme == KEEP {
        return; // 绝不碰指针
    }
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 指针包: 只写匹配到的角色, 其余保持现状(绝不重置)
    if scheme != DEFAULT_SCHEME {
        if let Some(dir) = pack_dirs().get(scheme).cloned() {
            let Some(files) = match_pack_files(&dir) else { return };
            if let Ok(key) = hkcu.open_subkey_with_flags(CURSORS_KEY, KEY_SET_VALUE | KEY_QUERY_VALUE) {
                for (i, n) in VALUE_NAMES.iter().enumerate() {
                    if let Some(f) = &files[i] {
                        let _ = key.set_value(n, f.as_str());
                    }
                }
                let _ = key.set_value("Scheme Source", 0u32);
                let _ = key.set_value("", scheme);
                reload_cursors();
            }
            return;
        }
        // 注册表方案: HKCU -> HKLM, 逗号分割 15 项
        if let Some(data) = read_scheme_data(scheme) {
            let parts: Vec<&str> = data.split(',').collect();
            if let Ok(key) = hkcu.open_subkey_with_flags(CURSORS_KEY, KEY_SET_VALUE | KEY_QUERY_VALUE) {
                for (i, n) in VALUE_NAMES.iter().enumerate() {
                    if i < parts.len() {
                        let f = parts[i].trim().trim_matches('"');
                        if !f.is_empty() {
                            let _ = key.set_value(n, f);
                        }
                    }
                }
                let _ = key.set_value("Scheme Source", 0u32);
                let _ = key.set_value("", scheme);
                reload_cursors();
            }
            return;
        }
        // 查无数据: 是启动时留过底的当前方案就按快照还原, 否则不动
        if let Some(snap) = active_snapshot() {
            if snap.name == scheme {
                if let Ok(key) = hkcu.open_subkey_with_flags(CURSORS_KEY, KEY_SET_VALUE | KEY_QUERY_VALUE) {
                    for (i, n) in VALUE_NAMES.iter().enumerate() {
                        if snap.files[i].is_empty() {
                            let _ = key.delete_value(n);
                        } else {
                            let _ = key.set_value(n, snap.files[i].as_str());
                        }
                    }
                    let _ = key.set_value("Scheme Source", snap.source);
                    let _ = key.set_value("", snap.name.as_str());
                    reload_cursors();
                }
            }
        }
        return;
    }

    // Windows 默认(系统方案): 清空全部角色值 = 系统默认指针
    if let Ok(key) = hkcu.open_subkey_with_flags(CURSORS_KEY, KEY_SET_VALUE | KEY_QUERY_VALUE) {
        for n in VALUE_NAMES.iter() {
            let _ = key.delete_value(n);
        }
        let _ = key.set_value("Scheme Source", 0u32);
        let _ = key.set_value("", "");
        reload_cursors();
    }
}

fn reload_cursors() {
    unsafe {
        SystemParametersInfoW(SPI_SETCURSORS, 0, std::ptr::null_mut(), SPIF_UPDATEINIFILE | SPIF_SENDCHANGE);
    }
}
