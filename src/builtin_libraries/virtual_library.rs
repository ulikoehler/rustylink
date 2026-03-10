//! Shared helpers for built-in virtual libraries.
//!
//! Virtual libraries are in-memory, structured representations of Simulink-like
//! libraries that rustylink can use when the actual `.slx` library file is not
//! present on disk.

use std::sync::Arc;

use once_cell::sync::OnceCell;
use std::sync::RwLock;

use crate::model::{Block, Port, PortCounts, System};

/// Rendering shape for a block body.
///
/// Determines how the background fill and border stroke of a block are drawn
/// in the egui viewer.  The default is [`BlockShape::Rectangle`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum BlockShape {
    /// Standard rectangular block (default).
    #[default]
    Rectangle,
    /// Right-pointing triangle (e.g. Gain blocks).
    Triangle,
    /// Circle (e.g. Sum blocks).
    Circle,
    /// Solid black rectangle with no interior label (e.g. Bus Creator/Selector).
    FilledBlack,
}

/// Side of a block where a port can be placed.
///
/// Used by [`PortPositionOverride`] to specify non-standard port locations,
/// e.g. a circular block with one input on the left and another on the top.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortPlacement {
    Left,
    Right,
    Top,
    Bottom,
}

/// Override the default position and placement of a single port.
///
/// By default, input ports are evenly distributed along the left edge and
/// output ports along the right edge.  A `PortPositionOverride` allows a
/// library block definition to move an individual port to any side of the
/// block and to place it at an arbitrary position along that side.
///
/// # Fields
///
/// * `is_input` – `true` for an input port, `false` for an output port.
/// * `port_index` – 1-based port index.
/// * `placement` – which side of the block the port sits on.
/// * `fraction` – position along that side as a fraction in `[0.0, 1.0]`.
///   For `Left`/`Right` sides, `0.0` is the top and `1.0` is the bottom.
///   For `Top`/`Bottom` sides, `0.0` is the left and `1.0` is the right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortPositionOverride {
    pub is_input: bool,
    pub port_index: u32,
    pub placement: PortPlacement,
    pub fraction: f32,
}

/// Description of a single block type that exists in a virtual library.
#[derive(Clone, Copy)]
pub struct VirtualBlock {
    /// Canonical name appearing in the library (case preserved).
    pub name: &'static str,
    /// Additional names that may appear in SLX files for the same block.
    ///
    /// This is used to bridge naming differences between Simulink versions,
    /// localized names, or shortened internal identifiers.
    pub aliases: &'static [&'static str],
    /// Number of input ports the block should have when rendered as a stub.
    pub ins: u32,
    /// Number of output ports the block should have when rendered as a stub.
    pub outs: u32,
    /// Rendering shape for this block's body in the egui viewer.
    pub shape: BlockShape,
    /// Optional icon to show for this block in the viewer. Paths are relative
    /// to the `icons/` folder embedded by `egui_app::icon_assets`.
    pub icon: Option<&'static str>,
    /// Optional per-instance label generator for this specific block type.
    ///
    /// Called with the full parsed `Block` (including `instance_data`);
    /// returns `Some(label)` when a label can be derived, or `None` to fall
    /// through to the default icon / value rendering.
    pub compute_instance_label: Option<fn(&Block) -> Option<String>>,
    /// Optional overrides for individual port positions and placement sides.
    ///
    /// When non-empty, these override the default evenly-distributed port
    /// layout for the specified ports.  Ports not listed here use the
    /// standard positioning.  Currently no built-in library block uses this
    /// facility, but it is available for custom virtual libraries.
    pub port_position_overrides: &'static [PortPositionOverride],
    /// Custom names for input ports (overrides "In1", "In2" etc.)
    pub input_port_names: &'static [&'static str],
    /// Custom names for output ports (overrides "Out1", "Out2" etc.)
    pub output_port_names: &'static [&'static str],
}

impl std::fmt::Debug for VirtualBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualBlock")
            .field("name", &self.name)
            .field("aliases", &self.aliases)
            .field("ins", &self.ins)
            .field("outs", &self.outs)
            .field("shape", &self.shape)
            .field("icon", &self.icon)
            .field(
                "compute_instance_label",
                &self.compute_instance_label.map(|_| "<fn>"),
            )
            .field("port_position_overrides", &self.port_position_overrides)
            .field("input_port_names", &self.input_port_names)
            .field("output_port_names", &self.output_port_names)
            .finish()
    }
}

