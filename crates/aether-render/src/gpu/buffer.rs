use windows::core::Result;
use windows::Win32::Graphics::Direct3D11::ID3D11Buffer;

use super::compute_context::GpuComputeContext;

/// GPU 缓冲区管理器
///
/// 提供缓冲区的分配、回收和复用功能。
pub struct GpuBufferManager {
    context: GpuComputeContext,
}

impl GpuBufferManager {
    pub fn new(context: GpuComputeContext) -> Self {
        Self { context }
    }

    /// 创建文本输入缓冲区
    pub fn create_text_buffer(&self, text: &[u8]) -> Result<ID3D11Buffer> {
        self.context.create_buffer(
            text.len(),
            super::compute_context::BufferUsage::Structured,
            Some(text),
        )
    }

    /// 创建 Token 输出缓冲区
    pub fn create_token_buffer(&self, max_tokens: usize) -> Result<ID3D11Buffer> {
        let size = max_tokens * std::mem::size_of::<super::lexer::GpuToken>();
        self.context
            .create_buffer(size, super::compute_context::BufferUsage::ReadWrite, None)
    }

    /// 创建字符分类缓冲区
    pub fn create_char_class_buffer(&self, text_len: usize) -> Result<ID3D11Buffer> {
        let size = text_len * std::mem::size_of::<u32>();
        self.context
            .create_buffer(size, super::compute_context::BufferUsage::ReadWrite, None)
    }

    /// 创建计数器缓冲区
    pub fn create_counter_buffer(&self) -> Result<ID3D11Buffer> {
        let size = std::mem::size_of::<u32>();
        self.context
            .create_buffer(size, super::compute_context::BufferUsage::ReadWrite, None)
    }
}
