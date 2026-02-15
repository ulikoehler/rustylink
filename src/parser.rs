use crate::model::*;
use anyhow::{Context, Result, anyhow};
use camino::{Utf8Path, Utf8PathBuf};
use rayon::prelude::*;
use roxmltree::{Document, Node};
use std::collections::{BTreeMap, HashMap};
use std::io::Read;

pub trait ContentSource {
    fn read_to_string(&mut self, path: &Utf8Path) -> Result<String>;
    /// List files in a directory path (logical path for the source), returning full paths
    fn list_dir(&mut self, path: &Utf8Path) -> Result<Vec<Utf8PathBuf>>;
}

pub struct FsSource;

impl FsSource {
    fn list_dir_impl(&mut self, path: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
        let mut files = Vec::new();
        for entry in
            std::fs::read_dir(path.as_std_path()).with_context(|| format!("Read dir {}", path))?
        {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let p = camino::Utf8PathBuf::from_path_buf(entry.path())
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
            if name.starts_with(&prefix) {
                // Only include direct children files (no trailing slash and no deeper directories)
                if !name.ends_with('/') {
                    // Accept any depth under prefix; the caller can filter by filename
                    files.push(Utf8PathBuf::from(name));
                }
            }
        }
        Ok(files)
    }
}

pub struct SimulinkParser<S: ContentSource> {
    root_dir: Utf8PathBuf,
    source: S,
    // Pre-parsed charts by id
    charts_by_id: BTreeMap<u32, Chart>,
    // Mapping from Simulink block path/name to chart id (from machine.xml if available)
    system_to_chart_map: BTreeMap<String, u32>,
    // Mapping from block SID (may be non-numeric like "47:2") to chart id
    sid_to_chart_id: BTreeMap<String, u32>,
    // Pre-parsed shallow systems keyed by full path string
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

    pub fn parse_system_file(&mut self, path: impl AsRef<Utf8Path>) -> Result<System> {
        let path = path.as_ref();
        // println!("[rustylink] Parsing system from file: {}", path);
        // Preload charts and systems (parallel parsing) before building a fully-linked system
        self.try_parse_stateflow_for(path);
        self.try_preload_systems_for(path);
        // Parse requested system shallowly, then link references using preloaded systems
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

    fn parse_system(&mut self, node: Node, base_dir: &Utf8Path) -> Result<System> {
        let mut properties = BTreeMap::new();
        let mut blocks = Vec::new();
        let mut lines = Vec::new();
        let mut annotations: Vec<Annotation> = Vec::new();
        for child in node.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "P" => {
                    if let Some(name) = child.attribute("Name") {
                        properties.insert(name.to_string(), child.text().unwrap_or("").to_string());
                    }
                }
                "Block" => {
                    // Use shallow parsing semantics to avoid cross-file recursion here
                    blocks.push(crate::block::parse_block_shallow(child, base_dir)?);
                }
                "Reference" => {
                    // Support <Reference ...> elements in the same way as <Block BlockType="Reference">.
                    blocks.push(crate::block::parse_block_shallow(child, base_dir)?);
                }
                "Line" => {
                    lines.push(crate::block::parse_line_node(child)?);
                }
                "Annotation" => match crate::block::parse_annotation_node(child) {
                    Ok(a) => annotations.push(a),
                    Err(err) => {
                        eprintln!("[rustylink] Warning: failed to parse <Annotation>: {}", err)
                    }
                },
                unknown => {
                    println!("Unknown tag in System: {}", unknown);
                }
            }
        }

        Ok(System {
            properties,
            blocks,
            lines,
            annotations,
            chart: None,
        })
    }

    fn parse_block(&mut self, node: Node, base_dir: &Utf8Path) -> Result<Block> {
        // Delegate block parsing to the `block` module which contains the refactored logic.
        crate::block::parse_block(node, base_dir)
    }
}

// Block-related parsing helpers were refactored into `src/block.rs`.

