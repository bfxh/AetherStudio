/// 语言特定的 DFA 表和关键字表生成器
///
/// 为不同编程语言生成 GPU 词法分析所需的 DFA 状态转换表和关键字哈希表。
pub struct LanguageLexerTables;

/// DFA 表（256 * num_states 字节）
pub struct DfaTable {
    pub data: Vec<u8>,
    pub num_states: u32,
}

/// 关键字哈希表
pub struct KeywordTable {
    pub data: Vec<u32>,
    pub keywords: Vec<String>,
}

impl LanguageLexerTables {
    /// 为指定语言生成 DFA 表和关键字表
    pub fn for_language(language: &str) -> (DfaTable, KeywordTable) {
        match language.to_lowercase().as_str() {
            "rust" => Self::rust_tables(),
            "c" | "cpp" | "c++" | "h" | "hpp" => Self::c_family_tables(),
            "javascript" | "js" | "typescript" | "ts" | "jsx" | "tsx" => Self::js_tables(),
            "python" | "py" => Self::python_tables(),
            "go" | "golang" => Self::go_tables(),
            "java" => Self::java_tables(),
            "json" => Self::json_tables(),
            "toml" => Self::toml_tables(),
            "markdown" | "md" => Self::markdown_tables(),
            "html" | "htm" | "xml" => Self::html_tables(),
            "css" | "scss" | "sass" => Self::css_tables(),
            _ => Self::generic_tables(),
        }
    }

    // === Rust ===
    fn rust_tables() -> (DfaTable, KeywordTable) {
        let keywords = vec![
            "as", "async", "await", "break", "const", "continue", "crate", "dyn",
            "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in",
            "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
            "self", "Self", "static", "struct", "super", "trait", "true", "type",
            "unsafe", "use", "where", "while", "yield",
            // 常用类型
            "i8", "i16", "i32", "i64", "i128", "isize",
            "u8", "u16", "u32", "u64", "u128", "usize",
            "f32", "f64", "bool", "char", "str", "String",
            "Vec", "Option", "Result", "Box", "Rc", "Arc",
            // 宏
            "println!", "format!", "vec!", "assert!", "panic!",
        ];

        let dfa = Self::build_generic_dfa();
        let keyword_table = Self::build_keyword_table(&keywords);

        (dfa, keyword_table)
    }

    // === C/C++ ===
    fn c_family_tables() -> (DfaTable, KeywordTable) {
        let keywords = vec![
            "auto", "break", "case", "char", "const", "continue", "default", "do",
            "double", "else", "enum", "extern", "float", "for", "goto", "if",
            "inline", "int", "long", "register", "restrict", "return", "short",
            "signed", "sizeof", "static", "struct", "switch", "typedef", "union",
            "unsigned", "void", "volatile", "while",
            // C++ 关键字
            "alignas", "alignof", "and", "and_eq", "asm", "bitand", "bitor",
            "bool", "catch", "class", "compl", "concept", "consteval", "constexpr",
            "constinit", "co_await", "co_return", "co_yield", "decltype", "delete",
            "dynamic_cast", "explicit", "export", "false", "friend", "mutable",
            "namespace", "new", "noexcept", "not", "not_eq", "nullptr", "operator",
            "or", "or_eq", "private", "protected", "public", "requires",
            "reinterpret_cast", "static_assert", "static_cast", "template", "this",
            "thread_local", "throw", "true", "try", "typename", "using", "virtual",
            "wchar_t", "xor", "xor_eq",
            // 常用类型
            "size_t", "ssize_t", "uint8_t", "uint16_t", "uint32_t", "uint64_t",
            "int8_t", "int16_t", "int32_t", "int64_t", "uintptr_t", "intptr_t",
            // 预处理
            "define", "ifdef", "ifndef", "endif", "include", "pragma", "undef",
        ];

        let dfa = Self::build_generic_dfa();
        let keyword_table = Self::build_keyword_table(&keywords);

        (dfa, keyword_table)
    }

    // === JavaScript / TypeScript ===
    fn js_tables() -> (DfaTable, KeywordTable) {
        let keywords = vec![
            "break", "case", "catch", "class", "const", "continue", "debugger",
            "default", "delete", "do", "else", "export", "extends", "false",
            "finally", "for", "function", "if", "import", "in", "instanceof",
            "new", "null", "return", "super", "switch", "this", "throw", "true",
            "try", "typeof", "var", "void", "while", "with", "yield",
            // ES6+
            "let", "static", "await", "async", "of",
            // 常用全局
            "undefined", "NaN", "Infinity", "console", "window", "document",
            "require", "module", "exports", "global", "process",
            // TypeScript
            "interface", "type", "namespace", "declare", "abstract", "readonly",
            "any", "number", "string", "boolean", "symbol", "object", "never",
            "unknown", "enum", "implements", "private", "protected", "public",
            "constructor", "get", "set",
        ];

        let dfa = Self::build_generic_dfa();
        let keyword_table = Self::build_keyword_table(&keywords);

        (dfa, keyword_table)
    }

