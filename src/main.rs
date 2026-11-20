// ThemeSwitcher2 引擎 — 托盘常驻, 按日出日落自动切换深浅色与鼠标指针
// 共享核心在 lib.rs(仪表盘二进制 src/bin/ui.rs 复用); 本文件只管托盘与心跳
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tray;

use chrono::{DateTime, Local};
use std::time::{Duration, Instant, SystemTime};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoop};
use themeswitcher2::{controller, cursor, location, settings, sun, theme, ENGINE_STATE, RESOLVED_LATLON, SNOOZE_UNTIL};
use tray_icon::menu::MenuEvent;

// 单实例守卫: 已有引擎在跑时返回 false。
// 没有它的话第二个实例会在建托盘/事件循环时 panic, release 下 panic=abort 直接崩(0xc0000409),
// 用户看到的现象就是"双击打不开"。
#[cfg(windows)]
fn claim_single_instance() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;
    unsafe {
        let name: Vec<u16> = "Local\\ThemeSwitcher2.Engine\0".encode_utf16().collect();
        let h = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        if h == 0 {
            return true; // 创建失败: 不因此阻止启动
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(h);
            return false;
        }
        // 句柄不关闭, 随进程退出由系统回收 —— 这正是"锁"的存活期
        std::mem::forget(h);
        true
    }
}