fn matches_ignore_case(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

pub(crate) fn resolve_system_reference(reference: &str, base_dir: &Utf8Path) -> Utf8PathBuf {
    // The XML uses values like "system_22" or "system_22.xml"; files are in sibling directory or same base.
    let mut candidate = Utf8PathBuf::from(reference);
    if !candidate.extension().is_some_and(|e| e == "xml") {
        candidate.set_extension("xml");
    }
    // If not absolute, join with base_dir
    let path = if candidate.is_absolute() {
        candidate
    } else {
        base_dir.join(candidate)
    };
    path
}

// Removed: resolving charts from system refs; charts are discovered via directory listing only.

impl<S: ContentSource> SimulinkParser<S> {
    /// Parse a Stateflow chart XML file and extract script and port metadata.
    pub fn parse_chart_file(&mut self, path: impl AsRef<Utf8Path>) -> Result<Chart> {
        let path = path.as_ref();
        let text = self.source.read_to_string(path)?;
        parse_chart_from_text(&text, Some(path.as_str()))
    }

    /// Parse `simulink/graphicalInterface.json` and return a strongly-typed
    /// `GraphicalInterface` structure. The JSON root is expected to be
    /// `{ "GraphicalInterface": { ... } }` — we deserialize the inner object.
    pub fn parse_graphical_interface_file(&mut self, path: impl AsRef<Utf8Path>) -> Result<GraphicalInterface> {
        let path = path.as_ref();
        let text = self.source.read_to_string(path)?;
        let v: serde_json::Value = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse JSON {}", path))?;
        let gi_value = v
            .get("GraphicalInterface")
            .ok_or_else(|| anyhow!("Missing top-level 'GraphicalInterface' object in {}", path))?;
        let gi: GraphicalInterface = serde_json::from_value(gi_value.clone())
            .with_context(|| format!("Failed to deserialize GraphicalInterface in {}", path))?;
        Ok(gi)
    }

    /// Convenience: return list of library names found in a parsed
    /// `graphicalInterface.json` (delegates to `GraphicalInterface::library_names`).
    pub fn graphical_interface_library_names(&mut self, path: impl AsRef<Utf8Path>) -> Result<Vec<String>> {
        let gi = self.parse_graphical_interface_file(path)?;
        Ok(gi.library_names())
    }

    /// Resolve library references in a parsed system by locating and parsing
    /// library .slx files, then copying referenced blocks' content into the system.
    ///
    /// This walks all blocks in `system`, finds those with a `SourceBlock` property,
    /// locates the corresponding library file, parses it, and copies the referenced
    /// block's subsystem into the referencing block. The referencing block is marked
    /// with `library_source` and `library_block_path` for tracking.
    ///
    /// # Arguments
    /// - `system`: The system to resolve references in (modified in-place)
    /// - `lib_paths`: Search paths for locating LIBNAME.slx files
    ///
    /// Note: This creates a new parser instance for each library, which may be slow
    /// for large models. Consider caching parsed libraries if needed.
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
        cache: &mut HashMap<String, System>,
    ) -> Result<()> {
        for block in &mut system.blocks {
            // Check if this block references a library block
            if let Some(source_block) = block.properties.get("SourceBlock").cloned() {
                // source_block is like "Regler/Joint_Interpolator" or "simulink/Logic and Bit Operations/Compare To Constant"
                if let Some((lib_name, block_path)) = source_block.split_once('/') {
                    let lib_name = lib_name.trim();
                    let block_path = block_path.trim();

                    // Try to locate and load the library
                    if !cache.contains_key(lib_name) {
                        let lookup = resolver.locate(std::iter::once(lib_name));
                        if let Some((_, lib_file)) = lookup.found.first() {
                            // Parse the library .slx file
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

                    // Look up the referenced block in the library
                    if let Some(lib_system) = cache.get(lib_name) {
                        if let Some(lib_block) = Self::find_block_by_name(lib_system, block_path) {
                            // Copy the library block's subsystem into this block
                            if let Some(ref lib_subsystem) = lib_block.subsystem {
                                block.subsystem = Some(lib_subsystem.clone());
                            }
                            // Mark this block with library source metadata
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

            // Recursively resolve in subsystems
            if let Some(ref mut subsystem) = block.subsystem {
                Self::resolve_library_references_recursive(subsystem, resolver, cache)?;
            }
        }
        Ok(())
    }

    /// Parse a library .slx file and return its root system.
    fn parse_library_file(lib_path: &Utf8Path) -> Result<System> {
        let file = std::fs::File::open(lib_path.as_std_path())
            .with_context(|| format!("Open library {}", lib_path))?;
        let reader = std::io::BufReader::new(file);
        let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
        let root = Utf8PathBuf::from("simulink/systems/system_root.xml");
        parser.parse_system_file(&root)
    }

    /// Find a block by name in a system (searches only top-level blocks).
    fn find_block_by_name(system: &System, name: &str) -> Option<Block> {
        system.blocks.iter().find(|b| b.name == name).cloned()
    }
}

impl<S: ContentSource> SimulinkParser<S> {
    /// Attempt to pre-load charts based on the location of a given system xml path.
    /// This function is idempotent and safe to call multiple times.
    fn try_parse_stateflow_for(&mut self, system_xml_path: &Utf8Path) {
        // Determine sim root: path like simulink/systems/system_XX.xml -> root is parent of "systems" which itself is under "simulink"
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
        // Discover and parse all chart_*.xml files via ContentSource directory listing
        let stateflow_dir = sim_root.join("stateflow");
        if let Ok(paths) = self.source.list_dir(&stateflow_dir) {
            let chart_paths: Vec<Utf8PathBuf> = paths
                .into_iter()
                .filter(|p| {
                    p.file_name()
                        .is_some_and(|f| f.starts_with("chart_") && f.ends_with(".xml"))
                })
                .collect();
            // Read texts sequentially (ContentSource is &mut self), then parse in parallel
            let mut texts: Vec<(String, String)> = Vec::new();
            for p in &chart_paths {
                if let Ok(t) = self.source.read_to_string(p) {
                    texts.push((p.as_str().to_string(), t));
                }
            }
            let parsed: Vec<Chart> = texts
                .par_iter()
                .filter_map(|(p, t)| parse_chart_from_text(t, Some(p)).ok())
                .collect();
            // Merge results serially
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
}

pub(crate) fn parse_points(s: &str) -> Vec<Point> {
    // Expected formats: "[x, y]" or "[x, y; x2, y2; ...]"
    let trimmed = s.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .unwrap_or(trimmed);
    let mut points = Vec::new();
    for pair in inner.split(';') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        // split by comma
        let mut it = pair.split(',').map(|v| v.trim()).filter(|t| !t.is_empty());
        if let (Some(x), Some(y)) = (it.next(), it.next()) {
            if let (Ok(xv), Ok(yv)) = (x.parse::<i32>(), y.parse::<i32>()) {
                points.push(Point { x: xv, y: yv });
            }
        }
    }
    points
}

pub(crate) fn parse_endpoint(s: &str) -> Result<EndpointRef> {
    // Formats observed: "5#out:1" or "11#in:2"
    // Split at '#'
    let (sid_str, rest) = s
        .split_once('#')
        .ok_or_else(|| anyhow!("Invalid endpoint format: {}", s))?;
    let sid: String = sid_str.trim().to_string();
    // rest like "out:1" or "in:2"
    let (ptype, pidx_str) = rest
        .split_once(':')
        .ok_or_else(|| anyhow!("Invalid endpoint port format: {}", s))?;
    let port_type = ptype.trim().to_string();
    let port_index: u32 = pidx_str.trim().parse()?;
    Ok(EndpointRef {
        sid,
        port_type,
        port_index,
    })
}

// ---------------- Free helper functions and shallow parsing ----------------

// --------------------- GraphicalInterface JSON types ---------------------

/// Type of external file reference in `graphicalInterface.json`.
///
/// The JSON uses strings like "LIBRARY_BLOCK"; unknown values are preserved
/// in the `Other` variant so parsing is forward-compatible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalFileReferenceType {
    LibraryBlock,
    Other(String),
}

impl<'de> serde::Deserialize<'de> for ExternalFileReferenceType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "LIBRARY_BLOCK" => Ok(ExternalFileReferenceType::LibraryBlock),
            other => Ok(ExternalFileReferenceType::Other(other.to_string())),
        }
    }
}

impl serde::Serialize for ExternalFileReferenceType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ExternalFileReferenceType::LibraryBlock => serializer.serialize_str("LIBRARY_BLOCK"),
            ExternalFileReferenceType::Other(s) => serializer.serialize_str(s),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ExternalFileReference {
    #[serde(rename = "Path")]
    pub path: String,
    #[serde(rename = "Reference")]
    pub reference: String,
    #[serde(rename = "SID")]
    pub sid: String,
    #[serde(rename = "Type")]
    pub r#type: ExternalFileReferenceType,
}

/// Solver name from the `graphicalInterface.json` file. Known value in the
/// repository is `FixedStepDiscrete`; unknown values are preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolverName {
    FixedStepDiscrete,
    Other(String),
}

impl<'de> serde::Deserialize<'de> for SolverName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "FixedStepDiscrete" => Ok(SolverName::FixedStepDiscrete),
            other => Ok(SolverName::Other(other.to_string())),
        }
    }
}

