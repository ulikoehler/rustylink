//! Simulink System XML parser.
//!
//! Provides [`SimulinkParser`] to load and parse Simulink XML system descriptions
//! into strongly-typed Rust structures. Sub-modules split the parser into focused
//! areas:
//!
//! - [`source`] – File I/O abstraction (filesystem vs. ZIP)
//! - [`helpers`] – Point / endpoint / reference parsing
//! - [`chart`] – Stateflow chart parsing
//! - [`graphical_interface`] – `graphicalInterface.json` types
//! - [`library`] – Library `.slx` file resolution

pub mod chart;
pub mod graphical_interface;
pub mod helpers;
pub mod library;
pub mod source;

// Re-export key types at the parser module level for backward compatibility.
pub use graphical_interface::*;
pub use helpers::{parse_endpoint, parse_points, resolve_system_reference};
pub use library::*;
pub use source::*;

use crate::model::*;
use anyhow::{Context, Result, anyhow};
use camino::{Utf8Path, Utf8PathBuf};
use rayon::prelude::*;
use roxmltree::Document;
use std::collections::BTreeMap;

/// Core Simulink parser. Generic over [`ContentSource`] so it can read from
/// the filesystem ([`FsSource`]) or from a ZIP archive ([`ZipSource`]).
pub struct SimulinkParser<S: ContentSource> {
    root_dir: Utf8PathBuf,
    source: S,
    charts_by_id: BTreeMap<u32, Chart>,
    system_to_chart_map: BTreeMap<String, u32>,
    sid_to_chart_id: BTreeMap<String, u32>,
    systems_shallow_by_path: BTreeMap<String, System>,
}

