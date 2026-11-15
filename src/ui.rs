// ui.rs — ThemeSwitcher2 玻璃仪表盘(Slint 软件渲染版)
// 数据源: 引擎的本机控制接口 127.0.0.1:27125
// 以独立进程运行: main.rs 收到 --ui 参数后调用 run(); 与引擎进程互不影响
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
    let req = ureq::post(&format!("{}{}", BASE, path)).timeout(Duration::from_secs(2));
    let req = match body {
        Some(v) => req.set("Content-Type", "application/json").send_string(&v.to_string()),
        None => req.send_string(""),
    };
    req.is_ok()
}

fn save_all(weak: slint::Weak<Dashboard>, cfg: serde_json::Value) {
    if SAVING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let ok = post("/settings", Some(cfg.clone()));
        if !ok {
            // 引擎不在线: 直接落盘兜底, 引擎下次启动/热重载会读到
            // ponytail: 不重试 POST; 引擎活着时不会走到这里
            if let Ok(s) = serde_json::from_value::<crate::settings::Settings>(cfg) {
                crate::settings::save(&s);
            }
        }
        SAVING.store(false, Ordering::SeqCst);
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_save_ok(ok);
                ui.set_save_flash(true);
            }
            slint::Timer::single_shot(Duration::from_secs(3), move || {
                if let Some(ui) = weak.upgrade() {
                    ui.set_save_flash(false);
                }
            });
        });
    });
}

