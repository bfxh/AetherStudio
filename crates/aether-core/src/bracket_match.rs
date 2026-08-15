//! 括号匹配：给定光标位置，查找匹配的括号（`()` `[]` `{}`）。
//!
//! 跳过字符串/字符字面量/行注释/块注释中的括号，避免误匹配。
//! 纯逻辑、无 UI 依赖，供编辑器括号匹配高亮与跳转使用。

/// 匹配括号对
const PAIRS: &[(u8, u8)] = &[(b'(', b')'), (b'[', b']'), (b'{', b'}')];

/// 判断字节是否为开括号
pub fn is_open_bracket(b: u8) -> bool {
    PAIRS.iter().any(|(o, _)| *o == b)
}

/// 判断字节是否为闭括号
pub fn is_close_bracket(b: u8) -> bool {
    PAIRS.iter().any(|(_, c)| *c == b)
}

/// 找到与 `pos` 处括号匹配的括号字节位置；`pos` 处不是括号时返回 None。
///
/// - `pos` 处是开括号：向右扫描找匹配闭括号（跳过字符串/注释）
/// - `pos` 处是闭括号：向左扫描找匹配开括号
/// - 无匹配（未闭合/越界）返回 None
pub fn find_matching_bracket(text: &str, pos: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if pos >= bytes.len() {
        return None;
    }
    let ch = bytes[pos];
    if let Some(&(open, close)) = PAIRS.iter().find(|(o, _)| *o == ch) {
        // 向右：开括号 → 闭括号
        let mut depth = 1usize;
        let mut i = pos + 1;
        while i < bytes.len() {
            if is_skipped(bytes, i, true) {
                i = skip_region_end(bytes, i) + 1;
                continue;
            }
            match bytes[i] {
                b if b == open => depth += 1,
                b if b == close => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        None
    } else if let Some(&(open, close)) = PAIRS.iter().find(|(_, c)| *c == ch) {
        // 向左：闭括号 → 开括号
        let mut depth = 1usize;
        let mut i = pos;
        while i > 0 {
            i -= 1;
            if is_skipped(bytes, i, false) {
                i = skip_region_start(bytes, i);
                continue;
            }
            match bytes[i] {
                b if b == close => depth += 1,
                b if b == open => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        None
    } else {
        None
    }
}

/// 判断 `pos` 是否位于需跳过的区域（字符串/注释）。
fn is_skipped(bytes: &[u8], pos: usize, _forward: bool) -> bool {
    let b = bytes[pos];
    if b == b'"' || b == b'\x27' || b == b'\x60' {
        return true;
    }
    if b == b'/' && pos + 1 < bytes.len() && bytes[pos + 1] == b'/' {
        return true;
    }
    if b == b'/' && pos + 1 < bytes.len() && bytes[pos + 1] == b'*' {
        return true;
    }
    if b == b'#' {
        return true;
    }
    false
}

/// 跳过区域（向右扫描）：返回区域内最后一个字节的位置。
fn skip_region_end(bytes: &[u8], pos: usize) -> usize {
    match bytes[pos] {
        b'"' => skip_quoted(bytes, pos, b'"'),
        b'\x27' => skip_quoted(bytes, pos, b'\x27'),
        b'\x60' => skip_quoted(bytes, pos, b'\x60'),
        b'/' => {
            if pos + 1 < bytes.len() && bytes[pos + 1] == b'/' {
                skip_line(bytes, pos)
            } else if pos + 1 < bytes.len() && bytes[pos + 1] == b'*' {
                skip_block(bytes, pos)
            } else {
                pos
            }
        }
        b'#' => skip_line(bytes, pos),
        _ => pos,
    }
}

/// 跳过区域（向左扫描）：返回区域内最前的位置（调用方继续 i-1）。
fn skip_region_start(bytes: &[u8], pos: usize) -> usize {
    let b = bytes[pos];
    match b {
        b'"' | b'\x27' | b'\x60' => {
            let mut i = pos;
            while i > 0 {
                i -= 1;
                if bytes[i] == b {
                    return i;
                }
            }
            0
        }
        b'\n' => pos,
        b'/' => {
            if pos > 0 && bytes[pos - 1] == b'*' {
                let mut i = pos;
                while i > 0 {
                    i -= 1;
                    if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                        return i;
                    }
                }
                0
            } else {
                let mut i = pos;
                while i > 0 {
                    i -= 1;
                    if bytes[i] == b'\n' {
                        return i + 1;
                    }
                }
                0
            }
        }
        _ => pos,
    }
}

fn skip_quoted(bytes: &[u8], pos: usize, quote: u8) -> usize {
    let mut i = pos + 1;
    while i < bytes.len() {
        if bytes[i] == quote {
            return i;
        }
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        i += 1;
    }
    bytes.len().saturating_sub(1)
}

fn skip_line(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos;
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i.saturating_sub(1)
}

fn skip_block(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos + 2;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            return i + 1;
        }
        i += 1;
    }
    bytes.len().saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_pair() {
        assert_eq!(find_matching_bracket("(a)", 0), Some(2));
        assert_eq!(find_matching_bracket("(a)", 2), Some(0));
        assert_eq!(find_matching_bracket("[1, 2]", 0), Some(5));
        assert_eq!(find_matching_bracket("{x}", 0), Some(2));
    }

    #[test]
    fn test_nested() {
        assert_eq!(find_matching_bracket("((()))", 0), Some(5));
        assert_eq!(find_matching_bracket("((()))", 1), Some(4));
        assert_eq!(find_matching_bracket("a(b(c)d)e", 1), Some(7));
        assert_eq!(find_matching_bracket("a(b(c)d)e", 7), Some(1));
    }

    #[test]
    fn test_skips_strings_and_comments() {
        // 字符串内的括号不参与匹配
        let s = "fn f() { let x = \"{)\"); }";
        let ob = s.find('{').unwrap();
        assert_eq!(find_matching_bracket(s, ob), Some(s.len() - 1));
        // 行注释内的括号不参与匹配
        let s2 = "fn f() { // comment ( }
}";
        let ob2 = s2.find('{').unwrap();
        assert_eq!(find_matching_bracket(s2, ob2), Some(s2.len() - 1));
        // 块注释
        let s3 = "fn f() { /* ( */ }";
        let ob3 = s3.find('{').unwrap();
        assert_eq!(find_matching_bracket(s3, ob3), Some(s3.len() - 1));
    }

    #[test]
    fn test_unmatched_returns_none() {
        assert_eq!(find_matching_bracket("(a", 0), None);
        assert_eq!(find_matching_bracket("a)", 1), None);
        assert_eq!(find_matching_bracket("", 0), None);
        assert_eq!(find_matching_bracket("abc", 1), None);
    }

    #[test]
    fn test_reverse_scan_nested() {
        assert_eq!(find_matching_bracket("a(b(c)d)", 7), Some(1));
        assert_eq!(find_matching_bracket("((a))", 4), Some(0));
    }

    #[test]
    fn test_mixed_pairs() {
        assert_eq!(find_matching_bracket("([)]", 0), Some(2));
        assert_eq!(find_matching_bracket("[()]", 0), Some(3));
    }
}