impl<S: ContentSource> SimulinkParser<S> {
    pub fn new(root_dir: impl AsRef<Utf8Path>, source: S) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
            source,
            charts_by_id: BTreeMap::new(),
            system_to_chart_map: BTreeMap::new(),
            sid_to_chart_id: BTreeMap::new(),
            systems_shallow_by_path: BTreeMap::new(),
        }
    }

    /// Parse a system XML file into a [`System`], resolving subsystem references.
    pub fn parse_system_file(&mut self, path: impl AsRef<Utf8Path>) -> Result<System> {
        let path = path.as_ref();
        self.try_parse_stateflow_for(path);
        self.try_preload_systems_for(path);
        let text = self.source.read_to_string(path)?;
        let doc =
            Document::parse(&text).with_context(|| format!("Failed to parse XML {}", path))?;
        let system_node = doc
            .descendants()
            .find(|n| n.has_tag_name("System"))
            .ok_or_else(|| anyhow!("No <System> root in {}", path))?;
        let base_dir_owned: Utf8PathBuf = path
            .parent()
            .map(|p| p.to_owned())
            .unwrap_or_else(|| self.root_dir.clone());
        let mut sys = crate::block::parse_system_shallow(system_node, base_dir_owned.as_path())?;
        self.link_system_refs(&mut sys, base_dir_owned.as_path());
        Ok(sys)
    }

    /// Parse a Stateflow chart XML file.
    pub fn parse_chart_file(&mut self, path: impl AsRef<Utf8Path>) -> Result<Chart> {
        let path = path.as_ref();
        let text = self.source.read_to_string(path)?;
        chart::parse_chart_from_text(&text, Some(path.as_str()))
    }

    /// Parse `simulink/graphicalInterface.json`.
    pub fn parse_graphical_interface_file(
        &mut self,
        path: impl AsRef<Utf8Path>,
    ) -> Result<GraphicalInterface> {
        let path = path.as_ref();
        let text = self.source.read_to_string(path)?;
        let v: serde_json::Value =
            serde_json::from_str(&text).with_context(|| format!("Failed to parse JSON {}", path))?;
        let gi_value = v
            .get("GraphicalInterface")
            .ok_or_else(|| anyhow!("Missing top-level 'GraphicalInterface' object in {}", path))?;
        let gi: GraphicalInterface = serde_json::from_value(gi_value.clone())
            .with_context(|| format!("Failed to deserialize GraphicalInterface in {}", path))?;
        Ok(gi)
    }

    /// Return list of library names from `graphicalInterface.json`.
    pub fn graphical_interface_library_names(
        &mut self,
        path: impl AsRef<Utf8Path>,
    ) -> Result<Vec<String>> {
        let gi = self.parse_graphical_interface_file(path)?;
        Ok(gi.library_names())
    }

    /// Resolve library references in a parsed system.
    pub fn resolve_library_references(
        system: &mut System,
        lib_paths: &[Utf8PathBuf],
    ) -> Result<()> {
        use std::collections::HashMap;
        let mut library_cache: HashMap<String, System> = HashMap::new();
        let resolver = LibraryResolver::new(lib_paths.iter());
        Self::resolve_library_references_recursive(system, &resolver, &mut library_cache)?;
        Ok(())
    }

    fn resolve_library_references_recursive(
        system: &mut System,
        resolver: &LibraryResolver,
        cache: &mut std::collections::HashMap<String, System>,
    ) -> Result<()> {
        for block in &mut system.blocks {
            if let Some(source_block) = block.properties.get("SourceBlock").cloned() {
                if let Some((lib_name, block_path)) = source_block.split_once('/') {
                    let lib_name = lib_name.trim();
                    let block_path = block_path.trim();
                    if !cache.contains_key(lib_name) {
                        let lookup = resolver.locate(std::iter::once(lib_name));
                        if let Some((_, lib_file)) = lookup.found.first() {
                            match Self::parse_library_file(lib_file) {
                                Ok(lib_system) => {
                                    cache.insert(lib_name.to_string(), lib_system);
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[rustylink] Warning: failed to parse library {}: {}",
                                        lib_name, e
                                    );
                                    continue;
                                }
                            }
                        } else {
                            eprintln!(
                                "[rustylink] Warning: library {} not found in search paths",
                                lib_name
                            );
                            continue;
                        }
                    }
                    if let Some(lib_system) = cache.get(lib_name) {
                        if let Some(lib_block) = Self::find_block_by_name(lib_system, block_path) {
                            if let Some(ref lib_subsystem) = lib_block.subsystem {
                                block.subsystem = Some(lib_subsystem.clone());
                            }
                            block.library_source = Some(lib_name.to_string());
                            block.library_block_path = Some(source_block.clone());
                        } else {
                            eprintln!(
                                "[rustylink] Warning: block '{}' not found in library '{}'",
                                block_path, lib_name
                            );
                        }
                    }
                }
            }
            if let Some(ref mut subsystem) = block.subsystem {
                Self::resolve_library_references_recursive(subsystem, resolver, cache)?;
            }
        }
        Ok(())
    }

    fn parse_library_file(lib_path: &Utf8Path) -> Result<System> {
        let file = std::fs::File::open(lib_path.as_std_path())
            .with_context(|| format!("Open library {}", lib_path))?;
        let reader = std::io::BufReader::new(file);
        let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
        let root = Utf8PathBuf::from("simulink/systems/system_root.xml");
        parser.parse_system_file(&root)
    }

    fn find_block_by_name(system: &System, name: &str) -> Option<Block> {
        system.blocks.iter().find(|b| b.name == name).cloned()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Stateflow & system preloading
// ────────────────────────────────────────────────────────────────────────────

impl<S: ContentSource> SimulinkParser<S> {
    fn try_parse_stateflow_for(&mut self, system_xml_path: &Utf8Path) {
        let mut found_root: Option<Utf8PathBuf> = None;
        for anc in system_xml_path.ancestors() {
            if anc.file_name() == Some("systems") {
                if let Some(parent) = anc.parent() {
                    if parent.file_name() == Some("simulink") {
                        found_root = Some(parent.to_path_buf());
                        break;
                    }
                }
            }
        }
        let sim_root: Utf8PathBuf = found_root.unwrap_or_else(|| self.root_dir.clone());
        let stateflow_dir = sim_root.join("stateflow");
        if let Ok(paths) = self.source.list_dir(&stateflow_dir) {
            let chart_paths: Vec<Utf8PathBuf> = paths
                .into_iter()
                .filter(|p| {
                    p.file_name()
                        .is_some_and(|f| f.starts_with("chart_") && f.ends_with(".xml"))
                })
                .collect();
            let mut texts: Vec<(String, String)> = Vec::new();
            for p in &chart_paths {
                if let Ok(t) = self.source.read_to_string(p) {
                    texts.push((p.as_str().to_string(), t));
                }
            }
            let parsed: Vec<Chart> = texts
                .par_iter()
                .filter_map(|(p, t)| chart::parse_chart_from_text(t, Some(p)).ok())
                .collect();
            for chart in parsed {
                if let Some(id) = chart.id {
                    let ch = self.charts_by_id.entry(id).or_insert(chart);
                    if let Some(nm) = ch.name.clone() {
                        self.system_to_chart_map.entry(nm).or_insert(id);
                    }
                }
            }
        }
    }

    pub fn get_charts(&self) -> &BTreeMap<u32, Chart> {
        &self.charts_by_id
    }
    pub fn get_system_to_chart_map(&self) -> &BTreeMap<String, u32> {
        &self.system_to_chart_map
    }
    pub fn get_chart(&self, id: u32) -> Option<&Chart> {
        self.charts_by_id.get(&id)
    }
    pub fn get_sid_to_chart_map(&self) -> &BTreeMap<String, u32> {
        &self.sid_to_chart_id
    }

    fn try_preload_systems_for(&mut self, system_xml_path: &Utf8Path) {
        let mut found_root: Option<Utf8PathBuf> = None;
        for anc in system_xml_path.ancestors() {
            if anc.file_name() == Some("systems") {
                if let Some(parent) = anc.parent() {
                    if parent.file_name() == Some("simulink") {
                        found_root = Some(parent.to_path_buf());
                        break;
                    }
                }
            }
        }
        let sim_root: Utf8PathBuf = found_root.unwrap_or_else(|| self.root_dir.clone());
        let systems_dir = sim_root.join("systems");
        if let Ok(paths) = self.source.list_dir(&systems_dir) {
            let sys_paths: Vec<Utf8PathBuf> = paths
                .into_iter()
                .filter(|p| {
                    p.file_name()
                        .is_some_and(|f| f.starts_with("system_") && f.ends_with(".xml"))
                })
                .collect();
            let to_read: Vec<Utf8PathBuf> = sys_paths
                .into_iter()
                .filter(|p| !self.systems_shallow_by_path.contains_key(p.as_str()))
                .collect();
            if to_read.is_empty() {
                return;
            }
            let mut pairs: Vec<(Utf8PathBuf, String)> = Vec::new();
            for p in &to_read {
                if let Ok(t) = self.source.read_to_string(p) {
                    pairs.push((p.clone(), t));
                }
            }
            let parsed: Vec<(Utf8PathBuf, Result<System>)> = pairs
                .par_iter()
                .map(|(p, t)| {
                    let res = Document::parse(t)
                        .with_context(|| format!("Failed to parse XML {}", p))
                        .and_then(|doc| {
                            let sysnode = doc
                                .descendants()
                                .find(|n| n.is_element() && n.has_tag_name("System"))
                                .ok_or_else(|| anyhow!("No <System> root in {}", p))?;
                            let base_dir_owned: Utf8PathBuf = p
                                .parent()
                                .map(|pp| pp.to_owned())
                                .unwrap_or_else(|| systems_dir.clone());
                            crate::block::parse_system_shallow(sysnode, base_dir_owned.as_path())
                        });
                    (p.clone(), res)
                })
                .collect();
            for (p, res) in parsed {
                if let Ok(sys) = res {
                    self.systems_shallow_by_path
                        .insert(p.as_str().to_string(), sys);
                }
            }
        }
    }

    fn link_system_refs(&self, system: &mut System, current_base: &Utf8Path) {
        for blk in &mut system.blocks {
            // Check for system_ref (external reference stored by the parser)
            if let Some(ref ref_name) = blk.system_ref {
                let ref_path = helpers::resolve_system_reference(ref_name, current_base);
                if let Some(sub) = self.systems_shallow_by_path.get(ref_path.as_str()) {
                    let mut sub_cloned = sub.clone();
                    let sub_base_dir = ref_path.parent().unwrap_or(current_base);
                    self.link_system_refs(&mut sub_cloned, sub_base_dir);
                    blk.subsystem = Some(Box::new(sub_cloned));
                }
            }
            if let Some(ref mut sub) = blk.subsystem {
                self.link_system_refs(sub, current_base);
            }
        }
    }
}