    // === Python ===
    fn python_tables() -> (DfaTable, KeywordTable) {
        let keywords = vec![
            "and", "as", "assert", "async", "await", "break", "class", "continue",
            "def", "del", "elif", "else", "except", "False", "finally", "for",
            "from", "global", "if", "import", "in", "is", "lambda", "None",
            "nonlocal", "not", "or", "pass", "raise", "return", "True", "try",
            "while", "with", "yield",
            // 常用内置
            "print", "len", "range", "list", "dict", "set", "tuple", "str",
            "int", "float", "bool", "type", "isinstance", "hasattr", "getattr",
            // 常用模块
            "self", "cls", "super",
        ];

        let dfa = Self::build_generic_dfa();
        let keyword_table = Self::build_keyword_table(&keywords);

        (dfa, keyword_table)
    }

    // === Go ===
    fn go_tables() -> (DfaTable, KeywordTable) {
        let keywords = vec![
            "break", "case", "chan", "const", "continue", "default", "defer",
            "else", "fallthrough", "for", "func", "go", "goto", "if", "import",
            "interface", "map", "package", "range", "return", "select", "struct",
            "switch", "type", "var",
            // 常用类型
            "bool", "byte", "complex64", "complex128", "error", "float32", "float64",
            "int", "int8", "int16", "int32", "int64", "rune", "string",
            "uint", "uint8", "uint16", "uint32", "uint64", "uintptr",
            // 内置函数
            "append", "cap", "close", "complex", "copy", "delete", "imag", "len",
            "make", "new", "panic", "print", "println", "real", "recover",
        ];

        let dfa = Self::build_generic_dfa();
        let keyword_table = Self::build_keyword_table(&keywords);

        (dfa, keyword_table)
    }

    // === Java ===
    fn java_tables() -> (DfaTable, KeywordTable) {
        let keywords = vec![
            "abstract", "assert", "boolean", "break", "byte", "case", "catch",
            "char", "class", "const", "continue", "default", "do", "double",
            "else", "enum", "extends", "final", "finally", "float", "for",
            "goto", "if", "implements", "import", "instanceof", "int",
            "interface", "long", "native", "new", "package", "private",
            "protected", "public", "return", "short", "static", "strictfp",
            "super", "switch", "synchronized", "this", "throw", "throws",
            "transient", "try", "void", "volatile", "while",
            // 常用类型
            "String", "Object", "Integer", "Double", "Boolean", "List", "Map",
            "Set", "ArrayList", "HashMap", "HashSet", "System", "out", "println",
        ];

        let dfa = Self::build_generic_dfa();
        let keyword_table = Self::build_keyword_table(&keywords);

        (dfa, keyword_table)
    }

    // === JSON ===
    fn json_tables() -> (DfaTable, KeywordTable) {
        let keywords = vec!["true", "false", "null"];

        let dfa = Self::build_generic_dfa();
        let keyword_table = Self::build_keyword_table(&keywords);

        (dfa, keyword_table)
    }

    // === TOML ===
    fn toml_tables() -> (DfaTable, KeywordTable) {
        let keywords = vec!["true", "false"];

        let dfa = Self::build_generic_dfa();
        let keyword_table = Self::build_keyword_table(&keywords);

        (dfa, keyword_table)
    }

    // === Markdown ===
    fn markdown_tables() -> (DfaTable, KeywordTable) {
        let keywords: Vec<&str> = vec![];

        let dfa = Self::build_generic_dfa();
        let keyword_table = Self::build_keyword_table(&keywords);

        (dfa, keyword_table)
    }

    // === HTML ===
    fn html_tables() -> (DfaTable, KeywordTable) {
        let keywords = vec![
            "!DOCTYPE", "a", "abbr", "address", "area", "article", "aside", "audio",
            "b", "base", "bdi", "bdo", "blockquote", "body", "br", "button",
            "canvas", "caption", "cite", "code", "col", "colgroup", "data",
            "datalist", "dd", "del", "details", "dfn", "dialog", "div", "dl",
            "dt", "em", "embed", "fieldset", "figcaption", "figure", "footer",
            "form", "h1", "h2", "h3", "h4", "h5", "h6", "head", "header",
            "hgroup", "hr", "html", "i", "iframe", "img", "input", "ins",
            "kbd", "label", "legend", "li", "link", "main", "map", "mark",
            "math", "menu", "meta", "meter", "nav", "noscript", "object", "ol",
            "optgroup", "option", "output", "p", "picture", "pre", "progress",
            "q", "rp", "rt", "ruby", "s", "samp", "script", "search", "section",
            "select", "slot", "small", "source", "span", "strong", "style",
            "sub", "summary", "sup", "svg", "table", "tbody", "td", "template",
            "textarea", "tfoot", "th", "thead", "time", "title", "tr", "track",
            "u", "ul", "var", "video", "wbr",
        ];

        let dfa = Self::build_generic_dfa();
        let keyword_table = Self::build_keyword_table(&keywords);

        (dfa, keyword_table)
    }

