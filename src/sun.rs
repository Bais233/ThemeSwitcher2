// sun.rs — 日出日落本地计算(NOAA 天文算法, 精度约 1 分钟, 不依赖网络)
// 与 sunrise-sunset.org 同一套公式; 时区按系统本地时区换算
use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};

pub fn get_sun_event(latlon: &str, sunrise: bool, day_offset: i64) -> Result<DateTime<Local>, String> {
    let date = Local::now().date_naive() + chrono::Duration::days(day_offset);
    compute_sun_event_local(latlon, sunrise, date).ok_or_else(|| format!("sun calc failed: {}", latlon))
}

fn compute_sun_event_local(latlon: &str, sunrise: bool, date: NaiveDate) -> Option<DateTime<Local>> {
    let (lat_s, lon_s) = latlon.split_once(',')?;
    let lat: f64 = lat_s.trim().parse().ok()?;
    let lon: f64 = lon_s.trim().parse().ok()?;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return None;
    }

    let n = date.ordinal() as f64;
    let lng_hour = lon / 15.0;
    let t = if sunrise {
        n + (6.0 - lng_hour) / 24.0
    } else {
        n + (18.0 - lng_hour) / 24.0
    };

    // 太阳真黄经
    let m = 0.9856 * t - 3.289;
    let l = norm360(m + 1.916 * sin_d(m) + 0.020 * sin_d(2.0 * m) + 282.634);

    // 赤经(修正象限), 转小时
    let mut ra = atan_d(0.91764 * tan_d(l));
    ra += f64::floor(l / 90.0) * 90.0 - f64::floor(ra / 90.0) * 90.0;
    ra /= 15.0;

    // 太阳赤纬
    let sin_dec = 0.39782 * sin_d(l);
    let cos_dec = f64::cos(f64::asin(sin_dec));

    // 时角(天顶距 90.833° 含大气折射)
    let rad = std::f64::consts::PI / 180.0;
    let cos_h = (cos_d(90.833) - sin_dec * f64::sin(lat * rad)) / (cos_dec * f64::cos(lat * rad));
    if !(-1.0..=1.0).contains(&cos_h) {
        return None; // 极昼/极夜
    }
    let mut h = if sunrise { 360.0 - acos_d(cos_h) } else { acos_d(cos_h) };
    h /= 15.0;

    let ut_hours = h + ra - 0.06571 * t - 6.622 - lng_hour;
    let utc_ndt: NaiveDateTime =
        date.and_hms_opt(0, 0, 0)? + chrono::Duration::minutes((ut_hours * 60.0).round() as i64);
    let dt = DateTime::<Utc>::from_naive_utc_and_offset(utc_ndt, Utc);
    Some(dt.with_timezone(&Local))
}

pub fn next_event(
    latlon: &str,
    rise_off: i64,
    set_off: i64,
    now: DateTime<Local>,
) -> Result<(DateTime<Local>, bool), String> {
    for day in 0..=1i64 {
        let rise = get_sun_event(latlon, true, day)
            .ok()
            .map(|t| t + chrono::Duration::minutes(rise_off));
        let set = get_sun_event(latlon, false, day)
            .ok()
            .map(|t| t + chrono::Duration::minutes(set_off));
        if rise.is_none() && set.is_none() {
            break;
        }
        if let Some(r) = rise {
            if now < r {
                return Ok((r, true));
            }
        }
        if let Some(s) = set {
            if now < s {
                return Ok((s, false));
            }
        }
    }
    Err("no sun data".into())
}

// 当前时刻应处的模式: 日出(含偏移)后到日落(含偏移)前 = 浅色, 其余 = 深色
pub fn current_light_for(latlon: &str, rise_off: i64, set_off: i64, now: DateTime<Local>) -> Option<bool> {
    let rise = get_sun_event(latlon, true, 0).ok()? + chrono::Duration::minutes(rise_off);
    let set = get_sun_event(latlon, false, 0).ok()? + chrono::Duration::minutes(set_off);
    Some(now >= rise && now < set)
}

fn sin_d(deg: f64) -> f64 {
    deg.to_radians().sin()
}
fn cos_d(deg: f64) -> f64 {
    deg.to_radians().cos()
}
fn tan_d(deg: f64) -> f64 {
    deg.to_radians().tan()
}
fn acos_d(v: f64) -> f64 {
    v.clamp(-1.0, 1.0).acos() * 180.0 / std::f64::consts::PI
}
fn atan_d(v: f64) -> f64 {
    v.atan() * 180.0 / std::f64::consts::PI
}
fn norm360(v: f64) -> f64 {
    let r = v % 360.0;
    if r < 0.0 {
        r + 360.0
    } else {
        r
    }
}
