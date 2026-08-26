//! Bridge from the unified catalog to the legacy [`BlockTypeConfig`] consumed
//! by the existing rendering geometry (port labels, shapes, default port
//! counts).  This lets the catalog be the single source of truth while the
//! detailed port-label/geometry code keeps working unchanged.

#![cfg(feature = "egui")]

use std::collections::HashMap;

use crate::block_types::{BlockTypeConfig, IconSpec};
use crate::simulink_libraries::stubs::{humanize_camel_case, normalize_block_name};

use super::types::{PortLabelPolicy, SimulinkBlockDefinition, SimulinkIcon};

/// Map a catalog icon to the legacy icon spec.
pub fn icon_to_spec(icon: SimulinkIcon) -> IconSpec {
    match icon {
        SimulinkIcon::Utf8(s) => IconSpec::Utf8(s),
        SimulinkIcon::Phosphor(s) => IconSpec::Phosphor(s),
        SimulinkIcon::Math(s) => IconSpec::Math(s),
        SimulinkIcon::Plot(s) => IconSpec::Plot(s),
    }
}

/// `(show_labels, fixed_names)` derived from a port-label policy.
fn names_from_policy(policy: &PortLabelPolicy) -> (bool, Vec<String>) {
    match policy {
        PortLabelPolicy::None => (false, Vec::new()),
        PortLabelPolicy::Fixed(list) => (true, list.iter().map(|s| s.to_string()).collect()),
        PortLabelPolicy::MetadataDependent(_) => (true, Vec::new()),
    }
}

/// Build a [`BlockTypeConfig`] from a catalog definition.
pub fn from_definition(def: &SimulinkBlockDefinition) -> BlockTypeConfig {
    let (show_in, in_names) = names_from_policy(&def.input_port_label);
    let (show_out, out_names) = names_from_policy(&def.output_port_label);
    BlockTypeConfig {
        background: None,
        border: None,
        icon: def.icon.map(icon_to_spec),
        show_input_port_labels: show_in,
        show_output_port_labels: show_out,
        shape: def.shape,
        default_ins: def.inputs.default_count(),
        default_outs: def.outputs.default_count(),
        known: true,
        port_position_overrides: def.port_position_overrides.to_vec(),
        input_port_names: in_names,
        output_port_names: out_names,
    }
}

/// Produce all `(key, config)` entries to seed the legacy registry from the
/// catalog.  Generates the same key variants the previous hardcoded registry
/// used (raw / normalized / humanized, optionally library-prefixed) so that
/// `get_block_type_cfg`'s multi-phase lookup keeps resolving every block.
pub fn block_type_config_entries() -> HashMap<String, BlockTypeConfig> {
    let mut map: HashMap<String, BlockTypeConfig> = HashMap::new();
    for &def in super::resolver::registry().all() {
        let cfg = from_definition(def);
        let lib = def.category;
        let mut names: Vec<&str> = Vec::with_capacity(1 + def.aliases.len());
        names.push(def.block_type);
        names.extend_from_slice(def.aliases);
        for name in names {
            let human = humanize_camel_case(name);
            let norm_raw = normalize_block_name(name);
            let norm_human = normalize_block_name(&human);
            for key in [
                name.to_string(),
                norm_raw.clone(),
                human.clone(),
                norm_human.clone(),
                format!("{lib}/{name}"),
                format!("{lib}/{norm_raw}"),
                format!("{lib}/{human}"),
                format!("{lib}/{norm_human}"),
            ] {
                // First definition wins on collision (hand-written core/dashboard
                // libraries are iterated before bridged ones).  Preserve an
                // existing icon if the newcomer has none.
                match map.get(&key) {
                    Some(existing) if existing.icon.is_some() && cfg.icon.is_none() => {}
                    Some(_) => {}
                    None => {
                        map.insert(key, cfg.clone());
                    }
                }
            }
        }
    }
    map
}
