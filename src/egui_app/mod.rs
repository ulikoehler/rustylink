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

pub use geometry::{
    PortSide, endpoint_pos, endpoint_pos_with_target, parse_block_rect, port_anchor_pos,
};
pub use navigation::{
    collect_subsystems_paths, resolve_subsystem_by_path, resolve_subsystem_by_vec,
};
pub use render::render_block_icon;
pub use state::{
    BlockContextMenuItem, BlockDialog, BlockDialogButton, ChartView, ClickedBlock, ClickedSignal,
    SignalContextMenuItem, SignalDialog, SignalDialogButton, SubsystemApp, SubsystemEntities,
};
pub use text::{highlight_query_job, matlab_syntax_job};
pub use ui::update;