pub fn run() {
    // 强制软件渲染: GT730 上零 GPU 依赖
    std::env::set_var("SLINT_BACKEND", "software");

    let ui = Dashboard::new().unwrap();
    // 渲染/调试: SLINT_DARK=1 强制深色(默认跟随系统)
    if std::env::var("SLINT_DARK").is_ok() {
        ui.window().set_dark_light_mode(slint::DarkLightMode::Dark);
    }
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
                *SCHEMES.lock().unwrap() = list.clone();
                if let Some(s) = get_json("/settings") {
                    *CFG.lock().unwrap() = Some(s.clone());
                    let light_idx = list
                        .iter()
                        .position(|x| *x == s["cursorLightScheme"].as_str().unwrap_or(""))
                        .unwrap_or(0);
                    let dark_idx = list
                        .iter()
                        .position(|x| *x == s["cursorDarkScheme"].as_str().unwrap_or(""))
                        .unwrap_or(0);
                    let _ = slint::invoke_from_event_loop(move || {
                        let Some(ui) = weak.upgrade() else { return };
                        ui.set_schemes(slint::ModelRc::new(slint::VecModel::from(
                            list.iter()
                                .map(|x| slint::SharedString::from(x.as_str()))
                                .collect::<Vec<slint::SharedString>>(),
                        )));
                        ui.set_light_idx(light_idx as i32);
                        ui.set_dark_idx(dark_idx as i32);
                        ui.set_sw_apps(s["switchApps"] == true);
                        ui.set_sw_system(s["switchSystem"] == true);
                        ui.set_sw_cursor(s["switchCursor"] == true);
                        ui.set_sw_auto(s["autostart"] == true);
                        ui.set_rise_off(s["sunriseOffsetMin"].as_i64().unwrap_or(0) as i32);
                        ui.set_set_off(s["sunsetOffsetMin"].as_i64().unwrap_or(0) as i32);
                        // 推迟时长档位: 15/30/60/120/240/480
                        let mins = s["snoozeMinutes"].as_i64().unwrap_or(15);
                        let snooze_idx = [15i64, 30, 60, 120, 240, 480]
                            .iter()
                            .position(|&m| m == mins)
                            .unwrap_or(0) as i32;
                        ui.set_snooze_idx(snooze_idx);
                    });
                }
            }
        });
    }

    // ---- 回调 ----
    {
        let weak = weak.clone();
        ui.on_to_light_clicked(move || {
            post("/theme/light", None);
            if let Some(u) = weak.upgrade() {
                u.set_light(true);
            }
        });
    }
    {
        let weak = weak.clone();
        ui.on_to_dark_clicked(move || {
            post("/theme/dark", None);
            if let Some(u) = weak.upgrade() {
                u.set_light(false);
            }
        });
    }
    fn toggle(weak: &slint::Weak<Dashboard>, key: &'static str, v: bool) {
        if let Some(c) = CFG.lock().unwrap().as_mut() {
            c[key] = serde_json::Value::Bool(v);
        }
        let owned = CFG.lock().unwrap().clone().unwrap_or_default();
        save_all(weak.clone(), owned);
    }
    {
        let weak = weak.clone();
        ui.on_apps_toggled(move || {
            if let Some(u) = weak.upgrade() {
                toggle(&weak, "switchApps", u.get_sw_apps());
            }
        });
    }
    {
        let weak = weak.clone();
        ui.on_system_toggled(move || {
            if let Some(u) = weak.upgrade() {
                toggle(&weak, "switchSystem", u.get_sw_system());
            }
        });
    }
    {
        let weak = weak.clone();
        ui.on_cursor_toggled(move || {
            if let Some(u) = weak.upgrade() {
                toggle(&weak, "switchCursor", u.get_sw_cursor());
            }
        });
    }
    {
        let weak = weak.clone();
        ui.on_auto_toggled(move || {
            if let Some(u) = weak.upgrade() {
                toggle(&weak, "autostart", u.get_sw_auto());
            }
        });
    }
    {
        let weak = weak.clone();
        ui.on_rise_changed(move |v| {
            let v = {
                let mut guard = CFG.lock().unwrap();
                if let Some(c) = guard.as_mut() {
                    let v = v.clamp(-720, 720);
                    c["sunriseOffsetMin"] = serde_json::Value::from(v);
                    v
                } else {
                    return;
                }
            };
            if let Some(ui) = weak.upgrade() {
                ui.set_rise_off(v);
            }
            let owned = CFG.lock().unwrap().clone().unwrap_or_default();
            save_all(weak.clone(), owned);
        });
    }
    {
        let weak = weak.clone();
        ui.on_sunset_changed(move |v| {
            let v = {
                let mut guard = CFG.lock().unwrap();
                if let Some(c) = guard.as_mut() {
                    let v = v.clamp(-720, 720);
                    c["sunsetOffsetMin"] = serde_json::Value::from(v);
                    v
                } else {
                    return;
                }
            };
            if let Some(ui) = weak.upgrade() {
                ui.set_set_off(v);
            }
            let owned = CFG.lock().unwrap().clone().unwrap_or_default();
            save_all(weak.clone(), owned);
        });
    }
    {
        // 推迟时长档位(分钟): 15/30/60/120/240/480
        const DUR: [i64; 6] = [15, 30, 60, 120, 240, 480];
        let weak = weak.clone();
        ui.on_snooze_dur_changed(move |idx| {
            if let Some(c) = CFG.lock().unwrap().as_mut() {
                c["snoozeMinutes"] = serde_json::Value::from(DUR[idx.clamp(0, 5) as usize]);
            }
            let owned = CFG.lock().unwrap().clone().unwrap_or_default();
            save_all(weak.clone(), owned);
        });
    }
    {
        let weak = weak.clone();
        ui.on_snooze_toggled(move |on| {
            let minutes = {
                let guard = CFG.lock().unwrap();
                guard.as_ref()
                    .and_then(|c| c["snoozeMinutes"].as_i64())
                    .unwrap_or(15)
            };
            let ok = if on {
                post("/snooze", Some(serde_json::json!({ "minutes": minutes })))
            } else {
                post("/snooze/clear", None)
            };
            if !ok {
                // 引擎不在线: 开关立刻回弹, 状态行由轮询刷新为未推迟
                if let Some(ui) = weak.upgrade() {
                    ui.set_snooze_active(false);
                }
            }
        });
    }
    {
        let weak = weak.clone();
        ui.on_light_scheme_changed(move |idx| {
            let list = SCHEMES.lock().unwrap().clone();
            if let Some(scheme) = list.get(idx as usize) {
                if let Some(c) = CFG.lock().unwrap().as_mut() {
                    c["cursorLightScheme"] = serde_json::Value::from(scheme.clone());
                }
            }
            weak.upgrade().map(|u| u.set_light_idx(idx));
            let owned = CFG.lock().unwrap().clone().unwrap_or_default();
            save_all(weak.clone(), owned);
        });
    }
    {
        let weak = weak.clone();
        ui.on_dark_scheme_changed(move |idx| {
            let list = SCHEMES.lock().unwrap().clone();
            if let Some(scheme) = list.get(idx as usize) {
                if let Some(c) = CFG.lock().unwrap().as_mut() {
                    c["cursorDarkScheme"] = serde_json::Value::from(scheme.clone());
                }
            }
            weak.upgrade().map(|u| u.set_dark_idx(idx));
            let owned = CFG.lock().unwrap().clone().unwrap_or_default();
            save_all(weak.clone(), owned);
        });
    }
    {
        let weak = weak.clone();
        ui.on_save_clicked(move || {
            let owned = CFG.lock().unwrap().clone().unwrap_or_default();
            save_all(weak.clone(), owned);
        });
    }
    ui.on_open_config_clicked(move || {
        let _ = std::process::Command::new("notepad.exe")
            .arg(crate::settings::config_path())
            .spawn();
    });

    // ---- 心跳轮询: /status → 界面属性 ----
    {
        let weak = weak.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(1));
            let Some(st) = get_json("/status") else {
                let weak = weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak.upgrade() {
                        ui.set_engine_ok(false);
                    }
                });
                continue;
            };
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                let Some(ui) = weak.upgrade() else { return };
                ui.set_engine_ok(true);
                ui.set_light(st["light"] == true);
                ui.set_next_when(st["nextWhen"].as_str().unwrap_or("").into());
                ui.set_next_light(st["nextLight"] == true);
                ui.set_loc(st["locationResolved"].as_str().unwrap_or("").into());
                ui.set_sunrise_t(st["sunrise"].as_str().unwrap_or("").into());
                ui.set_sunset_t(st["sunset"].as_str().unwrap_or("").into());
                ui.set_snooze_active(st["snoozeActive"] == true);
                ui.set_snooze_until(st["snoozeUntil"].as_str().unwrap_or("").into());
                ui.set_snooze_remain_min(st["snoozeRemainMin"].as_i64().unwrap_or(0).to_string().into());
            });
        });
    }

    ui.run().unwrap();
}