    // === CSS ===
    fn css_tables() -> (DfaTable, KeywordTable) {
        let keywords = vec![
            "align-content", "align-items", "align-self", "all", "animation",
            "background", "border", "bottom", "box-shadow", "color", "display",
            "flex", "flex-direction", "font", "font-family", "font-size",
            "font-weight", "grid", "height", "justify-content", "left",
            "margin", "max-height", "max-width", "min-height", "min-width",
            "opacity", "overflow", "padding", "position", "right", "top",
            "transform", "transition", "visibility", "width", "z-index",
            "@media", "@import", "@keyframes", "@font-face",
            // 常用值
            "absolute", "auto", "block", "center", "column", "fixed", "flex",
            "grid", "hidden", "inline", "inline-block", "none", "relative",
            "row", "static", "sticky", "transparent", "unset",
        ];

        let dfa = Self::build_generic_dfa();
        let keyword_table = Self::build_keyword_table(&keywords);

        (dfa, keyword_table)
    }

    // === 通用（未知语言） ===
    fn generic_tables() -> (DfaTable, KeywordTable) {
        let keywords: Vec<&str> = vec![];

        let dfa = Self::build_generic_dfa();
        let keyword_table = Self::build_keyword_table(&keywords);

        (dfa, keyword_table)
    }

    // === 通用 DFA 构建 ===
    /// 构建一个通用的字符分类 DFA
    ///
    /// 状态 0: 初始/未知
    /// 状态 1: 标识符（字母/数字/下划线）
    /// 状态 2: 数字
    /// 状态 3: 字符串（双引号）
    /// 状态 4: 字符串（单引号）
    /// 状态 5: 注释（//）
    /// 状态 6: 注释（/*）
    /// 状态 7: 空白
    /// 状态 8: 标点/运算符
    fn build_generic_dfa() -> DfaTable {
        const NUM_STATES: usize = 9;
        let mut table = vec![0u8; 256 * NUM_STATES];

        // 状态 0: 初始状态
        for c in b'a'..=b'z' {
            table[c as usize] = 1; // -> 标识符
        }
        for c in b'A'..=b'Z' {
            table[c as usize] = 1; // -> 标识符
        }
        table[b'_' as usize] = 1; // -> 标识符
        for c in b'0'..=b'9' {
            table[c as usize] = 2; // -> 数字
        }
        table[b'"' as usize] = 3; // -> 双引号字符串
        table[b'\'' as usize] = 4; // -> 单引号字符串
        table[b'/' as usize] = 8; // -> 可能是注释开始
        table[b' ' as usize] = 7; // -> 空白
        table[b'\t' as usize] = 7;
        table[b'\n' as usize] = 7;
        table[b'\r' as usize] = 7;
        // 标点/运算符
        for &c in &[b'+', b'-', b'*', b'%', b'=', b'!', b'<', b'>', b'&', b'|', b'^', b'~', b'?', b':', b';', b',', b'.', b'(', b')', b'[', b']', b'{', b'}', b'@', b'#', b'$', b'`'] {
            table[c as usize] = 8;
        }

        // 状态 1: 标识符中
        for c in b'a'..=b'z' {
            table[256 + c as usize] = 1;
        }
        for c in b'A'..=b'Z' {
            table[256 + c as usize] = 1;
        }
        table[256 + b'_' as usize] = 1;
        for c in b'0'..=b'9' {
            table[256 + c as usize] = 1;
        }

        // 状态 2: 数字中
        for c in b'0'..=b'9' {
            table[512 + c as usize] = 2;
        }
        table[512 + b'.' as usize] = 2;
        table[512 + b'e' as usize] = 2;
        table[512 + b'E' as usize] = 2;
        table[512 + b'x' as usize] = 2;
        table[512 + b'X' as usize] = 2;
        table[512 + b'a' as usize] = 2;
        table[512 + b'b' as usize] = 2;
        table[512 + b'c' as usize] = 2;
        table[512 + b'd' as usize] = 2;
        table[512 + b'f' as usize] = 2;
        table[512 + b'A' as usize] = 2;
        table[512 + b'B' as usize] = 2;
        table[512 + b'C' as usize] = 2;
        table[512 + b'D' as usize] = 2;
        table[512 + b'F' as usize] = 2;
        table[512 + b'_' as usize] = 2;

        // 其他状态保持默认（0）

        DfaTable {
            data: table,
            num_states: NUM_STATES as u32,
        }
    }

    /// 构建关键字哈希表（简单完美哈希）
    fn build_keyword_table(keywords: &[&str]) -> KeywordTable {
        let mut data: Vec<u32> = Vec::new();
        let mut keyword_strings: Vec<String> = Vec::new();

        for &kw in keywords {
            // 将关键字字符串编码为 u32 数组
            let bytes = kw.as_bytes();
            let len = bytes.len().min(255) as u32;
            data.push(len);
            for chunk in bytes.chunks(4) {
                let mut val: u32 = 0;
                for (i, &b) in chunk.iter().enumerate() {
                    val |= (b as u32) << (i * 8);
                }
                data.push(val);
            }
            keyword_strings.push(kw.to_string());
        }

        KeywordTable {
            data,
            keywords: keyword_strings,
        }
    }
}
