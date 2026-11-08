// ThemeSwitcher2 — 按日出日落自动切换 Windows 深色/浅色模式与鼠标指针(Rust 版)
// 架构: tao 事件循环(托盘消息泵) + 1 秒心跳, 无后台线程
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cursor;
mod location;
mod settings;
mod sun;
mod theme;
mod tray;

use chrono::{DateTime, Local};
use std::time::{Duration, Instant, SystemTime};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoop};
use tray_icon::menu::MenuEvent;

fn main() {
    settings::init();
    cursor::capture_active_cursor();
    let mut current_light = theme::sync_current_theme();

    // 定位优先级: 坐标 > 地名(联网解析一次) > IP 自动(联网一次)
    let s = settings::get();
    let latlon = resolve_latlon(&s.location);
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

    let event_loop = EventLoop::new();
    let (tray_obj, m) = tray::build();
    m.autostart.set_checked(theme::get_autostart());
    let menu_rx = MenuEvent::receiver();

    let mut next: Option<(DateTime<Local>, bool)> = None;
    let mut last_cfg = settings::timestamp();
    let mut last_text = String::new();

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
            } else if ev.id() == m.to_dark.id() {
                theme::set_theme(false);
                current_light = false;
                next = None;
            } else if ev.id() == m.autostart.id() {
                let on = m.autostart.is_checked();
                theme::set_autostart(on);
                let mut s = settings::get();
                s.autostart = on;
                settings::set(s.clone());
                settings::save(&s);
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

        // 配置文件热重载(别的编辑器改了 ini 也能跟上)
        let mt = settings::timestamp();
        if mt != last_cfg {
            last_cfg = mt;
            settings::set(settings::load());
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

        // 到点切换
        if let Some((when, light)) = next {
            if now >= when {
                theme::set_theme(light);
                current_light = light;
                next = None;
            }
        }

        // 跟随用户在系统设置里的手动修改
        current_light = theme::sync_current_theme();

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
