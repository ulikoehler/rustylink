//! SLX archive reading and writing.
//!
//! An SLX file is a ZIP archive. System XML files (`simulink/systems/system_*.xml`)
//! are parsed into [`System`] models; all other files are preserved as raw bytes.
//! When writing, system files are regenerated from the model and other files are
//! written verbatim, producing an exact round-trip.

use crate::block;
use crate::generator::system_xml;
use crate::model::*;
use anyhow::{Context, Result, anyhow};
use roxmltree::Document;
use std::io::{Read, Write, Seek};

/// Returns `true` if the given path is a system XML file that should be parsed.
fn is_system_xml(path: &str) -> bool {
    // Match paths like "simulink/systems/system_root.xml" or "simulink/systems/system_18.xml"
    let normalized = path.trim_start_matches("./").trim_start_matches('/');
    if let Some(rest) = normalized.strip_prefix("simulink/systems/") {
        rest.starts_with("system_") && rest.ends_with(".xml") && !rest.contains('/')
    } else {
        false
    }
}

impl SlxArchive {
    /// Read an SLX file from a reader (ZIP format).
    ///
    /// System XML files are parsed into [`System`] models; all other files are
    /// stored as raw bytes. The entry order and compression settings are preserved.
    pub fn from_reader<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut zip = zip::ZipArchive::new(reader).context("Failed to open SLX ZIP")?;
        let mut entries = Vec::with_capacity(zip.len());

        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let path = file.name().to_string();
            let compressed = file.compression() == zip::CompressionMethod::Deflated;

            let mut raw = Vec::new();
            file.read_to_end(&mut raw)?;

            if is_system_xml(&path) {
                // Parse into System model
                let text = String::from_utf8(raw)
                    .with_context(|| format!("Non-UTF8 content in {}", path))?;
                let doc = Document::parse(&text)
                    .with_context(|| format!("Failed to parse XML {}", path))?;
                let system_node = doc
                    .descendants()
                    .find(|n| n.is_element() && n.has_tag_name("System"))
                    .ok_or_else(|| anyhow!("No <System> root in {}", path))?;
                // Determine base directory for system reference resolution
                let base_dir = if let Some(idx) = path.rfind('/') {
                    camino::Utf8Path::new(&path[..idx])
                } else {
                    camino::Utf8Path::new("")
                };
                let system = block::parse_system_shallow(system_node, base_dir)?;
                entries.push(SlxArchiveEntry {
                    path,
                    content: SlxContent::SystemXml(system),
                    compressed,
                });
            } else {
                entries.push(SlxArchiveEntry {
                    path,
                    content: SlxContent::Raw(raw),
                    compressed,
                });
            }
        }

        Ok(SlxArchive { entries })
    }

    /// Read an SLX file from disk.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = std::fs::File::open(path.as_ref())
            .with_context(|| format!("Failed to open {}", path.as_ref().display()))?;
        let reader = std::io::BufReader::new(file);
        Self::from_reader(reader)
    }

    /// Write the archive to a writer in ZIP format.
    ///
    /// System XML entries are regenerated from their [`System`] model;
    /// all other entries are written from their raw bytes.
    pub fn write_to<W: Write + Seek>(&self, writer: W) -> Result<()> {
        let mut zip = zip::ZipWriter::new(writer);

        for entry in &self.entries {
            let options = if entry.compressed {
                zip::write::FileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated)
            } else {
                zip::write::FileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored)
            };

            zip.start_file(&entry.path, options)?;

            match &entry.content {
                SlxContent::Raw(data) => {
                    zip.write_all(data)?;
                }
                SlxContent::SystemXml(system) => {
                    let xml = system_xml::generate_system_xml(system);
                    zip.write_all(xml.as_bytes())?;
                }
            }
        }

        zip.finish()?;
        Ok(())
    }

    /// Write the archive to a file on disk.
    pub fn write_to_file(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let file = std::fs::File::create(path.as_ref())
            .with_context(|| format!("Failed to create {}", path.as_ref().display()))?;
        let writer = std::io::BufWriter::new(file);
        self.write_to(writer)
    }

    /// Get the System model for a given entry path.
    pub fn get_system(&self, path: &str) -> Option<&System> {
        self.entries.iter().find_map(|e| {
            if e.path == path {
                if let SlxContent::SystemXml(ref sys) = e.content {
                    Some(sys)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// Get a mutable reference to the System model for a given entry path.
    pub fn get_system_mut(&mut self, path: &str) -> Option<&mut System> {
        self.entries.iter_mut().find_map(|e| {
            if e.path == path {
                if let SlxContent::SystemXml(ref mut sys) = e.content {
                    Some(sys)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// Get the root system (from `simulink/systems/system_root.xml`).
    pub fn root_system(&self) -> Option<&System> {
        self.get_system("simulink/systems/system_root.xml")
    }

    /// List all entry paths in the archive.
    pub fn entry_paths(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.path.as_str()).collect()
    }
}
