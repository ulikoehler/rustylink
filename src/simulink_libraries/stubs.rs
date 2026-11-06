//! Parser-facing stub metadata for Simulink virtual libraries.
//!
//! This module is the **core (non-`egui`) half** of the unified catalog.  The
//! rich rendering definitions in [`super::libraries`] require the `egui`
//! feature (they carry painter-based renderers), but the parser and
//! [`crate::block`] – which run without `egui` – still need to know a handful of
//! structural facts about virtual-library blocks:
//!
//! * how many ports a referenced library block should have, so a stub system
//!   can be synthesised when the real `.slx` library file is absent, and
//! * which `BlockType`s are dashboard / UI widgets (so they get the right
//!   default port counts).
//!
//! Keeping this data here – rather than in a separate top-level module – means
//! the whole catalog lives under `simulink_libraries/`, while the parser only
//! ever touches the lightweight, dependency-free part.

use crate::model::{Block, BlockChildKind, Port, PortCounts, System};

/// Property under which the name of a MATLAB Function block's function is
/// recorded while the model is loaded (it lives in the Stateflow chart, out of
/// reach of the renderers).
pub const MATLAB_FUNCTION_NAME_PROPERTY: &str = "MATLABFunctionName";

/// A structural description of a virtual-library block (ports only – icons,
/// shapes and labels live in the `egui` catalog definitions).
#[derive(Clone, Copy, Debug)]
pub struct StubBlock {
    /// Canonical block name as it appears in the library.
    pub name: &'static str,
    /// Alternate names (CamelCase / shorthand) that resolve to the same block.
    pub aliases: &'static [&'static str],
    /// Default input port count for a synthesised stub.
    pub ins: u32,
    /// Default output port count for a synthesised stub.
    pub outs: u32,
}

impl StubBlock {
    const fn new(
        name: &'static str,
        aliases: &'static [&'static str],
        ins: u32,
        outs: u32,
    ) -> Self {
        Self {
            name,
            aliases,
            ins,
            outs,
        }
    }
}

/// A virtual library: a name matcher plus the blocks it exposes.
#[derive(Clone, Copy)]
pub struct StubLibrary {
    /// Returns `true` when a SourceBlock/library reference belongs here.
    pub matches_name: fn(&str) -> bool,
    /// The blocks this library provides.
    pub blocks: &'static [StubBlock],
}

// ── Per-library block tables ─────────────────────────────────────────────────

/// `matrix_library` blocks (ports must match the `egui` catalog in
/// `libraries/matrix.rs`).
pub const MATRIX_BLOCKS: &[StubBlock] = &[
    StubBlock::new("Identity Matrix", &["IdentityMatrix"], 0, 1),
    StubBlock::new("Is Triangular", &["IsTriangular"], 1, 1),
    StubBlock::new("Is Symmetric", &["IsSymmetric"], 1, 1),
    StubBlock::new("Cross Product", &[], 2, 1),
    StubBlock::new("Matrix Multiply", &[], 2, 1),
    StubBlock::new("Submatrix", &[], 1, 1),
    StubBlock::new("Transpose", &[], 1, 1),
    StubBlock::new("Hermitian Transpose", &[], 1, 1),
    StubBlock::new("Matrix Square", &["Square"], 1, 1),
    StubBlock::new(
        "Permute Matrix",
        &["Permute Columns", "PermuteMatrix", "PermuteColumns"],
        2,
        1,
    ),
    StubBlock::new("Extract Diagonal", &["ExtractDiag"], 1, 1),
    StubBlock::new("Create Diagonal Matrix", &["DiagonalMatrix"], 1, 1),
    StubBlock::new("Expand Scalar", &["ExpandScalar"], 1, 1),
    StubBlock::new("Is Hermitian", &["IsHermitian"], 1, 1),
    StubBlock::new("Matrix Concatenate", &[], 2, 1),
];

const DISCRETE_BLOCKS: &[StubBlock] = &[StubBlock::new("Discrete Derivative", &[], 1, 1)];

