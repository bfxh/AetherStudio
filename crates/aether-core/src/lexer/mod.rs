/// 通用词法分析器 trait
pub trait Lexer {
    /// 对单行文本进行全量词法分析
    fn lex_full(&self, text: &str) -> Vec<LexemeSpan>;
}

/// 通用 Token 类型（跨语言统一）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TokenKind {
    // === 通用类别 ===
    // 关键字
    Keyword,
    // 标识符
    Identifier,
    // 字符串字面量
    StringLiteral,
    // 字符字面量
    CharLiteral,
    // 数字字面量
    NumberLiteral,
    // 注释
    LineComment,
    BlockComment,
    DocComment,
    // 运算符
    Operator,
    // 分隔符/标点
    Punctuation,
    // 预处理/指令
    Preprocessor,
    // 属性/注解/装饰器
    Attribute,
    // 类型名
    TypeName,
    // 函数名/方法名
    Function,
    // 宏
    Macro,
    // 生命周期（Rust专用）
    Lifetime,
    // 模板/泛型参数
    Generic,
    // 正则表达式字面量
    RegexLiteral,
    // 格式化字符串
    FormatString,
    // Markdown 标题
    MdHeading,
    // Markdown 链接
    MdLink,
    // Markdown 代码标记
    MdCode,
    // Markdown 强调
    MdEmphasis,
    // JSON 键
    JsonKey,
    // TOML 表头
    TomlTable,
    // 空白
    Whitespace,
    // 换行
    Newline,
    // 未知
    Unknown,
    // 文件结束
    EOF,
}

/// 词法单元跨度
///
/// P0-A: 压缩从 24B 到 12B，单行内偏移不超过 4GB，单 token 长度不超过 4GB
#[derive(Clone, Debug, PartialEq)]
pub struct LexemeSpan {
    pub start: u32,
    pub len: u32,
    pub kind: TokenKind,
    pub flags: u8,
}

impl LexemeSpan {
    /// 便捷构造：接受 usize 自动截断为 u32（单行不可能超过 4GB）
    #[inline]
    pub fn new(start: usize, len: usize, kind: TokenKind) -> Self {
        Self {
            start: start as u32,
            len: len as u32,
            kind,
            flags: 0,
        }
    }
}

/// 语言类型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    C,
    Cpp,
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    Json,
    Markdown,
    Toml,
    Html,
    Css,
    PlainText,
    Image,
}

/// 语言注册表条目（单一来源，见 [`Language::ALL`]）
pub struct LanguageSpec {
    pub lang: Language,
    /// 文件扩展名（小写，不含点）
    pub extensions: &'static [&'static str],
    /// tree-sitter grammar id
    pub ts_id: Option<&'static str>,
    /// LSP language_id
    pub lsp_id: Option<&'static str>,
    /// 状态栏显示 id
    pub display_id: &'static str,
}

