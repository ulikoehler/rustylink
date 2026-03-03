use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ────────────────────────────────────────────────────────────────────────────
// SystemDoc – binary serialization wrapper
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDoc {
    pub system: System,
}

impl SystemDoc {
    /// Save the SystemDoc to a binary file with magic bytes and versioning.
    pub fn save_to_binary<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);
        std::io::Write::write_all(&mut writer, b"RUSTYLINK")?;
        std::io::Write::write_all(&mut writer, &1u32.to_le_bytes())?;
        bincode::serde::encode_into_std_write(self, &mut writer, bincode::config::standard())?;
        Ok(())
    }

    /// Load a SystemDoc from a binary file, checking magic bytes and version.
    pub fn load_from_binary<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut reader = std::io::BufReader::new(file);
        let mut magic = [0u8; 9];
        std::io::Read::read_exact(&mut reader, &mut magic)?;
        if &magic != b"RUSTYLINK" {
            anyhow::bail!("Invalid magic bytes: expected 'RUSTYLINK'");
        }
        let mut version_bytes = [0u8; 4];
        std::io::Read::read_exact(&mut reader, &mut version_bytes)?;
        let version = u32::from_le_bytes(version_bytes);
        if version != 1 {
            anyhow::bail!("Unsupported version: {}", version);
        }
        let doc: SystemDoc =
            bincode::serde::decode_from_std_read(&mut reader, bincode::config::standard())?;
        Ok(doc)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// System
// ────────────────────────────────────────────────────────────────────────────

/// A Simulink system containing blocks, lines, and annotations.
///
/// `properties` preserves the insertion order of `<P>` elements from the XML,
/// which is essential for round-trip regeneration of SLX files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct System {
    /// Ordered map of `<P Name="…">value</P>` properties.
    pub properties: IndexMap<String, String>,
    pub blocks: Vec<Block>,
    pub lines: Vec<Line>,
    /// Free-floating annotations inside this system.
    #[serde(default)]
    pub annotations: Vec<Annotation>,
    /// Optional Stateflow chart content.
    pub chart: Option<Chart>,
}

// ────────────────────────────────────────────────────────────────────────────
// Block
// ────────────────────────────────────────────────────────────────────────────

/// Identifies the kind of a child XML element inside a `<Block>` or
/// `<Reference>` element. Used by [`Block::child_order`] to preserve the
/// exact element ordering for round-trip XML generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlockChildKind {
    PortCounts,
    /// A `<P>` element (value is the `Name` attribute).
    P(String),
    InstanceData,
    PortProperties,
    Mask,
    System,
    LinkData,
    /// An `<Annotation>` element (value is the index in `Block::annotations`).
    Annotation(usize),
}

