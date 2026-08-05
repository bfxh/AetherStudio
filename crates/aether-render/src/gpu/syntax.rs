use windows::core::Result;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11ComputeShader, ID3D11ShaderResourceView, ID3D11UnorderedAccessView,
    D3D11_BUFFER_UAV, D3D11_UNORDERED_ACCESS_VIEW_DESC, D3D11_UNORDERED_ACCESS_VIEW_DESC_0,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_UINT;

use super::compute_context::GpuComputeContext;

/// 语法分类类型
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SyntaxClass {
    pub class_id: u32,
    pub confidence: u32, // 0-100，分类置信度
    pub _padding: [u32; 2],
}

/// 语法分类常量
pub mod syntax_classes {
    pub const SYNTAX_UNKNOWN: u32 = 0;
    pub const SYNTAX_FUNCTION_DECL: u32 = 1;
    pub const SYNTAX_FUNCTION_CALL: u32 = 2;
    pub const SYNTAX_TYPE_NAME: u32 = 3;
    pub const SYNTAX_VARIABLE_DECL: u32 = 4;
    pub const SYNTAX_VARIABLE_REF: u32 = 5;
    pub const SYNTAX_PARAMETER: u32 = 6;
    pub const SYNTAX_FIELD_ACCESS: u32 = 7;
    pub const SYNTAX_MACRO: u32 = 8;
    pub const SYNTAX_ATTRIBUTE: u32 = 9;
    pub const SYNTAX_LIFETIME: u32 = 10;
    pub const SYNTAX_MODULE: u32 = 11;
    pub const SYNTAX_TRAIT: u32 = 12;
    pub const SYNTAX_STRUCT: u32 = 13;
    pub const SYNTAX_ENUM: u32 = 14;
    pub const SYNTAX_IMPL: u32 = 15;
}

/// GPU 语法分类器
///
/// 基于 Token 流进行简单语法模式的 GPU 并行识别。
/// 复杂语法分析仍由 Tree-sitter 处理。
pub struct GpuSyntaxClassifier {
    context: GpuComputeContext,

    // 语言特定的语法模式
    patterns_buffer: ID3D11Buffer,
    patterns_srv: ID3D11ShaderResourceView,

    // Compute Shader
    classify_shader: ID3D11ComputeShader,
}

/// 语法模式定义（CPU 侧）
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SyntaxPattern {
    /// 模式类型
    pub pattern_type: u32,
    /// Token 类型序列（最多 4 个）
    pub token_sequence: [u32; 4],
    /// 序列长度
    pub sequence_len: u32,
    /// 输出语法分类
    pub output_class: u32,
    /// 优先级（高优先级覆盖低优先级）
    pub priority: u32,
    /// 保留
    pub _padding: [u32; 2],
}

impl GpuSyntaxClassifier {
    /// 创建语法分类器
    pub fn new(context: GpuComputeContext, language: &str) -> Result<Self> {
        let patterns = Self::build_patterns(language);
        let (patterns_buf, patterns_srv) = Self::create_patterns_buffer(&context, &patterns)?;

        let classify_shader = Self::load_shader(
            &context,
            &[], // 占位：实际使用时应加载预编译的 syntax_classify.cso
        )?;

        Ok(Self {
            context,
            patterns_buffer: patterns_buf,
            patterns_srv,
            classify_shader,
        })
    }

    /// 对 Token 流进行语法分类
    ///
    /// # Arguments
    /// * `tokens` - 输入 Token 列表（GPU 缓冲区）
    ///
    /// # Returns
    /// 语法分类结果（GPU 缓冲区）
    pub fn classify(&self, tokens: &ID3D11Buffer, token_count: usize) -> Result<ID3D11Buffer> {
        // 创建输出缓冲区
        let output_size = token_count * std::mem::size_of::<SyntaxClass>();
        let output = self.context.create_buffer(
            output_size,
            super::compute_context::BufferUsage::ReadWrite,
            None,
        )?;

        let output_uav = self.create_uav(&output, token_count as u32)?;
        let tokens_srv = self.context.create_srv(tokens)?;

        // 设置 Shader 资源
        self.context.set_compute_shader(&self.classify_shader);
        self.context
            .set_shader_resources(0, &[Some(tokens_srv), Some(self.patterns_srv.clone())]);
        self.context
            .set_unordered_access_views(0, &[Some(output_uav)]);

        // 分派
        let groups = ((token_count + 255) / 256) as u32;
        self.context.dispatch(&self.classify_shader, (groups, 1, 1));

        Ok(output)
    }

