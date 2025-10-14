use crate::model::*;
use anyhow::{anyhow, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use roxmltree::{Document, Node};
use std::collections::BTreeMap;
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
        for entry in std::fs::read_dir(path.as_std_path()).with_context(|| format!("Read dir {}", path))? {
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
    fn list_dir(&mut self, path: &Utf8Path) -> Result<Vec<Utf8PathBuf>> { self.list_dir_impl(path) }
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
        let mut prefix = path.as_str().trim_start_matches("./").trim_start_matches('/').to_string();
        if !prefix.is_empty() && !prefix.ends_with('/') { prefix.push('/'); }
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
        let mut block_type = node.attribute("BlockType").unwrap_or("").to_string();
        let name = node.attribute("Name").unwrap_or("").to_string();
        let sid = node.attribute("SID").map(|s| s.to_string());
        let mut properties = BTreeMap::new();
        let mut ports = Vec::new();
        let mut position = None;
        let mut zorder = None;
        let mut subsystem: Option<Box<System>> = None;
        let mut commented = false;
        let mut is_matlab_function = false;
        let mut c_output_code: Option<String> = None;
        let mut c_start_code: Option<String> = None;
        let mut c_term_code: Option<String> = None;
        let mut c_codegen_output: Option<String> = None;
        let mut c_codegen_start: Option<String> = None;
        let mut c_codegen_term: Option<String> = None;
        let mut mask: Option<Mask> = None;

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
                            // Capture CFunction code snippets
                            "OutputCode" => { c_output_code = Some(value.clone()); properties.insert(name_attr.to_string(), value); }
                            "StartCode" => { c_start_code = Some(value.clone()); properties.insert(name_attr.to_string(), value); }
                            "TerminateCode" => { c_term_code = Some(value.clone()); properties.insert(name_attr.to_string(), value); }
                            "CodegenOutputCode" => { c_codegen_output = Some(value.clone()); properties.insert(name_attr.to_string(), value); }
                            "CodegenStartCode" => { c_codegen_start = Some(value.clone()); properties.insert(name_attr.to_string(), value); }
                            "CodegenTerminateCode" => { c_codegen_term = Some(value.clone()); properties.insert(name_attr.to_string(), value); }
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
                "Mask" => {
                    match self.parse_mask(child) {
                        Ok(m) => mask = Some(m),
                        Err(err) => eprintln!("[rustylink] Error parsing <Mask> in block '{}': {}", name, err),
                    }
                }
                unknown => {
                    println!("Unknown tag in Block: {}", unknown);
                }
            }
        }
        // If block_type is SubSystem and is_matlab_function is true, override block_type string
        if block_type == "SubSystem" && is_matlab_function {
            block_type = "MATLAB Function".to_string();
        }
        let c_function = if block_type == "CFunction" {
            Some(crate::model::CFunctionCode {
                output_code: c_output_code,
                start_code: c_start_code,
                terminate_code: c_term_code,
                codegen_output_code: c_codegen_output,
                codegen_start_code: c_codegen_start,
                codegen_terminate_code: c_codegen_term,
            })
        } else { None };
        Ok(Block { block_type, name, sid, position, zorder, commented, is_matlab_function, properties, ports, subsystem, c_function, mask })
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

