// lib.rs — ThemeSwitcher2 共享核心(NOAA/定位/指针/主题/设置/控制接口)
// 引擎二进制(main.rs, 托盘)与仪表盘二进制(src/bin/ui.rs)共用此 crate
pub mod controller;
pub mod cursor;
pub mod location;
pub mod settings;
pub mod sun;
pub mod theme;
pub mod ui;

use std::sync::{Mutex, RwLock};
use std::time::Instant;

// 引擎状态(controller HTTP 线程读)与解析后的定位(启动时写一次)
pub static ENGINE_STATE: RwLock<controller::EngineStatus> = RwLock::new(controller::EngineStatus {
    light: true,
    next_when: String::new(),
    next_light: false,
});
pub static RESOLVED_LATLON: Mutex<String> = Mutex::new(String::new());

// 推迟自动切换: Some(到期时刻) = 推迟中, None = 未推迟。引擎心跳读写, HTTP 线程读写。
// 推迟到期后由心跳自动清除并补课对齐主题。
pub static SNOOZE_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);
