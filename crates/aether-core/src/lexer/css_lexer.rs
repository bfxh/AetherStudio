use super::common::{skip_block_comment, skip_quoted, skip_whitespace};
use super::{LexemeSpan, Lexer, TokenKind};

/// CSS 词法分析器（独立实现，替代复用 HTML lexer）。
/// 覆盖：块注释（CSS 无行注释）、字符串、数字（含 px/%/em 等单位）、
/// 标识符（属性/值/选择器）、at 规则（@media 等）、符号与颜色 #hex。
pub struct CssLexer;

impl CssLexer {
    pub fn new() -> Self {
        Self
    }

    fn lex_next(&self, bytes: &[u8], pos: usize) -> (LexemeSpan, usize) {
        if pos >= bytes.len() {
            return (LexemeSpan::new(pos, 0, TokenKind::EOF), pos);
        }
        let ch = bytes[pos];
        match ch {
            b' ' | b'\t' | b'\r' => {
                let end = skip_whitespace(bytes, pos);
                (LexemeSpan::new(pos, end - pos, TokenKind::Whitespace), end)
            }
            b'\n' => (LexemeSpan::new(pos, 1, TokenKind::Newline), pos + 1),
            b'/' => {
                if pos + 1 < bytes.len() && bytes[pos + 1] == b'*' {
                    let end = skip_block_comment(bytes, pos);
                    (
                        LexemeSpan::new(pos, end - pos, TokenKind::BlockComment),
                        end,
                    )
                } else {
                    (LexemeSpan::new(pos, 1, TokenKind::Operator), pos + 1)
                }
            }
            b'"' | b'\'' => {
                let end = skip_quoted(bytes, pos, ch);
                (
                    LexemeSpan::new(pos, end - pos, TokenKind::StringLiteral),
                    end,
                )
            }
            // at 规则：@media / @import —— 整段（含其后标识符）作为 Preprocessor
            b'@' => {
                let end = skip_at_rule(bytes, pos);
                (
                    LexemeSpan::new(pos, end - pos, TokenKind::Preprocessor),
                    end,
                )
            }
            // 颜色：#fff / #aabbcc
            b'#' => {
                let end = skip_hex_color(bytes, pos);
                (
                    LexemeSpan::new(pos, end - pos, TokenKind::NumberLiteral),
                    end,
                )
            }
            b'0'..=b'9' => {
                let end = skip_number_with_unit(bytes, pos);
                (
                    LexemeSpan::new(pos, end - pos, TokenKind::NumberLiteral),
                    end,
                )
            }
            b'-' => {
                if pos + 1 < bytes.len() && bytes[pos + 1].is_ascii_digit() {
                    let end = skip_number_with_unit(bytes, pos + 1);
                    (
                        LexemeSpan::new(pos, end - pos, TokenKind::NumberLiteral),
                        end,
                    )
                } else {
                    // 自定义属性 --var 或 标识符开头
                    let end = skip_identifier(bytes, pos);
                    if end > pos {
                        (LexemeSpan::new(pos, end - pos, TokenKind::Identifier), end)
                    } else {
                        (LexemeSpan::new(pos, 1, TokenKind::Operator), pos + 1)
                    }
                }
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let end = skip_identifier(bytes, pos);
                (LexemeSpan::new(pos, end - pos, TokenKind::Identifier), end)
            }
            b'!' => {
                // !important
                (LexemeSpan::new(pos, 1, TokenKind::Operator), pos + 1)
            }
            b'{' | b'}' | b'(' | b')' | b'[' | b']' | b':' | b';' | b',' | b'.' | b'*' | b'>'
            | b'+' | b'~' | b'=' => (LexemeSpan::new(pos, 1, TokenKind::Punctuation), pos + 1),
            _ => {
                let len = crate::lexer::utf8_char_len(bytes[pos]);
                (LexemeSpan::new(pos, len, TokenKind::Unknown), pos + len)
            }
        }
    }
}

impl Lexer for CssLexer {
    fn lex_full(&self, text: &str) -> Vec<LexemeSpan> {
        let mut tokens = Vec::with_capacity(text.len() / 4 + 1);
        let bytes = text.as_bytes();
        let mut pos = 0;
        while pos < bytes.len() {
            let (token, new_pos) = self.lex_next(bytes, pos);
            tokens.push(token);
            pos = new_pos;
        }
        tokens
    }
}

impl Default for CssLexer {
    fn default() -> Self {
        Self::new()
    }
}

/// at 规则：@ 后跟标识符（@media、@import、@keyframes 等）
fn skip_at_rule(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos + 1;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-') {
        i += 1;
    }
    i
}

/// 颜色 #hex：3 或 6 位十六进制
fn skip_hex_color(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos + 1;
    let mut n = 0;
    while i < bytes.len() && n < 6 && bytes[i].is_ascii_hexdigit() {
        i += 1;
        n += 1;
    }
    if n == 3 || n == 6 {
        i
    } else {
        pos + 1
    }
}

/// 数字 + 单位：12px / 1.5em / 50% / .5s
fn skip_number_with_unit(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos;
    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
        i += 1;
    }
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'%' {
        i += 1;
    }
    i
}

fn skip_identifier(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
    {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_css_rule_basic() {
        let toks = CssLexer::new().lex_full(".btn { color: red; margin: 12px; }");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Identifier));
        assert!(toks.iter().any(|t| t.kind == TokenKind::NumberLiteral));
        assert!(toks.iter().any(|t| t.kind == TokenKind::Punctuation));
    }

    #[test]
    fn test_css_at_rule() {
        let toks = CssLexer::new().lex_full("@media screen { body { color: red } }");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Preprocessor));
    }

    #[test]
    fn test_css_comment_and_string() {
        let toks = CssLexer::new().lex_full("/* c */ .a::before { content: \"x\"; }");
        assert!(toks.iter().any(|t| t.kind == TokenKind::BlockComment));
        assert!(toks.iter().any(|t| t.kind == TokenKind::StringLiteral));
    }

    #[test]
    fn test_css_number_units_and_color() {
        let toks = CssLexer::new().lex_full("width: 50%; height: 1.5em; color: #ff8800;");
        let nums: Vec<&LexemeSpan> = toks
            .iter()
            .filter(|t| t.kind == TokenKind::NumberLiteral)
            .collect();
        // NumberLiteral 至少 3 个（50% / 1.5em / #ff8800）
        assert!(nums.len() >= 3);
        // 颜色 #ff8800 应作为单个 7 字符 token（# + 6 hex）
        let color = nums.iter().find(|t| t.len == 7);
        assert!(color.is_some(), "hex color token missing");
    }
}
