use std::net::{Ipv4Addr, Ipv6Addr};
use url;
use chrono::{Utc, Local};
use chrono_tz::Tz;
use std::str::FromStr;
use std::collections::HashMap;
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref ANSI_CACHE: Mutex<HashMap<String, &'static str>> = Mutex::new(HashMap::new());
}

/// # 将 16 进制颜色转换为 ANSI 颜色代码
/// ## 参数
/// - hex: &str
/// ## 返回值
/// - &str
pub fn hex_to_ansi(hex: &str) -> &str {
    // 删除 #
    let hex = hex.trim_start_matches('#');
    
    // 检查缓存
    let hex_string = format!("#{}", hex);
    {
        let cache = ANSI_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(&hex_string) {
            return cached;
        }
    }
    
    // HEX -> RGB
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
    
    // 返回 ANSI
    let ansi_code = format!("\x1b[38;2;{};{};{}m", r, g, b);
    let static_str = Box::leak(ansi_code.into_boxed_str());
    ANSI_CACHE.lock().unwrap().insert(hex_string, static_str);
    static_str
}

/// # 验证是否为 IP 地址
/// ## 参数
/// - IP: &str
/// ## 返回值
/// - bool
#[allow(dead_code)]
pub fn is_valid_ip(ip: &str) -> bool {
    is_valid_ipv4(ip) || is_valid_ipv6(ip)
}


/// # 验证是否为 IPv4 地址
/// ## 参数
/// - IP: &str
/// ## 返回值
/// - bool
pub fn is_valid_ipv4(ip: &str) -> bool {
    ip.parse::<Ipv4Addr>().is_ok()
}


/// # 验证是否为 IPv6 地址
/// ## 参数
/// - IP: &str
/// ## 返回值
/// - bool
pub fn is_valid_ipv6(ip: &str) -> bool {
    ip.parse::<Ipv6Addr>().is_ok()
}


/// # 验证是否为网址
/// ## 参数
/// - url: &str
/// ## 返回值
/// - bool
#[allow(dead_code)]
pub fn is_valid_url(url: &str) -> bool {
    url::Url::parse(url).is_ok()
}

/// # 获取当前时间（格式化）
/// ## 格式
/// 2025年7月29日 16:30
/// ## 参数
/// - time_zone: &str
/// ## 返回值
/// - String
pub fn get_current_time(time_zone: &str) -> String {
    match Tz::from_str(time_zone) {
        Ok(tz) => {
            let now = Utc::now().with_timezone(&tz);
            now.format("%Y年%-m月%d日 %H:%M").to_string()
        },
        Err(_) => {
            // 如果时区解析失败，使用本地时间
            let now = Local::now();
            now.format("%Y年%-m月%d日 %H:%M").to_string()
        }
    }
}

