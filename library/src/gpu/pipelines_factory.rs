use crate::gpu::context::Context;
use crate::gpu::pipeline_code::{PipelineCode, ShaderHash};
use bitflags::bitflags;
use derive_more::Display;
use log::info;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use wgpu::{AdapterInfo, PipelineCache, PipelineCacheDescriptor};

pub (crate) struct PipelinesFactory {
    context: Rc<Context>,
    presentation_format: wgpu::TextureFormat,
    caches_path: Option<PathBuf>,
    caches: HashMap<String, CacheAndHash>,
    io: Rc<dyn Io>,
}

struct CacheAndHash {
    this: Rc<PipelineCache>,
    hash: ShaderHash,
}

impl PipelinesFactory {
    const RASTERIZATION_PIPELINE_LABEL: &'static str = "rasterization pipeline";
    const DISK_CACHE_VERSION_CODE: usize = 0;

    #[must_use]
    pub (crate) fn new(context: Rc<Context>, presentation_format: wgpu::TextureFormat, caches_path: Option<PathBuf>,) -> Self {
        Self::new_with_custom_io(context, presentation_format, caches_path, Rc::new(FileSystemIo))
    }

    #[must_use]
    fn new_with_custom_io(context: Rc<Context>, presentation_format: wgpu::TextureFormat, caches_path: Option<PathBuf>, io: Rc<dyn Io>,) -> Self {
        if let Some(path) = caches_path.clone()
            && let Err(e) = fs::create_dir_all(&path) {
                info!("failed to create directories in path {path:?}: {e}");
            }
        Self { context, presentation_format, caches_path, caches: HashMap::new(), io, }
    }

    #[must_use]
    pub(super) fn create_rasterization_pipeline(&mut self, code: &PipelineCode) -> wgpu::RenderPipeline {
        let (cache, status) = self.find_or_create_cache(code.human_readable_uid(), code.content_hash());
        let pipeline = self.context.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(Self::RASTERIZATION_PIPELINE_LABEL),
            layout: None,
            vertex: wgpu::VertexState {
                module: code.module().as_ref(),
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[], // full screen quad vertices specified as a const in the shader
            },
            fragment: Some(wgpu::FragmentState {
                module: code.module().as_ref(),
                entry_point: None,
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.presentation_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: cache.as_deref(),
        });
        
        self.handle_actions(code, cache, status);
        pipeline
    }

