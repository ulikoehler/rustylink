//! Centralized block-type-specific configuration and registry (egui feature).
//!
//! This module provides a global, mutable registry of block type configurations
//! that control visuals and labeling behavior in the egui viewer. Users can
//! read and modify this registry at runtime to customize the appearance of
//! specific Simulink block types.

#![cfg(feature = "egui")]

use std::collections::HashMap;
use std::sync::RwLock;

use once_cell::sync::OnceCell;

/// Simple RGB color independent of egui types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// Icon specification for a block type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconSpec {
    /// Use an egui-phosphor icon (single glyph string constant).
    Phosphor(&'static str),
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
}

impl Default for BlockTypeConfig {
    fn default() -> Self {
        Self {
            background: None,
            border: None,
            icon: None,
            show_input_port_labels: true,
            show_output_port_labels: true,
        }
    }
}

fn default_registry() -> HashMap<String, BlockTypeConfig> {
    use egui_phosphor::variants::regular;
    let mut m = HashMap::new();

    // Mirror the hardcoded icons previously used in egui_app.rs
    m.insert(
        "Product".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Phosphor(regular::X)),
            show_input_port_labels: false,
            show_output_port_labels: false,
            ..Default::default()
        },
    );
    m.insert(
        "Constant".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Phosphor(regular::WRENCH)),
            show_input_port_labels: false,
            show_output_port_labels: false,
            ..Default::default()
        },
    );
    m.insert(
        "Scope".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Phosphor(regular::WAVE_SINE)),
            show_input_port_labels: false,
            show_output_port_labels: false,
            ..Default::default()
        },
    );
    m.insert(
        "ManualSwitch".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Phosphor(regular::TOGGLE_LEFT)),
            ..Default::default()
        },
    );

    m
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
        let entry = w.entry(block_type.to_string()).or_insert_with(BlockTypeConfig::default);
        f(entry);
    }
}