impl serde::Serialize for SolverName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            SolverName::FixedStepDiscrete => serializer.serialize_str("FixedStepDiscrete"),
            SolverName::Other(s) => serializer.serialize_str(s),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct GraphicalInterface {
    #[serde(rename = "ExternalFileReferences")]
    pub external_file_references: Vec<ExternalFileReference>,
    #[serde(rename = "PreCompExecutionDomainType")]
    pub precomp_execution_domain_type: Option<String>,
    #[serde(rename = "SimulinkSubDomainType")]
    pub simulink_sub_domain_type: Option<String>,
    #[serde(rename = "SolverName")]
    pub solver_name: Option<SolverName>,
}

impl GraphicalInterface {
    /// Return a list of library names referenced by `ExternalFileReferences`.
    ///
    /// Behavior:
    /// - Only consider entries with `Type == LIBRARY_BLOCK`.
    /// - Extract the part before the first '/' from the `Reference` field.
    /// - Return unique names in the order they appear.
    pub fn library_names(&self) -> Vec<String> {
        use std::collections::HashSet;
        let mut out: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for r in &self.external_file_references {
            if r.r#type != ExternalFileReferenceType::LibraryBlock {
                continue;
            }
            let lib = r
                .reference
                .split_once('/')
                .map(|(a, _)| a.trim().to_string())
                .unwrap_or_else(|| r.reference.trim().to_string());
            if lib.is_empty() {
                continue;
            }
            if seen.insert(lib.clone()) {
                out.push(lib);
            }
        }
        out
    }
}

