//! Egui-based interactive viewer for Simulink systems (feature = "egui").
//!
//! This module splits the original monolithic implementation into smaller
//! submodules to improve readability and maintainability.

#![cfg(feature = "egui")]

pub mod dashboard_widgets;
mod geometry;
pub mod icon_assets;
mod navigation;
mod render;
pub mod scope_widget;
mod state;
pub mod text;
mod ui;

// Re-export geometry items needed by the editor module
pub use geometry::{
    PortSide, endpoint_pos_maybe_mirrored, parse_block_rect, parse_rect_str, port_anchor_pos,
    port_indicator_positions,
};
pub use navigation::{
    collect_subsystems_paths, resolve_subsystem_by_path, resolve_subsystem_by_vec,
};
pub use render::{get_block_type_cfg, render_block_icon, wrap_text_to_max_width};

// Helpers which are useful for integration tests
pub use render::{PortLabelMaxWidths, compute_icon_available_rect};
// Interior renderer registry access (needed by dashboard visualization tests)
pub use render::{InteriorRendererFn, get_interior_renderer};
#[cfg(feature = "dashboard")]
pub use state::ScopePopout;
pub use state::{
    BlockContextMenuItem, BlockDialog, BlockDialogButton, ChartView, SignalContextMenuItem,
    SignalDialog, SignalDialogButton, SubsystemApp, SubsystemEntities,
};
#[cfg(feature = "dashboard")]
pub use state::{DashboardControlEvent, DashboardControlValue};
pub use text::{highlight_query_job, matlab_syntax_job};
pub use ui::{
    ClickAction, UpdateResponse, apply_update_response, show_info_windows, update, update_with_info,
};
// Expose the canonical color utility module for reuse by the editor.
pub use ui::colors;

// Expose a couple of internal helpers for use by integration tests.
pub use ui::helpers::{block_dialog_title, clean_display_string};
// SVG parsing helper (also needed by some tests)
pub use render::embedded_egui_sans_fontdb;
