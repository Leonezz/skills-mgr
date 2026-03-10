use std::path::{Path, PathBuf};

/// Root directory structure for skills-mgr.
#[derive(Debug, Clone)]
pub struct AppDirs {
    base: PathBuf,
}

impl AppDirs {
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }

    pub fn base(&self) -> &Path {
        &self.base
    }
}
