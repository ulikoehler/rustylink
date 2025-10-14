use crate::model::*;
use anyhow::{anyhow, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use roxmltree::{Document, Node};
use std::collections::BTreeMap;
use std::io::Read;

pub trait ContentSource {
    fn read_to_string(&mut self, path: &Utf8Path) -> Result<String>;
}

pub struct FsSource;

impl ContentSource for FsSource {
    fn read_to_string(&mut self, path: &Utf8Path) -> Result<String> {
        Ok(std::fs::read_to_string(path.as_str())
            .with_context(|| format!("Failed to read {}", path))?)
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
}

pub struct SimulinkParser<S: ContentSource> {
    root_dir: Utf8PathBuf,
    source: S,
    // Pre-parsed charts by id
    charts_by_id: BTreeMap<u32, Chart>,
    // Mapping from Simulink block path/name to chart id (from machine.xml if available)
    system_to_chart_map: BTreeMap<String, u32>,
    // Mapping from block SID to chart id (resolved directly from System Ref -> chart_*.xml when possible)
    sid_to_chart_id: BTreeMap<u32, u32>,
}

impl<S: ContentSource> SimulinkParser<S> {
    pub fn new(root_dir: impl AsRef<Utf8Path>, source: S) -> Self {
    Self { root_dir: root_dir.as_ref().to_path_buf(), source, charts_by_id: BTreeMap::new(), system_to_chart_map: BTreeMap::new(), sid_to_chart_id: BTreeMap::new() }
    }

    pub fn parse_system_file(&mut self, path: impl AsRef<Utf8Path>) -> Result<System> {
        let path = path.as_ref();
        // println!("[rustylink] Parsing system from file: {}", path);
        // Pre-parse charts and machine mapping before parsing systems
        self.try_parse_stateflow_for(path);
        let text = self.source.read_to_string(path)?;
        let doc = Document::parse(&text).with_context(|| format!("Failed to parse XML {}", path))?;
        if let Some(system_node) = doc.descendants().find(|n| n.has_tag_name("System")) {
            // println!("[rustylink] Detected normal system in file: {} (found <System> root)", path);
            let base_dir_owned: Utf8PathBuf = path
                .parent()
                .map(|p| p.to_owned())
                .unwrap_or_else(|| self.root_dir.clone());
            self.parse_system(system_node, base_dir_owned.as_path())
        } else {
            Err(anyhow!("No <System> root in {}", path))
        }
    }

    fn parse_system(&mut self, node: Node, base_dir: &Utf8Path) -> Result<System> {
    let mut properties = BTreeMap::new();
        let mut blocks = Vec::new();
        let mut lines = Vec::new();

        for child in node.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "P" => {
                    if let Some(name) = child.attribute("Name") {
                        properties.insert(name.to_string(), child.text().unwrap_or("").to_string());
                    }
                }
                "Block" => {
                    blocks.push(self.parse_block(child, base_dir)?);
                }
                "Line" => {
                    lines.push(self.parse_line(child)?);
                }
                unknown => {
                    println!("Unknown tag in System: {}", unknown);
                }
            }
        }

        Ok(System { properties, blocks, lines, chart: None })
    }

    fn parse_block(&mut self, node: Node, base_dir: &Utf8Path) -> Result<Block> {
        let block_type = node.attribute("BlockType").unwrap_or("").to_string();
        let name = node.attribute("Name").unwrap_or("").to_string();
    let sid = node.attribute("SID").and_then(|s| s.parse::<u32>().ok());
        let mut properties = BTreeMap::new();
        let mut ports = Vec::new();
        let mut position = None;
        let mut zorder = None;
        let mut subsystem: Option<Box<System>> = None;
        let mut commented = false;
        let mut is_matlab_function = false;

        for child in node.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "P" => {
                    if let Some(name_attr) = child.attribute("Name") {
                        let value = child
                            .attribute("Ref")
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| child.text().unwrap_or("").to_string());
                        match name_attr {
                            "Position" => position = Some(value),
                            "ZOrder" => zorder = Some(value),
                            "Commented" => {
                                commented = value.eq_ignore_ascii_case("on");
                                properties.insert(name_attr.to_string(), value);
                            }
                            "SFBlockType" => {
                                if value == "MATLAB Function" { is_matlab_function = true; }
                                properties.insert(name_attr.to_string(), value);
                            }
                            _ => {
                                properties.insert(name_attr.to_string(), value);
                            }
                        }
                    }
                }
                "PortCounts" => {
                    let _ = child;
                }
                "PortProperties" => {
                    for pnode in child.children().filter(|c| c.is_element() && c.has_tag_name("Port")) {
                        let mut pprops = BTreeMap::new();
                        let port_type = pnode.attribute("Type").unwrap_or("").to_string();
                        let index = pnode.attribute("Index").and_then(|s| s.parse::<u32>().ok());
                        for pp in pnode.children().filter(|c| c.is_element() && c.has_tag_name("P")) {
                            if let Some(nm) = pp.attribute("Name") {
                                pprops.insert(nm.to_string(), pp.text().unwrap_or("").to_string());
                            }
                        }
                        ports.push(Port { port_type, index, properties: pprops });
                    }
                }
                "System" => {
                    if let Some(reference) = child.attribute("Ref") {
                        let resolved = resolve_system_reference(reference, base_dir);
                        // println!(
                        //     "[rustylink] Parsing referenced system: {} (resolved path: {})",
                        //     reference, resolved
                        // );
                        match self.parse_system_file(&resolved) {
                            Ok(sys) => {
                                subsystem = Some(Box::new(sys));
                            }
                            Err(_err) => {
                                // Tolerate missing referenced system files (e.g. MATLAB Function blocks
                                // that map to Stateflow charts only). Keep subsystem unset and continue.
                                /*eprintln!(
                                    "[rustylink] Warning: failed to parse referenced system '{}': {}",
                                    resolved, err
                                );*/
                            }
                        }
                        // Additionally, try to resolve chart_*.xml directly from this reference and map SID -> chart id
                        if let Some(sid_val) = sid {
                            let chart_path = resolve_chart_reference_from_system_ref(reference, base_dir);
                            if let Ok(chart) = SimulinkParser::parse_chart_file(self, &chart_path) {
                                if let Some(cid) = chart.id { self.charts_by_id.entry(cid).or_insert(chart.clone()); self.sid_to_chart_id.entry(sid_val).or_insert(cid); }
                            }
                        }
                    } else {
                        // println!("[rustylink] Parsing inline system block (no Ref attribute)");
                        match self.parse_system(child, base_dir) {
                            Ok(sys) => subsystem = Some(Box::new(sys)),
                            Err(err) => {
                                eprintln!("[rustylink] Warning: failed to parse inline system: {}", err);
                            }
                        }
                    }
                }
                unknown => {
                    println!("Unknown tag in Block: {}", unknown);
                }
            }
        }

        Ok(Block { block_type, name, sid, position, zorder, commented, is_matlab_function, properties, ports, subsystem })
    }

    fn parse_line(&self, node: Node) -> Result<Line> {
        let mut name = None;
        let mut zorder = None;
        let mut src: Option<EndpointRef> = None;
        let mut dst: Option<EndpointRef> = None;
        let mut labels = None;
        let mut points_list: Vec<Point> = Vec::new();
        let mut branches: Vec<Branch> = Vec::new();

        for child in node.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "P" => {
                    if let Some(nm) = child.attribute("Name") {
                        let val = child.text().unwrap_or("").to_string();
                        match nm {
                            "Name" => name = Some(val),
                            "ZOrder" => zorder = Some(val),
                            "Src" => src = parse_endpoint(&val).ok(),
                            "Dst" => dst = parse_endpoint(&val).ok(),
                            "Labels" => labels = Some(val),
                            "Points" => points_list.extend(parse_points(&val)),
                            _ => {}
                        }
                    }
                }
                "Branch" => {
                    branches.push(self.parse_branch(child)?);
                }
                unknown => {
                    println!("Unknown tag in Line: {}", unknown);
                }
            }
        }

        Ok(Line { name, zorder, src, dst, points: points_list, labels, branches })
    }

    fn parse_branch(&self, node: Node) -> Result<Branch> {
        let mut name = None;
        let mut zorder = None;
        let mut dst: Option<EndpointRef> = None;
        let mut labels = None;
        let mut points_list: Vec<Point> = Vec::new();
        let mut branches: Vec<Branch> = Vec::new();

        for child in node.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "P" => {
                    if let Some(nm) = child.attribute("Name") {
                        let val = child.text().unwrap_or("").to_string();
                        match nm {
                            "Name" => name = Some(val),
                            "ZOrder" => zorder = Some(val),
                            "Dst" => dst = parse_endpoint(&val).ok(),
                            "Labels" => labels = Some(val),
                            "Points" => points_list.extend(parse_points(&val)),
                            _ => {}
                        }
                    }
                }
                "Branch" => branches.push(self.parse_branch(child)?),
                unknown => {
                    println!("Unknown tag in Branch: {}", unknown);
                }
            }
        }

        Ok(Branch { name, zorder, dst, points: points_list, labels, branches })
    }
}

