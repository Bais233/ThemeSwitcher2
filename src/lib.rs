// lib.rs — ThemeSwitcher2 共享核心(NOAA/定位/指针/主题/设置/控制接口)
// 引擎二进制(main.rs, 托盘)与 Tauri 仪表盘(tauri-app)共用此 crate
pub mod controller;
pub mod cursor;
pub mod location;
pub mod settings;
pub mod sun;
pub mod theme;

use std::sync::{Mutex, RwLock};

// 引擎状态(controller HTTP 线程读)与解析后的定位(启动时写一次)
pub static ENGINE_STATE: RwLock<controller::EngineStatus> = RwLock::new(controller::EngineStatus {
    light: true,
    next_when: String::new(),
    next_light: false,
});
pub static RESOLVED_LATLON: Mutex<String> = Mutex::new(String::new());
