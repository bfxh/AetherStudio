use super::common::{skip_block_comment, skip_line_comment, skip_quoted, skip_whitespace};
use super::{LexemeSpan, Lexer, TokenKind};

/// Java 语言词法分析器（独立实现，替代复用 C lexer）。
/// 覆盖：行/块/文档注释、字符串、char、数字、标识符与 Java 关键词、注解标记。
pub struct JavaLexer;

impl JavaLexer {
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
                            let kind = if bytes[pos..end].starts_with(b"/**")
                                && !bytes[pos..end].starts_with(b"/**/")
                            {
                                TokenKind::DocComment
                            } else {
                                TokenKind::BlockComment
                            };
                            (LexemeSpan::new(pos, end - pos, kind), end)
                        }
                        b'=' => (LexemeSpan::new(pos, 2, TokenKind::Operator), pos + 2),
                        _ => (LexemeSpan::new(pos, 1, TokenKind::Operator), pos + 1),
                    }
                } else {
                    (LexemeSpan::new(pos, 1, TokenKind::Operator), pos + 1)
                }
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
            // Java 注解：@Override —— @ 作为 Punctuation，其后标识符
            b'@' => (LexemeSpan::new(pos, 1, TokenKind::Punctuation), pos + 1),
            b'0'..=b'9' => {
                let end = skip_number(bytes, pos);
                (
                    LexemeSpan::new(pos, end - pos, TokenKind::NumberLiteral),
                    end,
                )
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let end = skip_identifier(bytes, pos);
                let kind = if is_java_keyword(&bytes[pos..end]) {
                    TokenKind::Keyword
                } else {
                    TokenKind::Identifier
                };
                (LexemeSpan::new(pos, end - pos, kind), end)
            }
            b'+' | b'-' | b'*' | b'%' | b'=' | b'!' | b'<' | b'>' | b'&' | b'|' | b'^' | b'~'
            | b'?' => {
                let end = skip_operator(bytes, pos);
                (LexemeSpan::new(pos, end - pos, TokenKind::Operator), end)
            }
            b'(' | b')' | b'{' | b'}' | b'[' | b']' | b',' | b';' | b':' | b'.' => {
                (LexemeSpan::new(pos, 1, TokenKind::Punctuation), pos + 1)
            }
            _ => {
                let len = crate::lexer::utf8_char_len(bytes[pos]);
                (LexemeSpan::new(pos, len, TokenKind::Unknown), pos + len)
            }
        }
    }
}

impl Lexer for JavaLexer {
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

impl Default for JavaLexer {
    fn default() -> Self {
        Self::new()
    }
}

fn is_java_keyword(bytes: &[u8]) -> bool {
    matches!(
        bytes,
        b"abstract"
            | b"assert"
            | b"boolean"
            | b"break"
            | b"byte"
            | b"case"
            | b"catch"
            | b"char"
            | b"class"
            | b"const"
            | b"continue"
            | b"default"
            | b"do"
            | b"double"
            | b"else"
            | b"enum"
            | b"extends"
            | b"final"
            | b"finally"
            | b"float"
            | b"for"
            | b"goto"
            | b"if"
            | b"implements"
            | b"import"
            | b"instanceof"
            | b"int"
            | b"interface"
            | b"long"
            | b"native"
            | b"new"
            | b"package"
            | b"private"
            | b"protected"
            | b"public"
            | b"return"
            | b"short"
            | b"static"
            | b"strictfp"
            | b"super"
            | b"switch"
            | b"synchronized"
            | b"this"
            | b"throw"
            | b"throws"
            | b"transient"
            | b"try"
            | b"void"
            | b"volatile"
            | b"while"
    )
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
    while i < bytes.len() && b"+-*%=!<>&|^~?".contains(&bytes[i]) {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_java_keywords_and_identifiers() {
        let toks = JavaLexer::new().lex_full("public class Foo { return 0; }");
        let kinds: Vec<TokenKind> = toks.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Keyword));
        assert!(kinds.contains(&TokenKind::Identifier));
        assert!(kinds.contains(&TokenKind::NumberLiteral));
    }

    #[test]
    fn test_java_doc_comment() {
        let toks = JavaLexer::new().lex_full("/** doc */ int x;");
        assert!(toks.iter().any(|t| t.kind == TokenKind::DocComment));
    }

    #[test]
    fn test_java_annotation() {
        let toks = JavaLexer::new().lex_full("@Override void f() {}");
        assert!(toks
            .iter()
            .any(|t| t.kind == TokenKind::Punctuation && t.len == 1));
        assert!(toks.iter().any(|t| t.kind == TokenKind::Identifier));
    }

    #[test]
    fn test_java_string_and_char() {
        let toks = JavaLexer::new().lex_full("String s = \"hi\"; char c = 'x';");
        assert!(toks.iter().any(|t| t.kind == TokenKind::StringLiteral));
        assert!(toks.iter().any(|t| t.kind == TokenKind::CharLiteral));
    }
}
