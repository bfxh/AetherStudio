use windows::core::Result;
use windows::Win32::Graphics::Direct3D::Fxc::{
    D3DCompile, D3DCOMPILE_OPTIMIZATION_LEVEL3, D3DCOMPILE_ENABLE_STRICTNESS,
};
use windows::Win32::Graphics::Direct3D::ID3DBlob;

/// Shader 编译器
///
/// 使用 d3dcompiler_47.dll 将 HLSL 源码编译为 CSO (Compiled Shader Object)。
pub struct ShaderCompiler;

impl ShaderCompiler {
    /// 编译 HLSL 源码为 Compute Shader Blob
    ///
    /// # Arguments
    /// * `hlsl` - HLSL 源码字符串
    /// * `entry_point` - 入口函数名（如 "main"）
    /// * `target` - 目标 Shader Model（如 "cs_5_0"）
    ///
    /// # Returns
    /// 编译后的字节码
    pub fn compile_compute_shader(
        hlsl: &str,
        entry_point: &str,
        target: &str,
    ) -> Result<Vec<u8>> {
        if hlsl.is_empty() {
            return Err(windows::core::Error::new(
                windows::Win32::Foundation::E_FAIL,
                "Empty HLSL source",
            ));
        }

        let hlsl_bytes = hlsl.as_bytes();
        let entry = windows::core::PCSTR::from_raw(entry_point.as_ptr());
        let target_str = windows::core::PCSTR::from_raw(target.as_ptr());
        let source_name = windows::core::PCSTR::from_raw(b"shader.hlsl\0".as_ptr());

        let mut code_blob: Option<ID3DBlob> = None;
        let mut error_blob: Option<ID3DBlob> = None;

        let flags = D3DCOMPILE_OPTIMIZATION_LEVEL3 | D3DCOMPILE_ENABLE_STRICTNESS;

        unsafe {
            let hr = D3DCompile(
                hlsl_bytes.as_ptr() as *const _,
                hlsl_bytes.len(),
                source_name,
                None,
                None,
                entry,
                target_str,
                flags,
                0,
                &mut code_blob,
                Some(&mut error_blob),
            );

            if let Some(error) = error_blob {
                let ptr = error.GetBufferPointer();
                let size = error.GetBufferSize();
                if size > 0 && !ptr.is_null() {
                    let msg = std::slice::from_raw_parts(ptr as *const u8, size);
                    let error_str = String::from_utf8_lossy(msg);
                    eprintln!("Shader compilation error: {}", error_str);
                }
            }

            hr?;

            match code_blob {
                Some(blob) => {
                    let ptr = blob.GetBufferPointer();
                    let size = blob.GetBufferSize();
                    let bytecode = std::slice::from_raw_parts(ptr as *const u8, size).to_vec();
                    Ok(bytecode)
                }
                None => Err(windows::core::Error::new(
                    windows::Win32::Foundation::E_FAIL,
                    "D3DCompile succeeded but returned no bytecode",
                )),
            }
        }
    }

    /// 便捷方法：编译 Phase 1 字符分类 Shader
    pub fn compile_char_classify(hlsl: &str) -> Result<Vec<u8>> {
        Self::compile_compute_shader(hlsl, "main", "cs_5_0")
    }

    /// 便捷方法：编译 Phase 2 Token 扫描 Shader
    pub fn compile_token_scan(hlsl: &str) -> Result<Vec<u8>> {
        Self::compile_compute_shader(hlsl, "main", "cs_5_0")
    }

    /// 便捷方法：编译 Phase 3 关键字查找 Shader
    pub fn compile_keyword_lookup(hlsl: &str) -> Result<Vec<u8>> {
        Self::compile_compute_shader(hlsl, "main", "cs_5_0")
    }

    /// 便捷方法：编译语法分类 Shader
    pub fn compile_syntax_classify(hlsl: &str) -> Result<Vec<u8>> {
        Self::compile_compute_shader(hlsl, "main", "cs_5_0")
    }
}

/// 预编译 Shader 加载器
///
/// 从编译时嵌入的 CSO 文件加载 Shader。
pub struct PrecompiledShader;

impl PrecompiledShader {
    /// 加载预编译的 Compute Shader
    pub fn load_compute_shader(
        context: &super::compute_context::GpuComputeContext,
        bytecode: &[u8],
    ) -> Result<windows::Win32::Graphics::Direct3D11::ID3D11ComputeShader> {
        context.create_compute_shader(bytecode)
    }
}

/// Shader 常量缓冲区
///
/// 用于向 Shader 传递常量参数。
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LexerConstants {
    /// 文本长度
    pub text_length: u32,
    /// 最大 Token 数
    pub max_tokens: u32,
    /// DFA 状态数
    pub num_states: u32,
    /// 关键字表大小
    pub keyword_table_size: u32,
    /// 保留
    pub _padding: [u32; 4],
}

/// 创建常量缓冲区
pub fn create_constant_buffer<T: Sized>(
    context: &super::compute_context::GpuComputeContext,
    data: &T,
) -> Result<windows::Win32::Graphics::Direct3D11::ID3D11Buffer> {
    let size = std::mem::size_of::<T>();
    let bytes = unsafe {
        std::slice::from_raw_parts(
            data as *const T as *const u8,
            size,
        )
    };

    context.create_buffer(size, super::compute_context::BufferUsage::Constant, Some(bytes))
}
