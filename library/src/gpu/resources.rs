use crate::gpu::context::Context;
use crate::utils::bitmap_utils::{BitmapSize, BYTES_IN_RGBA_QUARTET};
use more_asserts::{assert_gt, assert_le};
use std::rc::Rc;
use wgpu::util::DeviceExt;
use wgpu::{BufferUsages, Sampler, SamplerBorderColor, Texture};

pub(crate) struct Resources {
    context: Rc<Context>,
}

impl Resources {
    #[must_use]
    pub(crate) fn new(context: Rc<Context>) -> Self {
        Self { context }
    }

    #[must_use]
    pub(crate) fn create_shader_module(&self, label: &str, shader_source_code: &str) -> Rc<wgpu::ShaderModule> {
        Rc::new(
            self.context.device().create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(shader_source_code.into()), //TODO: can we use naga module created above?
            })
        )
    }

    #[must_use]
    pub(crate) fn create_buffer(&self, label: &str, usage: BufferUsages, buffer_data: &[u8]) -> Rc<wgpu::Buffer> {
        let buffer = self.context.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: buffer_data,
            usage,
        });
        Rc::new(buffer)
    }

    #[must_use]
    pub(super) fn create_uniform_buffer(&self, label: &str, buffer_data: &[u8]) -> Rc<wgpu::Buffer> {
        self.create_buffer(label, BufferUsages::UNIFORM | BufferUsages::COPY_DST, buffer_data)
    }

    #[must_use]
    pub(crate) fn create_storage_buffer_write_only(&self, label: &str, buffer_data: &[u8]) -> Rc<wgpu::Buffer> {
        self.create_buffer(label, BufferUsages::STORAGE | BufferUsages::COPY_DST, buffer_data)
    }

    #[must_use]
    pub(crate) fn create_sampler(&self, label: &str) -> Sampler {
        self.context.device().create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: wgpu::AddressMode::ClampToBorder,
            address_mode_v: wgpu::AddressMode::ClampToBorder,
            address_mode_w: wgpu::AddressMode::ClampToBorder,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: f32::MAX,
            compare: None,
            anisotropy_clamp: 1,
            border_color: Some(SamplerBorderColor::OpaqueBlack),
        })
    }

    #[must_use]
    fn calculate_max_mips(width: usize, height: usize) -> u32 {
        (width.max(height) as f32).log2().floor() as u32 + 1
    }

    #[must_use]
    pub(crate) fn create_texture(&self, label: &str, mip_count: u32, atlas_page_size: BitmapSize) -> Texture {
        assert_le!(mip_count, Self::calculate_max_mips(atlas_page_size.width(), atlas_page_size.height()), "too many mip_count");
        assert_gt!(mip_count, 0, "mip_count must be greater than 0");

        self.context.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: atlas_page_size.width() as u32, height: atlas_page_size.height() as u32, depth_or_array_layers: 1, },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    pub(crate) fn write_whole_srgba_texture_data(&self, texture: &Texture, data: &[u8]) {
        assert_eq!(data.len(), BitmapSize::new(texture.size().width as usize, texture.size().height as usize).bytes_in_bitmap());

        self.context.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture.width() * BYTES_IN_RGBA_QUARTET as u32),
                rows_per_image: Some(texture.height()),
            },
            wgpu::Extent3d {
                width: texture.width(),
                height: texture.height(),
                depth_or_array_layers: 1,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use test_context::{test_context, TestContext};

    struct Context {
        system_under_test: Resources
    }

    impl TestContext for Context {
        fn setup() -> Context {
            Context { system_under_test: Resources{context: create_headless_wgpu_context()} }
        }

        fn teardown(self) {
        }
    }

    const TRIVIAL_SHADER_CODE: &str = r#"
        @vertex
        fn vs_main() -> @builtin(position) vec4<f32> {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }
    "#;

    const SHADER_CODE_WITH_SYNTAX_ERROR: &str = r#"
        @vertex
        fn vs_main() -> @builtin(position) vec4<f32> {
            return 1.0);
        }
    "#;

    const SHADER_CODE_WITH_VALIDATION_ERROR: &str = r#"
        @vertex
        fn vs_main() -> @builtin(position) vec4<f32> {
            return vec3<f32>(0, 0, 0, 1);
        }
    "#;

    const DUMMY_BYTE_ARRAY: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

    #[test_context(Context)]
    #[test]
    fn test_create_shader_module_successful_compilation(fixture: &mut Context) {
        let _ = fixture.system_under_test.create_shader_module(
            concat!("unit tests: file ", file!(), ", line: ", line!()), TRIVIAL_SHADER_CODE);
    }

    #[test_context(Context)]
    #[test]
    #[should_panic]
    fn test_create_shader_module_syntax_error_compilation(fixture: &Context) {
        let _ = fixture.system_under_test.create_shader_module(
            concat!("unit tests: file ", file!(), ", line: ", line!()), SHADER_CODE_WITH_SYNTAX_ERROR);
    }

    #[test_context(Context)]
    #[test]
    #[should_panic]
    fn test_create_shader_module_validation_error_compilation(fixture: &Context) {
        let _ = fixture.system_under_test.create_shader_module(
            concat!("unit tests: file ", file!(), ", line: ", line!()), SHADER_CODE_WITH_VALIDATION_ERROR);
    }

    #[test_context(Context)]
    #[test]
    fn test_create_uniform_buffer(fixture: &Context) {
        let buffer = fixture.system_under_test.create_uniform_buffer(
            concat!("unit tests: buffer ", file!(), ", line: ", line!()), &DUMMY_BYTE_ARRAY);

        // TODO: do we need to wait for the queue to finish write? guess, yes
        // TODO: system_under_test.queue.submit([]); - this will initiate the actual data transfer on GPU

        assert_eq!(buffer.usage(), BufferUsages::UNIFORM | BufferUsages::COPY_DST);
    }

    #[test_context(Context)]
    #[test]
    fn test_create_storage_buffer_write_only(fixture: &Context) {
        let buffer = fixture.system_under_test.create_storage_buffer_write_only(
            concat!("unit tests: buffer ", file!(), ", line: ", line!()), &DUMMY_BYTE_ARRAY);

        assert_eq!(buffer.usage(), BufferUsages::STORAGE | BufferUsages::COPY_DST);
    }
}