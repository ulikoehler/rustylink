use super::helpers::{block_dialog_title, is_block_subsystem};
use super::types::UpdateResponse;
use crate::egui_app::state::{BlockDialog, ChartView, SignalDialog, SubsystemApp};
use crate::egui_app::text::matlab_syntax_job;
use crate::model::EndpointRef;
use eframe::egui::{self, Color32, RichText};

fn build_chart_view_for_block(
    app: &SubsystemApp,
    block: &crate::model::Block,
) -> Option<ChartView> {
    let is_chart_block = block.block_type == "MATLAB Function"
        || (block.block_type == "SubSystem" && block.is_matlab_function);
    if !is_chart_block {
        return None;
    }
    let by_sid = block
        .sid
        .as_ref()
        .and_then(|sid| app.chart_map.get(sid))
        .cloned();
    let mut instance_name = if app.path.is_empty() {
        block.name.clone()
    } else {
        format!("{}/{}", app.path.join("/"), block.name)
    };
    instance_name = instance_name.trim_matches('/').to_string();
    let cid_opt = by_sid.or_else(|| app.chart_map.get(&instance_name).cloned());
    let chart = cid_opt.and_then(|cid| app.charts.get(&cid));
    chart.map(|chart| ChartView {
        title: chart
            .name
            .clone()
            .or(chart.eml_name.clone())
            .unwrap_or_else(|| block.name.clone()),
        script: chart.script.clone().unwrap_or_default(),
        open: true,
    })
}

pub fn apply_update_response(app: &mut SubsystemApp, response: &UpdateResponse) {
    match response {
        UpdateResponse::None => {}
        UpdateResponse::Signal {
            line_idx,
            line,
            handled,
            ..
        } => {
            if *handled {
                return;
            }
            let title = line.name.clone().unwrap_or("<signal>".into());
            app.signal_view = Some(SignalDialog {
                title,
                line_idx: *line_idx,
                open: true,
            });
        }
        UpdateResponse::Block { block, handled, .. } => {
            if *handled {
                return;
            }
            if is_block_subsystem(block) {
                return;
            }
            // build a cleaned title using our helper function
            let title_cleaned = block_dialog_title(block);

            if let Some(cv) = build_chart_view_for_block(app, block) {
                app.chart_view = Some(cv);
                app.block_view = Some(BlockDialog {
                    title: title_cleaned.clone(),
                    block: block.clone(),
                    open: true,
                });
            } else {
                app.block_view = Some(BlockDialog {
                    title: title_cleaned.clone(),
                    block: block.clone(),
                    open: true,
                });
            }
        }
    }
}

fn show_chart_window(app: &mut SubsystemApp, ui: &mut egui::Ui) {
    if let Some(cv) = &mut app.chart_view {
        let mut open_flag = cv.open;
        egui::Window::new(format!("Chart: {}", cv.title))
            .open(&mut open_flag)
            .resizable(true)
            .vscroll(true)
            .min_width(400.0)
            .min_height(200.0)
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let job = matlab_syntax_job(&cv.script);
                        ui.add(egui::Label::new(job).wrap());
                    });
            });
        cv.open = open_flag;
        if !cv.open {
            app.chart_view = None;
        }
    }
}

