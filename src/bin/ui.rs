// ui.rs — ThemeSwitcher2 玻璃仪表盘(Slint 软件渲染版)
// 数据源: 引擎 exe 的本机控制接口 127.0.0.1:27125
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

slint::include_modules!();

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

const BASE: &str = "http://127.0.0.1:27125";

static CFG: Mutex<Option<serde_json::Value>> = Mutex::new(None);
static SCHEMES: Mutex<Vec<String>> = Mutex::new(Vec::new());
static SAVING: AtomicBool = AtomicBool::new(false);

fn get_json(path: &str) -> Option<serde_json::Value> {
    let resp = ureq::get(&format!("{}{}", BASE, path))
        .timeout(Duration::from_secs(2))
        .call()
        .ok()?;
    serde_json::from_str(&resp.into_string().ok()?).ok()
}

fn post(path: &str, body: Option<serde_json::Value>) -> bool {
    let req = ureq::post(&format!("{}{}", BASE, path))
        .timeout(Duration::from_secs(2));
    let req = match body {
        Some(v) => req.set("Content-Type", "application/json").send_string(&v.to_string()),
        None => req.send_string(""),
    };
    req.is_ok()
}

fn save_all(cfg: &serde_json::Value) {
    if SAVING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        post("/settings", Some(cfg.clone()));
        SAVING.store(false, Ordering::SeqCst);
    });
}