    #[must_use]
    pub(crate) fn create_compute_pipeline(&mut self, routine: ComputeRoutineEntryPoint, code: &PipelineCode) -> wgpu::ComputePipeline {
        let (cache, actions) = self.find_or_create_cache(code.human_readable_uid(), code.content_hash());
        let pipeline = self.context.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: routine.name(),
            compilation_options: Default::default(),
            layout: None,
            module: code.module().as_ref(),
            entry_point: routine.name(),
            cache: cache.as_deref(),
        });
        
        self.handle_actions(code, cache, actions);
        pipeline
    }

    fn handle_actions(&mut self, code: &PipelineCode, cache: Option<Rc<PipelineCache>>, action: CacheAction) {
        if let Some(cache) = cache {
            if action.contains(CacheAction::SaveOnDisk) {
                self.store_cache_on_disk(code.human_readable_uid(), cache.get_data(), code.content_hash());
            }
            if action.contains(CacheAction::StoreInMemory) {
                let record = CacheAndHash { this: cache, hash: code.content_hash() };
                self.caches.insert(code.human_readable_uid().clone(), record);
            }   
        }
    }

    fn store_cache_on_disk(&self, uid: &str, data_or_none: Option<Vec<u8>>, hash: ShaderHash) {
        let Some(data) = data_or_none else { return };
        let Some(caches_directory) = self.caches_path.as_ref() else { return };

        let cache_file_path = Self::path_to_cache(uid, caches_directory);

        let memento = PipelineCacheMemento { hash, data, adapter_info: ShortAdapterInfo::new(self.context.adapter_info()), };
        self.io.save(cache_file_path.as_path(), &memento);
    }

    #[must_use]
    fn find_or_create_cache(&self, uid: &str, desired_hash: ShaderHash) -> (Option<Rc<PipelineCache>>, CacheAction) {
        if let Some(cache) = self.caches.get(uid) {
            if cache.hash == desired_hash {
                return (Some(cache.this.clone()), CacheAction::empty());    
            }
            return (self.create_pipeline_cache(uid, None), CacheAction::StoreInMemory | CacheAction::SaveOnDisk);
        }
        
        if let Some(cache_data) = self.try_load_cache_data_from_disk(uid, desired_hash) {
            (self.create_pipeline_cache(uid, Some(cache_data.as_slice())), CacheAction::StoreInMemory)
        } else {
            (self.create_pipeline_cache(uid, None), CacheAction::StoreInMemory | CacheAction::SaveOnDisk)
        }
    }
    
    #[must_use]
    fn create_pipeline_cache(&self, uid: &str, data: Option<&'_ [u8]>,) -> Option<Rc<PipelineCache>> {
        if false == self.context.pipeline_caching_supported() {
            return None;
        }
        unsafe { 
            Some(Rc::new(
                self.context.device().create_pipeline_cache(&PipelineCacheDescriptor {
                    label: Some(uid),
                    data,
                    fallback: true,
                })
            ))
        }
    }

    #[must_use]
    fn try_load_cache_data_from_disk(&self, uid: &str, desired_hash: ShaderHash) -> Option<Vec<u8>> {
        let caches_directory = self.caches_path.as_ref()?;
        let cache_file_path = Self::path_to_cache(uid, caches_directory);
        let memento = self.io.load(&cache_file_path)?;
        
        if memento.hash == desired_hash && memento.adapter_info.same_as(self.context.adapter_info()) {
            Some(memento.data)
        } else {
            None
        }
    }
    
    #[must_use]
    fn path_to_cache(uid: &str, caches_directory: &Path) -> PathBuf {
        let file_name = format!("{version_code}_{uid}_cache", version_code=Self::DISK_CACHE_VERSION_CODE, uid=uid);
        caches_directory.join(file_name)
    }
}

#[derive(Default)]
struct FileSystemIo;

impl Io for FileSystemIo {
    fn save(&self, path: &Path, memento: &PipelineCacheMemento) {
        memento.save_to_file(path).unwrap_or_else(|| {
            info!("failed to write pipeline cache file {path:?}");
        });
    }
    fn load(&self, path: &Path) -> Option<PipelineCacheMemento> {
        PipelineCacheMemento::load_from_file(path)
    }
}

trait Io {
    fn save(&self, path: &Path, memento: &PipelineCacheMemento);
    #[must_use]
    fn load(&self, path: &Path) -> Option<PipelineCacheMemento>;
}