/// Result for library resolution: which libraries were found (with path)
/// and which were not found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryLookupResult {
    pub found: Vec<(String, Utf8PathBuf)>,
    pub not_found: Vec<String>,
}

/// Resolver that searches for `LIBNAME.slx` files in an ordered list of
/// directories (first match wins).
#[derive(Debug, Clone)]
pub struct LibraryResolver {
    search_paths: Vec<Utf8PathBuf>,
}

impl LibraryResolver {
    /// Create a resolver that will search the provided directories in order.
    pub fn new<P: AsRef<Utf8Path>>(paths: impl IntoIterator<Item = P>) -> Self {
        Self {
            search_paths: paths.into_iter().map(|p| p.as_ref().to_path_buf()).collect(),
        }
    }

    /// Locate the given library names (e.g. `Regler`) by looking for
    /// `Regler.slx` under the configured search paths. Returns a list of
    /// found libraries (library name + full path) and a list of not-found names.
    pub fn locate<'a, I>(&self, libs: I) -> LibraryLookupResult
    where
        I: IntoIterator<Item = &'a str>,
    {
        use std::collections::HashSet;
        let mut found = Vec::new();
        let mut not_found = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for lib in libs {
            let lib = lib.trim();
            if lib.is_empty() || !seen.insert(lib.to_string()) {
                continue; // skip duplicates / empty
            }
            let file_name = format!("{}.slx", lib);
            let mut matched: Option<Utf8PathBuf> = None;
            for dir in &self.search_paths {
                let candidate = dir.join(&file_name);
                if candidate.exists() {
                    matched = Some(candidate);
                    break;
                }
            }
            if let Some(p) = matched {
                found.push((lib.to_string(), p));
            } else {
                not_found.push(lib.to_string());
            }
        }
        LibraryLookupResult { found, not_found }
    }
}

