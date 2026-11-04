//! Synthetic `.slx` archives: write through the real `SlxArchive` writer
//! (exercises `generator` e2e) or via a raw-zip escape hatch for malformed
//! inputs.

#![allow(dead_code)]

use anyhow::Result;
use rustylink::model::{SlxArchive, SlxArchiveEntry, SlxContent, System};
use std::io::Write;
use std::path::Path;

/// Write `systems` (entry-path → System) plus optional raw entries through the
/// real archive writer, producing a standards-conformant `.slx` file.
pub fn write_slx(
    path: &Path,
    systems: &[(&str, System)],
    raw: &[(&str, &[u8])],
    compressed: bool,
) -> Result<()> {
    let mut entries: Vec<SlxArchiveEntry> = systems
        .iter()
        .map(|(p, s)| SlxArchiveEntry {
            path: (*p).to_string(),
            content: SlxContent::SystemXml(Box::new(s.clone())),
            compressed,
        })
        .collect();
    entries.extend(raw.iter().map(|(p, d)| SlxArchiveEntry {
        path: (*p).to_string(),
        content: SlxContent::Raw(d.to_vec()),
        compressed,
    }));
    SlxArchive {
        entries,
        relationships: Default::default(),
    }
    .write_to_file(path)
}

/// Write a minimal model `.slx`: one root system file.
pub fn write_model_slx(path: &Path, root: &System) -> Result<()> {
    write_slx(
        path,
        &[("simulink/systems/system_root.xml", root.clone())],
        &[],
        true,
    )
}

/// Raw zip writer for malformed inputs (ERR/FZ cases) — no schema checking.
pub fn raw_zip(path: &Path, files: &[(&str, &[u8])]) -> Result<()> {
    let f = std::fs::File::create(path)?;
    let mut zw = zip::ZipWriter::new(f);
    let opts = zip::write::SimpleFileOptions::default();
    for (name, data) in files {
        zw.start_file(*name, opts)?;
        zw.write_all(data)?;
    }
    zw.finish()?;
    Ok(())
}
