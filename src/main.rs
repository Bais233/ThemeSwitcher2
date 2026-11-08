// ThemeSwitcher2 寮曟搸 鈥?鎵樼洏甯搁┗, 鎸夋棩鍑烘棩钀借嚜鍔ㄥ垏鎹㈡繁娴呰壊涓庨紶鏍囨寚閽?// 鍏变韩鏍稿績鍦?lib.rs(tauri 浠〃鐩樺鐢?; 鏈枃浠跺彧绠℃墭鐩樹笌蹇冭烦
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tray;

use chrono::{DateTime, Local};
use std::time::{Duration, Instant, SystemTime};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoop};
use crate::{controller, cursor, location, settings, sun, theme, RESOLVED_LATLON};
use tray_icon::menu::MenuEvent;

fn main() {
    settings::init();
    cursor::capture_active_cursor();
    let mut current_light = theme::sync_current_theme();

    // 瀹氫綅浼樺厛绾? 鍧愭爣 > 鍦板悕(鑱旂綉瑙ｆ瀽涓€娆? > IP 鑷姩(鑱旂綉涓€娆?
    let s = settings::get();
    let latlon = resolve_latlon(&s.location);
    *RESOLVED_LATLON.lock().unwrap() = latlon.clone();
    drop(s);

    // 寮€鏈鸿ˉ璇? 鍚姩鏃跺厛鎶婁富棰樺榻愬埌褰撳墠鏃跺埢搴旀湁鐨勭姸鎬?    let s = settings::get();
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

    // 鏈満鎺у埗鎺ュ彛(Flutter 浠〃鐩樼殑鏁版嵁婧?
    std::thread::spawn(controller::serve);

    let event_loop = EventLoop::new();
    let (tray_obj, m) = tray::build();
    m.autostart.set_checked(theme::get_autostart());
    let menu_rx = MenuEvent::receiver();

    let mut next: Option<(DateTime<Local>, bool)> = None;
    let mut last_cfg = settings::timestamp();
    let mut last_text = String::new();

    event_loop.run(move |event, _, control_flow| {
        // 1 绉掑績璺?        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(1));

        let Event::NewEvents(cause) = event else { return };
        if !matches!(
            cause,
            StartCause::Init | StartCause::ResumeTimeReached { .. }
        ) {
            return;
        }

        let now = Local::now();

        // 鎵樼洏鑿滃崟浜嬩欢
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
                last_cfg = SystemTime::UNIX_EPOCH; // 寮哄埗涓嬮潰閲嶈
            } else if ev.id() == m.quit.id() {
                let _ = tray_obj.set_visible(false);
                *control_flow = ControlFlow::Exit;
                return;
            }
        }
        if *control_flow == ControlFlow::Exit {
            return;
        }

        // 閰嶇疆鏂囦欢鐑噸杞?        let mt = settings::timestamp();
        if mt != last_cfg || controller::consume_reload() {
            last_cfg = mt;
            settings::set(settings::load());
            next = None;
            m.autostart.set_checked(theme::get_autostart());
        }

        let s = settings::get();

        // 璁＄畻涓嬫鍒囨崲鏃跺埢
        if next.is_none() {
            next = Some(match sun::next_event(&latlon, s.sunrise_offset_min, s.sunset_offset_min, now)
            {
                Ok(v) => v,
                Err(_) => (now + chrono::Duration::minutes(30), false), // 鎷夸笉鍒版暟鎹? 30 鍒嗛挓鍚庨噸璇?            });
        }

        // 鍒扮偣鍒囨崲
        if let Some((when, light)) = next {
            if now >= when {
                theme::set_theme(light);
                current_light = light;
                next = None;
            }
        }

        // 璺熼殢鐢ㄦ埛鍦ㄧ郴缁熻缃噷鐨勬墜鍔ㄤ慨鏀?        current_light = theme::sync_current_theme();

        // 鐘舵€佽(浠呭彉鍖栨椂鍒锋柊, 閬垮厤鑿滃崟閲嶇粯闂儊)
        let mode = if current_light { "娴呰壊妯″紡" } else { "娣辫壊妯″紡" };
        let next_txt = match next {
            Some((t, l)) => format!(
                "涓嬫鍒囨崲 {} 鈫?{}",
                t.format("%H:%M"),
                if l { "娴呰壊" } else { "娣辫壊" }
            ),
            None => "涓嬫鍒囨崲 鑾峰彇涓€?.to_string(),
        };
        let text = format!("{} 路 {}", mode, next_txt);
        if text != last_text {
            m.status.set_text(text.clone());
            last_text = text;
        }

        // 寮曟搸鐘舵€佸悓姝ョ粰鎺у埗鎺ュ彛
        let (next_when, next_light) = match next {
            Some((t, l)) => (t.format("%H:%M").to_string(), l),
            None => (String::new(), false),
        };
        *controller::ENGINE.write().unwrap() = controller::EngineStatus {
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
