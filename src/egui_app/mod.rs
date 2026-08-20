//! Egui-based interactive viewer for Simulink systems (feature = "egui").
//!
//! This module splits the original monolithic implementation into smaller
//! submodules to improve readability and maintainability.

use eframe::egui::{self, Rect};

pub mod dashboard_widgets;
pub mod fonts;
pub mod geometry;
pub mod icon_assets;
pub mod navigation;
pub mod render;
pub mod scope_widget;
pub mod state;
pub mod text;
pub mod ui;

// Re-export geometry items needed by the editor module
pub use geometry::{
    PortSide, endpoint_pos_maybe_mirrored, parse_block_rect, parse_rect_str, port_anchor_pos,
    port_indicator_positions,
};
pub use navigation::{
    collect_subsystems_paths, resolve_subsystem_by_path, resolve_subsystem_by_vec,
};
pub use render::wrap_text_to_max_width;

// Helpers which are useful for integration tests
pub use render::{PortLabelMaxWidths, compute_icon_available_rect};
#[cfg(feature = "dashboard")]
pub use state::ScopePopout;
pub use state::{
    BlockContextMenuItem, BlockDialog, BlockDialogButton, ChartView, LiveTooltipEntry,
    LiveTooltipKind, NavigationViewState, SignalContextMenuItem, SignalDialog, SignalDialogButton,
    SubsystemApp, SubsystemEntities,
};
#[cfg(feature = "dashboard")]
pub use state::{DashboardControlEvent, DashboardControlValue};
pub use text::{highlight_query_job, matlab_syntax_job};
pub(crate) use ui::view_transform::shared_canvas_text_font_px;
pub use ui::zoom_controls::show_zoom_controls;
pub use ui::{
    ClickAction, UpdateResponse, apply_update_response, show_info_windows, update, update_with_info,
};
// Expose the canonical color utility module for reuse by the editor.
pub use ui::colors;

// Expose a couple of internal helpers for use by integration tests.
pub use ui::helpers::{block_dialog_title, clean_display_string};
// SVG parsing helper (also needed by some tests)
pub use render::embedded_egui_sans_fontdb;

pub fn port_label_display_name(
    block: &crate::model::Block,
    index: u32,
    is_input: bool,
    cfg: &crate::block_types::BlockTypeConfig,
) -> String {
    port_label_defined_name(block, index, is_input, cfg)
        .unwrap_or_else(|| format!("{}{}", if is_input { "In" } else { "Out" }, index))
}

/// The label the model/catalog defines for a port, or `None` when there is
/// none.  Unlike [`port_label_display_name`] this does not invent an
/// `In<N>`/`Out<N>` placeholder, so callers can tell a genuinely named port
/// (which Simulink prints whether or not a signal is attached) from an
/// anonymous one.
pub fn port_label_defined_name(
    block: &crate::model::Block,
    index: u32,
    is_input: bool,
    cfg: &crate::block_types::BlockTypeConfig,
) -> Option<String> {
    let mirrored = block.block_mirror.unwrap_or(false);
    let logical_is_input = if mirrored { !is_input } else { is_input };

    let catalog_name = || {
        let names = if logical_is_input {
            &cfg.input_port_names
        } else {
            &cfg.output_port_names
        };
        names.get(index.checked_sub(1)? as usize).cloned()
    };

    render::subsystem_boundary_port_name(block, index, logical_is_input)
        .or_else(|| crate::simulink_libraries::render::port_label(block, index, logical_is_input))
        .or_else(catalog_name)
        .filter(|name| !name.is_empty())
}

/// Resolve a block's [`BlockTypeConfig`] from the unified catalog.
pub fn get_block_type_cfg(block: &crate::model::Block) -> crate::block_types::BlockTypeConfig {
    render::get_block_type_cfg(block)
}

/// Draw a block's icon via the single, catalog-driven icon renderer.
pub fn render_block_icon(
    painter: &egui::Painter,
    block: &crate::model::Block,
    rect: &Rect,
    font_scale: f32,
    port_label_widths: Option<PortLabelMaxWidths>,
) {
    let icon_color = render::block_icon_color(block);
    render::render_block_icon(
        painter,
        block,
        rect,
        font_scale,
        icon_color,
        port_label_widths,
    );
}