const LOGIC_BLOCKS: &[StubBlock] = &[
    StubBlock::new("Compare To Constant", &["CompareToConstant"], 1, 1),
    StubBlock::new("Detect Change", &["DetectChange"], 1, 1),
    StubBlock::new("Detect Increase", &["DetectIncrease"], 1, 1),
    StubBlock::new("Detect Decrease", &["DetectDecrease"], 1, 1),
    StubBlock::new("Relational Operator", &["RelationalOperator"], 2, 1),
];

const MATH_BLOCKS: &[StubBlock] = &[
    StubBlock::new("Gain", &[], 1, 1),
    StubBlock::new("Sum", &[], 2, 1),
];

const SIGNAL_ROUTING_BLOCKS: &[StubBlock] = &[
    StubBlock::new("BusCreator", &[], 2, 1),
    StubBlock::new("BusSelector", &[], 1, 2),
    StubBlock::new("Goto", &[], 1, 0),
    StubBlock::new("From", &[], 0, 1),
];

/// Dashboard / UI widgets.  Most have no ports; `Display` has one input.
const DASHBOARD_BLOCKS: &[StubBlock] = &[
    StubBlock::new("Checkbox", &["CheckboxBlock"], 0, 0),
    StubBlock::new("ComboBox", &["ComboBoxBlock"], 0, 0),
    StubBlock::new("EditField", &["EditFieldBlock"], 0, 0),
    StubBlock::new("KnobBlock", &["Knob"], 0, 0),
    StubBlock::new("PushButtonBlock", &["PushButton"], 0, 0),
    StubBlock::new("RadioButtonGroup", &["RadioButtonGroupBlock"], 0, 0),
    StubBlock::new("RockerSwitchBlock", &["RockerSwitch"], 0, 0),
    StubBlock::new("RotarySwitchBlock", &["RotarySwitch"], 0, 0),
    StubBlock::new("SliderBlock", &["Slider"], 0, 0),
    StubBlock::new("SliderSwitchBlock", &["SliderSwitch"], 0, 0),
    StubBlock::new("ToggleSwitchBlock", &["ToggleSwitch"], 0, 0),
    StubBlock::new("Display", &["DisplaySink"], 1, 0),
    StubBlock::new("DisplayBlock", &["DashboardDisplay"], 0, 0),
    StubBlock::new("LampBlock", &["Lamp"], 0, 0),
    StubBlock::new("CircularGaugeBlock", &["CircularGauge"], 0, 0),
    StubBlock::new(
        "SemiCircularGaugeBlock",
        &["SemiCircularGauge", "HalfGauge"],
        0,
        0,
    ),
    StubBlock::new("LinearGaugeBlock", &["LinearGauge"], 0, 0),
    StubBlock::new("QuarterGaugeBlock", &["QuarterGauge"], 0, 0),
    StubBlock::new("DashboardScope", &["DashboardScopeBlock"], 0, 0),
    StubBlock::new("Playback", &[], 0, 1),
    StubBlock::new("RealTimeDashboard", &[], 1, 0),
    StubBlock::new("Stopwatch", &[], 1, 0),
    StubBlock::new("TextArea", &["TextAreaBlock"], 1, 0),
];

/// All built-in virtual libraries keyed by their name matcher.
pub const STUB_LIBRARIES: &[StubLibrary] = &[
    StubLibrary {
        matches_name: is_matrix_library_name,
        blocks: MATRIX_BLOCKS,
    },
    StubLibrary {
        matches_name: is_simulink_discrete_name,
        blocks: DISCRETE_BLOCKS,
    },
    StubLibrary {
        matches_name: is_simulink_logic_and_bit_ops_name,
        blocks: LOGIC_BLOCKS,
    },
    StubLibrary {
        matches_name: is_simulink_math_operations_name,
        blocks: MATH_BLOCKS,
    },
    StubLibrary {
        matches_name: is_simulink_signal_routing_name,
        blocks: SIGNAL_ROUTING_BLOCKS,
    },
    StubLibrary {
        matches_name: is_simulink_dashboard_name,
        blocks: DASHBOARD_BLOCKS,
    },
];

