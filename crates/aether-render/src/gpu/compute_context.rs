use windows::core::Result;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11ComputeShader, ID3D11Device, ID3D11DeviceContext,
    ID3D11ShaderResourceView, ID3D11UnorderedAccessView,
    D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_SHADER_RESOURCE,
    D3D11_BIND_UNORDERED_ACCESS, D3D11_BUFFER_DESC, D3D11_BUFFER_SRV, D3D11_BUFFER_UAV,
    D3D11_CPU_ACCESS_READ, D3D11_CPU_ACCESS_WRITE, D3D11_RESOURCE_MISC_BUFFER_STRUCTURED,
    D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0,
    D3D11_SUBRESOURCE_DATA, D3D11_UNORDERED_ACCESS_VIEW_DESC,
    D3D11_UNORDERED_ACCESS_VIEW_DESC_0, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Direct3D::D3D11_SRV_DIMENSION_BUFFER;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_UINT;
use windows::Win32::Graphics::Direct3D11::D3D11CreateDevice;
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::D3D11_CREATE_DEVICE_BGRA_SUPPORT;

/// GPU 计算上下文，封装 D3D11 Compute Shader 所需的所有资源
///
/// 从现有 Direct2D 工厂获取底层 D3D11 设备，实现渲染和计算共享 GPU。
pub struct GpuComputeContext {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
}

/// GPU 缓冲区使用方式
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferUsage {
    /// 常量缓冲区 (CBV)，用于 DFA 表、关键字哈希表等只读数据
    Constant,
    /// 结构化缓冲区 (SRV)，用于输入数据
    Structured,
    /// 可读写缓冲区 (UAV)，用于 Compute Shader 输出
    ReadWrite,
    /// 暂存缓冲区 (Staging)，用于 CPU 读取 GPU 结果
    Staging,
}

impl GpuComputeContext {
    /// 从 D3D11 设备创建计算上下文
    ///
    /// 调用方需要从 D2DFactory 获取底层 DXGI 设备，再查询到 D3D11 设备。
    pub fn new(device: ID3D11Device) -> Result<GpuComputeContext> {
        let context = unsafe { device.GetImmediateContext()? };
        Ok(GpuComputeContext { device, context })
    }

