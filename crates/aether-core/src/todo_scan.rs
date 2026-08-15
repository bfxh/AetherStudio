//! TODO/FIXME 注释扫描：识别代码注释中的待办标记，供问题面板展示。
//!
//! 识别标记：TODO / FIXME / XXX / HACK（大小写不敏感，可带冒号）。
//! 只扫描注释内容（行注释 //、# 与块注释 /* */），跳过字符串与代码。

/// 待办项
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodoItem {
    /// 0 基行号
    pub line: usize,
    /// 0 基列号（标记起始位置）
    pub col: usize,
    /// 标记类型（TODO/FIXME/XXX/HACK）
    pub kind: String,
    /// 标记后的说明文字（trim 后；可能为空）
    pub text: String,
}

const MARKERS: &[&str] = &["TODO", "FIXME", "XXX", "HACK"];

/// 扫描单行文本中的 TODO 标记（line_no 供结果使用）。
fn scan_line(line: &str, line_no: usize) -> Option<TodoItem> {
    for m in MARKERS {
        // 大小写不敏感匹配
        let lower = line.to_lowercase();
        let m_lower = m.to_lowercase();
        if let Some(idx) = lower.find(m_lower.as_str()) {
            // 标记后必须是空白、冒号或行尾（避免误匹配 TODOList 等标识符）
            let after = line[idx + m.len()..].chars().next();
            let valid_after =
                after.is_none() || after.is_some_and(|c| c.is_whitespace() || c == ':' || c == '(');
            if valid_after {
                // 提取说明文字（去掉标记与冒号）
                let rest = line[idx + m.len()..].trim();
                let text = rest
                    .strip_prefix(':')
                    .or_else(|| rest.strip_prefix('(').and_then(|r| r.strip_suffix(')')))
                    .unwrap_or(rest)
                    .trim()
                    .to_string();
                return Some(TodoItem {
                    line: line_no,
                    col: idx,
                    kind: m.to_string(),
                    text,
                });
            }
        }
    }
    None
}

/// 扫描文本中的 TODO 注释项。
///
/// `text` 为完整文件内容；返回所有注释内命中的待办项（按行序）。
pub fn scan_todo(text: &str) -> Vec<TodoItem> {
    let mut items = Vec::new();
    let mut in_block_comment = false;
    for (line_no, raw) in text.lines().enumerate() {
        let line = raw.trim_start();
        // 块注释状态跟踪（简化：/* 到 */ 同一行内闭合）
        if in_block_comment {
            if let Some(end) = line.find("*/") {
                in_block_comment = false;
                if let Some(item) = scan_line(&line[..end + 2], line_no) {
                    items.push(item);
                }
                continue;
            }
            // 仍在块注释内
            if let Some(item) = scan_line(line, line_no) {
                items.push(item);
            }
            continue;
        }
        // 行注释 / 块注释开头
        let comment_start = line
            .find("//")
            .or_else(|| line.find('#'))
            .or_else(|| line.find("/*"));
        if let Some(cs) = comment_start {
            let comment = &line[cs..];
            if comment.starts_with("/*") && !comment.contains("*/") {
                in_block_comment = true;
            }
            if let Some(item) = scan_line(comment, line_no) {
                items.push(item);
            }
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_todo_and_fixme() {
        let text = "fn main() {\n    // TODO: 实现逻辑\n    // FIXME: 内存泄漏\n}";
        let items = scan_todo(text);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].kind, "TODO");
        assert_eq!(items[0].text, "实现逻辑");
        assert_eq!(items[0].line, 1);
        assert_eq!(items[1].kind, "FIXME");
        assert_eq!(items[1].line, 2);
    }

    #[test]
    fn test_case_insensitive_and_variants() {
        let text = "// todo 小写\n# HACK\n// XXX: 需要优化\n// fixme:无冒号空格";
        let items = scan_todo(text);
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].kind, "TODO");
        assert_eq!(items[1].kind, "HACK");
        assert_eq!(items[2].kind, "XXX");
        assert_eq!(items[3].kind, "FIXME");
    }

    #[test]
    fn test_skips_non_comment() {
        // 字符串里的 TODO 不算
        let text = "let s = \"TODO: 不是注释\";\n// TODO: 真注释";
        let items = scan_todo(text);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].line, 1);
    }

    #[test]
    fn test_block_comment() {
        let text = "/*\n * TODO: 块注释内\n * FIXME\n */\n// TODO: 行注释";
        let items = scan_todo(text);
        assert!(items.iter().any(|i| i.kind == "TODO" && i.line == 1));
        assert!(items.iter().any(|i| i.kind == "FIXME" && i.line == 2));
        assert!(items.iter().any(|i| i.line == 4));
    }

    #[test]
    fn test_identifier_not_matched() {
        // TODOList / FIXMEHandler 是标识符，不是标记
        let text = "let todolist = 1;\n// TODOList: 不匹配\n// TODO: 匹配";
        let items = scan_todo(text);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "匹配");
    }
}
