use crate::gpu::context::Context;
use crate::gpu::pipeline_code::{PipelineCode, ShaderHash};
use bitflags::bitflags;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;
use derive_more::Display;
use wgpu::{PipelineCache, PipelineCacheDescriptor};

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
        Self::new_with_custom_io(context, presentation_format, caches_path, Rc::new(FileSystemIo::default()))
    }

    #[must_use]
    fn new_with_custom_io(context: Rc<Context>, presentation_format: wgpu::TextureFormat, caches_path: Option<PathBuf>, io: Rc<dyn Io>,) -> Self {
        if let Some(path) = caches_path.clone() {
            if let Err(e) = fs::create_dir_all(&path) {
                info!("failed to create directories in path {:?}: {}", path, e);
            }
        }
        Self { context, presentation_format, caches_path, caches: HashMap::new(), io, }
    }

    #[must_use]
    pub(super) fn create_rasterization_pipeline(&mut self, code: &PipelineCode) -> wgpu::RenderPipeline {
        let (cache, status) = self.find_or_create_cache(code.human_readable_uid(), &code.content_hash());
        
        let start = Instant::now();
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
        let delta = start.elapsed();
        info!("rasterization pipeline creation time: {}", delta.as_millis());
        
        self.handle_actions(code, cache, status);
        pipeline
    }

    #[must_use]
    pub(crate) fn create_compute_pipeline(&mut self, routine: ComputeRoutineEntryPoint, code: &PipelineCode) -> wgpu::ComputePipeline {
        let (cache, actions) = self.find_or_create_cache(code.human_readable_uid(), &code.content_hash());
        
        let start = Instant::now();
        let pipeline = self.context.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: routine.name(),
            compilation_options: Default::default(),
            layout: None,
            module: code.module().as_ref(),
            entry_point: routine.name(),
            cache: cache.as_deref()
        });
        let delta = start.elapsed();
        info!("compute pipeline '{}' creation time: {}", routine, delta.as_millis());
        
        self.handle_actions(code, cache, actions);
        pipeline
    }

    fn handle_actions(&mut self, code: &PipelineCode, cache: Option<Rc<PipelineCache>>, action: CacheAction) {
        if let Some(cache) = cache {
            if action.contains(CacheAction::SaveOnDisk) {
                self.store_cache_on_disk(code.human_readable_uid(), cache.get_data(), &code.content_hash());
            }
            if action.contains(CacheAction::StoreInMemory) {
                let record = CacheAndHash { this: cache, hash: code.content_hash().clone() };
                self.caches.insert(code.human_readable_uid().clone(), record);
            }   
        }
    }

    fn store_cache_on_disk(&self, uid: &str, data_or_none: Option<Vec<u8>>, hash: &ShaderHash) {
        let Some(data) = data_or_none else { return };
        let Some(caches_directory) = self.caches_path.as_ref() else { return };

        let cache_file_path = Self::path_to_cache(uid, caches_directory);

        let memento = PipelineCacheMemento { hash: hash.as_bytes().to_vec(), data, };
        self.io.save(cache_file_path.as_path(), &memento);
    }

    #[must_use]
    fn find_or_create_cache(&self, uid: &str, hash: &ShaderHash) -> (Option<Rc<PipelineCache>>, CacheAction) {
        if let Some(cache) = self.caches.get(uid) {
            if cache.hash == *hash {
                return (Some(cache.this.clone()), CacheAction::empty());    
            }
            return (self.create_pipeline_cache(uid, None), CacheAction::StoreInMemory | CacheAction::SaveOnDisk);
        }
        
        if let Some(cache_data) = self.try_load_cache_data_from_disk(uid, hash) {
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
                    fallback: false,
                })
            ))
        }
    }

    #[must_use]
    fn try_load_cache_data_from_disk(&self, uid: &str, hash: &ShaderHash) -> Option<Vec<u8>> {
        let caches_directory = self.caches_path.as_ref()?;
        let cache_file_path = Self::path_to_cache(uid, &caches_directory);
        let memento = self.io.load(&cache_file_path)?;
        let loaded_content_hash = ShaderHash::from_slice(memento.hash.as_slice()).ok()?;
        
        if loaded_content_hash == *hash {
            Some(memento.data)
        } else {
            None 
        }
    }
    
    #[must_use]
    fn path_to_cache(uid: &str, caches_directory: &PathBuf) -> PathBuf {
        let file_name = format!("{version_code}_{uid}_cache", version_code=Self::DISK_CACHE_VERSION_CODE, uid=uid);
        let file_path = caches_directory.join(file_name);
        file_path
    }
}

#[derive(Default)]
struct FileSystemIo;

impl Io for FileSystemIo {
    fn save(&self, path: &Path, memento: &PipelineCacheMemento) {
        memento.save_to_file(path).unwrap_or_else(|| {
            info!("failed to write pipeline cache file {:?}", path);
        });
    }
    #[must_use]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PipelineCacheMemento {
    hash: Vec<u8>,
    data: Vec<u8>,
}

impl PipelineCacheMemento {
    #[must_use]
    fn save_to_file(&self, path: &Path) -> Option<()> {
        let file = File::create(path).ok()?;
        let writer = BufWriter::new(file);
        ciborium::ser::into_writer(self, writer).ok()
    }
    #[must_use]
    fn load_from_file(path: &Path) -> Option<Self> {
        let file = File::open(path).ok()?;
        ciborium::de::from_reader(file).ok()
    }
}

