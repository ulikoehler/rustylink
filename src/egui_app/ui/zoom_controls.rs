use eframe::egui::{self, Pos2, Rect, Vec2};

#[allow(clippy::too_many_arguments)]
pub fn show_zoom_controls(
    ctx: &egui::Context,
    area_id: egui::Id,
    fixed_pos: Pos2,
    zoom: &mut f32,
    pan: &mut Vec2,
    base_scale: f32,
    world_bounds: Rect,
    origin: Pos2,
    center: Pos2,
    reset_requested: &mut bool,
    monochrome: &mut bool,
) {
    egui::Area::new(area_id)
        .fixed_pos(fixed_pos)
        .show(ctx, |ui| {
            egui::Frame::menu(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut zoom_by = |factor: f32| {
                        let old_zoom = *zoom;
                        let new_zoom = (old_zoom * factor).clamp(0.2, 30.0);
                        if (new_zoom - old_zoom).abs() <= f32::EPSILON {
                            return;
                        }

                        let s_old = base_scale * old_zoom;
                        let s_new = base_scale * new_zoom;
                        let world_x = (center.x - origin.x - pan.x) / s_old + world_bounds.left();
                        let world_y = (center.y - origin.y - pan.y) / s_old + world_bounds.top();
                        *zoom = new_zoom;
                        pan.x = center.x - ((world_x - world_bounds.left()) * s_new + origin.x);
                        pan.y = center.y - ((world_y - world_bounds.top()) * s_new + origin.y);
                    };

                    if ui.small_button("−").clicked() {
                        zoom_by(0.9);
                    }
                    if ui.small_button("+").clicked() {
                        zoom_by(1.1);
                    }
                    if ui.small_button("Reset").clicked() {
                        *reset_requested = true;
                    }

                    // The readout is expressed in the model's measurement unit:
                    // 100% == one screen pixel per model unit. `base_scale * zoom`
                    // is exactly that screen-pixels-per-model-unit scale, so the
                    // value is decoupled from the fit-to-view factor.
                    let percent = (base_scale * *zoom * 100.0).round() as i32;
                    ui.label(format!("{}%", percent));

                    ui.separator();
                    ui.checkbox(monochrome, "Less color").on_hover_text(
                        "Flat Simulink-style blocks: white bodies with thin \
                         borders (areas keep their model colors)",
                    );
                    let mut dark = ui.visuals().dark_mode;
                    if ui
                        .checkbox(&mut dark, "Dark")
                        .on_hover_text("Toggle a dark canvas theme (signal lines stay visible)")
                        .changed()
                    {
                        ui.ctx().set_visuals(if dark {
                            egui::Visuals::dark()
                        } else {
                            egui::Visuals::light()
                        });
                    }
                });
            });
        });
}
