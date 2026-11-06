//! The block-definition catalog: one file per Simulink library group.
//!
//! Each group module exposes a `pub static BLOCKS: &[SimulinkBlockDefinition]`
//! holding the definitions of one Simulink palette group, and is listed in
//! [`ALL_LIBRARIES`].  To add your own blocks, extend the matching group (or
//! create a new file and list it here); no renderer changes are required.
//! Helpers shared by the group files live in [`prelude`].

#![cfg(feature = "egui")]

pub mod catalog;
pub mod continuous;
pub mod dashboard;
pub mod dashboard_misc;
pub mod datetime;
pub mod discontinuities;
pub mod discrete;
pub mod logic;
pub mod lookup;
pub mod math;
pub mod messages_events;
pub mod matrix;
pub mod ports_subsystems;
mod prelude;
pub mod signal_attributes;
pub mod signal_routing;
pub mod sinks;
pub mod sources;
pub mod strings;
pub mod user_defined;
pub mod utilities;
pub mod verification;

use crate::simulink_libraries::types::SimulinkLibrary;

macro_rules! group {
    ($name:literal, $module:ident) => {
        SimulinkLibrary {
            name: $name,
            blocks: self::$module::BLOCKS,
        }
    };
}

/// Hand-written libraries with full rendering metadata (icons, shapes, custom
/// static/live renderers).  Registered first so they win on key collisions.
pub static ALL_LIBRARIES: &[SimulinkLibrary] = &[
    group!("simulink", sources),
    group!("simulink", sinks),
    group!("simulink", continuous),
    group!("simulink", datetime),
    group!("simulink", discrete),
    group!("simulink", math),
    group!("simulink", messages_events),
    group!("simulink", logic),
    group!("simulink", discontinuities),
    group!("simulink", lookup),
    group!("simulink", signal_routing),
    group!("simulink", signal_attributes),
    group!("simulink", ports_subsystems),
    group!("simulink", user_defined),
    group!("simulink", utilities),
    group!("simulink", verification),
    group!("simulink", strings),
    group!("dashboard", dashboard),
    group!("dashboard", dashboard_misc),
    group!("matrix_library", matrix),
];

/// The metadata-only browser/palette catalog (~786 entries).  Registered after
/// the rich libraries and the bridged virtual libraries, so it only fills in
/// block types those do not already provide.
pub static PALETTE: &[crate::simulink_libraries::types::SimulinkBlockDefinition] =
    self::catalog::BLOCKS;