#[derive(Display)]
pub(crate) enum ComputeRoutineEntryPoint {
    ObjectId,

    RayTracingMonteCarlo,
    RayTracingDeterministic,

    #[cfg(test)] Default,
    #[cfg(test)] TestDefault,
}

impl ComputeRoutineEntryPoint {
    #[must_use]
    pub(crate) fn name(&self) -> Option<&'static str> {
        match self {
            ComputeRoutineEntryPoint::ObjectId => Some("compute_object_id_buffer"),
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
        #[must_use]
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
        let pipeline_code = PipelineCode::new(shader_module, blake3::hash(shader_code.as_bytes()), TEST_SHADER_UID.to_string());
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

    fn assert_saved_once_loaded_none(expected_cache_file: &PathBuf, io_spy: &Rc<SpyIo>, expected_hash: &ShaderHash) {
        assert_eq!(io_spy.saved_paths.borrow().len(), 1, "expected one disk write");
        assert_eq!(io_spy.saved_paths.borrow()[0].0, *expected_cache_file);
        assert_eq!(io_spy.saved_paths.borrow()[0].1.hash, expected_hash.as_bytes());
        
        assert!(io_spy.loaded_paths.borrow().is_empty(), "expected zero loads from disk");
    }

    fn assert_saved_once_loaded_once(expected_cache_file: &PathBuf, io_spy: &Rc<SpyIo>, expected_hash: &ShaderHash) {
        assert_eq!(io_spy.saved_paths.borrow().len(), 1, "expected one disk write");
        assert_eq!(io_spy.saved_paths.borrow()[0].0, *expected_cache_file);
        assert_eq!(io_spy.saved_paths.borrow()[0].1.hash, expected_hash.as_bytes());

        assert_eq!(io_spy.loaded_paths.borrow().len(), 1, "expected one load from disk");
        assert_eq!(io_spy.loaded_paths.borrow()[0].0, *expected_cache_file);
        assert_eq!(io_spy.loaded_paths.borrow()[0].1.hash, expected_hash.as_bytes());
    }
    
    #[test]
    fn test_compute_pipeline_in_memory_caching() {
        let (context, cache_directory, pipeline_code) = make_fixture(TRIVIAL_COMPUTE_SHADER);
        if false == context.pipeline_caching_supported() {return;}
        
        let expected_cache_file = expected_test_shader_path_to_cache(&cache_directory);
        let (io_spy, mut system_under_test) = make_system_under_test(context, &cache_directory);

        let hash = blake3::hash(TRIVIAL_COMPUTE_SHADER.as_bytes());
        
        let _ = system_under_test.create_compute_pipeline(ComputeRoutineEntryPoint::Default, &pipeline_code);
        assert_saved_once_loaded_none(&expected_cache_file, &io_spy, &hash);
        
        let _ = system_under_test.create_compute_pipeline(ComputeRoutineEntryPoint::Default, &pipeline_code);
        assert_saved_once_loaded_none(&expected_cache_file, &io_spy, &hash);
    }

    #[test]
    fn test_rasterization_pipeline_in_memory_caching() {
        let (context, cache_directory, pipeline_code) = make_fixture(TRIVIAL_RASTERIZATION_SHADER);
        if false == context.pipeline_caching_supported() {return;}
        
        let expected_cache_file = expected_test_shader_path_to_cache(&cache_directory);
        let (io_spy, mut system_under_test) = make_system_under_test(context, &cache_directory);

        let hash = blake3::hash(TRIVIAL_RASTERIZATION_SHADER.as_bytes());
        
        let _ = system_under_test.create_rasterization_pipeline(&pipeline_code);
        assert_saved_once_loaded_none(&expected_cache_file, &io_spy, &hash);

        let _ = system_under_test.create_rasterization_pipeline(&pipeline_code);
        assert_saved_once_loaded_none(&expected_cache_file, &io_spy, &hash);
    }

    #[test]
    fn test_compute_pipeline_on_disk_caching() {
        let (context, cache_directory, pipeline_code) = make_fixture(TRIVIAL_COMPUTE_SHADER);
        if false == context.pipeline_caching_supported() {return;}
        
        let expected_cache_file = expected_test_shader_path_to_cache(&cache_directory);

        let io_spy = Rc::new(SpyIo::default());
        let cache_directory = Some(PathBuf::from(cache_directory.path()));

        let hash = blake3::hash(TRIVIAL_COMPUTE_SHADER.as_bytes());
        
        {
            let mut system_under_test = PipelinesFactory::new_with_custom_io(context.clone(), COMMON_PRESENTATION_FORMAT, cache_directory.clone(), io_spy.clone());
            let _ = system_under_test.create_compute_pipeline(ComputeRoutineEntryPoint::Default, &pipeline_code);
            assert_saved_once_loaded_none(&expected_cache_file, &io_spy, &hash);
        }

        {
            let mut system_under_test = PipelinesFactory::new_with_custom_io(context, COMMON_PRESENTATION_FORMAT, cache_directory, io_spy.clone());
            let _ = system_under_test.create_compute_pipeline(ComputeRoutineEntryPoint::Default, &pipeline_code);
            assert_saved_once_loaded_once(&expected_cache_file, &io_spy, &hash);
        }
    }
}