fn main() {
    // 单 exe 双模式: 无参 = 托盘引擎(常驻), --ui = 独立进程跑设置界面
    // 仍是两个进程: UI 崩溃不影响引擎, 引擎自启不弹窗
    if std::env::args().any(|a| a == "--ui") {
        themeswitcher2::ui::run();
        return;
    }
    // 已有引擎在跑: 打开设置界面并退出, 而不是崩掉让用户以为程序坏了
    if !claim_single_instance() {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).arg("--ui").spawn();
        }
        return;
    }
    settings::init();
    cursor::capture_active_cursor();
    let mut current_light = theme::sync_current_theme();

    // 定位优先级: 坐标 > 地名(联网解析一次) > IP 自动(联网一次)
    let s = settings::get();
    let latlon = resolve_latlon(&s.location);
    *RESOLVED_LATLON.lock().unwrap() = latlon.clone();
    drop(s);

    // 开机补课: 启动时先把主题对齐到当前时刻应有的状态
    // (错过日出/日落才开机时, 也能立刻回到正确模式)
    let s = settings::get();
    if let Some(should) = sun::current_light_for(
        &latlon,
        s.sunrise_offset_min,
        s.sunset_offset_min,
        Local::now(),
    ) {
        if should != current_light {
            theme::set_theme(should);
            current_light = should;
        }
    }

    // 本机控制接口(仪表盘 UI 的数据源)
    std::thread::spawn(controller::serve);

    let event_loop = EventLoop::new();
    let (tray_obj, m) = tray::build();
    m.autostart.set_checked(theme::get_autostart());
    let menu_rx = MenuEvent::receiver();

    let mut next: Option<(DateTime<Local>, bool)> = None;
    let mut last_cfg = settings::timestamp();
    let mut last_text = String::new();
    // 最近一次真正写主题的时刻: 偏差纠正的节流基准, 避免与用户操作来回拉锯
    let mut last_set = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        // 1 秒心跳
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(1));

        let Event::NewEvents(cause) = event else { return };
        if !matches!(
            cause,
            StartCause::Init | StartCause::ResumeTimeReached { .. }
        ) {
            return;
        }

        let now = Local::now();

        // 托盘菜单事件
        while let Ok(ev) = menu_rx.try_recv() {
            if ev.id() == m.to_light.id() {
                theme::set_theme(true);
                current_light = true;
                next = None;
                last_set = Instant::now();
            } else if ev.id() == m.to_dark.id() {
                theme::set_theme(false);
                current_light = false;
                next = None;
                last_set = Instant::now();
            } else if ev.id() == m.autostart.id() {
                let on = m.autostart.is_checked();
                theme::set_autostart(on);
                let mut s = settings::get();
                s.autostart = on;
                settings::set(s.clone());
                settings::save(&s);
            } else if ev.id() == m.settings_ui.id() {
                // 启动自身 --ui 模式(独立进程)
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).arg("--ui").spawn();
                }
            } else if ev.id() == m.open_config.id() {
                let _ = std::process::Command::new("notepad.exe")
                    .arg(settings::config_path())
                    .spawn();
            } else if ev.id() == m.reload.id() {
                last_cfg = SystemTime::UNIX_EPOCH; // 强制下面重读
            } else if ev.id() == m.quit.id() {
                let _ = tray_obj.set_visible(false);
                *control_flow = ControlFlow::Exit;
                return;
            }
        }
        if *control_flow == ControlFlow::Exit {
            return;
        }

        // 配置文件热重载(别的编辑器改了 ini 也能跟上); UI 的 reload 请求也在这里消费
        let mt = settings::timestamp();
        if mt != last_cfg || controller::consume_reload() {
            last_cfg = mt;
            settings::set(settings::load());
            cursor::invalidate_pack_cache(); // 装了新指针包后重扫, 不必重启程序
            next = None;
            m.autostart.set_checked(theme::get_autostart());
        }

        let s = settings::get();

        // 计算下次切换时刻
        if next.is_none() {
            next = Some(match sun::next_event(&latlon, s.sunrise_offset_min, s.sunset_offset_min, now)
            {
                Ok(v) => v,
                Err(_) => (now + chrono::Duration::minutes(30), false), // 拿不到数据: 30 分钟后重试
            });
        }

        // 到点切换(推迟中跳过: 错过的事件留在 next 里, 推迟到期后下一秒补课对齐)
        let snoozing = {
            let mut g = SNOOZE_UNTIL.lock().unwrap();
            match *g {
                Some(t) if Instant::now() < t => true,
                Some(_) => {
                    *g = None; // 过期清理
                    false
                }
                None => false,
            }
        };
        if let Some((when, light)) = next {
            if now >= when && !snoozing {
                theme::set_theme(light);
                current_light = light;
                next = None;
                last_set = Instant::now();
            }
        }

        // 跟随用户在系统设置里的手动修改
        current_light = theme::sync_current_theme();

        // 持续校验: 当前主题与此刻应有的不符(手动改动/外部程序/休眠恢复)时纠正。
        // 到点切换只是"事件驱动", 光靠它会让错误状态一直持续到下一个日出/日落。
        // 5 秒节流避免与用户操作来回抖动; 推迟期间完全不动。
        if !snoozing {
            if let Some(should) =
                sun::current_light_for(&latlon, s.sunrise_offset_min, s.sunset_offset_min, now)
            {
                if should != current_light && last_set.elapsed() >= Duration::from_secs(5) {
                    theme::set_theme(should);
                    current_light = should;
                    last_set = Instant::now();
                }
            }
        }

        // 状态行(仅变化时刷新, 避免菜单重绘闪烁)
        let mode = if current_light { "浅色模式" } else { "深色模式" };
        let next_txt = match next {
            Some((t, l)) => format!(
                "下次切换 {} → {}",
                t.format("%H:%M"),
                if l { "浅色" } else { "深色" }
            ),
            None => "下次切换 获取中…".to_string(),
        };
        let text = format!("{} · {}", mode, next_txt);
        if text != last_text {
            m.status.set_text(text.clone());
            last_text = text;
        }

        // 引擎状态同步给控制接口
        let (next_when, next_light) = match next {
            Some((t, l)) => (t.format("%H:%M").to_string(), l),
            None => (String::new(), false),
        };
        *ENGINE_STATE.write().unwrap() = controller::EngineStatus {
            light: current_light,
            next_when,
            next_light,
        };
    });
}

fn resolve_latlon(loc: &str) -> String {
    let t = loc.trim();
    if t.is_empty() {
        return location::detect_location().unwrap_or_default();
    }
    if t.split(',').count() == 2 && t.split(',').all(|p| p.trim().parse::<f64>().is_ok()) {
        return t.to_string();
    }
    location::geocode(t).unwrap_or_default()
}
