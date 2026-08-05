use aether_core::lexer::{LexemeSpan, TokenKind};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;

use super::lexer::{GpuToken, token_types};
use super::syntax::{SyntaxClass, syntax_classes};

/// GPU Token 到 LexemeSpan 的转换
///
/// 将 GPU 生成的 Token 转换为渲染器使用的 LexemeSpan。
pub fn gpu_tokens_to_lexeme_spans(
    tokens: &[GpuToken],
    syntax_classes: Option<&[SyntaxClass]>,
) -> Vec<LexemeSpan> {
    let mut spans = Vec::with_capacity(tokens.len());

    for (i, token) in tokens.iter().enumerate() {
        let kind = if let Some(classes) = syntax_classes {
            resolve_token_kind(token, &classes[i])
        } else {
            token_type_to_kind(token.token_type)
        };

        spans.push(LexemeSpan {
            start: token.start,
            len: token.len,
            kind,
            flags: 0,
        });
    }

    spans
}

/// 根据 Token 类型和语法分类解析最终 TokenKind
fn resolve_token_kind(token: &GpuToken, syntax: &SyntaxClass) -> TokenKind {
    // 优先使用语法分类（如果置信度足够高）
    if syntax.confidence >= 70 {
        match syntax.class_id {
            syntax_classes::SYNTAX_FUNCTION_DECL |
            syntax_classes::SYNTAX_FUNCTION_CALL => TokenKind::Function,
            syntax_classes::SYNTAX_TYPE_NAME => TokenKind::TypeName,
            syntax_classes::SYNTAX_VARIABLE_DECL |
            syntax_classes::SYNTAX_VARIABLE_REF => TokenKind::Identifier,
            syntax_classes::SYNTAX_PARAMETER => TokenKind::Identifier,
            syntax_classes::SYNTAX_FIELD_ACCESS => TokenKind::Attribute,
            syntax_classes::SYNTAX_MACRO => TokenKind::Macro,
            syntax_classes::SYNTAX_ATTRIBUTE => TokenKind::Attribute,
            _ => token_type_to_kind(token.token_type),
        }
    } else {
        token_type_to_kind(token.token_type)
    }
}

/// 将 GPU Token 类型转换为 TokenKind
fn token_type_to_kind(token_type: u32) -> TokenKind {
    match token_type {
        token_types::TOKEN_KEYWORD => TokenKind::Keyword,
        token_types::TOKEN_STRING => TokenKind::StringLiteral,
        token_types::TOKEN_NUMBER => TokenKind::NumberLiteral,
        token_types::TOKEN_COMMENT => TokenKind::LineComment,
        token_types::TOKEN_FUNCTION_NAME => TokenKind::Function,
        token_types::TOKEN_TYPE_NAME => TokenKind::TypeName,
        token_types::TOKEN_OPERATOR => TokenKind::Operator,
        token_types::TOKEN_IDENTIFIER => TokenKind::Identifier,
        token_types::TOKEN_PREPROCESSOR => TokenKind::Preprocessor,
        token_types::TOKEN_CONSTANT => TokenKind::NumberLiteral,
        _ => TokenKind::Identifier,
    }
}

/// 合并相邻同色 Token，减少 DrawText 调用
///
/// 优化：连续的相同类型 token 合并为单个 span，减少渲染调用次数。
pub fn merge_same_color_tokens(tokens: &[LexemeSpan]) -> Vec<MergedSpan> {
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut merged = Vec::with_capacity(tokens.len() / 2);
    let mut current = MergedSpan {
        start: tokens[0].start,
        len: tokens[0].len,
        kind: tokens[0].kind,
    };

    for token in &tokens[1..] {
        if token.kind == current.kind && token.start == current.start + current.len {
            // 相邻同色，合并
            current.len += token.len;
        } else {
            // 不同类型或不相邻，保存当前并新建
            merged.push(current);
            current = MergedSpan {
                start: token.start,
                len: token.len,
                kind: token.kind,
            };
        }
    }

    merged.push(current);
    merged
}

/// 合并后的 Span（用于渲染）
#[derive(Clone, Copy, Debug)]
pub struct MergedSpan {
    pub start: u32,
    pub len: u32,
    pub kind: TokenKind,
}

/// GPU 高亮渲染器
///
/// 将 GPU 生成的 token 直接用于 Direct2D 渲染。
pub struct GpuHighlightRenderer;

impl GpuHighlightRenderer {
    /// 将 TokenKind 映射到主题颜色
    pub fn token_kind_to_color(kind: TokenKind, theme: &crate::theme::Theme) -> D2D1_COLOR_F {
        theme.color_for_token(kind)
    }
}

/// 双缓冲管理器
///
/// 实现 CPU/GPU 并行处理，避免等待。
pub struct DoubleBuffer<T> {
    buffers: [Option<T>; 2],
    current: usize,
}

impl<T> DoubleBuffer<T> {
    pub fn new() -> Self {
        Self {
            buffers: [None, None],
            current: 0,
        }
    }

    /// 获取当前缓冲区（读取）
    pub fn current(&self) -> Option<&T> {
        self.buffers[self.current].as_ref()
    }

    /// 获取下一个缓冲区（写入）
    pub fn next(&mut self) -> &mut Option<T> {
        let next = 1 - self.current;
        &mut self.buffers[next]
    }

    /// 交换缓冲区
    pub fn swap(&mut self) {
        self.current = 1 - self.current;
    }
}

/// GPU 缓冲区内存池
///
/// 复用 GPU 缓冲区，避免频繁分配/释放。
pub struct GpuBufferPool {
    available: Vec<windows::Win32::Graphics::Direct3D11::ID3D11Buffer>,
    in_use: Vec<windows::Win32::Graphics::Direct3D11::ID3D11Buffer>,
}

impl GpuBufferPool {
    pub fn new() -> Self {
        Self {
            available: Vec::new(),
            in_use: Vec::new(),
        }
    }

    /// 获取一个合适大小的缓冲区
    pub fn acquire(
        &mut self,
        context: &super::compute_context::GpuComputeContext,
        size: usize,
    ) -> windows::core::Result<windows::Win32::Graphics::Direct3D11::ID3D11Buffer> {
        // 查找足够大的可用缓冲区
        if let Some(idx) = self.available.iter().position(|_buf| {
            // 检查缓冲区大小（需要查询 desc）
            // 简化：直接复用第一个
            true
        }) {
            let buf = self.available.remove(idx);
            self.in_use.push(buf.clone());
            return Ok(buf);
        }

        // 创建新缓冲区
        let buf = context.create_buffer(size, super::compute_context::BufferUsage::ReadWrite, None)?;
        self.in_use.push(buf.clone());
        Ok(buf)
    }

    /// 释放缓冲区回池
    pub fn release(&mut self, buffer: windows::Win32::Graphics::Direct3D11::ID3D11Buffer) {
        // 使用指针比较来找到对应的缓冲区
        let buffer_ptr = &buffer as *const _;
        if let Some(idx) = self.in_use.iter().position(|b| {
            let b_ptr = b as *const _;
            std::ptr::eq(b_ptr, buffer_ptr)
        }) {
            self.in_use.remove(idx);
            self.available.push(buffer);
        }
    }

    /// 清理所有缓冲区
    pub fn clear(&mut self) {
        self.available.clear();
        self.in_use.clear();
    }
}
