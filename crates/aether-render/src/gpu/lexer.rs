use windows::core::Result;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11ComputeShader, ID3D11ShaderResourceView,
    ID3D11UnorderedAccessView,
    D3D11_BUFFER_UAV, D3D11_UNORDERED_ACCESS_VIEW_DESC, D3D11_UNORDERED_ACCESS_VIEW_DESC_0,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_UINT;

use super::compute_context::{GpuComputeContext, BufferUsage};
use super::shader::ShaderCompiler;

/// GPU Token 结构，与 Shader 中的结构体对齐
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuToken {
    /// Token 起始位置（字节偏移）
    pub start: u32,
    /// Token 长度（字节）
    pub len: u32,
    /// Token 类型
    pub token_type: u32,
    /// 关键字 ID（如果是关键字）
    pub keyword_id: u32,
    /// 语法分类（由语法分类器填充）
    pub syntax_class: u32,
    /// 保留字段
    pub _padding: u32,
}

/// Token 类型常量
pub mod token_types {
    pub const TOKEN_UNKNOWN: u32 = 0;
    pub const TOKEN_IDENTIFIER: u32 = 1;
    pub const TOKEN_KEYWORD: u32 = 2;
    pub const TOKEN_STRING: u32 = 3;
    pub const TOKEN_NUMBER: u32 = 4;
    pub const TOKEN_COMMENT: u32 = 5;
    pub const TOKEN_OPERATOR: u32 = 6;
    pub const TOKEN_PUNCTUATION: u32 = 7;
    pub const TOKEN_WHITESPACE: u32 = 8;
    pub const TOKEN_NEWLINE: u32 = 9;
    pub const TOKEN_PREPROCESSOR: u32 = 10;
    pub const TOKEN_TYPE_NAME: u32 = 11;
    pub const TOKEN_FUNCTION_NAME: u32 = 12;
    pub const TOKEN_VARIABLE: u32 = 13;
    pub const TOKEN_CONSTANT: u32 = 14;
}

/// GPU 词法分析器
///
/// 使用 D3D11 Compute Shader 实现并行词法分析。
pub struct GpuLexer {
    context: GpuComputeContext,

    // DFA 状态表（常量缓冲区）
    dfa_table: ID3D11Buffer,
    dfa_srv: ID3D11ShaderResourceView,

    // 关键字完美哈希表（常量缓冲区）
    keyword_table: ID3D11Buffer,
    keyword_srv: ID3D11ShaderResourceView,

    // Compute Shaders
    char_classify_shader: ID3D11ComputeShader,
    token_scan_shader: ID3D11ComputeShader,
    keyword_lookup_shader: ID3D11ComputeShader,

    // 工作缓冲区
    char_classes_buffer: Option<ID3D11Buffer>,
    tokens_buffer: Option<ID3D11Buffer>,
    tokens_uav: Option<ID3D11UnorderedAccessView>,
    token_count_buffer: Option<ID3D11Buffer>,
    token_count_uav: Option<ID3D11UnorderedAccessView>,
}

impl GpuLexer {
    /// 创建 GPU 词法分析器（使用预编译 Shader bytecode）
    ///
    /// # Arguments
    /// * `context` - GPU 计算上下文
    /// * `dfa_table` - DFA 状态转换表（256 * num_states 字节）
    /// * `keyword_hash` - 关键字完美哈希表
    pub fn new(
        context: GpuComputeContext,
        dfa_table: &[u8],
        keyword_hash: &[u32],
    ) -> Result<Self> {
        // 创建 DFA 表缓冲区
        let (dfa_buf, dfa_srv) = Self::create_dfa_buffer(&context, dfa_table)?;

        // 创建关键字哈希表缓冲区
        let (keyword_buf, keyword_srv) = Self::create_keyword_buffer(&context, keyword_hash)?;

        // 加载预编译的 Shader - 使用空 bytecode 作为占位
        // 实际部署时应使用预编译的 CSO 文件或 new_with_shaders
        let char_classify = Self::load_shader(&context, &[])?;
        let token_scan = Self::load_shader(&context, &[])?;
        let keyword_lookup = Self::load_shader(&context, &[])?;

        Ok(Self {
            context,
            dfa_table: dfa_buf,
            dfa_srv,
            keyword_table: keyword_buf,
            keyword_srv,
            char_classify_shader: char_classify,
            token_scan_shader: token_scan,
            keyword_lookup_shader: keyword_lookup,
            char_classes_buffer: None,
            tokens_buffer: None,
            tokens_uav: None,
            token_count_buffer: None,
            token_count_uav: None,
        })
    }

