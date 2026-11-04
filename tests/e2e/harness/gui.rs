//! Headless GUI drivers: run real `editor_update`/`ui::update` frames inside
//! an `egui::Context` with synthesized `RawInput`.
//!
//! Screen positions are obtained via `app.last_canvas_transform` — the exact
//! transform the renderer used last frame — so tests never recompute layout.

#![allow(dead_code)]

use super::input;
use eframe::egui::{self, Pos2, Rect};
use rustylink::editor::EditorState;
use rustylink::editor::state::DragMode;
use rustylink::egui_app::SubsystemApp;
use rustylink::egui_app::geometry::parse_block_rect;
use rustylink::egui_app::ui::view_transform::ViewTransform;
use rustylink::model::{Block, Chart, System};
use std::collections::BTreeMap;

fn no_charts() -> (BTreeMap<u32, Chart>, BTreeMap<String, u32>) {
    (BTreeMap::new(), BTreeMap::new())
}

fn new_ctx() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    rustylink::egui_app::fonts::install(&ctx);
    ctx
}

// ────────────────────────────────────────────────────────────────────────────
// EditorHarness
// ────────────────────────────────────────────────────────────────────────────

/// Headless editor: real `EditorState`, real canvas paint, synthesized input.
pub struct EditorHarness {
    pub ctx: egui::Context,
    pub state: EditorState,
    last: Option<egui::FullOutput>,
}

impl EditorHarness {
    pub fn new(root: System) -> Self {
        let (charts, chart_map) = no_charts();
        Self {
            ctx: new_ctx(),
            state: EditorState::new(root, Vec::new(), charts, chart_map),
            last: None,
        }
    }

