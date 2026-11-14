// controller.rs — 本机 JSON 控制接口(127.0.0.1:27125), 供仪表盘 UI 调用
// 端点: GET /status | GET+POST /settings | POST /theme/light|dark | GET /schemes | POST /snooze|/snooze/clear | POST /reload
use crate::settings;
use crate::theme;
use crate::SNOOZE_UNTIL;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tiny_http::{Header, Method, Response, Server};

pub const PORT: u16 = 27125;

// 引擎心跳每秒写入, HTTP 线程只读
#[derive(Clone, Default)]
pub struct EngineStatus {
    pub light: bool,
    pub next_when: String, // "HH:MM", 空串 = 获取中
    pub next_light: bool,
}

// UI/接口触发的重算请求, 心跳消费后清零
pub static RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn request_reload() {
    RELOAD_REQUESTED.store(true, Ordering::SeqCst);
}

pub fn consume_reload() -> bool {
    RELOAD_REQUESTED.swap(false, Ordering::SeqCst)
}

pub fn serve() {
    let server = match Server::http(("127.0.0.1", PORT)) {
        Ok(s) => s,
        Err(_) => return, // 端口被占: 放弃接口, 托盘功能不受影响
    };
    let json_hdr = Header::from_bytes("Content-Type", "application/json").unwrap();
    for mut request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().trim_end_matches('/').to_string();
        let body = {
            let mut buf = String::new();
            let _ = request.as_reader().read_to_string(&mut buf);
            buf
        };

        let (code, payload) = match (&method, url.as_str()) {
            (Method::Get, "/status") => {
                let st = crate::ENGINE_STATE.read().unwrap().clone();
                let s = settings::get();
                let lat = latlon_of(&s);
                // 今日日出日落(本地 NOAA, 即算即用)
                let (sunrise, sunset) = match crate::sun::get_sun_event(&lat, true, 0) {
                    Ok(t) => match crate::sun::get_sun_event(&lat, false, 0) {
                        Ok(t2) => (t.format("%H:%M").to_string(), t2.format("%H:%M").to_string()),
                        Err(_) => (String::new(), String::new()),
                    },
                    Err(_) => (String::new(), String::new()),
                };
                // 推迟状态: 剩余分钟(向上取整, 至少 1)
                let guard = SNOOZE_UNTIL.lock().unwrap();
                let snoozing = guard.map(|t| Instant::now() < t);
                let (snooze_until, snooze_remain) = match snoozing {
                    Some(true) => {
                        let left = guard.unwrap().saturating_duration_since(Instant::now());
                        let until = chrono::Local::now() + chrono::Duration::from_std(left).unwrap_or_default();
                        (until.format("%H:%M").to_string(), (left.as_secs() + 59) / 60)
                    }
                    _ => (String::new(), 0),
                };
                drop(guard);
                (
                    200,
                    json!({
                        "light": st.light,
                        "nextWhen": st.next_when,
                        "nextLight": st.next_light,
                        "locationResolved": lat,
                        "sunrise": sunrise,
                        "sunset": sunset,
                        "snoozeActive": snoozing.unwrap_or(false),
                        "snoozeUntil": snooze_until,
                        "snoozeRemainMin": snooze_remain,
                        "engine": "running",
                    })
                    .to_string(),
                )
            }
            (Method::Get, "/settings") => (
                200,
                serde_json::to_string(&settings::get()).unwrap_or_default(),
            ),
            (Method::Post, "/settings") => match serde_json::from_str::<settings::Settings>(&body) {
                Ok(new_s) => {
                    let old = settings::get();
                    if old.autostart != new_s.autostart {
                        theme::set_autostart(new_s.autostart);
                    }
                    settings::set(new_s.clone());
                    settings::save(&new_s);
                    request_reload();
                    (200, json!({ "ok": true }).to_string())
                }
                Err(e) => (400, json!({ "ok": false, "error": e.to_string() }).to_string()),
            },
            (Method::Post, "/theme/light") => {
                theme::set_theme(true);
                request_reload();
                (200, json!({ "ok": true, "light": true }).to_string())
            }
            (Method::Post, "/theme/dark") => {
                theme::set_theme(false);
                request_reload();
                (200, json!({ "ok": true, "light": false }).to_string())
            }
            (Method::Get, "/schemes") => (200, cursor_schemes_json()),
            (Method::Post, "/snooze") => {
                // body: {"minutes": 15} — 只认正数, 上限 8 小时
                let minutes = serde_json::from_str::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|v| v["minutes"].as_i64())
                    .unwrap_or(0);
                if !(1..=480).contains(&minutes) {
                    (400, json!({ "ok": false, "error": "minutes must be 1..=480" }).to_string())
                } else {
                    *SNOOZE_UNTIL.lock().unwrap() = Some(Instant::now() + std::time::Duration::from_secs(minutes as u64 * 60));
                    (200, json!({ "ok": true, "minutes": minutes }).to_string())
                }
            }
            (Method::Post, "/snooze/clear") => {
                *SNOOZE_UNTIL.lock().unwrap() = None;
                (200, json!({ "ok": true }).to_string())
            }
            (Method::Post, "/reload") => {
                request_reload();
                (200, json!({ "ok": true }).to_string())
            }
            _ => (404, json!({ "error": "not found" }).to_string()),
        };

        let resp = Response::from_string(payload)
            .with_status_code(code)
            .with_header(json_hdr.clone());
        let _ = request.respond(resp);
    }
}

fn latlon_of(s: &settings::Settings) -> String {
    let t = s.location.trim();
    if t.is_empty() {
        return crate::RESOLVED_LATLON.lock().unwrap().clone();
    }
    t.to_string()
}

fn cursor_schemes_json() -> String {
    let list = crate::cursor::get_cursor_schemes();
    json!({ "schemes": list }).to_string()
}