    /// 创建 GPU 词法分析器（使用 HLSL 源码编译 Shader）
    ///
    /// # Arguments
    /// * `context` - GPU 计算上下文
    /// * `dfa_table` - DFA 状态转换表
    /// * `keyword_hash` - 关键字完美哈希表
    /// * `char_classify_hlsl` - Phase 1 字符分类 HLSL 源码
    /// * `token_scan_hlsl` - Phase 2 Token 扫描 HLSL 源码
    /// * `keyword_lookup_hlsl` - Phase 3 关键字查找 HLSL 源码
    pub fn new_with_shaders(
        context: GpuComputeContext,
        dfa_table: &[u8],
        keyword_hash: &[u32],
        char_classify_hlsl: &str,
        token_scan_hlsl: &str,
        keyword_lookup_hlsl: &str,
    ) -> Result<Self> {
        // 创建 DFA 表缓冲区
        let (dfa_buf, dfa_srv) = Self::create_dfa_buffer(&context, dfa_table)?;

        // 创建关键字哈希表缓冲区
        let (keyword_buf, keyword_srv) = Self::create_keyword_buffer(&context, keyword_hash)?;

        // 编译 HLSL Shader
        let char_classify_bc = ShaderCompiler::compile_char_classify(char_classify_hlsl)?;
        let token_scan_bc = ShaderCompiler::compile_token_scan(token_scan_hlsl)?;
        let keyword_lookup_bc = ShaderCompiler::compile_keyword_lookup(keyword_lookup_hlsl)?;

        let char_classify = context.create_compute_shader(&char_classify_bc)?;
        let token_scan = context.create_compute_shader(&token_scan_bc)?;
        let keyword_lookup = context.create_compute_shader(&keyword_lookup_bc)?;

        Ok(Self {
            context,
            dfa_table: dfa_buf,
            dfa_srv,
            keyword_table: keyword_buf,
            keyword_srv,
            char_classify_shader: char_classify,
            token_scan_shader: token_scan,
            keyword_lookup_shader: keyword_lookup,
            char_classes_buffer: None,
            tokens_buffer: None,
            tokens_uav: None,
            token_count_buffer: None,
            token_count_uav: None,
        })
    }

    /// 创建 GPU 词法分析器（使用嵌入的 HLSL 源码编译 Shader）
    ///
    /// 从编译时嵌入的 HLSL 文件加载并编译 Shader。
    pub fn new_with_embedded_shaders(
        context: GpuComputeContext,
        dfa_table: &[u8],
        keyword_hash: &[u32],
    ) -> Result<Self> {
        const CHAR_CLASSIFY_HLSL: &str = include_str!("shaders/char_classify.hlsl");
        const TOKEN_SCAN_HLSL: &str = include_str!("shaders/token_scan.hlsl");
        const KEYWORD_LOOKUP_HLSL: &str = include_str!("shaders/keyword_lookup.hlsl");

        Self::new_with_shaders(
            context,
            dfa_table,
            keyword_hash,
            CHAR_CLASSIFY_HLSL,
            TOKEN_SCAN_HLSL,
            KEYWORD_LOOKUP_HLSL,
        )
    }