    /// Run exactly one frame with the given input batch.
    pub fn frame(&mut self, raw: egui::RawInput) -> &egui::FullOutput {
        let mut out = self.ctx.run_ui(raw, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                rustylink::editor::ui::editor_update_with_info(&mut self.state, ui);
            });
        });
        // No GPU backend: discard pending texture uploads so `FullOutput` can
        // be kept for shape introspection without a panic-on-drop.
        out.textures_delta.clear();
        self.last = Some(out);
        self.last.as_ref().unwrap()
    }

    /// Run `n` idle frames (let layout/repaint/resolver settle).
    pub fn idle(&mut self, n: usize) {
        for _ in 0..n {
            self.frame(input::idle());
        }
    }

    /// Run a sequence of input batches, one frame each.
    pub fn run(&mut self, inputs: impl IntoIterator<Item = egui::RawInput>) {
        for i in inputs {
            self.frame(i);
        }
    }

    /// press at `from`, interpolate to `to` over `steps` frames, release.
    pub fn drag(&mut self, from: Pos2, to: Pos2, steps: usize) {
        self.frame(input::press(from));
        self.run(input::drag_path(from, to, steps));
    }

    /// Like [`Self::drag`] but holding `modifiers` during the press.
    pub fn drag_with(&mut self, from: Pos2, to: Pos2, steps: usize, modifiers: egui::Modifiers) {
        self.frame(input::press_with(from, modifiers));
        self.run(input::drag_path_with(from, to, steps, modifiers));
    }

    pub fn click(&mut self, pos: Pos2) {
        self.run(input::click(pos));
    }

    pub fn key(&mut self, key: egui::Key, modifiers: egui::Modifiers) {
        self.frame(input::key(key, modifiers));
    }

    // ── geometry ─────────────────────────────────────────────────────────

    /// The transform used by the last rendered frame.
    pub fn transform(&self) -> ViewTransform {
        self.state
            .app
            .last_canvas_transform
            .expect("canvas transform available after ≥1 rendered frame")
    }

    pub fn canvas_rect(&self) -> Rect {
        self.transform().avail
    }

    /// Model-space rect of a block by SID (from its `Position` property).
    pub fn block_model_rect(&self, sid: &str) -> Option<Rect> {
        self.state
            .current_system()?
            .blocks
            .iter()
            .find(|b| b.sid.as_deref() == Some(sid))
            .and_then(parse_block_rect)
    }

    /// Screen-space rect of a block via the last frame's transform.
    pub fn block_screen_rect(&self, sid: &str) -> Option<Rect> {
        let r = self.block_model_rect(sid)?;
        let t = self.transform();
        Some(Rect::from_min_max(t.to_screen(r.min), t.to_screen(r.max)))
    }

    pub fn block_center(&self, sid: &str) -> Pos2 {
        self.block_screen_rect(sid).unwrap().center()
    }

    /// Screen position of a port **hit-area center**, replicating the editor
    /// canvas formula: `y = top + height * i/(n+1)` (i = 1-based index),
    /// `x = left` (in) / `right` (out), mirrored-aware.
    pub fn port_hit_pos(&self, sid: &str, kind: &str, index: u32) -> Pos2 {
        let (b, r) = self.block_and_screen_rect(sid).expect("block exists");
        let mirrored = b.block_mirror.unwrap_or(false);
        let x = match (kind, mirrored) {
            ("in", false) | ("out", true) => r.left(),
            _ => r.right(),
        };
        let n = match kind {
            "in" => b.port_counts.as_ref().and_then(|p| p.ins).unwrap_or(1),
            _ => b.port_counts.as_ref().and_then(|p| p.outs).unwrap_or(1),
        }
        .max(1);
        let y = r.top() + r.height() * (index as f32) / (n as f32 + 1.0);
        Pos2::new(x, y)
    }

    /// Model-space position of a port the way `find_snap_port` computes it:
    /// `y = t + (2i − 0.5) · h/(2n+1)`.
    pub fn port_snap_model_pos(&self, sid: &str, kind: &str, index: u32) -> Pos2 {
        let (b, _) = self.block_and_screen_rect(sid).expect("block exists");
        let r = self.block_model_rect(sid).unwrap();
        let mirrored = b.block_mirror.unwrap_or(false);
        let n = match kind {
            "in" => b.port_counts.as_ref().and_then(|p| p.ins).unwrap_or(1),
            _ => b.port_counts.as_ref().and_then(|p| p.outs).unwrap_or(1),
        }
        .max(index) as f32;
        let total = n * 2.0 + 1.0;
        let dy = r.height() / total;
        let y = r.top() + (2.0 * index as f32 - 0.5) * dy;
        let x = match (kind, mirrored) {
            ("in", false) | ("out", true) => r.left(),
            _ => r.right(),
        };
        Pos2::new(x, y)
    }

    fn block_and_screen_rect(&self, sid: &str) -> Option<(&Block, Rect)> {
        let b = self
            .state
            .current_system()?
            .blocks
            .iter()
            .find(|b| b.sid.as_deref() == Some(sid))?;
        Some((b, self.block_screen_rect(sid)?))
    }

    /// A point inside the canvas that is NOT covered by any block rect —
    /// used to arm `SelectionRect`/`Pan` on background.
    pub fn empty_canvas_pos(&self) -> Pos2 {
        let avail = self.canvas_rect();
        let t = self.transform();
        // Candidate grid across the canvas; return first point outside all blocks.
        for fy in [0.5_f32, 0.75, 0.25, 0.9, 0.1] {
            for fx in [0.5_f32, 0.75, 0.25, 0.9, 0.1] {
                let p = Pos2::new(
                    avail.left() + avail.width() * fx,
                    avail.top() + avail.height() * fy,
                );
                let covered = self
                    .state
                    .current_system()
                    .map(|s| {
                        s.blocks.iter().any(|b| {
                            parse_block_rect(b).is_some_and(|r| {
                                Rect::from_min_max(t.to_screen(r.min), t.to_screen(r.max))
                                    .contains(p)
                            })
                        })
                    })
                    .unwrap_or(false);
                if !covered {
                    return p;
                }
            }
        }
        avail.center()
    }

    /// Screen position of resize `handle` (0=TL … 7=L) on a block's rect —
    /// only valid while the block is selected (handles are drawn then).
    pub fn resize_handle_pos(&self, sid: &str, handle: u8) -> Pos2 {
        let r = self.block_screen_rect(sid).unwrap();
        match handle {
            0 => r.left_top(),
            1 => Pos2::new(r.center().x, r.top()),
            2 => r.right_top(),
            3 => Pos2::new(r.right(), r.center().y),
            4 => r.right_bottom(),
            5 => Pos2::new(r.center().x, r.bottom()),
            6 => r.left_bottom(),
            7 => Pos2::new(r.left(), r.center().y),
            _ => panic!("handle must be 0..=7"),
        }
    }

    // ── introspection ─────────────────────────────────────────────────────

    pub fn shapes(&self) -> &[egui::epaint::ClippedShape] {
        &self.last.as_ref().unwrap().shapes
    }

    pub fn sys(&self) -> &System {
        self.state.current_system().unwrap()
    }

    pub fn selected_blocks(&self) -> &[usize] {
        &self.state.selection.selected_blocks
    }

    pub fn block_position(&self, sid: &str) -> (i32, i32, i32, i32) {
        let b = self
            .sys()
            .blocks
            .iter()
            .find(|b| b.sid.as_deref() == Some(sid))
            .unwrap();
        super::parse_pos(b.position.as_deref().unwrap()).unwrap()
    }

    pub fn drag_mode_is_none(&self) -> bool {
        matches!(self.state.drag_mode, DragMode::None)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// ViewerHarness
// ────────────────────────────────────────────────────────────────────────────

/// Headless viewer (`SubsystemApp`) for navigation/live/dashboard tests.
pub struct ViewerHarness {
    pub ctx: egui::Context,
    pub app: SubsystemApp,
    last: Option<egui::FullOutput>,
}

impl ViewerHarness {
    pub fn new(root: System) -> Self {
        let (charts, chart_map) = no_charts();
        Self {
            ctx: new_ctx(),
            app: SubsystemApp::new(root, Vec::new(), charts, chart_map),
            last: None,
        }
    }

    pub fn frame(&mut self, raw: egui::RawInput) -> &egui::FullOutput {
        let mut out = self.ctx.run_ui(raw, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                rustylink::egui_app::ui::update_with_info(&mut self.app, ui);
            });
        });
        out.textures_delta.clear();
        self.last = Some(out);
        self.last.as_ref().unwrap()
    }

    pub fn idle(&mut self, n: usize) {
        for _ in 0..n {
            self.frame(input::idle());
        }
    }

    pub fn run(&mut self, inputs: impl IntoIterator<Item = egui::RawInput>) {
        for i in inputs {
            self.frame(i);
        }
    }

    pub fn transform(&self) -> ViewTransform {
        self.app
            .last_canvas_transform
            .expect("canvas transform available after ≥1 rendered frame")
    }

    pub fn block_screen_rect(&self, sid: &str) -> Option<Rect> {
        let sys = self.app.current_system()?;
        let r = sys
            .blocks
            .iter()
            .find(|b| b.sid.as_deref() == Some(sid))
            .and_then(parse_block_rect)?;
        let t = self.transform();
        Some(Rect::from_min_max(t.to_screen(r.min), t.to_screen(r.max)))
    }

    #[cfg(feature = "dashboard")]
    pub fn drain_dashboard_events(&mut self) -> Option<rustylink::egui_app::DashboardControlEvent> {
        self.app.pending_dashboard_control.take()
    }

    pub fn shapes(&self) -> &[egui::epaint::ClippedShape] {
        &self.last.as_ref().unwrap().shapes
    }
}