impl<S: ContentSource> SimulinkParser<S> {
    fn parse_mask(&self, node: Node) -> Result<Mask> {
        let mut display: Option<String> = None;
        let mut description: Option<String> = None;
        let mut initialization: Option<String> = None;
        let mut parameters: Vec<MaskParameter> = Vec::new();
        let mut dialog: Vec<DialogControl> = Vec::new();

        for child in node.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "Display" => display = child.text().map(|s| s.to_string()),
                "Description" => description = child.text().map(|s| s.to_string()),
                "Initialization" => initialization = child.text().map(|s| s.to_string()),
                "MaskParameter" => {
                    parameters.push(self.parse_mask_parameter(child));
                }
                "DialogControl" => {
                    dialog.push(self.parse_dialog_control(child));
                }
                other => {
                    println!("Unknown tag in Mask: {}", other);
                }
            }
        }

        Ok(Mask { display, description, initialization, parameters, dialog })
    }

    fn parse_mask_parameter(&self, node: Node) -> MaskParameter {
        let name = node.attribute("Name").unwrap_or("").to_string();
        let tattr = node.attribute("Type").unwrap_or("");
        let param_type = match tattr {
            t if t.eq_ignore_ascii_case("popup") => MaskParamType::Popup,
            t if t.eq_ignore_ascii_case("edit") => MaskParamType::Edit,
            other => {
                println!("Unknown MaskParameter Type: {} (Name='{}')", other, name);
                MaskParamType::Unknown(other.to_string())
            }
        };
        let tunable = node
            .attribute("Tunable")
            .map(|v| matches_ignore_case(v, "on") || v == "1");
        let visible = node
            .attribute("Visible")
            .map(|v| matches_ignore_case(v, "on") || v == "1");

        // Report unexpected attributes
        for attr in node.attributes() {
            let key = attr.name();
            if key != "Name" && key != "Type" && key != "Tunable" && key != "Visible" {
                println!(
                    "Unknown attribute in MaskParameter(Name='{}'): {}='{}'",
                    name,
                    key,
                    attr.value()
                );
            }
        }

        let mut prompt: Option<String> = None;
        let mut value: Option<String> = None;
        let mut type_options: Vec<String> = Vec::new();

        for child in node.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "Prompt" => prompt = child.text().map(|s| s.to_string()),
                "Value" => value = child.text().map(|s| s.to_string()),
                "TypeOptions" => {
                    for to in child.children().filter(|c| c.is_element()) {
                        if to.has_tag_name("Option") {
                            if let Some(t) = to.text() { type_options.push(t.to_string()); }
                        } else {
                            println!("Unknown tag in MaskParameter TypeOptions: {}", to.tag_name().name());
                        }
                    }
                }
                other => {
                    println!("Unknown tag in MaskParameter(Name='{}'): {}", name, other);
                }
            }
        }

        MaskParameter { name, param_type, prompt, value, tunable, visible, type_options }
    }

    fn parse_dialog_control(&self, node: Node) -> DialogControl {
        let tattr = node.attribute("Type").unwrap_or("");
        let control_type = match tattr {
            t if t.eq_ignore_ascii_case("Group") => DialogControlType::Group,
            t if t.eq_ignore_ascii_case("Text") => DialogControlType::Text,
            t if t.eq_ignore_ascii_case("Edit") => DialogControlType::Edit,
            other => {
                println!("Unknown DialogControl Type: {}", other);
                DialogControlType::Unknown(other.to_string())
            }
        };
        let name = node.attribute("Name").map(|s| s.to_string());

        // Report unexpected attributes
        for attr in node.attributes() {
            let key = attr.name();
            if key != "Type" && key != "Name" {
                println!(
                    "Unknown attribute in DialogControl(Name='{}'): {}='{}'",
                    name.clone().unwrap_or_default(),
                    key,
                    attr.value()
                );
            }
        }

        let mut prompt: Option<String> = None;
        let mut children: Vec<DialogControl> = Vec::new();

        for child in node.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "Prompt" => prompt = child.text().map(|s| s.to_string()),
                "DialogControl" => children.push(self.parse_dialog_control(child)),
                other => println!("Unknown tag in DialogControl(Name='{}'): {}", name.clone().unwrap_or_default(), other),
            }
        }

        DialogControl { control_type, name, prompt, children }
    }
}

fn matches_ignore_case(a: &str, b: &str) -> bool { a.eq_ignore_ascii_case(b) }

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

// Removed: resolving charts from system refs; charts are discovered via directory listing only.

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
            for p in paths {
                if let Some(fname) = p.file_name() {
                    if fname.starts_with("chart_") && fname.ends_with(".xml") {
                        if let Ok(chart) = SimulinkParser::parse_chart_file(self, &p) {
                            if let Some(id) = chart.id {
                                let ch = self.charts_by_id.entry(id).or_insert(chart);
                                if let Some(nm) = ch.name.clone() {
                                    self.system_to_chart_map.entry(nm).or_insert(id);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn get_charts(&self) -> &BTreeMap<u32, Chart> { &self.charts_by_id }
    pub fn get_system_to_chart_map(&self) -> &BTreeMap<String, u32> { &self.system_to_chart_map }
    pub fn get_chart(&self, id: u32) -> Option<&Chart> { self.charts_by_id.get(&id) }
    pub fn get_sid_to_chart_map(&self) -> &BTreeMap<String, u32> { &self.sid_to_chart_id }
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
    let sid: String = sid_str.trim().to_string();
    // rest like "out:1" or "in:2"
    let (ptype, pidx_str) = rest
        .split_once(':')
        .ok_or_else(|| anyhow!("Invalid endpoint port format: {}", s))?;
    let port_type = ptype.trim().to_string();
    let port_index: u32 = pidx_str.trim().parse()?;
    Ok(EndpointRef { sid, port_type, port_index })
}
