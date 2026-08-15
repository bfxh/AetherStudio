pub mod client;
pub mod session;
pub mod transport;
pub mod types;

/// 获取系统语言区域名（如 "zh-CN" / "en-US"）；DAP initialize 的 locale 字段使用。
#[cfg(windows)]
pub fn system_locale() -> String {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buf = [0u16; 64];
    let n = unsafe { GetUserDefaultLocaleName(&mut buf) };
    if n > 0 {
        String::from_utf16_lossy(&buf[..n as usize - 1])
    } else {
        "en-US".to_string()
    }
}

/// 非 Windows 平台：回退到 LANG 环境变量
#[cfg(not(windows))]
pub fn system_locale() -> String {
    std::env::var("LANG")
        .map(|l| l.split('.').next().unwrap_or("en-US").to_string())
        .unwrap_or_else(|_| "en-US".to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_system_locale_non_empty() {
        let loc = super::system_locale();
        assert!(!loc.is_empty());
        assert!(loc.len() < 64);
    }
}

pub use client::DapClient;
pub use types::*;
