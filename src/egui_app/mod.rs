//! Egui-based interactive viewer for Simulink systems (feature = "egui").
//!
//! This module splits the original monolithic implementation into smaller
//! submodules to improve readability and maintainability.

#![cfg(feature = "egui")]

mod geometry;
mod navigation;
mod render;
mod state;
pub mod text;
mod ui;

// Re-export geometry items needed by the editor module
pub use geometry::{
    PortSide, endpoint_pos, endpoint_pos_maybe_mirrored,
    endpoint_pos_with_target, endpoint_pos_with_target_maybe_mirrored,
    parse_block_rect, parse_rect_str, port_anchor_pos,
};
pub use navigation::{
    collect_subsystems_paths, resolve_subsystem_by_path, resolve_subsystem_by_vec,
};
pub(crate) use render::{get_block_type_cfg, render_block_icon};
pub use state::{
    BlockContextMenuItem, BlockDialog, BlockDialogButton, ChartView, SignalContextMenuItem,
    SignalDialog, SignalDialogButton, SubsystemApp, SubsystemEntities,
};
pub use text::{highlight_query_job, matlab_syntax_job};
pub use ui::{
    ClickAction, UpdateResponse, apply_update_response, show_info_windows, update, update_with_info,
};