fn resolve_system_reference(reference: &str, base_dir: &Utf8Path) -> Utf8PathBuf {
    // The XML uses values like "system_22" or "system_22.xml"; files are in sibling directory or same base.
    let mut candidate = Utf8PathBuf::from(reference);
    if !candidate.extension().is_some_and(|e| e == "xml") {
        candidate.set_extension("xml");
    }
    // If not absolute, join with base_dir
    let path = if candidate.is_absolute() { candidate } else { base_dir.join(candidate) };
    path
}

fn resolve_chart_reference_from_system_ref(reference: &str, base_dir: &Utf8Path) -> Utf8PathBuf {
    // reference like "system_18" or "system_18.xml" -> build simulink/stateflow/chart_18.xml
    let name = reference.trim_end_matches(".xml");
    let id_part = name.strip_prefix("system_").unwrap_or(name);
    let filename = format!("chart_{}.xml", id_part);
    let sim_root = base_dir.parent().unwrap_or(base_dir);
    sim_root.join("stateflow").join(filename)
}

impl<S: ContentSource> SimulinkParser<S> {
    /// Parse a Stateflow chart XML file and extract script and port metadata.
    pub fn parse_chart_file(&mut self, path: impl AsRef<Utf8Path>) -> Result<Chart> {
        let path = path.as_ref();
        let text = self.source.read_to_string(path)?;
        let doc = Document::parse(&text).with_context(|| format!("Failed to parse XML {}", path))?;
        let chart_node = doc
            .descendants()
            .find(|n| n.is_element() && n.has_tag_name("chart"))
            .ok_or_else(|| anyhow!("No <chart> root in {}", path))?;

        // Collect top-level properties
        let mut properties = BTreeMap::new();
        for p in chart_node.children().filter(|c| c.is_element() && c.has_tag_name("P")) {
            if let Some(nm) = p.attribute("Name") {
                properties.insert(nm.to_string(), p.text().unwrap_or("").to_string());
            }
        }

        let id = chart_node.attribute("id").and_then(|s| s.parse::<u32>().ok());
        let name = properties.get("name").cloned();

        // EML name
        let eml_name = chart_node
            .children()
            .find(|c| c.is_element() && c.has_tag_name("eml"))
            .and_then(|eml| eml.children().find(|c| c.is_element() && c.has_tag_name("P") && c.attribute("Name") == Some("name")))
            .and_then(|p| p.text())
            .map(|s| s.to_string());

        // Script: search for P Name="script" under any <state>/<eml>
        let mut script: Option<String> = None;
        for st in chart_node.descendants().filter(|c| c.is_element() && c.has_tag_name("state")) {
            if let Some(eml) = st.children().find(|c| c.is_element() && c.has_tag_name("eml")) {
                if let Some(scr) = eml
                    .children()
                    .find(|c| c.is_element() && c.has_tag_name("P") && c.attribute("Name") == Some("script"))
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
        for data in chart_node.descendants().filter(|c| c.is_element() && c.has_tag_name("data")) {
            let port_name = data.attribute("name").unwrap_or("").to_string();
            if port_name.is_empty() { continue; }
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
                                    if let Some(szp) = pp
                                        .children()
                                        .find(|c| c.is_element() && c.has_tag_name("P") && c.attribute("Name") == Some("size"))
                                    {
                                        size = szp.text().map(|s| s.to_string());
                                    }
                                }
                                "type" => {
                                    for tprop in pp.children().filter(|c| c.is_element() && c.has_tag_name("P")) {
                                        if let Some(nm) = tprop.attribute("Name") {
                                            let val = tprop.text().unwrap_or("").to_string();
                                            match nm {
                                                "method" => method = Some(val),
                                                "primitive" => primitive = Some(val),
                                                "isSigned" => is_signed = val.parse::<i32>().ok().map(|v| v != 0),
                                                "wordLength" => word_length = val.parse::<u32>().ok(),
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                "unit" => {
                                    if let Some(up) = pp
                                        .children()
                                        .find(|c| c.is_element() && c.has_tag_name("P") && c.attribute("Name") == Some("name"))
                                    {
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

            let port = ChartPort { name: port_name, size, method, primitive, is_signed, word_length, complexity, frame, data_type, unit };
            match scope.as_deref() {
                Some("INPUT_DATA") => inputs.push(port),
                Some("OUTPUT_DATA") => outputs.push(port),
                _ => {}
            }
        }

        Ok(Chart { id, name, eml_name, script, inputs, outputs, properties })
    }
}

impl<S: ContentSource> SimulinkParser<S> {
    /// Attempt to parse stateflow/machine.xml and all referenced charts based on the location of a given system xml path.
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
        let machine_path = sim_root.join("stateflow").join("machine.xml");
        // Try to read; if fails, just return silently
        let text = match self.source.read_to_string(&machine_path) {
            Ok(t) => t,
            Err(_) => return,
        };
        if let Ok(doc) = Document::parse(&text) {
            // Build list of chart files from <machine>/<Children>/<chart Ref="chart_XX"/>
            let mut chart_refs: Vec<String> = Vec::new();
            if let Some(machine) = doc.descendants().find(|n| n.is_element() && n.has_tag_name("machine")) {
                if let Some(children) = machine.children().find(|n| n.is_element() && n.has_tag_name("Children")) {
                    for ch in children.children().filter(|c| c.is_element() && c.has_tag_name("chart")) {
                        if let Some(r) = ch.attribute("Ref") { chart_refs.push(r.to_string()); }
                    }
                }
            }
            // Parse <instance> mapping name -> chart id
            for inst in doc.descendants().filter(|n| n.is_element() && n.has_tag_name("instance")) {
                let mut name: Option<String> = None;
                let mut chart_id: Option<u32> = None;
                for p in inst.children().filter(|c| c.is_element() && c.has_tag_name("P")) {
                    match p.attribute("Name") {
                        Some("name") => name = p.text().map(|s| s.to_string()),
                        Some("chart") => chart_id = p.text().and_then(|s| s.parse::<u32>().ok()),
                        _ => {}
                    }
                }
                if let (Some(n), Some(cid)) = (name, chart_id) {
                    self.system_to_chart_map.entry(n).or_insert(cid);
                }
            }
            // Load chart files
            for r in chart_refs {
                let chart_path = sim_root.join("stateflow").join(format!("{}.xml", r.trim_end_matches(".xml")));
                if let Ok(chart) = SimulinkParser::parse_chart_file(self, &chart_path) {
                    if let Some(id) = chart.id { self.charts_by_id.entry(id).or_insert(chart); }
                }
            }
        }
    }

    pub fn get_charts(&self) -> &BTreeMap<u32, Chart> { &self.charts_by_id }
    pub fn get_system_to_chart_map(&self) -> &BTreeMap<String, u32> { &self.system_to_chart_map }
    pub fn get_chart(&self, id: u32) -> Option<&Chart> { self.charts_by_id.get(&id) }
    pub fn get_sid_to_chart_map(&self) -> &BTreeMap<u32, u32> { &self.sid_to_chart_id }
}

fn parse_points(s: &str) -> Vec<Point> {
    // Expected formats: "[x, y]" or "[x, y; x2, y2; ...]"
    let trimmed = s.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .unwrap_or(trimmed);
    let mut points = Vec::new();
    for pair in inner.split(';') {
        let pair = pair.trim();
        if pair.is_empty() { continue; }
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

fn parse_endpoint(s: &str) -> Result<EndpointRef> {
    // Formats observed: "5#out:1" or "11#in:2"
    // Split at '#'
    let (sid_str, rest) = s
        .split_once('#')
        .ok_or_else(|| anyhow!("Invalid endpoint format: {}", s))?;
    let sid: u32 = sid_str.trim().parse()?;
    // rest like "out:1" or "in:2"
    let (ptype, pidx_str) = rest
        .split_once(':')
        .ok_or_else(|| anyhow!("Invalid endpoint port format: {}", s))?;
    let port_type = ptype.trim().to_string();
    let port_index: u32 = pidx_str.trim().parse()?;
    Ok(EndpointRef { sid, port_type, port_index })
}
