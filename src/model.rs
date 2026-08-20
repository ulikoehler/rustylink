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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum NameLocation {
    Top,
    #[default]
    Bottom,
    Left,
    Right,
}

/// Represents the `<PortCounts in="…" out="…" enable="…" trigger="…"
/// reset="…" event="…"/>` XML element.  The control ports (everything but
/// `in`/`out`) sit on the top edge of the block rather than on the input side.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PortCounts {
    pub ins: Option<u32>,
    pub outs: Option<u32>,
    #[serde(default)]
    pub enable: Option<u32>,
    #[serde(default)]
    pub trigger: Option<u32>,
    #[serde(default)]
    pub reset: Option<u32>,
    #[serde(default)]
    pub event: Option<u32>,
}

impl PortCounts {
    /// Number of control ports on the block's top edge.
    pub fn control_count(&self) -> u32 {
        self.enable.unwrap_or(0)
            + self.trigger.unwrap_or(0)
            + self.reset.unwrap_or(0)
            + self.event.unwrap_or(0)
    }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ValueKind {
    #[default]
    Unknown,
    Scalar,
    Vector,
    Matrix,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DashboardTargetPath {
    /// Optional output/logical port index encoded by Simulink for this target path.
    pub port_index: Option<u32>,
    /// Optional nested path or member selector stored by Simulink.
    pub sub_path: Option<String>,
    /// Optional element selector for parameter bindings.
    pub element: Option<String>,
    /// Optional raw array/vector selector string as stored by Simulink.
    pub element_raw_input: Option<String>,
}

impl DashboardTargetPath {
    pub fn is_empty(&self) -> bool {
        self.port_index.is_none()
            && self.sub_path.is_none()
            && self.element.is_none()
            && self.element_raw_input.is_none()
    }

    pub fn element_index_zero_based(&self) -> Option<usize> {
        if let Some(element) = self.element.as_deref()
            && let Ok(index) = element.trim().parse::<usize>()
        {
            return Some(index);
        }

        let raw = self.element_raw_input.as_deref()?.trim();
        let inner = raw
            .strip_prefix('(')
            .and_then(|value| value.strip_suffix(')'))
            .or_else(|| {
                raw.strip_prefix('[')
                    .and_then(|value| value.strip_suffix(']'))
            })?;
        inner.trim().parse::<usize>().ok()?.checked_sub(1)
    }
}

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
        /// Structured target-path metadata extracted from the mxarray payload.
        target_path: DashboardTargetPath,
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
        /// Structured target-path metadata extracted from the mxarray payload.
        target_path: DashboardTargetPath,
        /// Unique identifier for this binding.
        uuid: String,
    },
}

impl DashboardBinding {
    /// Returns the UUID of this binding, regardless of variant.
    pub fn uuid(&self) -> &str {
        match self {
            DashboardBinding::ParamSource { uuid, .. } => uuid,
            DashboardBinding::SignalSpec { uuid, .. } => uuid,
        }
    }
}