bitflags! {
    struct CacheAction: u32 {
        const StoreInMemory = 0b00000001;
        const SaveOnDisk = 0b00000010;
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ShortAdapterInfo {
    vendor: u32,
    device: u32,
    device_type: u32,
    backend: u32,
    driver: Vec<u8>,
    driver_info: Vec<u8>,
}

impl ShortAdapterInfo {
    #[must_use]
    fn new(source: &AdapterInfo) -> ShortAdapterInfo {
        ShortAdapterInfo {
            vendor: source.vendor,
            device: source.device,
            device_type: source.device_type as u32,
            backend: source.backend as u32,
            driver: source.driver.as_bytes().to_vec(),
            driver_info: source.driver_info.as_bytes().to_vec(),
        }
    }
    #[must_use]
    fn same_as(&self, target: &AdapterInfo) -> bool {
        self.vendor == target.vendor &&
        self.device == target.device &&
        self.device_type == target.device_type as u32 &&
        self.backend == target.backend as u32 &&
        self.driver == target.driver.as_bytes() &&
        self.driver_info == target.driver_info.as_bytes()
    }
}

#[derive(Debug, Clone)]
struct PipelineCacheMemento {
    hash: u64,
    data: Vec<u8>,
    adapter_info: ShortAdapterInfo,
}

impl PipelineCacheMemento {
    #[must_use]
    fn save_to_file(&self, path: &Path) -> Option<()> {
        let file = File::create(path).ok()?;
        let mut writer = BufWriter::new(file);
        
        let header = CacheFileHeader::new(self.hash, self.data.len() as u64, &self.adapter_info);
        writer.write_all(header.as_bytes()).ok()?;

        writer.write_all(&self.adapter_info.driver).ok()?;
        writer.write_all(&self.adapter_info.driver_info).ok()?;
        writer.write_all(&self.data).ok()?;
        
        writer.flush().ok()
    }
    
    #[must_use]
    fn load_from_file(path: &Path) -> Option<Self> {
        let file = File::open(path).ok()?;
        let mut reader = BufReader::new(file);

        // Read header
        let mut header_bytes = vec![0u8; size_of::<CacheFileHeader>()];
        reader.read_exact(&mut header_bytes).ok()?;

        let header = CacheFileHeader::from_bytes(&header_bytes)?;
        
        if false == header.is_valid() {
            return None;
        }
        
        let mut adapter_driver = vec![0u8; header.adapter_driver_size as usize];
        reader.read_exact(&mut adapter_driver).ok()?;

        let mut adapter_driver_info = vec![0u8; header.adapter_driver_info_size as usize];
        reader.read_exact(&mut adapter_driver_info).ok()?;
        
        let mut data = vec![0u8; header.data_size as usize];
        reader.read_exact(&mut data).ok()?;

        Some(Self {
            hash: header.data_hash,
            data,
            adapter_info: ShortAdapterInfo {
                vendor:      header.adapter_vendor,
                device:      header.adapter_device,     
                device_type: header.adapter_device_type,
                backend:     header.adapter_backend,    
                driver:      adapter_driver,     
                driver_info: adapter_driver_info,
            }
        })
    }
}

#[repr(C, packed)]
struct CacheFileHeader {
    magic: [u8; 4], // Magic bytes "PCCH" (Pipeline Cache Header)
    data_hash: u64,
    data_size: u64,
    adapter_vendor: u32,
    adapter_device: u32,
    adapter_device_type: u32,
    adapter_backend: u32,
    adapter_driver_size: u32,
    adapter_driver_info_size: u32,
}

impl CacheFileHeader {
    const MAGIC: [u8; 4] = *b"PCCH";

    #[must_use]
    fn new(hash: u64, data_size: u64, adapter_info: &ShortAdapterInfo) -> Self {
        Self {
            magic: Self::MAGIC,
            data_hash: hash,
            data_size,
            adapter_vendor: adapter_info.vendor,
            adapter_device: adapter_info.device,
            adapter_device_type: adapter_info.device_type,
            adapter_backend: adapter_info.backend,
            adapter_driver_size: adapter_info.driver.len() as u32,
            adapter_driver_info_size: adapter_info.driver_info.len() as u32,
        }
    }

    #[must_use]
    fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
    }

    #[must_use]
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const Self as *const u8,
                size_of::<Self>()
            )
        }
    }

    #[must_use]
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < size_of::<Self>() {
            return None;
        }

        let header = unsafe {
            std::ptr::read_unaligned(bytes.as_ptr() as *const Self)
        };

        if header.is_valid() {
            Some(header)
        } else {
            None
        }
    }
}

#[derive(Display)]
pub(crate) enum ComputeRoutineEntryPoint {
    SurfaceAttributes,

    RayTracingMonteCarlo,
    RayTracingDeterministic,

    #[cfg(test)] Default,
    #[cfg(test)] TestDefault,
}