    /// 从 D2D Factory 创建 GPU 计算上下文
    ///
    /// 通过 D3D11CreateDevice 创建独立的 D3D11 设备用于 Compute Shader。
    /// 与 D2D 渲染设备分离，避免互相影响。
    pub fn create_from_d2d(_d2d_factory: &super::super::d2d::factory::D2DFactory) -> Result<GpuComputeContext> {
        unsafe {
            let mut device = None;
            let mut context = None;
            let feature_levels = [D3D_FEATURE_LEVEL_11_0];
            let hr = D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&feature_levels),
                windows::Win32::Graphics::Direct3D11::D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            );
            hr?;
            let device = device.ok_or_else(|| windows::core::Error::new(
                windows::Win32::Foundation::E_FAIL,
                "D3D11CreateDevice returned no device",
            ))?;
            let context = context.ok_or_else(|| windows::core::Error::new(
                windows::Win32::Foundation::E_FAIL,
                "D3D11CreateDevice returned no context",
            ))?;
            Ok(GpuComputeContext { device, context })
        }
    }

    /// 获取 D3D11 设备引用
    pub fn device(&self) -> &ID3D11Device {
        &self.device
    }

    /// 获取 D3D11 设备上下文引用
    pub fn context(&self) -> &ID3D11DeviceContext {
        &self.context
    }

    /// 创建 Compute Shader
    ///
    /// # Arguments
    /// * `bytecode` - 预编译的 Shader Blob (CSO)
    pub fn create_compute_shader(&self, bytecode: &[u8]) -> Result<ID3D11ComputeShader> {
        if bytecode.is_empty() {
            // 返回空 shader 作为占位
            return Err(windows::core::Error::new(
                windows::Win32::Foundation::E_FAIL,
                "Empty shader bytecode - GPU lexing requires compiled CSO files",
            ));
        }
        unsafe {
            let mut shader = None;
            self.device
                .CreateComputeShader(bytecode, None, Some(&mut shader))?;
            Ok(shader.unwrap())
        }
    }

    /// 创建 GPU 缓冲区
    ///
    /// # Arguments
    /// * `size` - 缓冲区字节大小
    /// * `usage` - 缓冲区使用方式
    /// * `data` - 可选的初始数据
    pub fn create_buffer(
        &self,
        size: usize,
        usage: BufferUsage,
        data: Option<&[u8]>,
    ) -> Result<ID3D11Buffer> {
        let (desc, subresource) = Self::build_buffer_desc(size, usage, data)?;

        unsafe {
            let mut buffer = None;
            self.device.CreateBuffer(
                &desc,
                subresource.as_ref().map(|s| s as *const _),
                Some(&mut buffer),
            )?;
            Ok(buffer.unwrap())
        }
    }

    /// 创建结构化缓冲区及其 UAV
    ///
    /// 用于 Compute Shader 的输入/输出。
    pub fn create_structured_buffer<T: Sized>(
        &self,
        count: usize,
        initial_data: Option<&[T]>,
        read_write: bool,
    ) -> Result<(ID3D11Buffer, Option<ID3D11UnorderedAccessView>)> {
        let element_size = std::mem::size_of::<T>();
        let total_size = count * element_size;

        let mut bind_flags = D3D11_BIND_SHADER_RESOURCE.0;
        if read_write {
            bind_flags |= D3D11_BIND_UNORDERED_ACCESS.0;
        }

        let desc = D3D11_BUFFER_DESC {
            ByteWidth: total_size as u32,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: bind_flags as u32,
            CPUAccessFlags: 0,
            MiscFlags: D3D11_RESOURCE_MISC_BUFFER_STRUCTURED.0 as u32,
            StructureByteStride: element_size as u32,
        };

        let subresource = initial_data.map(|data| {
            D3D11_SUBRESOURCE_DATA {
                pSysMem: data.as_ptr() as *const _,
                SysMemPitch: 0,
                SysMemSlicePitch: 0,
            }
        });

        let buffer = unsafe {
            let mut buffer = None;
            self.device.CreateBuffer(
                &desc,
                subresource.as_ref().map(|s| s as *const _),
                Some(&mut buffer),
            )?;
            buffer.unwrap()
        };

        let uav = if read_write {
            let uav_desc = D3D11_UNORDERED_ACCESS_VIEW_DESC {
                Format: DXGI_FORMAT_R32_UINT,
                ViewDimension: windows::Win32::Graphics::Direct3D11::D3D11_UAV_DIMENSION_BUFFER,
                Anonymous: D3D11_UNORDERED_ACCESS_VIEW_DESC_0 {
                    Buffer: D3D11_BUFFER_UAV {
                        FirstElement: 0,
                        NumElements: (total_size / 4) as u32,
                        Flags: 0,
                    },
                },
            };
            let mut uav = None;
            unsafe {
                self.device.CreateUnorderedAccessView(&buffer, Some(&uav_desc), Some(&mut uav))?;
            }
            uav
        } else {
            None
        };

        Ok((buffer, uav))
    }

    /// 创建 Shader Resource View (SRV)
    pub fn create_srv(&self, buffer: &ID3D11Buffer) -> Result<ID3D11ShaderResourceView> {
        let srv_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
            Format: DXGI_FORMAT_R32_UINT,
            ViewDimension: D3D11_SRV_DIMENSION_BUFFER,
            Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                Buffer: D3D11_BUFFER_SRV {
                    Anonymous1: windows::Win32::Graphics::Direct3D11::D3D11_BUFFER_SRV_0 {
                        FirstElement: 0,
                    },
                    Anonymous2: windows::Win32::Graphics::Direct3D11::D3D11_BUFFER_SRV_1 {
                        NumElements: 0, // 由缓冲区大小推断
                    },
                },
            },
        };
        unsafe {
            let mut srv = None;
            self.device.CreateShaderResourceView(buffer, Some(&srv_desc), Some(&mut srv))?;
            Ok(srv.unwrap())
        }
    }

    /// 分派 Compute Shader
    ///
    /// # Arguments
    /// * `shader` - Compute Shader
    /// * `thread_groups` - (X, Y, Z) 线程组数量
    pub fn dispatch(
        &self,
        _shader: &ID3D11ComputeShader,
        thread_groups: (u32, u32, u32),
    ) {
        unsafe {
            self.context.Dispatch(thread_groups.0, thread_groups.1, thread_groups.2);
        }
    }

    /// 设置 Compute Shader
    pub fn set_compute_shader(&self, shader: &ID3D11ComputeShader) {
        unsafe {
            self.context.CSSetShader(shader, None);
        }
    }

    /// 设置 Shader Resource Views
    pub fn set_shader_resources(&self, start_slot: u32, srvs: &[Option<ID3D11ShaderResourceView>]) {
        unsafe {
            self.context.CSSetShaderResources(start_slot, Some(srvs));
        }
    }

    /// 设置 Unordered Access Views
    pub fn set_unordered_access_views(
        &self,
        start_slot: u32,
        uavs: &[Option<ID3D11UnorderedAccessView>],
    ) {
        unsafe {
            self.context.CSSetUnorderedAccessViews(
                start_slot,
                uavs.len() as u32,
                Some(uavs.as_ptr()),
                None,
            );
        }
    }

    /// 从 GPU 读取缓冲区数据
    ///
    /// 使用暂存缓冲区实现异步回读。
    pub fn read_buffer(&self, src: &ID3D11Buffer, dest: &mut [u8]) -> Result<()> {
        let desc = D3D11_BUFFER_DESC {
            ByteWidth: dest.len() as u32,
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
            StructureByteStride: 0,
        };

        let staging = unsafe {
            let mut buffer = None;
            self.device.CreateBuffer(&desc, None, Some(&mut buffer))?;
            buffer.unwrap()
        };

        unsafe {
            self.context.CopyResource(&staging, src);

            let mut mapped = windows::Win32::Graphics::Direct3D11::D3D11_MAPPED_SUBRESOURCE::default();
            self.context.Map(
                &staging,
                0,
                windows::Win32::Graphics::Direct3D11::D3D11_MAP_READ,
                0,
                Some(&mut mapped),
            )?;

            std::ptr::copy_nonoverlapping(
                mapped.pData as *const u8,
                dest.as_mut_ptr(),
                dest.len(),
            );

            self.context.Unmap(&staging, 0);
        }

        Ok(())
    }

    /// 上传数据到 GPU 缓冲区
    pub fn write_buffer(&self, buffer: &ID3D11Buffer, data: &[u8]) -> Result<()> {
        let desc = D3D11_BUFFER_DESC {
            ByteWidth: data.len() as u32,
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            MiscFlags: 0,
            StructureByteStride: 0,
        };

        let staging = unsafe {
            let mut buffer = None;
            self.device.CreateBuffer(&desc, None, Some(&mut buffer))?;
            buffer.unwrap()
        };

        unsafe {
            let mut mapped = windows::Win32::Graphics::Direct3D11::D3D11_MAPPED_SUBRESOURCE::default();
            self.context.Map(
                &staging,
                0,
                windows::Win32::Graphics::Direct3D11::D3D11_MAP_WRITE,
                0,
                Some(&mut mapped),
            )?;

            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                mapped.pData as *mut u8,
                data.len(),
            );

            self.context.Unmap(&staging, 0);
            self.context.CopyResource(buffer, &staging);
        }

        Ok(())
    }

    /// 构建缓冲区描述
    fn build_buffer_desc(
        size: usize,
        usage: BufferUsage,
        data: Option<&[u8]>,
    ) -> Result<(D3D11_BUFFER_DESC, Option<D3D11_SUBRESOURCE_DATA>)> {
        let (usage_type, bind_flags, cpu_access) = match usage {
            BufferUsage::Constant => (
                D3D11_USAGE_DEFAULT,
                D3D11_BIND_CONSTANT_BUFFER.0,
                0,
            ),
            BufferUsage::Structured => (
                D3D11_USAGE_DEFAULT,
                D3D11_BIND_SHADER_RESOURCE.0,
                0,
            ),
            BufferUsage::ReadWrite => (
                D3D11_USAGE_DEFAULT,
                D3D11_BIND_UNORDERED_ACCESS.0 | D3D11_BIND_SHADER_RESOURCE.0,
                0,
            ),
            BufferUsage::Staging => (
                D3D11_USAGE_STAGING,
                0,
                D3D11_CPU_ACCESS_READ.0 | D3D11_CPU_ACCESS_WRITE.0,
            ),
        };

        let desc = D3D11_BUFFER_DESC {
            ByteWidth: size as u32,
            Usage: usage_type,
            BindFlags: bind_flags as u32,
            CPUAccessFlags: cpu_access as u32,
            MiscFlags: 0,
            StructureByteStride: 0,
        };

        let subresource = data.map(|d| D3D11_SUBRESOURCE_DATA {
            pSysMem: d.as_ptr() as *const _,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        });

        Ok((desc, subresource))
    }
}

impl Clone for GpuComputeContext {
    fn clone(&self) -> Self {
        Self {
            device: self.device.clone(),
            context: self.context.clone(),
        }
    }
}