/// Extract readable ASCII strings (length ≥ 3) from raw binary data.
fn extract_ascii_strings(data: &[u8], min_len: usize) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    let mut start = None;
    for (i, &b) in data.iter().enumerate() {
        if (0x20..=0x7e).contains(&b) {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start.take()
            && i - s >= min_len
            && let Ok(text) = std::str::from_utf8(&data[s..i])
        {
            results.push((s, text.to_string()));
        }
    }
    // Handle string at end of data
    if let Some(s) = start {
        let i = data.len();
        if i - s >= min_len
            && let Ok(text) = std::str::from_utf8(&data[s..i])
        {
            results.push((s, text.to_string()));
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
    let raw_strings = extract_ascii_strings(data, 1);

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
    let field_set: std::collections::HashSet<&str> = MXARRAY_FIELD_NAMES.iter().copied().collect();

    let data_strings: Vec<(usize, &str)> = raw_strings
        .iter()
        .filter(|(offset, s)| *offset > 900 && *offset < 1800 && !field_set.contains(s.as_str()))
        .map(|(offset, s)| (*offset, s.as_str()))
        .collect();

    let meaningful_text_values = data_strings
        .iter()
        .copied()
        .filter(|(_, value)| is_meaningful_binding_text_value(value))
        .collect::<Vec<_>>();
    let selector_values = data_strings
        .iter()
        .copied()
        .filter(|(_, value)| is_meaningful_binding_selector_value(value))
        .collect::<Vec<_>>();

    let uuid = meaningful_text_values
        .iter()
        .find_map(|(_, value)| looks_like_uuid(value).then(|| (*value).to_string()))
        .unwrap_or_default();
    let named_text_values = meaningful_text_values
        .iter()
        .copied()
        .filter(|(_, value)| !looks_like_uuid(value))
        .collect::<Vec<_>>();
    let ascii_port_index =
        find_numeric_field_value(&raw_strings, &["OutputPortIndex_", "LogicalPortIndex_"]);

    let target_path = DashboardTargetPath {
        port_index: None,
        sub_path: named_text_values
            .get(2)
            .map(|(_, value)| (*value).to_string()),
        element: None,
        element_raw_input: None,
    };

    if is_param {
        let block_path = named_text_values
            .first()
            .map(|(_, value)| (*value).to_string())?;
        let param_name = named_text_values
            .get(1)
            .map(|(_, value)| (*value).to_string())
            .unwrap_or_else(|| "Value".to_string());
        let selector_start_offset = named_text_values
            .get(1)
            .map(|(offset, _)| *offset)
            .or_else(|| named_text_values.first().map(|(offset, _)| *offset))
            .unwrap_or(0);
        let selector_end_offset = meaningful_text_values
            .iter()
            .find_map(|(offset, value)| looks_like_uuid(value).then_some(*offset))
            .unwrap_or(usize::MAX);
        let selector_values = selector_values
            .iter()
            .copied()
            .filter(|(offset, _)| *offset > selector_start_offset && *offset < selector_end_offset)
            .collect::<Vec<_>>();
        let target_path = DashboardTargetPath {
            element: selector_values
                .iter()
                .find(|(_, value)| value.chars().all(|ch| ch.is_ascii_digit()))
                .map(|(_, value)| (*value).to_string()),
            element_raw_input: selector_values
                .iter()
                .find(|(_, value)| looks_like_selector_expression(value))
                .map(|(_, value)| (*value).to_string()),
            ..target_path
        };
        Some(DashboardBinding::ParamSource {
            block_path,
            param_name,
            target_path,
            uuid,
        })
    } else {
        let (block_path_offset, block_path_value) = named_text_values.first().copied()?;
        let block_path = block_path_value.to_string();
        let signal_name = named_text_values
            .get(1)
            .map(|(_, value)| (*value).to_string())
            .unwrap_or_default();
        let target_path = DashboardTargetPath {
            port_index: find_binary_signal_port_index(data, &raw_strings, block_path_offset)
                .or(ascii_port_index),
            ..target_path
        };
        Some(DashboardBinding::SignalSpec {
            block_path,
            signal_name,
            target_path,
            uuid,
        })
    }
}

fn is_meaningful_binding_text_value(value: &str) -> bool {
    looks_like_uuid(value) || (value.len() > 1 && value.chars().any(|ch| ch.is_ascii_alphabetic()))
}

fn is_meaningful_binding_selector_value(value: &str) -> bool {
    !looks_like_uuid(value)
        && !looks_like_sid_token(value)
        && (value.chars().all(|ch| ch.is_ascii_digit()) || looks_like_selector_expression(value))
}

fn looks_like_selector_expression(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .any(|ch| matches!(ch, '[' | ']' | '(' | ')' | '.' | ':' | '/' | ','))
}

fn find_numeric_field_value(strings: &[(usize, String)], field_names: &[&str]) -> Option<u32> {
    for field_name in field_names {
        for (idx, (field_offset, text)) in strings.iter().enumerate() {
            if text != field_name {
                continue;
            }

            for (value_offset, value) in strings.iter().skip(idx + 1) {
                let distance = value_offset.saturating_sub(*field_offset);
                if distance > 1024 {
                    break;
                }
                if value.chars().all(|ch| ch.is_ascii_digit())
                    && let Ok(parsed) = value.parse::<u32>()
                {
                    return Some(parsed);
                }
            }
        }
    }

    None
}

fn find_binary_signal_port_index(
    data: &[u8],
    strings: &[(usize, String)],
    block_path_offset: usize,
) -> Option<u32> {
    let repeated_field_offset = strings
        .iter()
        .filter(|(_, text)| text == "BlockPath_")
        .nth(1)
        .map(|(offset, _)| *offset)?;
    let sid_end_offset = strings
        .iter()
        .filter(|(offset, value)| {
            *offset > block_path_offset && *offset < repeated_field_offset && value.len() > 1
        })
        .find(|(_, value)| looks_like_sid_token(value))
        .map(|(offset, value)| *offset + value.len())?;

    let scalar_header = [0x09, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00];
    for offset in sid_end_offset..repeated_field_offset.saturating_sub(16) {
        if data.get(offset..offset + scalar_header.len()) != Some(scalar_header.as_slice()) {
            continue;
        }

        let value_bytes = data.get(offset + scalar_header.len()..offset + 16)?;
        let value = f64::from_le_bytes(value_bytes.try_into().ok()?);
        let rounded = value.round();
        if value.is_finite() && (1.0..=64.0).contains(&value) && (value - rounded).abs() <= 1e-9 {
            return Some(rounded as u32 - 1);
        }
    }

    None
}

fn looks_like_uuid(value: &str) -> bool {
    value.len() == 36
        && value.chars().enumerate().all(|(index, ch)| match index {
            8 | 13 | 18 | 23 => ch == '-',
            _ => ch.is_ascii_hexdigit(),
        })
}

fn looks_like_sid_token(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit())
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
#[allow(clippy::large_enum_variant)]
pub enum SlxContent {
    /// Raw bytes for files that are preserved verbatim.
    Raw(Vec<u8>),
    /// A parsed system XML file that will be regenerated from the [`System`] model.
    SystemXml(System),
}