/// A Simulink block or reference.
///
/// The `properties` map preserves the original insertion order of `<P>` elements
/// and stores **all** `<P>` values (including Position, ZOrder, etc.) so that
/// system XML files can be exactly regenerated.
///
/// Properties that use the XML `Ref` attribute instead of text content
/// are tracked in `ref_properties`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block type (e.g. "Gain", "SubSystem", "Reference").
    #[serde(rename = "type")]
    pub block_type: String,
    pub name: String,
    pub sid: Option<String>,

    /// XML element tag name: `"Block"` or `"Reference"`.
    #[serde(default = "default_block_tag")]
    pub tag_name: String,

    /// Convenience: parsed Position string (also stored in `properties`).
    pub position: Option<String>,
    /// Convenience: parsed ZOrder string (also stored in `properties`).
    pub zorder: Option<String>,
    pub commented: bool,
    /// Location of the block name label (defaults to Bottom if not specified).
    #[serde(default)]
    pub name_location: NameLocation,
    /// True if this block is a Stateflow MATLAB Function block.
    #[serde(default)]
    pub is_matlab_function: bool,
    /// Optional block value as text (e.g., for Constant blocks).
    #[serde(default)]
    pub value: Option<String>,
    /// Parsed value kind (scalar/vector/matrix).
    #[serde(default)]
    pub value_kind: ValueKind,
    #[serde(default)]
    pub value_rows: Option<u32>,
    #[serde(default)]
    pub value_cols: Option<u32>,

    /// Ordered map of all `<P>` element key-value pairs, including Position
    /// and ZOrder in their original order.
    pub properties: IndexMap<String, String>,

    /// Names of properties whose XML value is stored in a `Ref` attribute
    /// rather than as text content (e.g., `LibrarySourceProduct`).
    #[serde(default)]
    pub ref_properties: std::collections::BTreeSet<String>,

    /// PortCounts element (`<PortCounts in="…" out="…"/>`).
    /// `None` means no `<PortCounts>` element in the XML.
    #[serde(default)]
    pub port_counts: Option<PortCounts>,

    pub ports: Vec<Port>,
    /// Resolved nested system (subsystem content).
    pub subsystem: Option<Box<System>>,

    /// If the `<System>` child used a `Ref` attribute (e.g., `Ref="system_18"`),
    /// this field stores that reference name for round-trip output.
    #[serde(default)]
    pub system_ref: Option<String>,

    /// Present when this is a CFunction block.
    #[serde(default)]
    pub c_function: Option<CFunctionCode>,
    /// Optional per-instance data.
    #[serde(default)]
    pub instance_data: Option<InstanceData>,
    /// Optional link data (preserves pass-through dialog parameters).
    #[serde(default)]
    pub link_data: Option<LinkData>,
    /// Optional Simulink mask.
    #[serde(default)]
    pub mask: Option<Mask>,
    /// Annotations attached to the block.
    #[serde(default)]
    pub annotations: Vec<Annotation>,
    /// Convenience: parsed background color.
    #[serde(default)]
    pub background_color: Option<String>,
    /// Convenience: parsed show-name flag.
    #[serde(default)]
    pub show_name: Option<bool>,
    /// Convenience: parsed font size.
    #[serde(default)]
    pub font_size: Option<u32>,
    /// Convenience: parsed font weight.
    #[serde(default)]
    pub font_weight: Option<String>,
    /// Evaluated display text from mask's Display script.
    #[serde(default)]
    pub mask_display_text: Option<String>,
    /// Optional current setting for blocks like ManualSwitch.
    #[serde(default)]
    pub current_setting: Option<String>,
    /// Whether the block is mirrored.
    #[serde(default)]
    pub block_mirror: Option<bool>,
    /// Library source name this block was copied from.
    #[serde(default)]
    pub library_source: Option<String>,
    /// Full library block path.
    #[serde(default)]
    pub library_block_path: Option<String>,
    /// Parsed dashboard binding from a `BindingPersistence` `.mxarray` file.
    ///
    /// Present only for Dashboard / HMI blocks that carry a `BindingPersistence`
    /// property in the SLX archive.
    #[serde(default)]
    pub dashboard_binding: Option<DashboardBinding>,

    /// Order of child XML elements inside this block, used for round-trip
    /// XML generation. When empty, a default order is used.
    #[serde(default)]
    pub child_order: Vec<BlockChildKind>,
}

fn default_block_tag() -> String {
    "Block".to_string()
}

impl Block {
    /// Returns the full path to this block as `<subsystem>/<block name>`.
    pub fn get_full_path(&self, root: &System) -> Option<String> {
        let mut result: Option<String> = None;
        let mut path = Vec::new();
        root.walk_blocks(&mut path, &mut |p, b| {
            if std::ptr::eq(b, self) {
                let mut full = p.join("/");
                if !full.is_empty() {
                    full.push('/');
                }
                full.push_str(&self.name);
                result = Some(full);
            }
        });
        result
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Supporting types
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NameLocation {
    Top,
    Bottom,
    Left,
    Right,
}

impl Default for NameLocation {
    fn default() -> Self {
        NameLocation::Bottom
    }
}

/// Represents the `<PortCounts in="…" out="…"/>` XML element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortCounts {
    pub ins: Option<u32>,
    pub outs: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub port_type: String,
    pub index: Option<u32>,
    pub properties: IndexMap<String, String>,
}

/// A signal line connecting blocks.
///
/// `properties` stores all raw `<P>` elements in their original order for
/// round-trip fidelity. The typed fields (`name`, `zorder`, etc.) are derived
/// convenience accessors populated during parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub name: Option<String>,
    pub zorder: Option<String>,
    pub src: Option<EndpointRef>,
    pub dst: Option<EndpointRef>,
    pub points: Vec<Point>,
    pub labels: Option<String>,
    pub branches: Vec<Branch>,
    /// Ordered map of raw `<P>` key-value pairs for round-trip XML generation.
    #[serde(default)]
    pub properties: IndexMap<String, String>,
}

/// A branch of a signal line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: Option<String>,
    pub zorder: Option<String>,
    pub dst: Option<EndpointRef>,
    pub points: Vec<Point>,
    pub labels: Option<String>,
    pub branches: Vec<Branch>,
    /// Ordered map of raw `<P>` key-value pairs for round-trip XML generation.
    #[serde(default)]
    pub properties: IndexMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRef {
    pub sid: String,
    pub port_type: String,
    pub port_index: u32,
}

