//! 代码折叠区域检测。
//!
//! - C 家族（C/Cpp/Rust/Go/Java/JS/TS）：按大括号深度折叠
//! - Python：按缩进折叠
//! - 其余语言暂不折叠（返回空）
//!
//! 说明：当前实现不做字符串/注释内括号的跳过（简化版），折叠区域仅供
//! 编辑器侧槽标记与折叠/展开交互使用。

use crate::lexer::Language;

/// 折叠区域（行号均为 0 基）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoldingRegion {
    pub start_line: usize,
    pub end_line: usize,
}

/// 检测折叠区域
pub fn detect_folding_regions(lines: &[&str], lang: Language) -> Vec<FoldingRegion> {
    match lang {
        Language::C
        | Language::Cpp
        | Language::Rust
        | Language::Go
        | Language::Java
        | Language::JavaScript
        | Language::TypeScript => fold_by_braces(lines),
        Language::Python => fold_by_indent(lines),
        _ => Vec::new(),
    }
}

/// 大括号深度折叠：每行统计 { 与 } 数量；单行内开闭抵消的不产生区域。
fn fold_by_braces(lines: &[&str]) -> Vec<FoldingRegion> {
    let mut regions = Vec::new();
    // 栈：(起始行, 起始时深度)
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut depth = 0usize;
    for (idx, line) in lines.iter().enumerate() {
        let opens = line.bytes().filter(|b| *b == b'{').count();
        let closes = line.bytes().filter(|b| *b == b'}').count();
        for _ in 0..opens {
            depth += 1;
            stack.push((idx, depth));
        }
        for _ in 0..closes {
            if let Some((start, start_depth)) = stack.pop() {
                // 区域至少 2 行才折叠（单行 {} 不折叠）
                if idx > start && idx.saturating_sub(start) >= 1 {
                    regions.push(FoldingRegion {
                        start_line: start,
                        end_line: idx,
                    });
                }
                let _ = start_depth;
            }
            depth = depth.saturating_sub(1);
        }
        // 同层关闭后栈顶若深度不同（如 } 多于 { 的脏行），清理不一致栈顶
        while stack.last().is_some_and(|&(_, d)| d > depth) {
            stack.pop();
        }
    }
    regions
}

/// 缩进折叠（Python）：行首空格数减少时关闭区域。
///
/// 区域起点为缩进块的定义行（上一非空行）；同缩进不关闭（兄弟块延续）。
fn fold_by_indent(lines: &[&str]) -> Vec<FoldingRegion> {
    let mut regions = Vec::new();
    // 栈：(起始行, 缩进)
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        // 缩进减少：关闭所有更深区域（end = 当前行 - 1）
        while stack.last().is_some_and(|&(_, d)| d > indent) {
            let (start, _) = stack.pop().unwrap();
            if idx > start + 1 {
                regions.push(FoldingRegion {
                    start_line: start,
                    end_line: idx - 1,
                });
            }
        }
        if indent > 0 {
            if stack.is_empty() {
                // 新块：起点为定义行（上一行）
                stack.push((idx.saturating_sub(1), indent));
            } else if indent > stack.last().unwrap().1 {
                // 更深嵌套：起点为定义行（上一行）
                stack.push((idx.saturating_sub(1), indent));
            }
        }
    }
    // 文件尾：关闭所有剩余区域
    let last = lines.len().saturating_sub(1);
    while let Some((start, _)) = stack.pop() {
        // 区域至少 2 行（定义行 + 至少一行内容）
        if last >= start + 1 {
            regions.push(FoldingRegion {
                start_line: start,
                end_line: last,
            });
        }
    }
    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_style_nested() {
        let text = "fn main() {\n    let x = 1;\n    if x > 0 {\n        println!();\n    }\n}";
        let lines: Vec<&str> = text.lines().collect();
        let regions = detect_folding_regions(&lines, Language::Rust);
        // 外层函数区域 [0,5] 与 if 区域 [2,4]
        assert!(regions.contains(&FoldingRegion {
            start_line: 0,
            end_line: 5
        }));
        assert!(regions.contains(&FoldingRegion {
            start_line: 2,
            end_line: 4
        }));
    }

    #[test]
    fn test_single_line_brace_not_folded() {
        let text = "let x = { 1 };";
        let lines: Vec<&str> = text.lines().collect();
        let regions = detect_folding_regions(&lines, Language::Rust);
        assert!(regions.is_empty());
    }

    #[test]
    fn test_python_indent() {
        let text =
            "def f():\n    x = 1\n    if x:\n        y = 2\n    return x\ndef g():\n    pass";
        let lines: Vec<&str> = text.lines().collect();
        let regions = detect_folding_regions(&lines, Language::Python);
        // f 区域 [0,4]；if 区域 [2,3]；g 区域 [5,6]（无后续内容行则到文件尾）
        assert!(regions.contains(&FoldingRegion {
            start_line: 0,
            end_line: 4
        }));
        assert!(regions.contains(&FoldingRegion {
            start_line: 2,
            end_line: 3
        }));
        assert!(regions.contains(&FoldingRegion {
            start_line: 5,
            end_line: 6
        }));
    }

    #[test]
    fn test_unsupported_language_empty() {
        let lines: Vec<&str> = vec!["hello", "world"];
        assert!(detect_folding_regions(&lines, Language::Markdown).is_empty());
    }

    #[test]
    fn test_python_simple_block() {
        let text = "if True:\n    do_something()\nnext_line()";
        let lines: Vec<&str> = text.lines().collect();
        let regions = detect_folding_regions(&lines, Language::Python);
        assert!(regions.contains(&FoldingRegion {
            start_line: 0,
            end_line: 1
        }));
    }
}