    /// 执行词法分析
    ///
    /// # Arguments
    /// * `text` - 输入文本（UTF-8）
    ///
    /// # Returns
    /// 识别出的 Token 列表
    pub fn lex(&mut self, text: &[u8]) -> Result<Vec<GpuToken>> {
        if text.is_empty() {
            return Ok(Vec::new());
        }

        let text_len = text.len();
        let max_tokens = text_len / 2 + 1; // 最坏情况：每个字符都是 token

        // 1. 确保工作缓冲区足够大
        self.ensure_buffers(text_len, max_tokens)?;

        // 2. 上传文本到 GPU
        let text_buffer = self.upload_text(text)?;

        // 3. Phase 1: 字符分类
        self.run_char_classify(&text_buffer, text_len)?;

        // 4. Phase 2: Token 扫描
        self.run_token_scan(text_len, max_tokens)?;

        // 5. Phase 3: 关键字查找
        self.run_keyword_lookup(max_tokens)?;

        // 6. 回读 token 数量和列表
        let tokens = self.readback_tokens(max_tokens)?;

        Ok(tokens)
    }

    /// 检查 GPU 是否可用
    pub fn is_available(&self) -> bool {
        true // 如果能创建成功，就视为可用
    }

    // 私有辅助方法

    fn create_dfa_buffer(
        context: &GpuComputeContext,
        data: &[u8],
    ) -> Result<(ID3D11Buffer, ID3D11ShaderResourceView)> {
        let buffer = context.create_buffer(data.len(), BufferUsage::Structured, Some(data))?;
        let srv = context.create_srv(&buffer)?;
        Ok((buffer, srv))
    }

