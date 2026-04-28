use std::{env, fs, path};

/// Temporary directory that is deleted when [`Drop::drop`] is called
#[derive(Debug)]
pub struct TempDir(path::PathBuf);

impl TempDir {
    pub fn new() -> Self {
        Self(env::temp_dir())
    }

    pub fn path(&self) -> &path::Path {
        &self.0
    }
}

impl Default for TempDir {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.path());
    }
}