/// Descriptor for a built-in virtual library.
///
/// This allows generic code (e.g. icon registry population, stub creation,
/// etc.) to iterate over all known virtual libraries without hard-coding
/// per-library details.
///
/// Per-instance label generation and icon selection are delegated to each
/// individual [`VirtualBlock`] entry via its `compute_instance_label` and
/// `icon` fields rather than being handled at the library level.
#[derive(Clone, Copy)]
pub struct VirtualLibrarySpec {
    /// Canonical library name as used in SourceBlock paths (e.g. "matrix_library").
    pub name: &'static str,
    /// Returns true if the provided library reference belongs to this library.
    pub matches_name: fn(&str) -> bool,
    /// Returns all virtual block definitions this library exposes.
    ///
    /// Typically this just returns a `&'static` slice constant, but the
    /// function-pointer form allows libraries to build the list lazily if
    /// needed.
    pub get_blocks: fn() -> &'static [VirtualBlock],
}

impl VirtualBlock {
    /// A default `VirtualBlock` value with all fields set to their zero/None values.
    ///
    /// Useful with struct-update syntax (`..VirtualBlock::DEFAULT`) in `const`
    /// block list definitions to avoid repeating `compute_instance_label: None`
    /// on every entry.
    pub const DEFAULT: VirtualBlock = VirtualBlock {
        name: "",
        aliases: &[],
        ins: 0,
        outs: 0,
        shape: BlockShape::Rectangle,
        icon: None,
        compute_instance_label: None,
        port_position_overrides: &[],
        input_port_names: &[],
        output_port_names: &[],
    };
}

/// Normalize a library block name for matching purposes.
///
/// All whitespace sequences are collapsed to a single ASCII space and the
/// result is lowercased. This keeps `foo   bar` equivalent to `foo bar`.
pub fn normalize_block_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

/// Convert a CamelCase identifier to a human-readable name by inserting spaces
/// before uppercase letters.
///
/// This is intentionally simplistic; it is used for producing alternative keys
/// like `Matrix Multiply` from `MatrixMultiply`.
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

/// Construct a minimal `Block` stub suitable for rendering.
///
/// The returned block has the provided `name` as both `block_type` and `name`
/// and a set of ports matching `ins`/`outs`. Other fields are left as defaults.
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
        })
    } else {
        None
    };

    let mut child_order = Vec::new();
    if port_counts.is_some() {
        child_order.push(crate::model::BlockChildKind::PortCounts);
    }
    child_order.push(crate::model::BlockChildKind::P("BlockType".to_string()));
    if port_counts.is_some() {
        child_order.push(crate::model::BlockChildKind::PortProperties);
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
        dashboard_binding: None,
        child_order,
    }
}

/// Build the initial `System` for a virtual library from a list of known blocks.
pub fn initial_system(blocks: &[VirtualBlock]) -> System {
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

// ── Dynamic (user-registered) virtual library API ────────────────────────────

/// Owned version of [`VirtualBlock`] for dynamic (runtime) virtual library
/// registration.
///
/// Unlike [`VirtualBlock`], all fields are owned `String`s and there is no
/// `'static` lifetime requirement.
pub struct OwnedVirtualBlock {
    /// Canonical name of the block.  Prefer title-case with spaces over
    /// CamelCase – e.g. `"My Block"` rather than `"MyBlock"`.
    pub name: String,
    /// Additional names recognised as aliases for this block type.
    pub aliases: Vec<String>,
    /// Number of input ports the block should have when rendered as a stub.
    pub ins: u32,
    /// Number of output ports the block should have when rendered as a stub.
    pub outs: u32,
    /// Optional per-instance label generator for this specific block type.
    ///
    /// Mirrors [`VirtualBlock::compute_instance_label`] for the dynamic
    /// (user-registered) library API.
    pub compute_instance_label: Option<Arc<dyn Fn(&Block) -> Option<String> + Send + Sync>>,
    /// Optional overrides for individual port positions and placement sides.
    ///
    /// See [`VirtualBlock::port_position_overrides`] for semantics.
    pub port_position_overrides: Vec<PortPositionOverride>,
    /// Custom names for input ports
    pub input_port_names: Vec<String>,
    /// Custom names for output ports
    pub output_port_names: Vec<String>,
}

impl std::fmt::Debug for OwnedVirtualBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedVirtualBlock")
            .field("name", &self.name)
            .field("aliases", &self.aliases)
            .field("ins", &self.ins)
            .field("outs", &self.outs)
            .field(
                "compute_instance_label",
                &self.compute_instance_label.as_ref().map(|_| "<fn>"),
            )
            .field("port_position_overrides", &self.port_position_overrides)
            .field("input_port_names", &self.input_port_names)
            .field("output_port_names", &self.output_port_names)
            .finish()
    }
}