fn show_signal_window(app: &mut SubsystemApp, ui: &mut egui::Ui) {
    if let Some(sd) = &app.signal_view {
        let mut open_flag = sd.open;
        let title = format!("Signal: {}", sd.title);
        let sys = app.current_system().map(|s| s.clone());
        let line_idx = sd.line_idx;
        egui::Window::new(title)
            .open(&mut open_flag)
            .resizable(true)
            .vscroll(true)
            .min_width(360.0)
            .min_height(200.0)
            .show(ui.ctx(), |ui| {
                if let Some(sys) = &sys {
                    if let Some(line) = sys.lines.get(line_idx) {
                        ui.label(RichText::new("General").strong());
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!(
                                "Name: {}",
                                line.name.clone().unwrap_or("<unnamed>".into())
                            ));
                            if let Some(z) = &line.zorder {
                                ui.label(format!("Z: {}", z));
                            }
                        });
                        ui.separator();
                        let mut outputs: Vec<EndpointRef> = Vec::new();
                        fn collect_branch_dsts(
                            br: &crate::model::Branch,
                            out: &mut Vec<EndpointRef>,
                        ) {
                            if let Some(d) = &br.dst {
                                out.push(d.clone());
                            }
                            for s in &br.branches {
                                collect_branch_dsts(s, out);
                            }
                        }
                        if let Some(d) = &line.dst {
                            outputs.push(d.clone());
                        }
                        for b in &line.branches {
                            collect_branch_dsts(b, &mut outputs);
                        }
                        egui::CollapsingHeader::new("Inputs")
                            .default_open(true)
                            .show(ui, |ui| {
                                if let Some(src) = &line.src {
                                    let bname = sys
                                        .blocks
                                        .iter()
                                        .find(|b| b.sid.as_ref() == Some(&src.sid))
                                        .map(|b| b.name.clone())
                                        .unwrap_or_else(|| format!("SID{}", src.sid));
                                    let pname = sys
                                        .blocks
                                        .iter()
                                        .find(|b| b.sid.as_ref() == Some(&src.sid))
                                        .and_then(|b| {
                                            b.ports.iter().find(|p| {
                                                p.port_type == src.port_type
                                                    && p.index.unwrap_or(0) == src.port_index
                                            })
                                        })
                                        .and_then(|p| {
                                            p.properties
                                                .get("Name")
                                                .cloned()
                                                .or_else(|| p.properties.get("name").cloned())
                                        })
                                        .unwrap_or_else(|| {
                                            format!(
                                                "{}{}",
                                                if src.port_type == "in" { "In" } else { "Out" },
                                                src.port_index
                                            )
                                        });
                                    ui.label(format!(
                                        "{} • {}{} ({}): {}",
                                        bname,
                                        if src.port_type == "in" { "In" } else { "Out" },
                                        src.port_index,
                                        src.port_type,
                                        pname
                                    ));
                                } else {
                                    ui.label("<no source>");
                                }
                            });
                        egui::CollapsingHeader::new("Outputs")
                            .default_open(true)
                            .show(ui, |ui| {
                                if outputs.is_empty() {
                                    ui.label("<none>");
                                }
                                for d in outputs {
                                    let bname = sys
                                        .blocks
                                        .iter()
                                        .find(|b| b.sid.as_ref() == Some(&d.sid))
                                        .map(|b| b.name.clone())
                                        .unwrap_or_else(|| format!("SID{}", d.sid));
                                    let pname = sys
                                        .blocks
                                        .iter()
                                        .find(|b| b.sid.as_ref() == Some(&d.sid))
                                        .and_then(|b| {
                                            b.ports.iter().find(|p| {
                                                p.port_type == d.port_type
                                                    && p.index.unwrap_or(0) == d.port_index
                                            })
                                        })
                                        .and_then(|p| {
                                            p.properties
                                                .get("Name")
                                                .cloned()
                                                .or_else(|| p.properties.get("name").cloned())
                                        })
                                        .unwrap_or_else(|| {
                                            format!(
                                                "{}{}",
                                                if d.port_type == "in" { "In" } else { "Out" },
                                                d.port_index
                                            )
                                        });
                                    ui.label(format!(
                                        "{} • {}{} ({}): {}",
                                        bname,
                                        if d.port_type == "in" { "In" } else { "Out" },
                                        d.port_index,
                                        d.port_type,
                                        pname
                                    ));
                                }
                            });
                        if !app.signal_buttons.is_empty() {
                            ui.separator();
                            ui.label(RichText::new("Actions").strong());
                            ui.horizontal_wrapped(|ui| {
                                for btn in &app.signal_buttons {
                                    if (btn.filter)(line) {
                                        if ui.button(&btn.label).clicked() {
                                            (btn.on_click)(line);
                                        }
                                    }
                                }
                            });
                        }
                    } else {
                        ui.colored_label(
                            Color32::RED,
                            "Selected signal no longer exists in this view",
                        );
                    }
                }
            });
        if let Some(sd_mut) = &mut app.signal_view {
            sd_mut.open = open_flag;
            if !sd_mut.open {
                app.signal_view = None;
            }
        }
    }
}

