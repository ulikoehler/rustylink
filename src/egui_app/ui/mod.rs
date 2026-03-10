pub mod types;
pub mod helpers;
pub mod colors;
pub mod dialogs;
pub mod line_coloring;
pub mod signal_routing;
pub mod view_transform;
pub mod update;

pub use types::{ClickAction, UpdateResponse};
pub use dialogs::{apply_update_response, show_info_windows};

use crate::egui_app::state::SubsystemApp;
use eframe::egui;
use update::update_internal;

pub fn update(app: &mut SubsystemApp, ui: &mut egui::Ui) -> UpdateResponse {
    update_internal(app, ui, false)
}

pub fn update_with_info(app: &mut SubsystemApp, ui: &mut egui::Ui) -> UpdateResponse {
    let response = update_internal(app, ui, true);
    apply_update_response(app, &response);
    show_info_windows(app, ui);
    response
}