    /// 回读语法分类结果
    pub fn readback_classes(
        &self,
        buffer: &ID3D11Buffer,
        count: usize,
    ) -> Result<Vec<SyntaxClass>> {
        let mut classes = vec![SyntaxClass::default(); count];
        let bytes = unsafe {
            std::slice::from_raw_parts_mut(
                classes.as_mut_ptr() as *mut u8,
                count * std::mem::size_of::<SyntaxClass>(),
            )
        };

        self.context.read_buffer(buffer, bytes)?;
        Ok(classes)
    }

    // 私有方法

    fn build_patterns(_language: &str) -> Vec<SyntaxPattern> {
        // 通用语法模式（适用于类 C 语言）
        vec![
            // 函数声明: type name( 或 fn name(
            SyntaxPattern {
                pattern_type: 1,
                token_sequence: [
                    super::lexer::token_types::TOKEN_IDENTIFIER,
                    super::lexer::token_types::TOKEN_IDENTIFIER,
                    super::lexer::token_types::TOKEN_PUNCTUATION,
                    0,
                ],
                sequence_len: 3,
                output_class: syntax_classes::SYNTAX_FUNCTION_DECL,
                priority: 100,
                _padding: [0; 2],
            },
            // 函数调用: name(
            SyntaxPattern {
                pattern_type: 2,
                token_sequence: [
                    super::lexer::token_types::TOKEN_IDENTIFIER,
                    super::lexer::token_types::TOKEN_PUNCTUATION,
                    0,
                    0,
                ],
                sequence_len: 2,
                output_class: syntax_classes::SYNTAX_FUNCTION_CALL,
                priority: 80,
                _padding: [0; 2],
            },
            // 类型声明: struct/enum/trait Name
            SyntaxPattern {
                pattern_type: 3,
                token_sequence: [
                    super::lexer::token_types::TOKEN_KEYWORD,
                    super::lexer::token_types::TOKEN_IDENTIFIER,
                    0,
                    0,
                ],
                sequence_len: 2,
                output_class: syntax_classes::SYNTAX_TYPE_NAME,
                priority: 90,
                _padding: [0; 2],
            },
            // 变量声明: let mut name
            SyntaxPattern {
                pattern_type: 4,
                token_sequence: [
                    super::lexer::token_types::TOKEN_KEYWORD,
                    super::lexer::token_types::TOKEN_KEYWORD,
                    super::lexer::token_types::TOKEN_IDENTIFIER,
                    0,
                ],
                sequence_len: 3,
                output_class: syntax_classes::SYNTAX_VARIABLE_DECL,
                priority: 85,
                _padding: [0; 2],
            },
            // 字段访问: .name
            SyntaxPattern {
                pattern_type: 5,
                token_sequence: [
                    super::lexer::token_types::TOKEN_PUNCTUATION,
                    super::lexer::token_types::TOKEN_IDENTIFIER,
                    0,
                    0,
                ],
                sequence_len: 2,
                output_class: syntax_classes::SYNTAX_FIELD_ACCESS,
                priority: 70,
                _padding: [0; 2],
            },
            // 宏调用: name!
            SyntaxPattern {
                pattern_type: 6,
                token_sequence: [
                    super::lexer::token_types::TOKEN_IDENTIFIER,
                    super::lexer::token_types::TOKEN_PUNCTUATION,
                    0,
                    0,
                ],
                sequence_len: 2,
                output_class: syntax_classes::SYNTAX_MACRO,
                priority: 75,
                _padding: [0; 2],
            },
        ]
    }

    fn create_patterns_buffer(
        context: &GpuComputeContext,
        patterns: &[SyntaxPattern],
    ) -> Result<(ID3D11Buffer, ID3D11ShaderResourceView)> {
        let bytes = unsafe {
            std::slice::from_raw_parts(
                patterns.as_ptr() as *const u8,
                patterns.len() * std::mem::size_of::<SyntaxPattern>(),
            )
        };

        let buffer = context.create_buffer(
            bytes.len(),
            super::compute_context::BufferUsage::Structured,
            Some(bytes),
        )?;

        let srv = context.create_srv(&buffer)?;

        Ok((buffer, srv))
    }

    fn load_shader(context: &GpuComputeContext, bytecode: &[u8]) -> Result<ID3D11ComputeShader> {
        context.create_compute_shader(bytecode)
    }

    fn create_uav(
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
            self.context.device().CreateUnorderedAccessView(
                buffer,
                Some(&uav_desc),
                Some(&mut uav),
            )?;
            Ok(uav.unwrap())
        }
    }
}