fn show_block_window(app: &mut SubsystemApp, ui: &mut egui::Ui) {
    if let Some(bd) = &app.block_view {
        let mut open_flag = bd.open;
        let block = bd.block.clone();
        // the title was cleaned when the dialog was created; normalize again just
        // in case the string was mutated by a custom button handler.
        let win_title = crate::parser::helpers::clean_whitespace(&bd.title);
        egui::Window::new(format!("Block: {}", win_title))
            .open(&mut open_flag)
            .resizable(true)
            .vscroll(true)
            .min_width(360.0)
            .min_height(220.0)
            .show(ui.ctx(), |ui| {
                ui.label(RichText::new("General").strong());
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!(
                        "Name: {}",
                        crate::parser::helpers::clean_whitespace(&block.name)
                    ));
                    ui.label(format!(
                        "Type: {}",
                        crate::parser::helpers::clean_whitespace(&block.block_type)
                    ));
                    if let Some(sid) = block.sid.as_ref() {
                        ui.label(format!("SID: {}", sid));
                    }
                    if let Some(z) = &block.zorder {
                        ui.label(format!("Z: {}", z));
                    }
                    if block.commented {
                        ui.label("commented");
                    }
                });
                ui.separator();
                egui::CollapsingHeader::new("Properties")
                    .default_open(true)
                    .show(ui, |ui| {
                        if block.properties.is_empty() {
                            ui.label("<none>");
                        }
                        for (k, v) in &block.properties {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(crate::parser::helpers::clean_whitespace(k))
                                        .strong(),
                                );
                                ui.label(crate::parser::helpers::clean_whitespace(v));
                            });
                        }
                    });
                if let Some(id) = &block.instance_data {
                    if !id.properties.is_empty() {
                        ui.separator();
                        egui::CollapsingHeader::new("Instance Parameters")
                            .default_open(true)
                            .show(ui, |ui| {
                                for (k, v) in &id.properties {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(
                                                crate::parser::helpers::clean_whitespace(k),
                                            )
                                            .strong(),
                                        );
                                        ui.label(crate::parser::helpers::clean_whitespace(v));
                                    });
                                }
                            });
                    }
                }
                if block.block_type == "CFunction" {
                    if let Some(cfg) = &block.c_function {
                        ui.separator();
                        egui::CollapsingHeader::new("C/C++ Code")
                            .default_open(true)
                            .show(ui, |ui| {
                                if let Some(s) = &cfg.start_code {
                                    ui.label(RichText::new("StartCode").strong());
                                    ui.add(
                                        egui::TextEdit::multiline(&mut s.clone())
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                                if let Some(s) = &cfg.output_code {
                                    ui.label(RichText::new("OutputCode").strong());
                                    ui.add(
                                        egui::TextEdit::multiline(&mut s.clone())
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                                if let Some(s) = &cfg.terminate_code {
                                    ui.label(RichText::new("TerminateCode").strong());
                                    ui.add(
                                        egui::TextEdit::multiline(&mut s.clone())
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                                if let Some(s) = &cfg.codegen_start_code {
                                    ui.label(RichText::new("CodegenStartCode").strong());
                                    ui.add(
                                        egui::TextEdit::multiline(&mut s.clone())
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                                if let Some(s) = &cfg.codegen_output_code {
                                    ui.label(RichText::new("CodegenOutputCode").strong());
                                    ui.add(
                                        egui::TextEdit::multiline(&mut s.clone())
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                                if let Some(s) = &cfg.codegen_terminate_code {
                                    ui.label(RichText::new("CodegenTerminateCode").strong());
                                    ui.add(
                                        egui::TextEdit::multiline(&mut s.clone())
                                            .desired_width(f32::INFINITY),
                                    );
                                }
                            });
                    }
                }
                egui::CollapsingHeader::new("Ports")
                    .default_open(true)
                    .show(ui, |ui| {
                        if block.ports.is_empty() {
                            ui.label("<none>");
                            return;
                        }
                        let mut ins: Vec<&crate::model::Port> =
                            block.ports.iter().filter(|p| p.port_type == "in").collect();
                        let mut outs: Vec<&crate::model::Port> = block
                            .ports
                            .iter()
                            .filter(|p| p.port_type == "out")
                            .collect();
                        ins.sort_by_key(|p| p.index.unwrap_or(0));
                        outs.sort_by_key(|p| p.index.unwrap_or(0));
                        if !ins.is_empty() {
                            ui.label(RichText::new("Inputs").strong());
                        }
                        for p in ins {
                            let idx = p.index.unwrap_or(0);
                            let name = p
                                .properties
                                .get("Name")
                                .or_else(|| p.properties.get("name"))
                                .cloned()
                                .unwrap_or_else(|| format!("In{}", idx));
                            ui.label(format!("{}{}: {}", "In", idx, name));
                        }
                        if !outs.is_empty() {
                            ui.separator();
                            ui.label(RichText::new("Outputs").strong());
                        }
                        for p in outs {
                            let idx = p.index.unwrap_or(0);
                            let name = p
                                .properties
                                .get("Name")
                                .or_else(|| p.properties.get("name"))
                                .cloned()
                                .unwrap_or_else(|| format!("Out{}", idx));
                            ui.label(format!("{}{}: {}", "Out", idx, name));
                        }
                    });
                if !app.block_buttons.is_empty() {
                    ui.separator();
                    ui.label(RichText::new("Actions").strong());
                    ui.horizontal_wrapped(|ui| {
                        for btn in &app.block_buttons {
                            if (btn.filter)(&block) {
                                if ui.button(&btn.label).clicked() {
                                    (btn.on_click)(&block);
                                }
                            }
                        }
                    });
                }
            });
        if let Some(bd_mut) = &mut app.block_view {
            bd_mut.open = open_flag;
            if !bd_mut.open {
                app.block_view = None;
            }
        }
    }
}

/// Show a scope popout window with an interactive liveplot.
#[cfg(feature = "dashboard")]
fn show_scope_popout_window(app: &mut SubsystemApp, ui: &mut egui::Ui) {
    let viewer_instance_id = app.instance_id;
    if let Some(popout) = &mut app.scope_popout {
        let mut open_flag = popout.open;
        let scope_key = popout.scope_key.clone();
        let storage_key = format!("popout::{scope_key}");
        egui::Window::new(format!("Scope: {}", popout.title))
            .id(egui::Id::new((
                "rustylink_viewer",
                viewer_instance_id,
                ("scope", "window", scope_key.as_str()),
            )))
            .open(&mut open_flag)
            .resizable(true)
            .default_size([400.0, 300.0])
            .min_width(200.0)
            .min_height(150.0)
            .show(ui.ctx(), |ui| {
                let mut scopes = app.scope_instances.lock().unwrap();
                let scope = scopes.entry(storage_key.clone()).or_insert_with(|| {
                    crate::egui_app::scope_widget::MiniScope::new((
                        viewer_instance_id,
                        "scope_popout",
                        scope_key.as_str(),
                    ))
                });
                scope.show(ui);
            });
        popout.open = open_flag;
        if !popout.open {
            app.scope_popout = None;
        }
    }
}

pub fn show_info_windows(app: &mut SubsystemApp, ui: &mut egui::Ui) {
    show_chart_window(app, ui);
    show_signal_window(app, ui);
    show_block_window(app, ui);
    #[cfg(feature = "dashboard")]
    show_scope_popout_window(app, ui);
}