// ────────────────────────────────────────────────────────────────────────────
// Stateflow Chart
// ────────────────────────────────────────────────────────────────────────────

/// Minimal representation of a Stateflow chart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chart {
    pub id: Option<u32>,
    pub name: Option<String>,
    pub eml_name: Option<String>,
    pub script: Option<String>,
    pub inputs: Vec<ChartPort>,
    pub outputs: Vec<ChartPort>,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartPort {
    pub name: String,
    pub size: Option<String>,
    pub method: Option<String>,
    pub primitive: Option<String>,
    pub is_signed: Option<bool>,
    pub word_length: Option<u32>,
    pub complexity: Option<String>,
    pub frame: Option<String>,
    pub data_type: Option<String>,
    pub unit: Option<String>,
}

// ────────────────────────────────────────────────────────────────────────────
// CFunction / Mask / InstanceData / Annotation
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CFunctionCode {
    pub output_code: Option<String>,
    pub start_code: Option<String>,
    pub terminate_code: Option<String>,
    pub codegen_output_code: Option<String>,
    pub codegen_start_code: Option<String>,
    pub codegen_terminate_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Mask {
    pub display: Option<String>,
    /// Attributes on the `<Display>` element (e.g., `RunInitForIconRedraw`).
    #[serde(default)]
    pub display_attrs: IndexMap<String, String>,
    pub description: Option<String>,
    pub initialization: Option<String>,
    pub help: Option<String>,
    pub parameters: Vec<MaskParameter>,
    pub dialog: Vec<DialogControl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum MaskParamType {
    Popup,
    Edit,
    Checkbox,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskParameter {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: MaskParamType,
    pub prompt: Option<String>,
    pub value: Option<String>,
    pub callback: Option<String>,
    pub tunable: Option<bool>,
    pub visible: Option<bool>,
    pub type_options: Vec<String>,
    /// All XML attributes in their original order, used for round-trip generation.
    /// Contains Name, Type, Tunable, Visible, ShowTooltip, etc.
    #[serde(default)]
    pub all_attrs: IndexMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum DialogControlType {
    Group,
    Text,
    Edit,
    CheckBox,
    Popup,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogControl {
    #[serde(rename = "type")]
    pub control_type: DialogControlType,
    pub name: Option<String>,
    pub prompt: Option<String>,
    #[serde(default)]
    pub control_options: Option<ControlOptions>,
    pub children: Vec<DialogControl>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControlOptions {
    pub prompt_location: Option<String>,
}

/// `<LinkData>` element containing dialog parameter overrides for reference blocks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LinkData {
    pub dialog_parameters: Vec<DialogParametersEntry>,
}

/// `<DialogParameters>` element with a `BlockName` attribute and P children.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogParametersEntry {
    pub block_name: String,
    pub properties: IndexMap<String, String>,
}

/// Key-value map from `<InstanceData><P …>…</P></InstanceData>`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstanceData {
    pub properties: IndexMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValueKind {
    Unknown,
    Scalar,
    Vector,
    Matrix,
}

impl Default for ValueKind {
    fn default() -> Self {
        ValueKind::Unknown
    }
}

/// Simulink annotation (text or HTML) with position.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Annotation {
    pub sid: Option<String>,
    pub text: Option<String>,
    pub position: Option<String>,
    pub zorder: Option<String>,
    pub interpreter: Option<String>,
    pub properties: IndexMap<String, String>,
}

// ────────────────────────────────────────────────────────────────────────────
// Dashboard binding (from BindingPersistence mxarray files)
// ────────────────────────────────────────────────────────────────────────────

/// Describes how a Simulink Dashboard / HMI block is bound to a model signal
/// or parameter.
///
/// Dashboard blocks do **not** use traditional signal lines. Instead they carry
/// a `BindingPersistence` property whose `Ref` attribute points to a binary
/// `.mxarray` file inside the SLX archive. This struct holds the information
/// extracted from that file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DashboardBinding {
    /// The dashboard block **writes** to a block parameter (input widget).
    ///
    /// Used by `Checkbox`, `ComboBox`, `PushButton`, `Slider`, `Knob`,
    /// `ToggleSwitchBlock`, etc.
    ParamSource {
        /// Name (or path) of the target block whose parameter is written
        /// (e.g. `"CheckBox"`).
        block_path: String,
        /// Parameter name that is written (typically `"Value"`).
        param_name: String,
        /// Unique identifier for this binding.
        uuid: String,
    },
    /// The dashboard block **reads** a signal from another block (output widget).
    ///
    /// Used by `DashboardScope`, `DisplayBlock`, `CircularGaugeBlock`,
    /// `LampBlock`, etc.
    SignalSpec {
        /// Name (or path) of the source block producing the signal
        /// (e.g. `"Edit"`).
        block_path: String,
        /// Name of the signal (e.g. `"Edit_signal"`).
        signal_name: String,
        /// Unique identifier for this binding.
        uuid: String,
    },
}

/// Extract readable ASCII strings (length ≥ 3) from raw binary data.
fn extract_ascii_strings(data: &[u8], min_len: usize) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    let mut start = None;
    for (i, &b) in data.iter().enumerate() {
        if b >= 0x20 && b <= 0x7e {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start.take() {
            if i - s >= min_len {
                if let Ok(text) = std::str::from_utf8(&data[s..i]) {
                    results.push((s, text.to_string()));
                }
            }
        }
    }
    // Handle string at end of data
    if let Some(s) = start {
        let i = data.len();
        if i - s >= min_len {
            if let Ok(text) = std::str::from_utf8(&data[s..i]) {
                results.push((s, text.to_string()));
            }
        }
    }
    results
}

/// Field names that appear in the schema section of mxarray files and should
/// be excluded when looking for data values.
const MXARRAY_FIELD_NAMES: &[&str] = &[
    "MCOS",
    "FileWrapper__",
    "Simulink.HMI.ParamSourceInfo",
    "Simulink.HMI.SignalSpecification",
    "Simulink.HMI",
    "Simulink",
    "ParamSourceInfo",
    "SignalSpecification",
    "BlockPath_",
    "BlockPath",
    "path",
    "ssid",
    "sub_path",
    "ParamName_",
    "UUID",
    "Label_",
    "VarName_",
    "Element_",
    "ElementRawInput_",
    "WksType_",
    "SID_",
    "SignalName_",
    "SubPath_",
    "OutputPortIndex_",
    "LogicalPortIndex_",
    "SubSysPath_",
    "Decimation_",
    "MaxPoints_",
    "TargetBufferedStreaming_",
    "IsFrameBased_",
    "HideInSDI_",
    "DomainType_",
    "VisualType_",
    "DomainParams_",
];

/// Parse a raw `.mxarray` binary blob from a `BindingPersistence` entry into a
/// [`DashboardBinding`].
///
/// The function extracts readable ASCII strings from the binary data, identifies
/// the binding type (`ParamSourceInfo` or `SignalSpecification`), then pulls out
/// the data values (block path, parameter/signal name, UUID).
///
/// Returns `None` if the data does not contain a recognised binding pattern.
pub fn parse_mxarray_binding(data: &[u8]) -> Option<DashboardBinding> {
    let strings = extract_ascii_strings(data, 3);

    // Determine binding type from class name.
    let is_param = strings
        .iter()
        .any(|(_, s)| s == "Simulink.HMI.ParamSourceInfo");
    let is_signal = strings
        .iter()
        .any(|(_, s)| s == "Simulink.HMI.SignalSpecification");

    if !is_param && !is_signal {
        return None;
    }

    // Collect data-value strings: those that are NOT known field names and
    // appear in the data region (offset > 900) of the first instance. The
    // field names repeat a second time further in the file; we stop before
    // that second copy by limiting offset.
    let field_set: std::collections::HashSet<&str> =
        MXARRAY_FIELD_NAMES.iter().copied().collect();

    let data_strings: Vec<&str> = strings
        .iter()
        .filter(|(offset, s)| *offset > 900 && *offset < 1800 && !field_set.contains(s.as_str()))
        .map(|(_, s)| s.as_str())
        .collect();

    if is_param {
        // ParamSourceInfo data layout: [block_path, sid, param_name, uuid]
        let block_path = data_strings.first()?.to_string();
        let param_name = data_strings.get(2).unwrap_or(&"Value").to_string();
        let uuid = data_strings.get(3).unwrap_or(&"").to_string();
        Some(DashboardBinding::ParamSource {
            block_path,
            param_name,
            uuid,
        })
    } else {
        // SignalSpecification data layout: [uuid, block_path, sid, signal_name]
        let uuid = data_strings.first()?.to_string();
        let block_path = data_strings.get(1)?.to_string();
        let signal_name = data_strings.get(3).unwrap_or(&"").to_string();
        Some(DashboardBinding::SignalSpec {
            block_path,
            signal_name,
            uuid,
        })
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Relationship (from blockdiagram.xml.rels)
// ────────────────────────────────────────────────────────────────────────────

/// A relationship entry parsed from an OPC-style `.rels` file such as
/// `simulink/_rels/blockdiagram.xml.rels`.
///
/// Each relationship maps an `Id` to a `Target` path within the archive.
/// The `relationship_type` URI classifies the kind of linked resource (e.g.,
/// `modelMxArray`, `system`, `graphicalInterface`, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Identifier for this relationship (e.g. `"BindingPersistence_151"`).
    pub id: String,
    /// Target path relative to the containing directory (e.g.
    /// `"bdmxdata/BindingPersistence_151.mxarray"`).
    pub target: String,
    /// Full relationship type URI.
    pub relationship_type: String,
}

/// Parse an OPC-style `_rels/*.rels` XML string into a list of
/// [`Relationship`] entries.
///
/// The XML uses the namespace
/// `http://schemas.openxmlformats.org/package/2006/relationships` with
/// `<Relationship Id="…" Target="…" Type="…"/>` children.
pub fn parse_rels_xml(xml: &str) -> Vec<Relationship> {
    let mut rels = Vec::new();
    // Use roxmltree for namespace-aware parsing.
    if let Ok(doc) = roxmltree::Document::parse(xml) {
        for node in doc.descendants() {
            if node.is_element() && node.tag_name().name() == "Relationship" {
                let id = node.attribute("Id").unwrap_or("").to_string();
                let target = node.attribute("Target").unwrap_or("").to_string();
                let rel_type = node.attribute("Type").unwrap_or("").to_string();
                if !id.is_empty() {
                    rels.push(Relationship {
                        id,
                        target,
                        relationship_type: rel_type,
                    });
                }
            }
        }
    }
    rels
}

// ────────────────────────────────────────────────────────────────────────────
// System walk helpers
// ────────────────────────────────────────────────────────────────────────────

impl System {
    /// Walk all blocks recursively, calling `cb` for every block.
    pub fn walk_blocks<F>(&self, path: &mut Vec<String>, cb: &mut F)
    where
        F: FnMut(&[String], &Block),
    {
        for blk in &self.blocks {
            cb(path, blk);
            if let Some(sub) = &blk.subsystem {
                path.push(blk.name.clone());
                sub.walk_blocks(path, cb);
                path.pop();
            }
        }
    }

    /// Find all blocks of a given type, returning `(path, Block)` pairs.
    pub fn find_blocks_by_type(&self, block_type: &str) -> Vec<(Vec<String>, Block)> {
        let mut result = Vec::new();
        let mut path = Vec::new();
        self.walk_blocks(&mut path, &mut |p, b| {
            if b.block_type == block_type {
                result.push((p.to_vec(), b.clone()));
            }
        });
        result
    }
}

// ────────────────────────────────────────────────────────────────────────────
// SLX Archive – round-trip read/write of complete .slx files
// ────────────────────────────────────────────────────────────────────────────

/// Represents a complete SLX (`.slx`) archive for round-trip I/O.
///
/// An SLX file is a ZIP archive containing XML system files, stateflow charts,
/// metadata, and binary data. This struct preserves all entries so that the
/// archive can be regenerated exactly.
///
/// System XML files (`simulink/systems/system_*.xml`) are parsed into [`System`]
/// models and regenerated from them during write. All other files are preserved
/// as raw bytes.
#[derive(Debug, Clone)]
pub struct SlxArchive {
    /// All entries in the archive, in their original ZIP order.
    pub entries: Vec<SlxArchiveEntry>,
    /// Parsed relationships from `simulink/_rels/blockdiagram.xml.rels`.
    ///
    /// Keys are the `Id` attribute values (e.g. `"BindingPersistence_151"`),
    /// values are [`Relationship`] structs holding the target path and type URI.
    pub relationships: std::collections::BTreeMap<String, Relationship>,
}

/// A single entry in an SLX ZIP archive.
#[derive(Debug, Clone)]
pub struct SlxArchiveEntry {
    /// Path within the ZIP (e.g., `"simulink/systems/system_root.xml"`).
    pub path: String,
    /// Content of this entry.
    pub content: SlxContent,
    /// Whether this entry was stored compressed (deflated) in the original ZIP.
    pub compressed: bool,
}

/// Content of an SLX archive entry.
#[derive(Debug, Clone)]
pub enum SlxContent {
    /// Raw bytes for files that are preserved verbatim.
    Raw(Vec<u8>),
    /// A parsed system XML file that will be regenerated from the [`System`] model.
    SystemXml(System),
}
