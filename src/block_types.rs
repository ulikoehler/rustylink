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
    Utf8(&'static str),
    Svg(&'static str),
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

/// Register icon/config entries for one virtual-block name into the map.
///
/// Generates all useful key variants:
/// - raw name and its whitespace-normalized lowercase form
/// - CamelCase-humanized name and its normalized form
/// - All of the above prefixed with `{lib_name}/`
///
/// Duplicate keys are silently skipped.
fn register_virtual_keys(
    m: &mut HashMap<String, BlockTypeConfig>,
    lib_name: &str,
    raw_name: &str,
    cfg: BlockTypeConfig,
) {
    use crate::builtin_libraries::virtual_library::{humanize_camel_case, normalize_block_name};
    let human = humanize_camel_case(raw_name);
    let norm_raw = normalize_block_name(raw_name);
    let norm_human = normalize_block_name(&human);
    let mut keys: Vec<String> = vec![
        raw_name.to_string(),
        norm_raw.clone(),
        human.clone(),
        norm_human.clone(),
        format!("{lib_name}/{raw_name}"),
        format!("{lib_name}/{norm_raw}"),
        format!("{lib_name}/{human}"),
        format!("{lib_name}/{norm_human}"),
    ];

    // De-duplicate while preserving insertion order.
    let mut seen = std::collections::HashSet::new();
    keys.retain(|k| seen.insert(k.clone()));

    for k in keys {
        m.insert(k, cfg.clone());
    }
}

fn default_registry() -> HashMap<String, BlockTypeConfig> {
    let mut m = HashMap::new();

    // Mirror the hardcoded icons previously used in egui_app.rs
    m.insert(
        "Product".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Utf8("×")),
            show_input_port_labels: false,
            show_output_port_labels: false,
            ..Default::default()
        },
    );
    m.insert(
        "Constant".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Utf8("C")),
            show_input_port_labels: false,
            show_output_port_labels: false,
            ..Default::default()
        },
    );
    m.insert(
        "Scope".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Utf8("〰")),
            show_input_port_labels: false,
            show_output_port_labels: false,
            ..Default::default()
        },
    );
    m.insert(
        "ManualSwitch".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Utf8("🕂")),
            show_input_port_labels: false,
            show_output_port_labels: false,
            ..Default::default()
        },
    );
    m.insert(
        "MATLAB Function".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Utf8("🖹")),
            ..Default::default()
        },
    );
    m.insert(
        "SubSystem".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Utf8("")), // This is a box icon but VScode doesnt render it
            ..Default::default()
        },
    );
    m.insert(
        "Inport".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Utf8("⬅")),
            show_input_port_labels: false,
            show_output_port_labels: false,
            ..Default::default()
        },
    );
    m.insert(
        "Outport".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Utf8("➡")),
            show_input_port_labels: false,
            show_output_port_labels: false,
            ..Default::default()
        },
    );
    m.insert(
        "Concatenate".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Utf8("☰")),
            show_input_port_labels: false,
            show_output_port_labels: false,
            ..Default::default()
        },
    );
    m.insert(
        "CFunction".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Utf8("📁")),
            ..Default::default()
        },
    );
    m.insert(
        "Terminator".to_string(),
        BlockTypeConfig {
            icon: Some(IconSpec::Utf8("⏹")),
            show_input_port_labels: false,
            show_output_port_labels: false,
            ..Default::default()
        },
    );

    // Register icons advertised by built-in virtual libraries.
    for lib in crate::builtin_libraries::VIRTUAL_LIBRARIES {
        for b in lib.blocks {
            // Register canonical name and any aliases.
            let mut names: Vec<&'static str> = Vec::with_capacity(1 + b.aliases.len());
            names.push(b.name);
            names.extend_from_slice(b.aliases);

            for &n in &names {
                if let Some(icon) = b.icon {
                    register_virtual_keys(
                        &mut m,
                        lib.name,
                        n,
                        BlockTypeConfig {
                            icon: Some(IconSpec::Svg(icon)),
                            ..Default::default()
                        },
                    );
                } else {
                    // Generic placeholder for virtual-library blocks that
                    // don't yet advertise a dedicated icon.
                    register_virtual_keys(
                        &mut m,
                        lib.name,
                        n,
                        BlockTypeConfig {
                            icon: Some(IconSpec::Utf8("👁")),
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }

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
        let entry = w
            .entry(block_type.to_string())
            .or_insert_with(BlockTypeConfig::default);
        f(entry);
    }
}

/// Register icon configurations for all currently-registered user virtual
/// libraries.
///
/// Call this after calling
/// [`register_virtual_library`](crate::builtin_libraries::register_virtual_library)
/// to make blocks from user libraries visible in the icon registry.  A default
/// placeholder icon (`👁`) is used for each block.  You can later override
/// individual entries with [`set_block_type_config`].
///
/// It is safe to call this function multiple times; already-present keys are
/// simply overwritten with the same placeholder.
pub fn register_user_library_block_types() {
    let map = get_block_type_config_map();
    if let Ok(mut m) = map.write() {
        crate::builtin_libraries::virtual_library::for_each_user_library_block(|lib_name, block| {
            let cfg = BlockTypeConfig {
                icon: Some(IconSpec::Utf8("👁")),
                ..Default::default()
            };
            let mut names: Vec<String> = vec![block.name.clone()];
            names.extend(block.aliases.iter().cloned());
            for name in names {
                register_virtual_keys(&mut m, lib_name, &name, cfg.clone());
            }
        });
    }
}