// ── Library name matchers ────────────────────────────────────────────────────

/// Determine if `name` refers to the matrix virtual library.
///
/// Accepts `matrix_library`, `matrix_library.slx` and any path within them,
/// case-insensitively.
pub fn is_matrix_library_name(name: &str) -> bool {
    let norm = name.trim().replace('\\', "/").to_ascii_lowercase();
    norm == "matrix_library"
        || norm == "matrix_library.slx"
        || norm.starts_with("matrix_library/")
        || norm.starts_with("matrix_library.slx/")
}

fn is_simulink_discrete_name(name: &str) -> bool {
    let norm = name.trim().replace('\\', "/").to_ascii_lowercase();
    norm == "simulink/discrete" || norm.starts_with("simulink/discrete/")
}

fn is_simulink_logic_and_bit_ops_name(name: &str) -> bool {
    let norm = name.trim().replace('\\', "/").to_ascii_lowercase();
    norm == "simulink/logic and bit operations"
        || norm.starts_with("simulink/logic and bit operations/")
        || norm == "simulink/logic and bit"
        || norm.starts_with("simulink/logic and bit/")
}

fn is_simulink_math_operations_name(name: &str) -> bool {
    let norm = name.trim().replace('\\', "/").to_ascii_lowercase();
    norm == "simulink/math operations" || norm.starts_with("simulink/math operations/")
}

fn is_simulink_signal_routing_name(name: &str) -> bool {
    let norm = name.trim().replace('\\', "/").to_ascii_lowercase();
    norm == "simulink/signal routing" || norm.starts_with("simulink/signal routing/")
}

/// Determine if `name` refers to the Simulink Dashboard virtual library.
pub fn is_simulink_dashboard_name(name: &str) -> bool {
    let norm = name.trim().replace('\\', "/").to_ascii_lowercase();
    norm == "simulink/dashboard" || norm.starts_with("simulink/dashboard/")
}

// ── Name normalisation helpers ───────────────────────────────────────────────

/// Normalise a library block name for matching: collapse whitespace runs to a
/// single ASCII space and lowercase.
pub fn normalize_block_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

/// Convert a CamelCase identifier to a spaced form (e.g. `MatrixMultiply` →
/// `Matrix Multiply`).  Intentionally simplistic.
pub fn humanize_camel_case(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            let prev = name.chars().nth(i - 1).unwrap();
            if !prev.is_uppercase() {
                out.push(' ');
            }
        }
        out.push(ch);
    }
    out
}

// ── Stub construction ────────────────────────────────────────────────────────

/// Construct a minimal [`Block`] stub with `ins`/`outs` ports, suitable for
/// rendering and library resolution.
pub fn create_stub_block(name: &str, ins: u32, outs: u32) -> Block {
    let mut ports = Vec::new();
    for i in 1..=ins {
        let mut p = Port {
            port_type: "in".to_string(),
            index: Some(i),
            properties: indexmap::IndexMap::new(),
        };
        p.properties.insert("Name".to_string(), String::new());
        ports.push(p);
    }
    for i in 1..=outs {
        let mut p = Port {
            port_type: "out".to_string(),
            index: Some(i),
            properties: indexmap::IndexMap::new(),
        };
        p.properties.insert("Name".to_string(), String::new());
        ports.push(p);
    }

    let port_counts = if ins > 0 || outs > 0 {
        Some(PortCounts {
            ins: Some(ins),
            outs: Some(outs),
            ..Default::default()
        })
    } else {
        None
    };

    let mut child_order = Vec::new();
    if port_counts.is_some() {
        child_order.push(BlockChildKind::PortCounts);
    }
    child_order.push(BlockChildKind::P("BlockType".to_string()));
    if port_counts.is_some() {
        child_order.push(BlockChildKind::PortProperties);
    }

    Block {
        block_type: name.to_string(),
        name: name.to_string(),
        sid: None,
        tag_name: "Block".to_string(),
        position: None,
        zorder: None,
        commented: false,
        name_location: Default::default(),
        is_matlab_function: false,
        value: None,
        value_kind: Default::default(),
        value_rows: None,
        value_cols: None,
        properties: indexmap::IndexMap::new(),
        ref_properties: Default::default(),
        port_counts,
        ports,
        subsystem: None,
        system_ref: None,
        c_function: None,
        instance_data: None,
        link_data: None,
        mask: None,
        annotations: Vec::new(),
        background_color: None,
        show_name: None,
        font_size: None,
        font_weight: None,
        mask_display_text: None,
        current_setting: None,
        block_mirror: None,
        library_source: None,
        library_block_path: None,
        library_missing: false,
        dashboard_binding: None,
        child_order,
    }
}

