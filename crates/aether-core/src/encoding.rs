//! 文本编码检测与解码（UTF-8 / UTF-16 BOM / GBK）。
//!
//! 用途：编辑器打开文件时判断编码，避免 GBK/UTF-16 等非 UTF-8 文件
//! 被误判为二进制拒绝打开，或按 lossy 解码产生乱码。

use std::borrow::Cow;

/// 检测出的文本编码
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEncoding {
    /// UTF-8（含无 BOM 与 UTF-8 BOM）
    Utf8,
    /// UTF-16 LE（带 BOM）
    Utf16Le,
    /// UTF-16 BE（带 BOM）
    Utf16Be,
    /// GBK / GB2312 / GB18030 兼容编码
    Gbk,
}

/// 检测字节样本的文本编码；无法判定为文本（二进制）时返回 `None`。
///
/// 判定顺序：
/// 1. BOM 嗅探（UTF-8 BOM / UTF-16 LE/BE BOM）
/// 2. 完整 UTF-8 校验（含纯 ASCII）
/// 3. GBK 尝试解码（无 replacement 字符才算命中，防二进制误判）
pub fn detect_encoding(bytes: &[u8]) -> Option<TextEncoding> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Some(TextEncoding::Utf8);
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return Some(TextEncoding::Utf16Le);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return Some(TextEncoding::Utf16Be);
    }
    if std::str::from_utf8(bytes).is_ok() {
        return Some(TextEncoding::Utf8);
    }
    if is_gbk_decodeable(bytes) {
        return Some(TextEncoding::Gbk);
    }
    None
}

/// GBK 可解码性判定：使用无 replacement 解码，全部字节都有映射才算命中。
fn is_gbk_decodeable(bytes: &[u8]) -> bool {
    let mut decoder = encoding_rs::GBK.new_decoder();
    let mut out = String::with_capacity(bytes.len() * 2 + 32);
    let (result, _) = decoder.decode_to_string_without_replacement(bytes, &mut out, true);
    result == encoding_rs::DecoderResult::InputEmpty && !out.is_empty()
}

/// 按检测结果解码为 UTF-8 字符串；无法检测时按 lossy UTF-8 处理。
pub fn decode_text(bytes: &[u8]) -> Cow<'_, str> {
    match detect_encoding(bytes) {
        None | Some(TextEncoding::Utf8) => String::from_utf8_lossy(bytes),
        Some(TextEncoding::Utf16Le) => {
            let body = bytes.strip_prefix(&[0xFF, 0xFE]).unwrap_or(bytes);
            let (text, _, _) = encoding_rs::UTF_16LE.decode(body);
            text.into_owned().into()
        }
        Some(TextEncoding::Utf16Be) => {
            let body = bytes.strip_prefix(&[0xFE, 0xFF]).unwrap_or(bytes);
            let (text, _, _) = encoding_rs::UTF_16BE.decode(body);
            text.into_owned().into()
        }
        Some(TextEncoding::Gbk) => {
            let (text, _, _) = encoding_rs::GBK.decode(bytes);
            text.into_owned().into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "中文测试" 的 GBK 字节
    const GBK_SAMPLE: &[u8] = b"\xd6\xd0\xce\xc4\xb2\xe2\xca\xd4";

    #[test]
    fn test_detect_utf8_and_ascii() {
        assert_eq!(detect_encoding(b"hello world"), Some(TextEncoding::Utf8));
        assert_eq!(
            detect_encoding("中文 UTF-8 内容".as_bytes()),
            Some(TextEncoding::Utf8)
        );
        assert_eq!(
            detect_encoding(b"\xEF\xBB\xBFhello"),
            Some(TextEncoding::Utf8)
        );
    }

    #[test]
    fn test_detect_utf16_bom() {
        assert_eq!(
            detect_encoding(b"\xFF\xFEh\x00i\x00"),
            Some(TextEncoding::Utf16Le)
        );
        assert_eq!(
            detect_encoding(b"\xFE\xFF\x00h\x00i"),
            Some(TextEncoding::Utf16Be)
        );
    }

    #[test]
    fn test_detect_gbk() {
        assert_eq!(detect_encoding(GBK_SAMPLE), Some(TextEncoding::Gbk));
    }

    #[test]
    fn test_detect_binary_rejected() {
        // 随机二进制（非 UTF-8 且 GBK 无完整映射）→ None
        let bin: Vec<u8> = (0u8..=255).collect();
        assert_eq!(detect_encoding(&bin), None);
        // 0xFF 既非合法 UTF-8 首字节也非 GBK 首字节（GBK 首字节 81-FE）-> None
        assert_eq!(detect_encoding(b"\x00\xff\x00\xff"), None);
    }

    #[test]
    fn test_decode_gbk() {
        let s = decode_text(GBK_SAMPLE);
        assert_eq!(s, "中文测试");
    }

    #[test]
    fn test_decode_utf16le_with_bom() {
        // "hi" 的 UTF-16LE + BOM
        let s = decode_text(b"\xFF\xFEh\x00i\x00");
        assert_eq!(s, "hi");
    }

    #[test]
    fn test_decode_utf8_lossy_fallback() {
        let s = decode_text(b"abc\xff\xfe");
        assert!(s.starts_with("abc"));
    }

    #[test]
    fn test_gbk_ascii_mixed() {
        // GBK 兼容 ASCII：纯 ASCII 部分照常
        let mixed = b"int main() {\xd6\xd0\xce\xc4}";
        assert_eq!(detect_encoding(mixed), Some(TextEncoding::Gbk));
        let s = decode_text(mixed);
        assert!(s.contains("int main() {"));
        assert!(s.contains("中文"));
    }
}
