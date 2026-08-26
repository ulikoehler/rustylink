//! Centralized block-type-specific configuration and registry (egui feature).
//!
//! This module provides a global, mutable registry of block type configurations
//! that control visuals and labeling behavior in the egui viewer. Users can
//! read and modify this registry at runtime to customize the appearance of
//! specific Simulink block types.

use std::collections::HashMap;
use std::sync::RwLock;

pub use crate::simulink_libraries::types::SimulinkShape as BlockShape;
use once_cell::sync::OnceCell;

/// Simple RGB color independent of egui types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// Icon specification for a block type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconSpec {
    Utf8(&'static str),
    Phosphor(&'static str),
    /// Typeset math (fraction bar / superscript / overbar); see
    /// [`crate::egui_app::render::draw_math_icon`].
    Math(&'static str),
    /// Line-art drawn from a compact polyline notation; see
    /// [`crate::egui_app::render::draw_plot_icon`].  Simulink draws many block
    /// icons (waveforms, saturation curves, scope screens) as vector line art
    /// rather than glyphs.
    Plot(&'static str),
}

/// Configuration for a specific block type.
#[derive(Clone, Debug)]
pub struct BlockTypeConfig {
    /// Optional background color for the block body.
    /// Defaults to the viewer's gray: 210,210,210.
    pub background: Option<Rgb>,
    /// Optional border color for the block.
    /// Defaults to the viewer's current stroke color: 180,180,200.
    pub border: Option<Rgb>,
    /// Optional icon to render at the center of the block.
    pub icon: Option<IconSpec>,
    /// Whether to display input port labels inside the block. Default: true.
    pub show_input_port_labels: bool,
    /// Whether to display output port labels inside the block. Default: true.
    pub show_output_port_labels: bool,
    /// Rendering shape for this block's body.
    pub shape: BlockShape,
    /// Default number of input ports when `port_counts` is absent on the block.
    pub default_ins: u32,
    /// Default number of output ports when `port_counts` is absent on the block.
    pub default_outs: u32,
    /// `true` when this entry was explicitly registered (e.g. as a known
    /// virtual-library block).  Blocks with `known = true` but `icon = None`
    /// will silently render a `"?"` placeholder without emitting a terminal
    /// warning – they are recognised block types that just lack a dedicated icon.
    pub known: bool,
    /// Optional overrides for individual port positions and placement sides.
    ///
    /// When non-empty, these override the default evenly-distributed port
    /// layout for the specified ports.  Ports not listed here use the
    /// standard positioning.
    pub port_position_overrides: Vec<crate::simulink_libraries::types::PortPositionOverride>,
    /// Custom names for input ports
    pub input_port_names: Vec<String>,
    /// Custom names for output ports
    pub output_port_names: Vec<String>,
}

impl Default for BlockTypeConfig {
    fn default() -> Self {
        Self {
            background: None,
            border: None,
            icon: None,
            show_input_port_labels: true,
            show_output_port_labels: true,
            shape: BlockShape::Rectangle,
            default_ins: 0,
            default_outs: 0,
            known: false,
            port_position_overrides: Vec::new(),
            input_port_names: Vec::new(),
            output_port_names: Vec::new(),
        }
    }
}

/// Build the default registry by seeding it from the unified Simulink block
/// definition catalog (the single source of truth).  Every catalog definition
/// — hand-written core/dashboard libraries plus the bridged built-in virtual
/// libraries — contributes its `BlockTypeConfig` under all the key variants the
/// multi-phase lookup in `get_block_type_cfg` expects.
fn default_registry() -> HashMap<String, BlockTypeConfig> {
    crate::simulink_libraries::config::block_type_config_entries()
}

static REGISTRY: OnceCell<RwLock<HashMap<String, BlockTypeConfig>>> = OnceCell::new();

/// Get a handle to the global block type configuration map.
///
/// The returned [`RwLock`] guards a [`HashMap<String, BlockTypeConfig>`].
/// Callers may acquire a read lock to inspect existing configuration or a write
/// lock to add/modify entries at runtime.
pub fn get_block_type_config_map() -> &'static RwLock<HashMap<String, BlockTypeConfig>> {
    REGISTRY.get_or_init(|| RwLock::new(default_registry()))
}

/// Replace or insert a configuration for a block type.
pub fn set_block_type_config<T: Into<String>>(block_type: T, cfg: BlockTypeConfig) {
    let map = get_block_type_config_map();
    if let Ok(mut w) = map.write() {
        w.insert(block_type.into(), cfg);
    }
}

/// Update an existing configuration in-place, creating a default if missing.
pub fn update_block_type_config<F>(block_type: &str, f: F)
where
    F: FnOnce(&mut BlockTypeConfig),
{
    let map = get_block_type_config_map();
    if let Ok(mut w) = map.write() {
        let entry = w
            .entry(block_type.to_string())
            .or_insert_with(BlockTypeConfig::default);
        f(entry);
    }
}
