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

    // NOTE: earlier versions manually registered icons for a handful of
    // matrix-library blocks.  We now embed those paths directly in the
    // library definitions and populate the registry by iterating
    // `matrix_library::BLOCKS` later below.  That keeps the icon knowledge
    // colocated with the library data and avoids drift when other icons are
    // added.

    // (explicit cases removed; see library-driven registration further down)

    // Register any icons that the matrix virtual library itself advertises.
    // The library names are CamelCase, but Simulink often writes them with a
    // space between words ("Matrix Multiply", "Cross product", etc).  To
    // ensure the viewer resolves the icon regardless of which variant appears
    // in `SourceBlock` or `library_block_path`, we register both the raw name
    // and a space-separated version.  We also register prefixed forms to
    // handle `matrix_library/...` keys.
    fn humanize(name: &str) -> String {
        let mut out = String::new();
        for (i, ch) in name.chars().enumerate() {
            if i > 0 && ch.is_uppercase() && !name.chars().nth(i - 1).unwrap().is_uppercase() {
                out.push(' ');
            }
            out.push(ch);
        }
        out
    }

    let mut matrix_icon_names = std::collections::HashSet::new();
    for b in crate::builtin_libraries::matrix_library::BLOCKS {
        if let Some(icon) = b.icon {
            matrix_icon_names.insert(b.name);
            let human = humanize(b.name);
            for key in &[
                b.name.to_string(),
                human.clone(),
                format!("matrix_library/{}", b.name),
                format!("matrix_library/{}", human),
            ] {
                m.insert(
                    key.clone(),
                    BlockTypeConfig {
                        icon: Some(IconSpec::Svg(icon)),
                        ..Default::default()
                    },
                );
            }
        }
    }

    // The remaining matrix-library virtual blocks currently share a generic placeholder.
    for name in [
        "Transpose",
        "HermitianTranspose",
        "MatrixSquare",
        "PermuteColumns",
        "ExtractDiagonal",
        "CreateDiagonalMatrix",
        "ExpandScalar",
        "IsHermitian",
        "MatrixConcatenate",
    ] {
        if matrix_icon_names.contains(name) {
            continue; // the library provided its own icon
        }
        for key in &[name.to_string(), format!("matrix_library/{}", name)] {
            m.insert(
                key.clone(),
                BlockTypeConfig {
                    icon: Some(IconSpec::Utf8("👁")),
                    // we generally want input/output labels visible so that the
                    // automatically-generated stub ports are readable
                    ..Default::default()
                },
            );
        }
    }

    // Register icons advertised by the `simulink/Discrete` virtual library.
    for b in crate::builtin_libraries::simulink_discrete::BLOCKS {
        if let Some(icon) = b.icon {
            for key in &[
                b.name.to_string(),
                format!("{}/{}", crate::builtin_libraries::simulink_discrete::LIB_NAME, b.name),
            ] {
                m.insert(
                    key.clone(),
                    BlockTypeConfig {
                        icon: Some(IconSpec::Svg(icon)),
                        ..Default::default()
                    },
                );
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