impl Language {
    /// 语言注册表（单一来源）：扩展名、tree-sitter grammar id、LSP language_id、
    /// 状态栏显示 id 全部集中在此，禁止在其它模块再写散落的 match 映射。
    ///
    /// 约定：
    /// - `ts_id`：有 tree-sitter grammar 的语言返回 id，否则 None（fallback 手写 lexer）
    /// - `lsp_id`：有默认 LSP 服务器配置的语言返回 id，否则 None（不自动启动 LSP）
    /// - `extensions` 为空表示无扩展名映射（PlainText 兜底）
    pub const ALL: &'static [LanguageSpec] = &[
        LanguageSpec {
            lang: Language::C,
            extensions: &["c", "h", "m", "mm"],
            ts_id: Some("c"),
            lsp_id: Some("c"),
            display_id: "c",
        },
        LanguageSpec {
            lang: Language::Cpp,
            extensions: &["cpp", "hpp", "cc", "cxx"],
            ts_id: Some("cpp"),
            lsp_id: Some("cpp"),
            display_id: "cpp",
        },
        LanguageSpec {
            lang: Language::Rust,
            extensions: &["rs"],
            ts_id: Some("rust"),
            lsp_id: Some("rust"),
            display_id: "rust",
        },
        LanguageSpec {
            lang: Language::Python,
            extensions: &["py", "pyw", "pyi", "pyx", "pxd"],
            ts_id: Some("python"),
            lsp_id: Some("python"),
            display_id: "python",
        },
        LanguageSpec {
            lang: Language::JavaScript,
            extensions: &["js", "jsx", "mjs", "cjs", "es", "es6"],
            ts_id: Some("javascript"),
            lsp_id: Some("javascript"),
            display_id: "javascript",
        },
        LanguageSpec {
            lang: Language::TypeScript,
            extensions: &["ts", "tsx", "mts", "cts"],
            ts_id: Some("typescript"),
            lsp_id: Some("typescript"),
            display_id: "typescript",
        },
        LanguageSpec {
            lang: Language::Go,
            extensions: &["go"],
            ts_id: Some("go"),
            lsp_id: Some("go"),
            display_id: "go",
        },
        LanguageSpec {
            lang: Language::Java,
            extensions: &["java"],
            ts_id: Some("java"),
            lsp_id: Some("java"),
            display_id: "java",
        },
        LanguageSpec {
            lang: Language::Json,
            extensions: &["json", "jsonc", "jsonl"],
            ts_id: Some("json"),
            lsp_id: None,
            display_id: "json",
        },
        LanguageSpec {
            lang: Language::Markdown,
            extensions: &["md", "markdown", "mdx"],
            ts_id: None,
            lsp_id: None,
            display_id: "markdown",
        },
        LanguageSpec {
            lang: Language::Toml,
            extensions: &["toml", "ini", "cfg", "conf", "config"],
            ts_id: Some("toml"),
            lsp_id: None,
            display_id: "toml",
        },
        LanguageSpec {
            lang: Language::Html,
            extensions: &[
                "html",
                "htm",
                "xhtml",
                "vue",
                "svelte",
                "wxml",
                "axml",
                "ftl",
                "jinja",
                "j2",
                "njk",
                "mustache",
                "handlebars",
                "hbs",
                "ejs",
                "erb",
                "haml",
                "pug",
                "jade",
                "liquid",
                "razor",
                "cshtml",
            ],
            ts_id: None,
            lsp_id: None,
            display_id: "html",
        },
        LanguageSpec {
            lang: Language::Css,
            extensions: &[
                "css", "scss", "sass", "less", "styl", "stylus", "wxss", "acss",
            ],
            ts_id: None,
            lsp_id: None,
            display_id: "css",
        },
        LanguageSpec {
            lang: Language::PlainText,
            extensions: &[],
            ts_id: None,
            lsp_id: None,
            display_id: "text",
        },
        LanguageSpec {
            lang: Language::Image,
            extensions: &[
                "png", "jpg", "jpeg", "gif", "bmp", "webp", "ico", "svg", "tiff", "tif", "raw",
                "psd",
            ],
            ts_id: None,
            lsp_id: None,
            display_id: "image",
        },
    ];

    /// 查注册表（按枚举值）
    pub fn spec(self) -> &'static LanguageSpec {
        Self::ALL
            .iter()
            .find(|s| s.lang == self)
            .expect("Language 枚举值必须存在于注册表")
    }

    /// tree-sitter grammar id（无 grammar 的语言返回 None，由调用方 fallback 手写 lexer）
    pub fn ts_id(self) -> Option<&'static str> {
        self.spec().ts_id
    }

    /// LSP language_id（无默认 LSP 服务器的语言返回 None）
    pub fn lsp_id(self) -> Option<&'static str> {
        self.spec().lsp_id
    }

    /// 状态栏等场景使用的显示 id
    pub fn display_id(self) -> &'static str {
        self.spec().display_id
    }

    /// 根据文件扩展名检测语言
    /// 对于没有独立 lexer 的扩展名，尽量归入语义相近的语言（如 vue/wxml 用 HTML lexer），
    /// 完全未知的扩展名统一归为 PlainText，保证任何文本文件都能被查看。
    pub fn from_extension(ext: &str) -> Self {
        let ext = ext.to_lowercase();
        for spec in Self::ALL {
            if spec.extensions.contains(&ext.as_str()) {
                return spec.lang;
            }
        }
        Language::PlainText
    }

    /// 根据文件路径检测语言
    pub fn from_path(path: &std::path::Path) -> Self {
        path.extension()
            .and_then(|e| e.to_str())
            .map(Language::from_extension)
            .unwrap_or(Language::PlainText)
    }

    /// 创建对应语言的词法分析器
    pub fn create_lexer(&self) -> Box<dyn Lexer> {
        match self {
            Language::C => Box::new(c_lexer::CLexer::new()),
            // C++ 暂复用 C 家族 lexer（注释/字符串/数字/大括号等公共结构），
            // 高亮优先走 tree-sitter（ts_id = "cpp"）
            Language::Cpp => Box::new(c_lexer::CLexer::new()),
            Language::Rust => Box::new(rust_lexer::RustLexer::new()),
            Language::Python => Box::new(python_lexer::PythonLexer::new()),
            Language::JavaScript | Language::TypeScript => Box::new(js_lexer::JsLexer::new()),
            // Go/Java 无独立 lexer，复用 C 家族 lexer 作为 fallback（仅在 tree-sitter
            // 不可用时使用，如大文件），可高亮注释、字符串、数字、大括号等公共结构
            Language::Go | Language::Java => Box::new(c_lexer::CLexer::new()),
            Language::Json => Box::new(json_lexer::JsonLexer::new()),
            Language::Markdown => Box::new(markdown_lexer::MarkdownLexer::new()),
            Language::Toml => Box::new(toml_lexer::TomlLexer::new()),
            Language::Html => Box::new(html_lexer::HtmlLexer::new()),
            // CSS 暂时没有独立 lexer，复用 HTML lexer 至少能高亮注释、字符串、标签等公共结构
            Language::Css => Box::new(html_lexer::HtmlLexer::new()),
            Language::PlainText => Box::new(PlainTextLexer::new()),
            Language::Image => Box::new(PlainTextLexer::new()),
        }
    }

    /// 直接对指定语言的文本进行词法分析，使用静态分发，无 Box 分配与动态分发开销。
    pub fn lex_full(&self, text: &str) -> Vec<LexemeSpan> {
        match self {
            Language::C => c_lexer::CLexer::new().lex_full(text),
            Language::Cpp => c_lexer::CLexer::new().lex_full(text),
            Language::Rust => rust_lexer::RustLexer::new().lex_full(text),
            Language::Python => python_lexer::PythonLexer::new().lex_full(text),
            Language::JavaScript | Language::TypeScript => js_lexer::JsLexer::new().lex_full(text),
            Language::Go | Language::Java => c_lexer::CLexer::new().lex_full(text),
            Language::Json => json_lexer::JsonLexer::new().lex_full(text),
            Language::Markdown => markdown_lexer::MarkdownLexer::new().lex_full(text),
            Language::Toml => toml_lexer::TomlLexer::new().lex_full(text),
            Language::Html => html_lexer::HtmlLexer::new().lex_full(text),
            Language::Css => html_lexer::HtmlLexer::new().lex_full(text),
            Language::PlainText => PlainTextLexer::new().lex_full(text),
            Language::Image => PlainTextLexer::new().lex_full(text),
        }
    }
}

