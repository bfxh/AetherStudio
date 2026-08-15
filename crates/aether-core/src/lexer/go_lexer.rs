use super::common::{skip_block_comment, skip_line_comment, skip_quoted, skip_whitespace};
use super::{LexemeSpan, Lexer, TokenKind};

/// Go 语言词法分析器（独立实现，替代复用 C lexer）。
/// 覆盖：行/块注释、普通字符串、反引号原始字符串、rune、数字、标识符与 Go 关键词。
pub struct GoLexer;

impl GoLexer {
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
                if pos + 1 < bytes.len() {
                    match bytes[pos + 1] {
                        b'/' => {
                            let end = skip_line_comment(bytes, pos);
                            (LexemeSpan::new(pos, end - pos, TokenKind::LineComment), end)
                        }
                        b'*' => {
                            let end = skip_block_comment(bytes, pos);
                            (
                                LexemeSpan::new(pos, end - pos, TokenKind::BlockComment),
                                end,
                            )
                        }
                        b'=' => (LexemeSpan::new(pos, 2, TokenKind::Operator), pos + 2),
                        _ => (LexemeSpan::new(pos, 1, TokenKind::Operator), pos + 1),
                    }
                } else {
                    (LexemeSpan::new(pos, 1, TokenKind::Operator), pos + 1)
                }
            }
            // Go 原始字符串：反引号到下一个反引号（支持跨行）
            b'`' => {
                let end = skip_raw_string(bytes, pos);
                (
                    LexemeSpan::new(pos, end - pos, TokenKind::StringLiteral),
                    end,
                )
            }
            b'"' => {
                let end = skip_quoted(bytes, pos, b'"');
                (
                    LexemeSpan::new(pos, end - pos, TokenKind::StringLiteral),
                    end,
                )
            }
            b'\'' => {
                let end = skip_quoted(bytes, pos, b'\'');
                (LexemeSpan::new(pos, end - pos, TokenKind::CharLiteral), end)
            }
            b'0'..=b'9' => {
                let end = skip_number(bytes, pos);
                (
                    LexemeSpan::new(pos, end - pos, TokenKind::NumberLiteral),
                    end,
                )
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let end = skip_identifier(bytes, pos);
                let kind = if is_go_keyword(&bytes[pos..end]) {
                    TokenKind::Keyword
                } else {
                    TokenKind::Identifier
                };
                (LexemeSpan::new(pos, end - pos, kind), end)
            }
            b'+' | b'-' | b'*' | b'%' | b'=' | b'!' | b'<' | b'>' | b'&' | b'|' | b'^' | b':'
            | b'?' => {
                let end = skip_operator(bytes, pos);
                (LexemeSpan::new(pos, end - pos, TokenKind::Operator), end)
            }
            b'(' | b')' | b'{' | b'}' | b'[' | b']' | b',' | b';' | b'.' => {
                (LexemeSpan::new(pos, 1, TokenKind::Punctuation), pos + 1)
            }
            _ => {
                let len = crate::lexer::utf8_char_len(bytes[pos]);
                (LexemeSpan::new(pos, len, TokenKind::Unknown), pos + len)
            }
        }
    }
}

impl Lexer for GoLexer {
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

impl Default for GoLexer {
    fn default() -> Self {
        Self::new()
    }
}

fn is_go_keyword(bytes: &[u8]) -> bool {
    matches!(
        bytes,
        b"break"
            | b"case"
            | b"chan"
            | b"const"
            | b"continue"
            | b"default"
            | b"defer"
            | b"else"
            | b"fallthrough"
            | b"for"
            | b"func"
            | b"go"
            | b"goto"
            | b"if"
            | b"import"
            | b"interface"
            | b"map"
            | b"package"
            | b"range"
            | b"return"
            | b"select"
            | b"struct"
            | b"switch"
            | b"type"
            | b"var"
    )
}

/// 反引号原始字符串：直到下一个反引号（Go 原始字符串内无转义）
fn skip_raw_string(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos + 1;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            return i + 1;
        }
        i += 1;
    }
    i
}

fn skip_number(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'.')
    {
        i += 1;
    }
    i
}

fn skip_identifier(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    i
}

fn skip_operator(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos;
    while i < bytes.len() && b"+-*%=!<>&|^:?".contains(&bytes[i]) {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_keywords_and_identifiers() {
        let toks = GoLexer::new().lex_full("func main() { return 42 }");
        let kinds: Vec<TokenKind> = toks.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Keyword));
        assert!(kinds.contains(&TokenKind::Identifier));
        assert!(kinds.contains(&TokenKind::NumberLiteral));
        assert!(kinds.contains(&TokenKind::Punctuation));
    }

    #[test]
    fn test_go_raw_string() {
        let toks = GoLexer::new().lex_full("s := `raw\nstring`");
        assert!(toks.iter().any(|t| t.kind == TokenKind::StringLiteral));
    }

    #[test]
    fn test_go_comments() {
        let toks = GoLexer::new().lex_full("// line\n/* block */ x");
        assert!(toks.iter().any(|t| t.kind == TokenKind::LineComment));
        assert!(toks.iter().any(|t| t.kind == TokenKind::BlockComment));
    }

    #[test]
    fn test_go_channel_operator() {
        let toks = GoLexer::new().lex_full("ch <- 1");
        assert!(toks.iter().any(|t| t.kind == TokenKind::Operator));
    }
}