impl ComputeRoutineEntryPoint {
    #[must_use]
    pub(crate) fn name(&self) -> Option<&'static str> {
        match self {
            ComputeRoutineEntryPoint::SurfaceAttributes => Some("compute_surface_attributes_buffer"),
            ComputeRoutineEntryPoint::RayTracingMonteCarlo => Some("compute_color_buffer_monte_carlo"),
            ComputeRoutineEntryPoint::RayTracingDeterministic => Some("compute_color_buffer_deterministic"),
            
            #[cfg(test)] ComputeRoutineEntryPoint::TestDefault => Some("main"),
            #[cfg(test)] ComputeRoutineEntryPoint::Default => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use crate::gpu::resources::Resources;
    use crate::utils::tests::common_values::tests::COMMON_PRESENTATION_FORMAT;
    use tempfile::{tempdir, TempDir};

    #[derive(Default)]
    struct SpyIo {
        backend: FileSystemIo,
        saved_paths: std::cell::RefCell<Vec<(PathBuf, PipelineCacheMemento)>>,
        loaded_paths: std::cell::RefCell<Vec<(PathBuf, PipelineCacheMemento)>>,
    }

    impl Io for SpyIo {
        fn save(&self, path: &Path, memento: &PipelineCacheMemento) {
            self.saved_paths.borrow_mut().push((path.to_path_buf(), memento.clone()));
            self.backend.save(path, memento);
        }
        fn load(&self, path: &Path) -> Option<PipelineCacheMemento> {
            let memento = self.backend.load(path);
            let memento_copy_or_none = memento.clone();
            if let Some(copy) = memento_copy_or_none {
                self.loaded_paths.borrow_mut().push((path.to_path_buf(), copy.clone()));    
            }
            memento
        }
    }
    
    const TRIVIAL_COMPUTE_SHADER: &str = "@compute @workgroup_size(1) fn main() {}";

    const TRIVIAL_RASTERIZATION_SHADER: &str = 
        "@vertex fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {return vec4f(0.0);} \
         @fragment fn fs_main() -> @location(0) vec4f {return vec4f(0.0);}";
    
    #[must_use]
    fn make_system_under_test(context: Rc<Context>, directory: &TempDir) -> (Rc<SpyIo>, PipelinesFactory) {
        let io_spy = Rc::new(SpyIo::default());
        let cache_directory = Some(PathBuf::from(directory.path()));
        
        (
            io_spy.clone(),
            PipelinesFactory::new_with_custom_io(context, COMMON_PRESENTATION_FORMAT, cache_directory, io_spy.clone()),
        )
    }
    
    #[must_use]
    fn make_fixture(shader_code: &str) -> (Rc<Context>, TempDir, PipelineCode) {
        let cache_directory = tempdir().unwrap();
        let context = create_headless_wgpu_context();
        let resources = Resources::new(context.clone());
        let shader_module = resources.create_shader_module(TEST_SHADER_LABEL, shader_code);
        let pipeline_code = PipelineCode::new(shader_module, seahash::hash(shader_code.as_bytes()), TEST_SHADER_UID.to_string());
        (context, cache_directory, pipeline_code)
    }

    #[test]
    fn warn_if_pipeline_caching_is_not_supported() {
        let (context, _, _) = make_fixture(TRIVIAL_COMPUTE_SHADER);
        
        if false == context.pipeline_caching_supported() {
            println!("WARNING: current test adapter does not support pipeline caching -> tests of this feature will run, but are unfunctional");
        }
    }
    
    const TEST_SHADER_UID: &str = "test_shader";
    const TEST_SHADER_LABEL: &str = "test_compute";
    
    #[must_use]
    fn expected_test_shader_path_to_cache(cache_directory: &TempDir) -> PathBuf {
        PipelinesFactory::path_to_cache(TEST_SHADER_UID, &PathBuf::from(cache_directory.path()))
    }

    fn assert_saved_once_loaded_none(expected_cache_file: &PathBuf, io_spy: &Rc<SpyIo>, expected_hash: ShaderHash) {
        assert_eq!(io_spy.saved_paths.borrow().len(), 1, "expected one disk write");
        assert_eq!(io_spy.saved_paths.borrow()[0].0, *expected_cache_file);
        assert_eq!(io_spy.saved_paths.borrow()[0].1.hash, expected_hash);
        
        assert!(io_spy.loaded_paths.borrow().is_empty(), "expected zero loads from disk");
    }

    fn assert_saved_once_loaded_once(expected_cache_file: &PathBuf, io_spy: &Rc<SpyIo>, expected_hash: ShaderHash) {
        assert_eq!(io_spy.saved_paths.borrow().len(), 1, "expected one disk write");
        assert_eq!(io_spy.saved_paths.borrow()[0].0, *expected_cache_file);
        assert_eq!(io_spy.saved_paths.borrow()[0].1.hash, expected_hash);

        assert_eq!(io_spy.loaded_paths.borrow().len(), 1, "expected one load from disk");
        assert_eq!(io_spy.loaded_paths.borrow()[0].0, *expected_cache_file);
        assert_eq!(io_spy.loaded_paths.borrow()[0].1.hash, expected_hash);
    }
    
    #[test]
    fn test_compute_pipeline_in_memory_caching() {
        let (context, cache_directory, pipeline_code) = make_fixture(TRIVIAL_COMPUTE_SHADER);
        if false == context.pipeline_caching_supported() {return;}
        
        let expected_cache_file = expected_test_shader_path_to_cache(&cache_directory);
        let (io_spy, mut system_under_test) = make_system_under_test(context, &cache_directory);

        let hash = seahash::hash(TRIVIAL_COMPUTE_SHADER.as_bytes());
        
        let _ = system_under_test.create_compute_pipeline(ComputeRoutineEntryPoint::Default, &pipeline_code);
        assert_saved_once_loaded_none(&expected_cache_file, &io_spy, hash);
        
        let _ = system_under_test.create_compute_pipeline(ComputeRoutineEntryPoint::Default, &pipeline_code);
        assert_saved_once_loaded_none(&expected_cache_file, &io_spy, hash);
    }

    #[test]
    fn test_rasterization_pipeline_in_memory_caching() {
        let (context, cache_directory, pipeline_code) = make_fixture(TRIVIAL_RASTERIZATION_SHADER);
        if false == context.pipeline_caching_supported() {return;}
        
        let expected_cache_file = expected_test_shader_path_to_cache(&cache_directory);
        let (io_spy, mut system_under_test) = make_system_under_test(context, &cache_directory);

        let hash = seahash::hash(TRIVIAL_RASTERIZATION_SHADER.as_bytes());
        
        let _ = system_under_test.create_rasterization_pipeline(&pipeline_code);
        assert_saved_once_loaded_none(&expected_cache_file, &io_spy, hash);

        let _ = system_under_test.create_rasterization_pipeline(&pipeline_code);
        assert_saved_once_loaded_none(&expected_cache_file, &io_spy, hash);
    }

    #[test]
    fn test_compute_pipeline_on_disk_caching() {
        let (context, cache_directory, pipeline_code) = make_fixture(TRIVIAL_COMPUTE_SHADER);
        if false == context.pipeline_caching_supported() {return;}
        
        let expected_cache_file = expected_test_shader_path_to_cache(&cache_directory);

        let io_spy = Rc::new(SpyIo::default());
        let cache_directory = Some(PathBuf::from(cache_directory.path()));

        let hash = seahash::hash(TRIVIAL_COMPUTE_SHADER.as_bytes());
        
        {
            let mut system_under_test = PipelinesFactory::new_with_custom_io(context.clone(), COMMON_PRESENTATION_FORMAT, cache_directory.clone(), io_spy.clone());
            let _ = system_under_test.create_compute_pipeline(ComputeRoutineEntryPoint::Default, &pipeline_code);
            assert_saved_once_loaded_none(&expected_cache_file, &io_spy, hash);
        }

        {
            let mut system_under_test = PipelinesFactory::new_with_custom_io(context, COMMON_PRESENTATION_FORMAT, cache_directory, io_spy.clone());
            let _ = system_under_test.create_compute_pipeline(ComputeRoutineEntryPoint::Default, &pipeline_code);
            assert_saved_once_loaded_once(&expected_cache_file, &io_spy, hash);
        }
    }
}