pub mod c_lexer;
pub mod common;
pub mod html_lexer;
pub mod js_lexer;
pub mod json_lexer;
pub mod markdown_lexer;
pub mod python_lexer;
pub mod rust_lexer;
pub mod toml_lexer;

/// 纯文本词法分析器（无高亮）
pub struct PlainTextLexer;

impl PlainTextLexer {
    pub fn new() -> Self {
        Self
    }
}

impl Lexer for PlainTextLexer {
    fn lex_full(&self, text: &str) -> Vec<LexemeSpan> {
        if text.is_empty() {
            return Vec::new();
        }
        vec![LexemeSpan::new(0, text.len(), TokenKind::Unknown)]
    }
}

impl Default for PlainTextLexer {
    fn default() -> Self {
        Self::new()
    }
}

/// 根据 UTF-8 首字节推断字符的字节长度。
/// 非法或 ASCII 字节返回 1，保证 lexer 至少能前进一步。
pub(crate) fn utf8_char_len(first_byte: u8) -> usize {
    match first_byte {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    /// 注册表必须覆盖枚举全部变体（防新增枚举忘加表条目）
    #[test]
    fn test_registry_covers_all_language_variants() {
        let variants = [
            Language::C,
            Language::Cpp,
            Language::Rust,
            Language::Python,
            Language::JavaScript,
            Language::TypeScript,
            Language::Go,
            Language::Java,
            Language::Json,
            Language::Markdown,
            Language::Toml,
            Language::Html,
            Language::Css,
            Language::PlainText,
            Language::Image,
        ];
        assert_eq!(variants.len(), Language::ALL.len());
        for v in variants {
            assert!(Language::ALL.iter().any(|s| s.lang == v));
            assert!(!v.display_id().is_empty());
        }
    }

    /// 注册表扩展名无重叠（同一扩展名只属于一种语言）
    #[test]
    fn test_registry_extensions_no_overlap() {
        let mut seen: Vec<&str> = Vec::new();
        for spec in Language::ALL {
            for e in spec.extensions {
                assert!(!seen.contains(e), "扩展名 {} 重复注册", e);
                seen.push(e);
            }
        }
    }

    #[test]
    fn test_from_extension_cpp_split() {
        assert_eq!(Language::from_extension("cpp"), Language::Cpp);
        assert_eq!(Language::from_extension("hpp"), Language::Cpp);
        assert_eq!(Language::from_extension("cc"), Language::Cpp);
        assert_eq!(Language::from_extension("CXX"), Language::Cpp);
        // Objective-C 保持归 C（行为不变）
        assert_eq!(Language::from_extension("m"), Language::C);
        assert_eq!(Language::from_extension("mm"), Language::C);
        assert_eq!(Language::from_extension("c"), Language::C);
    }

    #[test]
    fn test_ts_and_lsp_ids() {
        assert_eq!(Language::Cpp.ts_id(), Some("cpp"));
        assert_eq!(Language::Cpp.lsp_id(), Some("cpp"));
        assert_eq!(Language::Rust.ts_id(), Some("rust"));
        assert_eq!(Language::Go.lsp_id(), Some("go"));
        assert_eq!(Language::Java.lsp_id(), Some("java"));
        // 无 grammar/LSP 的语言
        assert_eq!(Language::Markdown.ts_id(), None);
        assert_eq!(Language::Html.ts_id(), None);
        assert_eq!(Language::Css.lsp_id(), None);
        assert_eq!(Language::Json.lsp_id(), None);
        assert_eq!(Language::PlainText.display_id(), "text");
    }

    use super::*;

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
        assert_eq!(Language::from_extension("JS"), Language::JavaScript);
        assert_eq!(Language::from_extension("TSX"), Language::TypeScript);
        assert_eq!(Language::from_extension("json"), Language::Json);
        assert_eq!(Language::from_extension("md"), Language::Markdown);
        assert_eq!(Language::from_extension("toml"), Language::Toml);
        assert_eq!(Language::from_extension("html"), Language::Html);
        assert_eq!(Language::from_extension("css"), Language::Css);
        assert_eq!(Language::from_extension("png"), Language::Image);
        assert_eq!(Language::from_extension("unknown"), Language::PlainText);
    }

    #[test]
    fn test_language_from_path() {
        let path = std::path::Path::new("src/main.rs");
        assert_eq!(Language::from_path(path), Language::Rust);
        let no_ext = std::path::Path::new("Makefile");
        assert_eq!(Language::from_path(no_ext), Language::PlainText);
    }

    #[test]
    fn test_create_lexer() {
        let lexer = Language::Rust.create_lexer();
        let tokens = lexer.lex_full("fn main() {}");
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_lex_full_static_dispatch() {
        let tokens = Language::Rust.lex_full("let x = 42;");
        assert!(!tokens.is_empty());
        let plain = Language::PlainText.lex_full("hello world");
        assert_eq!(plain.len(), 1);
        assert_eq!(plain[0].kind, TokenKind::Unknown);
    }

    #[test]
    fn test_plain_text_lexer() {
        let lexer = PlainTextLexer::new();
        assert!(lexer.lex_full("").is_empty());
        let tokens = lexer.lex_full("hello");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Unknown);
        assert_eq!(tokens[0].len, 5);
    }

    #[test]
    fn test_utf8_char_len() {
        assert_eq!(utf8_char_len(b'a'), 1);
        assert_eq!(utf8_char_len(0xC0), 2);
        assert_eq!(utf8_char_len(0xE4), 3);
        assert_eq!(utf8_char_len(0xF0), 4);
        assert_eq!(utf8_char_len(0x80), 1); // 非法首字节
    }
}