fn main() {
    // 强制软件渲染: GT730 上零 GPU 依赖
    std::env::set_var("SLINT_BACKEND", "software");

    let ui = Dashboard::new().unwrap();
    let weak = ui.as_weak();

    // ---- 初始数据拉取(后台线程) ----
    {
        let weak = weak.clone();
        std::thread::spawn(move || {
            if let Some(s) = get_json("/schemes") {
                let list: Vec<String> = s["schemes"]
                    .as_array()
                    .map(|a| a.iter().map(|x| x.as_str().unwrap_or("").to_string()).collect())
                    .unwrap_or_default();
                if let Some(s) = get_json("/settings") {
                    *CFG.lock().unwrap() = Some(s.clone());
                    let light_idx = list.iter().position(|x| *x == s["cursorLightScheme"].as_str().unwrap_or(""));
                    let dark_idx = list.iter().position(|x| *x == s["cursorDarkScheme"].as_str().unwrap_or(""));
                    let _ = slint::invoke_from_event_loop(move || {
                        let Some(ui) = weak.upgrade() else { return };
                        ui.set_schemes(slint::ModelRc::new(slint::VecModel::from(
                            list.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
                        )));
                        ui.set_light_idx(light_idx.unwrap_or(0) as i32);
                        ui.set_dark_idx(dark_idx.unwrap_or(0) as i32);
                        ui.set_sw_apps(s["switchApps"] == true);
                        ui.set_sw_system(s["switchSystem"] == true);
                        ui.set_sw_cursor(s["switchCursor"] == true);
                        ui.set_sw_auto(s["autostart"] == true);
                        ui.set_rise_off(s["sunriseOffsetMin"].as_i64().unwrap_or(0) as i32);
                        ui.set_set_off(s["sunsetOffsetMin"].as_i64().unwrap_or(0) as i32);
                    });
                }
            }
        });
    }

    // ---- 回调: 模式切换 / 开关 / 偏移 / 下拉 / 按钮 ----
    macro_rules! on_cb {
        ($ui:expr, $name:ident, $body:expr) => {
            $ui.$name(move |_| $body)
        };
    }
    {
        let weak = weak.clone();
        ui.on_to_light_clicked(move || {
            post("/theme/light", None);
            if let Some(u) = weak.upgrade() { u.set_light(true); }
        });
    }
    {
        let weak = weak.clone();
        ui.on_to_dark_clicked(move || {
            post("/theme/dark", None);
            if let Some(u) = weak.upgrade() { u.set_light(false); }
        });
    }
    {
        let weak = weak.clone();
        ui.on_apps_toggled(move || {
            if let Some(u) = weak.upgrade() {
                let v = u.get_sw_apps();
                if let Some(c) = CFG.lock().unwrap().as_mut() { c["switchApps"] = serde_json::Value::Bool(v); }
                save_all(&CFG.lock().unwrap().clone().unwrap_or_default());
            }
        });
    }
    {
        let weak = weak.clone();
        ui.on_system_toggled(move || {
            if let Some(u) = weak.upgrade() {
                let v = u.get_sw_system();
                if let Some(c) = CFG.lock().unwrap().as_mut() { c["switchSystem"] = serde_json::Value::Bool(v); }
                save_all(&CFG.lock().unwrap().clone().unwrap_or_default());
            }
        });
    }
    {
        let weak = weak.clone();
        ui.on_cursor_toggled(move || {
            if let Some(u) = weak.upgrade() {
                let v = u.get_sw_cursor();
                if let Some(c) = CFG.lock().unwrap().as_mut() { c["switchCursor"] = serde_json::Value::Bool(v); }
                save_all(&CFG.lock().unwrap().clone().unwrap_or_default());
            }
        });
    }
    {
        let weak = weak.clone();
        ui.on_auto_toggled(move || {
            if let Some(u) = weak.upgrade() {
                let v = u.get_sw_auto();
                if let Some(c) = CFG.lock().unwrap().as_mut() { c["autostart"] = serde_json::Value::Bool(v); }
                save_all(&CFG.lock().unwrap().clone().unwrap_or_default());
            }
        });
    }
    ui.on_rise_step(move |d| {
        if let Some(c) = CFG.lock().unwrap().as_mut() {
            let v = (c["sunriseOffsetMin"].as_i64().unwrap_or(0) + d as i64).clamp(-720, 720);
            c["sunriseOffsetMin"] = serde_json::Value::from(v);
            if let Some(ui) = weak.upgrade() { ui.set_rise_off(v as i32); }
            save_all(&c.clone());
        }
    });
    ui.on_set_step(move |d| {
        if let Some(c) = CFG.lock().unwrap().as_mut() {
            let v = (c["sunsetOffsetMin"].as_i64().unwrap_or(0) + d as i64).clamp(-720, 720);
            c["sunsetOffsetMin"] = serde_json::Value::from(v);
            if let Some(ui) = weak.upgrade() { ui.set_set_off(v as i32); }
            save_all(&c.clone());
        }
    });
    {
        let weak = weak.clone();
        ui.on_light_scheme_changed(move |idx| {
            let list = SCHEMES.lock().unwrap().clone();
            if let Some(scheme) = list.get(idx as usize) {
                if let Some(c) = CFG.lock().unwrap().as_mut() { c["cursorLightScheme"] = serde_json::Value::from(scheme.clone()); }
                save_all(&CFG.lock().unwrap().clone().unwrap_or_default());
            }
            weak.upgrade().map(|u| u.set_light_idx(idx));
        });
    }
    {
        let weak = weak.clone();
        ui.on_dark_scheme_changed(move |idx| {
            let list = SCHEMES.lock().unwrap().clone();
            if let Some(scheme) = list.get(idx as usize) {
                if let Some(c) = CFG.lock().unwrap().as_mut() { c["cursorDarkScheme"] = serde_json::Value::from(scheme.clone()); }
                save_all(&CFG.lock().unwrap().clone().unwrap_or_default());
            }
            weak.upgrade().map(|u| u.set_dark_idx(idx));
        });
    }
    {
        let weak = weak.clone();
        ui.on_save_clicked(move || {
            save_all(&CFG.lock().unwrap().clone().unwrap_or_default());
        });
    }
    ui.on_open_config_clicked(move || {
        let _ = std::process::Command::new("notepad.exe")
            .arg(themeswitcher2::settings::config_path())
            .spawn();
    });

    // ---- 心跳轮询: /status → 界面属性 ----
    {
        let weak = weak.clone();
        std::thread::spawn(move || {
            let mut last = String::new();
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let Some(st) = get_json("/status") else {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() { ui.set_engine_ok(false); }
                    });
                    continue;
                };
                let key = st.to_string();
                if key == last {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = weak.upgrade() { ui.set_engine_ok(true); }
                    });
                    continue;
                }
                last = key;
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(ui) = weak.upgrade() else { return };
                    ui.set_engine_ok(true);
                    ui.set_light(st["light"] == true);
                    ui.set_next_when(st["nextWhen"].as_str().unwrap_or("").into());
                    ui.set_next_light(st["nextLight"] == true);
                    ui.set_loc(st["locationResolved"].as_str().unwrap_or("").into());
                    ui.set_sunrise_t(st["sunrise"].as_str().unwrap_or("").into());
                    ui.set_sunset_t(st["sunset"].as_str().unwrap_or("").into());
                });
            }
        });
    }

    ui.run().unwrap();
}
