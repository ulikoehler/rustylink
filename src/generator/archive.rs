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
use std::collections::BTreeMap;
use std::io::{Read, Seek, Write};

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

        // Parse blockdiagram.xml.rels if present.
        let relationships = Self::parse_rels_from_entries(&entries);

        let mut archive = SlxArchive {
            entries,
            relationships,
        };

        // Resolve BindingPersistence refs for dashboard/HMI blocks.
        archive.resolve_dashboard_bindings();

        Ok(archive)
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

    /// Resolve a `Ref="bdmxdata:…"` style reference to a file path within the
    /// archive, using the parsed relationships from `blockdiagram.xml.rels`.
    ///
    /// For example, `"bdmxdata:BindingPersistence_151"` resolves to
    /// `"simulink/bdmxdata/BindingPersistence_151.mxarray"` when the rels
    /// file maps the id `BindingPersistence_151` to
    /// `"bdmxdata/BindingPersistence_151.mxarray"`.
    ///
    /// Returns `None` if the reference format is invalid or the relationship
    /// id is not found.
    pub fn resolve_ref(&self, ref_value: &str) -> Option<String> {
        let id = ref_value.strip_prefix("bdmxdata:")?;
        let rel = self.relationships.get(id)?;
        // The target in blockdiagram.xml.rels is relative to `simulink/`.
        Some(format!("simulink/{}", rel.target))
    }

    /// Get the raw bytes of an entry by its archive path.
    pub fn get_raw(&self, path: &str) -> Option<&[u8]> {
        self.entries.iter().find_map(|e| {
            if e.path == path {
                if let SlxContent::Raw(ref data) = e.content {
                    Some(data.as_slice())
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// Look up a BindingPersistence ref and return the raw `.mxarray` bytes.
    ///
    /// The `ref_value` should be of the form `"bdmxdata:BindingPersistence_NNN"`.
    pub fn resolve_binding_persistence(&self, ref_value: &str) -> Option<&[u8]> {
        let archive_path = self.resolve_ref(ref_value)?;
        self.get_raw(&archive_path)
    }

    /// Walk every block in the archive and, for those that have a
    /// `BindingPersistence` property, resolve the reference to its
    /// `.mxarray` file, parse the binary data, and populate the
    /// `dashboard_binding` field on the block.
    fn resolve_dashboard_bindings(&mut self) {
        // First collect the binding data we need: for each system entry index,
        // collect (block_index_path, ref_value) pairs.  Because we cannot
        // borrow `self` mutably while also reading raw entries, we first
        // gather the parsed bindings into a temporary map keyed on the
        // ref-value string.
        let mut binding_cache: BTreeMap<String, crate::model::DashboardBinding> = BTreeMap::new();

        // Pre-populate the cache with all resolvable bindings.
        for entry in &self.entries {
            if let SlxContent::SystemXml(system) = &entry.content {
                Self::collect_binding_refs(system, &mut |ref_value| {
                    if !binding_cache.contains_key(ref_value) {
                        if let Some(archive_path) = {
                            let id = ref_value.strip_prefix("bdmxdata:");
                            id.and_then(|id| self.relationships.get(id))
                                .map(|rel| format!("simulink/{}", rel.target))
                        } {
                            if let Some(raw) = self.entries.iter().find_map(|e| {
                                if e.path == archive_path {
                                    if let SlxContent::Raw(ref data) = e.content {
                                        Some(data.as_slice())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            }) {
                                if let Some(binding) =
                                    crate::model::parse_mxarray_binding(raw)
                                {
                                    binding_cache.insert(ref_value.to_string(), binding);
                                }
                            }
                        }
                    }
                });
            }
        }

        // Now apply the cached bindings to mutable blocks.
        for entry in &mut self.entries {
            if let SlxContent::SystemXml(ref mut system) = entry.content {
                Self::apply_bindings(system, &binding_cache);
            }
        }
    }

    /// Recursively collect all `BindingPersistence` ref values from a system.
    fn collect_binding_refs<F: FnMut(&str)>(system: &System, cb: &mut F) {
        for block in &system.blocks {
            if block.ref_properties.contains("BindingPersistence") {
                if let Some(ref_val) = block.properties.get("BindingPersistence") {
                    cb(ref_val);
                }
            }
            if let Some(sub) = &block.subsystem {
                Self::collect_binding_refs(sub, cb);
            }
        }
    }

    /// Recursively apply parsed bindings to blocks that have matching
    /// `BindingPersistence` ref values.
    fn apply_bindings(
        system: &mut System,
        cache: &BTreeMap<String, crate::model::DashboardBinding>,
    ) {
        for block in &mut system.blocks {
            if block.ref_properties.contains("BindingPersistence") {
                if let Some(ref_val) = block.properties.get("BindingPersistence") {
                    if let Some(binding) = cache.get(ref_val) {
                        block.dashboard_binding = Some(binding.clone());
                    }
                }
            }
            if let Some(sub) = &mut block.subsystem {
                Self::apply_bindings(sub, cache);
            }
        }
    }

    // ── High-level assembly helpers ─────────────────────────────────────

    /// Assemble the full system tree rooted at `system_root.xml`.
    ///
    /// This replicates the logic of `SimulinkParser::link_system_refs` +
    /// `try_preload_systems_for` but operates entirely on the already-parsed
    /// archive entries, so no ZIP or filesystem access is needed afterwards.
    pub fn assembled_root_system(&self) -> Result<System> {
        let root = self
            .root_system()
            .ok_or_else(|| anyhow!("No root system in archive"))?
            .clone();

        // Build lookup: path → &System  (for all system XML entries)
        let systems_by_path: BTreeMap<&str, &System> = self
            .entries
            .iter()
            .filter_map(|e| {
                if let SlxContent::SystemXml(ref sys) = e.content {
                    Some((e.path.as_str(), sys))
                } else {
                    None
                }
            })
            .collect();

        let mut assembled = root;
        let base = camino::Utf8Path::new("simulink/systems");
        Self::link_system_refs_recursive(&mut assembled, base, &systems_by_path);

        Ok(assembled)
    }

    /// Recursively resolve `system_ref` fields using the pre-parsed entries.
    fn link_system_refs_recursive(
        system: &mut System,
        current_base: &camino::Utf8Path,
        lookup: &BTreeMap<&str, &System>,
    ) {
        for blk in &mut system.blocks {
            if let Some(ref ref_name) = blk.system_ref {
                let ref_path =
                    crate::parser::helpers::resolve_system_reference(ref_name, current_base);
                if let Some(sub) = lookup.get(ref_path.as_str()) {
                    let mut sub_cloned = (*sub).clone();
                    let sub_base =
                        ref_path.parent().unwrap_or(current_base);
                    Self::link_system_refs_recursive(&mut sub_cloned, sub_base, lookup);
                    blk.subsystem = Some(Box::new(sub_cloned));
                }
            }
            if let Some(ref mut sub) = blk.subsystem {
                Self::link_system_refs_recursive(sub, current_base, lookup);
            }
        }
    }

    /// Parse all stateflow charts found in the archive.
    ///
    /// Returns `(charts_by_id, chart_map)` where `chart_map` maps chart names
    /// and SID strings to chart IDs, suitable for passing to `SubsystemApp`.
    pub fn parse_charts(
        &self,
    ) -> (
        BTreeMap<u32, crate::model::Chart>,
        BTreeMap<String, u32>,
    ) {
        use rayon::prelude::*;

        // Collect all stateflow chart XML entries
        let chart_texts: Vec<(&str, String)> = self
            .entries
            .iter()
            .filter(|e| {
                let norm = e.path.trim_start_matches("./").trim_start_matches('/');
                if let Some(rest) = norm.strip_prefix("simulink/stateflow/") {
                    rest.starts_with("chart_") && rest.ends_with(".xml") && !rest.contains('/')
                } else {
                    false
                }
            })
            .filter_map(|e| {
                if let SlxContent::Raw(ref data) = e.content {
                    std::str::from_utf8(data)
                        .ok()
                        .map(|s| (e.path.as_str(), s.to_string()))
                } else {
                    None
                }
            })
            .collect();

        let parsed: Vec<crate::model::Chart> = chart_texts
            .par_iter()
            .filter_map(|(path, text)| {
                crate::parser::chart::parse_chart_from_text(text, Some(path)).ok()
            })
            .collect();

        let mut charts_by_id: BTreeMap<u32, crate::model::Chart> = BTreeMap::new();
        let mut chart_map: BTreeMap<String, u32> = BTreeMap::new();

        for chart in parsed {
            if let Some(id) = chart.id {
                let ch = charts_by_id.entry(id).or_insert(chart);
                if let Some(nm) = ch.name.clone() {
                    chart_map.entry(nm).or_insert(id);
                }
            }
        }

        (charts_by_id, chart_map)
    }

    /// Return library names from `simulink/graphicalInterface.json`.
    ///
    /// Reads the raw entry, deserializes the JSON, and extracts library names
    /// from `ExternalFileReferences` of type `LIBRARY_BLOCK`.
    pub fn graphical_interface_library_names(&self) -> Result<Vec<String>> {
        const GI_PATH: &str = "simulink/graphicalInterface.json";
        let raw = self
            .get_raw(GI_PATH)
            .ok_or_else(|| anyhow!("{} not found in archive", GI_PATH))?;
        let text = std::str::from_utf8(raw)
            .with_context(|| format!("Non-UTF8 content in {}", GI_PATH))?;
        let v: serde_json::Value = serde_json::from_str(text)
            .with_context(|| format!("Failed to parse JSON {}", GI_PATH))?;
        let gi_value = v
            .get("GraphicalInterface")
            .ok_or_else(|| anyhow!("Missing 'GraphicalInterface' in {}", GI_PATH))?;
        let gi: crate::parser::graphical_interface::GraphicalInterface =
            serde_json::from_value(gi_value.clone())
                .with_context(|| format!("Failed to deserialize GraphicalInterface in {}", GI_PATH))?;
        Ok(gi.library_names())
    }

    // ── Internal helpers ────────────────────────────────────────────────

    /// Scan entries for `simulink/_rels/blockdiagram.xml.rels` and parse it.
    fn parse_rels_from_entries(entries: &[SlxArchiveEntry]) -> BTreeMap<String, Relationship> {
        const RELS_PATH: &str = "simulink/_rels/blockdiagram.xml.rels";
        let mut map = BTreeMap::new();
        for entry in entries {
            if entry.path == RELS_PATH {
                if let SlxContent::Raw(ref data) = entry.content {
                    if let Ok(xml) = std::str::from_utf8(data) {
                        for rel in parse_rels_xml(xml) {
                            map.insert(rel.id.clone(), rel);
                        }
                    }
                }
            }
        }
        map
    }
}
