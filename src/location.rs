// location.rs — IP 定位(ip-api.com)与地名解析(Nominatim), 免费无 key
const UA: &str = "ThemeSwitcher2/2.0 (github.com/Bais233/ThemeSwitcher2)";

pub fn http_get(url: &str) -> Result<String, String> {
    let resp = ureq::get(url)
        .set("User-Agent", UA)
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .map_err(|e| e.to_string())?;
    resp.into_string().map_err(|e| e.to_string())
}

pub fn detect_location() -> Result<String, String> {
    let json = http_get("http://ip-api.com/json/?fields=status,lat,lon")?;
    if !json.contains("\"status\":\"success\"") {
        return Err(format!("ip-api failed: {}", json));
    }
    Ok(format!("{},{}", json_num(&json, "lat")?, json_num(&json, "lon")?))
}

pub fn geocode(place: &str) -> Result<String, String> {
    let url = format!(
        "https://nominatim.openstreetmap.org/search?format=json&limit=1&q={}",
        urlencode(place)
    );
    let json = http_get(&url)?;
    if json.len() < 5 {
        return Err(format!("place not found: {}", place));
    }
    Ok(format!("{},{}", json_num(&json, "lat")?, json_num(&json, "lon")?))
}

// 极简 JSON 字段提取(兼容数字与带引号的字符串/日期), 移植自 C# 版 JsonNum
pub fn json_num(json: &str, field: &str) -> Result<String, String> {
    let pat = format!("\"{}\"", field);
    let mut i = json.find(&pat).ok_or_else(|| format!("field not found: {}", field))?;
    let bytes = json.as_bytes();
    i += pat.len();
    while i < bytes.len() && bytes[i] != b':' {
        i += 1;
    }
    i += 1;
    while i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b' ') {
        i += 1;
    }
    let start = i;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric()
            || bytes[i] == b'-'
            || bytes[i] == b'.'
            || bytes[i] == b'+'
            || bytes[i] == b':')
    {
        i += 1;
    }
    let mut end = i;
    while end > start && bytes[end - 1] == b'"' {
        end -= 1;
    }
    Ok(json[start..end].to_string())
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