impl Clone for OwnedVirtualBlock {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            aliases: self.aliases.clone(),
            ins: self.ins,
            outs: self.outs,
            compute_instance_label: self.compute_instance_label.clone(),
            port_position_overrides: self.port_position_overrides.clone(),
            input_port_names: self.input_port_names.clone(),
            output_port_names: self.output_port_names.clone(),
        }
    }
}

/// Dynamic (runtime) virtual library specification.
///
/// Unlike [`VirtualLibrarySpec`] (which carries `&'static` references and plain
/// function pointers), `UserVirtualLibrarySpec` is fully owned and uses
/// [`Arc`]-wrapped closures.  This makes it suitable for libraries registered at
/// runtime by downstream crates or applications.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use rustylink::builtin_libraries::{OwnedVirtualBlock, UserVirtualLibrarySpec,
///                                    register_virtual_library};
///
/// let spec = UserVirtualLibrarySpec {
///     name: "my_lib".to_string(),
///     blocks: vec![OwnedVirtualBlock {
///         name: "My Block".to_string(),
///         aliases: vec!["MyBlock".to_string()],
///         ins: 1,
///         outs: 1,
///         compute_instance_label: None,
///         port_position_overrides: vec![],
///     }],
///     matches_name: Arc::new(|name| {
///         name.to_ascii_lowercase().starts_with("my_lib")
///     }),
///     initial_system: Arc::new(|| {
///         rustylink::model::System {
///             properties: Default::default(),
///             blocks: vec![],
///             lines: vec![],
///             annotations: vec![],
///             chart: None,
///         }
///     }),
/// };
/// register_virtual_library(spec);
/// ```
pub struct UserVirtualLibrarySpec {
    /// Canonical name of this library (e.g. `"my_lib"`).
    pub name: String,
    /// All virtual blocks the library exposes.
    ///
    /// Per-block label generation is controlled by each entry's
    /// `compute_instance_label` field.
    pub blocks: Vec<OwnedVirtualBlock>,
    /// Returns `true` when the provided name (library path or source-block
    /// path) refers to this library.
    pub matches_name: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    /// Construct the initial virtual system for this library.
    pub initial_system: Arc<dyn Fn() -> System + Send + Sync>,
}

static USER_LIBRARIES: OnceCell<RwLock<Vec<UserVirtualLibrarySpec>>> = OnceCell::new();

fn user_libraries_lock() -> &'static RwLock<Vec<UserVirtualLibrarySpec>> {
    USER_LIBRARIES.get_or_init(|| RwLock::new(Vec::new()))
}

/// Register a user-defined virtual library.
///
/// After calling this, all rustylink dispatch functions – port-count lookup,
/// icon selection, instance-label generation, `virtual_library_initial_system`,
/// etc. – will recognise blocks from the new library.
///
/// When the `egui` feature is enabled, also call
/// [`rustylink::block_types::register_user_library_block_types`] to populate
/// the icon registry for the new library's blocks (a default placeholder icon
/// is used unless you later call [`rustylink::set_block_type_config`] to
/// override individual entries).
pub fn register_virtual_library(spec: UserVirtualLibrarySpec) {
    if let Ok(mut w) = user_libraries_lock().write() {
        w.push(spec);
    }
}

/// Find a value by searching all user-registered virtual libraries.
///
/// Calls `f` for each registered library and returns the first `Some` result.
/// Returns `None` if no library returns a value.
pub(crate) fn find_in_user_libraries<F, R>(f: F) -> Option<R>
where
    F: Fn(&UserVirtualLibrarySpec) -> Option<R>,
{
    let Ok(guard) = user_libraries_lock().read() else {
        return None;
    };
    for spec in guard.iter() {
        if let Some(r) = f(spec) {
            return Some(r);
        }
    }
    None
}