/// Build the initial in-memory [`System`] for a list of stub blocks.
fn initial_system(blocks: &[StubBlock]) -> System {
    System {
        properties: indexmap::IndexMap::new(),
        blocks: blocks
            .iter()
            .map(|b| create_stub_block(b.name, b.ins, b.outs))
            .collect(),
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    }
}

/// Return an initial in-memory system for a virtual library carrying structured
/// metadata (ports/known blocks), or `None` if `lib_name` is not a known
/// structured virtual library.
pub fn virtual_library_initial_system(lib_name: &str) -> Option<System> {
    for lib in STUB_LIBRARIES {
        if (lib.matches_name)(lib_name) {
            return Some(initial_system(lib.blocks));
        }
    }
    None
}

// ── Matrix-library lookups (used directly by the core parser) ─────────────────

/// Port counts for a matrix-library block name, or `None` when unrecognised.
///
/// Matching collapses whitespace, lowercases, and also tries the humanised
/// (space-separated) form of CamelCase SLX names.
pub fn matrix_port_counts_if_known(name: &str) -> Option<(u32, u32)> {
    let norm = normalize_block_name(name);
    let norm_humanized = normalize_block_name(&humanize_camel_case(name));
    for b in MATRIX_BLOCKS {
        if normalize_block_name(b.name) == norm || normalize_block_name(b.name) == norm_humanized {
            return Some((b.ins, b.outs));
        }
        for &alias in b.aliases {
            if normalize_block_name(alias) == norm || normalize_block_name(alias) == norm_humanized
            {
                return Some((b.ins, b.outs));
            }
        }
    }
    None
}

/// Construct a stub block for a matrix-library block name, defaulting to
/// `(1, 1)` ports when the name is unrecognised.
pub fn create_matrix_stub(name: &str) -> Block {
    let (ins, outs) = matrix_port_counts_if_known(name).unwrap_or((1, 1));
    create_stub_block(name, ins, outs)
}

// ── Dashboard detection ──────────────────────────────────────────────────────

/// All canonical dashboard block type names recognised natively.
pub const DASHBOARD_BLOCK_TYPES: &[&str] = &[
    "Checkbox",
    "ComboBox",
    "EditField",
    "KnobBlock",
    "PushButtonBlock",
    "RadioButtonGroup",
    "RockerSwitchBlock",
    "RotarySwitchBlock",
    "SliderBlock",
    "SliderSwitchBlock",
    "ToggleSwitchBlock",
    "Display",
    "DisplayBlock",
    "LampBlock",
    "CircularGaugeBlock",
    "SemiCircularGaugeBlock",
    "LinearGaugeBlock",
    "QuarterGaugeBlock",
    "DashboardScope",
];

/// Return `true` when `block_type` is a known dashboard / UI widget type.
pub fn is_dashboard_block_type(block_type: &str) -> bool {
    DASHBOARD_BLOCKS.iter().any(|b| {
        b.name.eq_ignore_ascii_case(block_type)
            || b.aliases.iter().any(|a| a.eq_ignore_ascii_case(block_type))
    })
}