    fn create_keyword_buffer(
        context: &GpuComputeContext,
        data: &[u32],
    ) -> Result<(ID3D11Buffer, ID3D11ShaderResourceView)> {
        let bytes = unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * std::mem::size_of::<u32>(),
            )
        };
        let buffer = context.create_buffer(bytes.len(), BufferUsage::Structured, Some(bytes))?;
        let srv = context.create_srv(&buffer)?;
        Ok((buffer, srv))
    }

    fn load_shader(context: &GpuComputeContext, bytecode: &[u8]) -> Result<ID3D11ComputeShader> {
        context.create_compute_shader(bytecode)
    }

    fn ensure_buffers(&mut self, text_len: usize, max_tokens: usize) -> Result<()> {
        // 检查并重新分配字符分类缓冲区
        if self.char_classes_buffer.is_none() {
            let buf = self.context.create_buffer(
                text_len * std::mem::size_of::<u32>(),
                BufferUsage::ReadWrite,
                None,
            )?;
            self.char_classes_buffer = Some(buf);
        }

        // 检查并重新分配 token 缓冲区
        if self.tokens_buffer.is_none() {
            let (buf, uav) = self.context.create_structured_buffer::<GpuToken>(
                max_tokens,
                None::<&[GpuToken]>,
                true,
            )?;
            self.tokens_buffer = Some(buf);
            self.tokens_uav = uav;
        }

        // 检查并重新分配 token 计数缓冲区
        if self.token_count_buffer.is_none() {
            let counter_size = std::mem::size_of::<u32>();
            let buf = self.context.create_buffer(counter_size, BufferUsage::ReadWrite, None)?;

            // 创建 UAV
            let uav_desc = D3D11_UNORDERED_ACCESS_VIEW_DESC {
                Format: DXGI_FORMAT_R32_UINT,
                ViewDimension: windows::Win32::Graphics::Direct3D11::D3D11_UAV_DIMENSION_BUFFER,
                Anonymous: D3D11_UNORDERED_ACCESS_VIEW_DESC_0 {
                    Buffer: D3D11_BUFFER_UAV {
                        FirstElement: 0,
                        NumElements: 1,
                        Flags: windows::Win32::Graphics::Direct3D11::D3D11_BUFFER_UAV_FLAG_COUNTER.0 as u32,
                    },
                },
            };
            let mut uav = None;
            unsafe {
                self.context.device().CreateUnorderedAccessView(&buf, Some(&uav_desc), Some(&mut uav))?;
            }

            self.token_count_buffer = Some(buf);
            self.token_count_uav = uav;
        }

        Ok(())
    }

    fn upload_text(&self, text: &[u8]) -> Result<ID3D11Buffer> {
        self.context.create_buffer(text.len(), BufferUsage::Structured, Some(text))
    }

    fn run_char_classify(&self, text_buffer: &ID3D11Buffer, text_len: usize) -> Result<()> {
        let srv = self.context.create_srv(text_buffer)?;

        let char_classes_uav = self.create_uav_from_buffer(
            self.char_classes_buffer.as_ref().unwrap(),
            text_len as u32,
        )?;

        self.context.set_compute_shader(&self.char_classify_shader);
        self.context.set_shader_resources(0, &[Some(srv)]);
        self.context.set_unordered_access_views(0, &[Some(char_classes_uav)]);

        let groups = ((text_len + 255) / 256) as u32;
        self.context.dispatch(&self.char_classify_shader, (groups, 1, 1));

        Ok(())
    }

    fn run_token_scan(&self, text_len: usize, _max_tokens: usize) -> Result<()> {
        let srv = self.context.create_srv(self.char_classes_buffer.as_ref().unwrap())?;

        self.context.set_compute_shader(&self.token_scan_shader);
        self.context.set_shader_resources(0, &[Some(srv)]);
        self.context.set_unordered_access_views(
            0,
            &[
                self.tokens_uav.clone(),
                self.token_count_uav.clone(),
            ],
        );

        let groups = ((text_len + 255) / 256) as u32;
        self.context.dispatch(&self.token_scan_shader, (groups, 1, 1));

        Ok(())
    }

    fn run_keyword_lookup(&self, max_tokens: usize) -> Result<()> {
        self.context.set_compute_shader(&self.keyword_lookup_shader);
        self.context.set_shader_resources(
            0,
            &[
                Some(self.keyword_srv.clone()),
            ],
        );
        self.context.set_unordered_access_views(
            0,
            &[self.tokens_uav.clone()],
        );

        let groups = ((max_tokens + 255) / 256) as u32;
        self.context.dispatch(&self.keyword_lookup_shader, (groups, 1, 1));

        Ok(())
    }

    fn readback_tokens(&self, max_tokens: usize) -> Result<Vec<GpuToken>> {
        // 读取 token 数量
        let mut count = 0u32;
        self.context.read_buffer(
            self.token_count_buffer.as_ref().unwrap(),
            unsafe {
                std::slice::from_raw_parts_mut(
                    &mut count as *mut u32 as *mut u8,
                    std::mem::size_of::<u32>(),
                )
            },
        )?;

        let token_count = count.min(max_tokens as u32) as usize;
        if token_count == 0 {
            return Ok(Vec::new());
        }

        // 读取 token 列表
        let mut tokens = vec![GpuToken::default(); token_count];
        let token_bytes = unsafe {
            std::slice::from_raw_parts_mut(
                tokens.as_mut_ptr() as *mut u8,
                token_count * std::mem::size_of::<GpuToken>(),
            )
        };

        self.context.read_buffer(
            self.tokens_buffer.as_ref().unwrap(),
            token_bytes,
        )?;

        Ok(tokens)
    }

    fn create_uav_from_buffer(
        &self,
        buffer: &ID3D11Buffer,
        num_elements: u32,
    ) -> Result<ID3D11UnorderedAccessView> {
        let uav_desc = D3D11_UNORDERED_ACCESS_VIEW_DESC {
            Format: DXGI_FORMAT_R32_UINT,
            ViewDimension: windows::Win32::Graphics::Direct3D11::D3D11_UAV_DIMENSION_BUFFER,
            Anonymous: D3D11_UNORDERED_ACCESS_VIEW_DESC_0 {
                Buffer: D3D11_BUFFER_UAV {
                    FirstElement: 0,
                    NumElements: num_elements,
                    Flags: 0,
                },
            },
        };
        unsafe {
            let mut uav = None;
            self.context.device().CreateUnorderedAccessView(buffer, Some(&uav_desc), Some(&mut uav))?;
            Ok(uav.unwrap())
        }
    }
}
