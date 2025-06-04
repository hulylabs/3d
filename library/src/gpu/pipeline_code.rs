use std::rc::Rc;

pub type ShaderHash = blake3::Hash;

pub(crate) struct PipelineCode {
    module: Rc<wgpu::ShaderModule>,
    source_hash: ShaderHash,
    human_readable_uid: String,
}

impl PipelineCode {
    #[must_use]
    pub(crate) fn new(module: Rc<wgpu::ShaderModule>, content_hash: ShaderHash, human_readable_uid: String) -> Self {
        assert!(Self::contains_only_letters_and_underscores(human_readable_uid.as_str()), "'{}' — uid can only contain letters and underscores", human_readable_uid);
        Self { module, source_hash: content_hash, human_readable_uid, }
    }
    
    #[must_use]
    pub(crate) fn content_hash(&self) -> ShaderHash {
        self.source_hash.clone()
    }

    #[must_use]
    pub(crate) fn module(&self) -> &Rc<wgpu::ShaderModule> {
        &self.module
    }

    #[must_use]
    pub(crate) fn human_readable_uid(&self) -> &String {
        &self.human_readable_uid
    }

    #[must_use]
    fn contains_only_letters_and_underscores(s: &str) -> bool {
        s.chars().all(|symbol| symbol.is_alphabetic() || '_' == symbol)
    }
}