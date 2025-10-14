use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDoc {
    pub system: System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct System {
    pub properties: BTreeMap<String, String>,
    pub blocks: Vec<Block>,
    pub lines: Vec<Line>,
    /// Free-floating annotations inside this system
    #[serde(default)]
    pub annotations: Vec<Annotation>,
    /// Optional Stateflow chart content when a system reference resolves to a chart (chart_XX.xml)
    pub chart: Option<Chart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    #[serde(rename = "type")]
    pub block_type: String,
    pub name: String,
    pub sid: Option<String>,
    pub position: Option<String>,
    pub zorder: Option<String>,
    pub commented: bool,
    /// True if this block is a Stateflow MATLAB Function block (SFBlockType == "MATLAB Function")
    #[serde(default)]
    pub is_matlab_function: bool,
    pub properties: BTreeMap<String, String>,
    pub ports: Vec<Port>,
    pub subsystem: Option<Box<System>>, // resolved nested system if present
    /// Present when this is a CFunction block
    #[serde(default)]
    pub c_function: Option<CFunctionCode>,
    /// Optional per-instance data as a simple key-value map
    #[serde(default)]
    pub instance_data: Option<InstanceData>,
    /// Optional Simulink mask associated with this block
    #[serde(default)]
    pub mask: Option<Mask>,
    /// Optional annotations attached to the block
    #[serde(default)]
    pub annotations: Vec<Annotation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortCounts {
    pub ins: Option<u32>,
    pub outs: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub port_type: String, // in/out
    pub index: Option<u32>,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub name: Option<String>,
    pub zorder: Option<String>,
    pub src: Option<EndpointRef>,
    pub dst: Option<EndpointRef>,
    pub points: Vec<Point>,
    pub labels: Option<String>,
    pub branches: Vec<Branch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: Option<String>,
    pub zorder: Option<String>,
    pub dst: Option<EndpointRef>,
    pub points: Vec<Point>,
    pub labels: Option<String>,
    pub branches: Vec<Branch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRef {
    pub sid: String,
    pub port_type: String, // "in" | "out"
    pub port_index: u32,
}

/// Minimal representation of a Stateflow chart needed for current use cases
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
    /// Raw size/dimensions as found in <array><P Name="size">...</P></array>
    pub size: Option<String>,
    /// Type information as found in <type>...</type>
    pub method: Option<String>,
    pub primitive: Option<String>,
    pub is_signed: Option<bool>,
    pub word_length: Option<u32>,
    pub complexity: Option<String>,
    pub frame: Option<String>,
    /// Optional display string such as "Inherit: Same as Simulink"
    pub data_type: Option<String>,
    pub unit: Option<String>,
}

/// Structured data for a CFunction block's code snippets
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CFunctionCode {
    pub output_code: Option<String>,
    pub start_code: Option<String>,
    pub terminate_code: Option<String>,
    pub codegen_output_code: Option<String>,
    pub codegen_start_code: Option<String>,
    pub codegen_terminate_code: Option<String>,
}

/// Simulink block mask definition
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Mask {
    pub display: Option<String>,
    pub description: Option<String>,
    pub initialization: Option<String>,
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
    pub tunable: Option<bool>,
    pub visible: Option<bool>,
    /// Only for popup types; raw options text
    pub type_options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum DialogControlType {
    Group,
    Text,
    Edit,
    CheckBox,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogControl {
    #[serde(rename = "type")]
    pub control_type: DialogControlType,
    pub name: Option<String>,
    pub prompt: Option<String>,
    pub children: Vec<DialogControl>,
}

/// InstanceData is a simple key-value map extracted from <InstanceData><P Name="...">...</P></InstanceData>
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstanceData {
    pub properties: BTreeMap<String, String>,
}

/// Simulink annotation (text or HTML) with position
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Annotation {
    /// Numeric SID when available
    pub sid: Option<String>,
    /// The raw text content of the annotation (may contain HTML entities)
    pub text: Option<String>,
    /// The position rectangle in Simulink coordinates, like "[l, t, r, b]"
    pub position: Option<String>,
    /// Z-Order if specified
    pub zorder: Option<String>,
    /// Interpreter (e.g., "rich" for HTML), if provided
    pub interpreter: Option<String>,
    /// Raw properties map for any other attributes
    pub properties: BTreeMap<String, String>,
}

impl System {
    /// Walk all blocks in this system recursively, calling `cb` for every block.
    ///
    /// The callback receives the path of subsystem names from the root to the
    /// containing subsystem (not including the block name) and a reference to
    /// the block itself. The path is returned as a slice of Strings where each
    /// element is the name of the subsystem block that introduced that level.
    pub fn walk_blocks<F>(&self, path: &mut Vec<String>, cb: &mut F)
    where
        F: FnMut(&[String], &Block),
    {
        for blk in &self.blocks {
            cb(&path, blk);
            if let Some(sub) = &blk.subsystem {
                // descend into subsystem: push the block name as part of path
                path.push(blk.name.clone());
                sub.walk_blocks(path, cb);
                path.pop();
            }
        }
    }

    /// Find all blocks that have `block_type` (case sensitive) and return a
    /// vector of (path, Block) pairs where `path` is the vector of subsystem
    /// names from root down to the containing subsystem.
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
