//! Content source abstraction for reading files from the filesystem or ZIP archives.

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use std::io::Read;

/// Trait for abstracting file I/O (filesystem vs. ZIP source).
pub trait ContentSource {
    /// Read a file at the given logical path and return its content as a string.
    fn read_to_string(&mut self, path: &Utf8Path) -> Result<String>;
    /// List files in a directory path (logical path for the source), returning full paths.
    fn list_dir(&mut self, path: &Utf8Path) -> Result<Vec<Utf8PathBuf>>;
}

/// Reads files directly from the local filesystem.
pub struct FsSource;

impl FsSource {
    fn list_dir_impl(&mut self, path: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
        let mut files = Vec::new();
        for entry in
            std::fs::read_dir(path.as_std_path()).with_context(|| format!("Read dir {}", path))?
        {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let p = Utf8PathBuf::from_path_buf(entry.path())
                    .map_err(|_| anyhow::anyhow!("Non-UTF8 path in {}", path))?;
                files.push(p);
            }
        }
        Ok(files)
    }
}

impl ContentSource for FsSource {
    fn read_to_string(&mut self, path: &Utf8Path) -> Result<String> {
        Ok(std::fs::read_to_string(path.as_str())
            .with_context(|| format!("Failed to read {}", path))?)
    }
    fn list_dir(&mut self, path: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
        self.list_dir_impl(path)
    }
}

/// Reads files from a ZIP archive (used for `.slx` files).
pub struct ZipSource<R: Read + std::io::Seek> {
    zip: zip::ZipArchive<R>,
}

impl<R: Read + std::io::Seek> ZipSource<R> {
    pub fn new(reader: R) -> Result<Self> {
        let zip = zip::ZipArchive::new(reader).context("Failed to open zip archive")?;
        Ok(Self { zip })
    }
}

impl<R: Read + std::io::Seek> ContentSource for ZipSource<R> {
    fn read_to_string(&mut self, path: &Utf8Path) -> Result<String> {
        let p = path
            .as_str()
            .trim_start_matches("./")
            .trim_start_matches('/')
            .to_string();
        let mut f = self
            .zip
            .by_name(&p)
            .with_context(|| format!("File {} not found in zip", p))?;
        let mut s = String::new();
        f.read_to_string(&mut s)
            .with_context(|| format!("Failed to read {} from zip", p))?;
        Ok(s)
    }

    fn list_dir(&mut self, path: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
        let mut files = Vec::new();
        let mut prefix = path
            .as_str()
            .trim_start_matches("./")
            .trim_start_matches('/')
            .to_string();
        if !prefix.is_empty() && !prefix.ends_with('/') {
            prefix.push('/');
        }
        for i in 0..self.zip.len() {
            let name = self.zip.by_index(i)?.name().to_string();
            if name.starts_with(&prefix) && !name.ends_with('/') {
                files.push(Utf8PathBuf::from(name));
            }
        }
        Ok(files)
    }
}