// --------------------- end JSON types ---------------------

// Block-related parsing helpers were moved to `src/block.rs`.

fn parse_chart_from_text(text: &str, path_hint: Option<&str>) -> Result<Chart> {

    let doc = Document::parse(text)
        .with_context(|| format!("Failed to parse XML {}", path_hint.unwrap_or("<chart>")))?;
    let chart_node = doc
        .descendants()
        .find(|n| n.is_element() && n.has_tag_name("chart"))
        .ok_or_else(|| anyhow!("No <chart> root in {}", path_hint.unwrap_or("<chart>")))?;

    // Collect top-level properties
    let mut properties = BTreeMap::new();
    for p in chart_node
        .children()
        .filter(|c| c.is_element() && c.has_tag_name("P"))
    {
        if let Some(nm) = p.attribute("Name") {
            properties.insert(nm.to_string(), p.text().unwrap_or("").to_string());
        }
    }

    let id = chart_node
        .attribute("id")
        .and_then(|s| s.parse::<u32>().ok());
    let name = properties.get("name").cloned();

    // EML name
    let eml_name = chart_node
        .children()
        .find(|c| c.is_element() && c.has_tag_name("eml"))
        .and_then(|eml| {
            eml.children().find(|c| {
                c.is_element() && c.has_tag_name("P") && c.attribute("Name") == Some("name")
            })
        })
        .and_then(|p| p.text())
        .map(|s| s.to_string());

    // Script: search for P Name="script" under any <state>/<eml>
    let mut script: Option<String> = None;
    for st in chart_node
        .descendants()
        .filter(|c| c.is_element() && c.has_tag_name("state"))
    {
        if let Some(eml) = st
            .children()
            .find(|c| c.is_element() && c.has_tag_name("eml"))
        {
            if let Some(scr) = eml
                .children()
                .find(|c| {
                    c.is_element() && c.has_tag_name("P") && c.attribute("Name") == Some("script")
                })
                .and_then(|p| p.text())
            {
                script = Some(scr.to_string());
                break;
            }
        }
    }

    // Ports: inputs and outputs based on <data> nodes and their <P Name="scope">
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    for data in chart_node
        .descendants()
        .filter(|c| c.is_element() && c.has_tag_name("data"))
    {
        let port_name = data.attribute("name").unwrap_or("").to_string();
        if port_name.is_empty() {
            continue;
        }
        let mut scope: Option<String> = None;
        let mut size: Option<String> = None;
        let mut method: Option<String> = None;
        let mut primitive: Option<String> = None;
        let mut is_signed: Option<bool> = None;
        let mut word_length: Option<u32> = None;
        let mut complexity: Option<String> = None;
        let mut frame: Option<String> = None;
        let mut unit: Option<String> = None;
        let mut data_type: Option<String> = None;

        for child in data.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "P" => {
                    if let Some(nm) = child.attribute("Name") {
                        let val = child.text().unwrap_or("").to_string();
                        match nm {
                            "scope" => scope = Some(val),
                            "dataType" => data_type = Some(val),
                            _ => {}
                        }
                    }
                }
                "props" => {
                    // Inside props we may find array, type, complexity, frame, unit
                    for pp in child.children().filter(|c| c.is_element()) {
                        match pp.tag_name().name() {
                            "array" => {
                                if let Some(szp) = pp.children().find(|c| {
                                    c.is_element()
                                        && c.has_tag_name("P")
                                        && c.attribute("Name") == Some("size")
                                }) {
                                    size = szp.text().map(|s| s.to_string());
                                }
                            }
                            "type" => {
                                for tprop in pp
                                    .children()
                                    .filter(|c| c.is_element() && c.has_tag_name("P"))
                                {
                                    if let Some(nm) = tprop.attribute("Name") {
                                        let val = tprop.text().unwrap_or("").to_string();
                                        match nm {
                                            "method" => method = Some(val),
                                            "primitive" => primitive = Some(val),
                                            "isSigned" => {
                                                is_signed = val.parse::<i32>().ok().map(|v| v != 0)
                                            }
                                            "wordLength" => word_length = val.parse::<u32>().ok(),
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            "unit" => {
                                if let Some(up) = pp.children().find(|c| {
                                    c.is_element()
                                        && c.has_tag_name("P")
                                        && c.attribute("Name") == Some("name")
                                }) {
                                    unit = up.text().map(|s| s.to_string());
                                }
                            }
                            _ => {
                                // also P nodes directly under props
                                if pp.has_tag_name("P") {
                                    if let Some(nm) = pp.attribute("Name") {
                                        let val = pp.text().unwrap_or("").to_string();
                                        match nm {
                                            "complexity" => complexity = Some(val),
                                            "frame" => frame = Some(val),
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let port = ChartPort {
            name: port_name,
            size,
            method,
            primitive,
            is_signed,
            word_length,
            complexity,
            frame,
            data_type,
            unit,
        };
        match scope.as_deref() {
            Some("INPUT_DATA") => inputs.push(port),
            Some("OUTPUT_DATA") => outputs.push(port),
            _ => {}
        }
    }

    Ok(Chart {
        id,
        name,
        eml_name,
        script,
        inputs,
        outputs,
        properties,
    })
}

impl<S: ContentSource> SimulinkParser<S> {
    /// Preload and shallow-parse all systems in the systems directory for the given system path.
    fn try_preload_systems_for(&mut self, system_xml_path: &Utf8Path) {
        // Determine sim root as in charts
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
            // Avoid re-parsing if already loaded (basic check)
            let to_read: Vec<Utf8PathBuf> = sys_paths
                .into_iter()
                .filter(|p| !self.systems_shallow_by_path.contains_key(p.as_str()))
                .collect();
            if to_read.is_empty() {
                return;
            }
            // Read texts sequentially
            let mut pairs: Vec<(Utf8PathBuf, String)> = Vec::new();
            for p in &to_read {
                if let Ok(t) = self.source.read_to_string(p) {
                    pairs.push((p.clone(), t));
                }
            }
            // Parse in parallel to shallow systems
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
            // Merge serially
            for (p, res) in parsed {
                if let Ok(sys) = res {
                    self.systems_shallow_by_path
                        .insert(p.as_str().to_string(), sys);
                }
            }
        }
    }

    /// Link pass: resolve "__SystemRef" properties into actual nested subsystems from preloaded map.
    fn link_system_refs(&self, system: &mut System, current_base: &Utf8Path) {
        for blk in &mut system.blocks {
            if let Some(ref_path) = blk.properties.get("__SystemRef").cloned() {
                if let Some(sub) = self.systems_shallow_by_path.get(&ref_path) {
                    let mut sub_cloned = sub.clone();
                    // The referenced system might have its own references; link recursively.
                    let sub_base = Utf8PathBuf::from(&ref_path);
                    let sub_base_dir = sub_base.parent().unwrap_or(current_base);
                    self.link_system_refs(&mut sub_cloned, sub_base_dir);
                    blk.subsystem = Some(Box::new(sub_cloned));
                }
            }
            // Also link inline-parsed subsystem recursively
            if let Some(ref mut sub) = blk.subsystem {
                self.link_system_refs(sub, current_base);
            }
        }
    }
}
