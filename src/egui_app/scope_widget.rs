//! Miniature scope widget rendered inside `DashboardScope` blocks.
//!
//! Uses the [`liveplot`] crate to draw a small scope/waveform preview inside
//! the block's bounding rectangle in the egui viewer.  A static sine wave is
//! rendered as a placeholder; in a live simulation the trace data would be fed
//! from the model's signal values.

#![cfg(feature = "egui")]

use egui::{Color32, Rect, Ui};

/// State for a single miniature scope instance.
///
/// Each `DashboardScope` block in the model owns one of these.  The liveplot
/// infrastructure manages trace data and scope display; we pre-populate it
/// with a demonstration sine wave.
pub struct MiniScope {
    panel: liveplot::LiveplotPanel,
    traces: liveplot::data::traces::TracesCollection,
}

impl MiniScope {
    /// Create a new `MiniScope` with a single demonstration trace.
    pub fn new(id: impl std::hash::Hash) -> Self {
        let (sink, rx) = liveplot::channel_plot();
        let mut traces = liveplot::data::traces::TracesCollection::new(rx);

        // Create a demonstration sine trace
        let trace = sink.create_trace("signal", None::<&str>);
        let n_points = 200;
        let points: Vec<liveplot::PlotPoint> = (0..n_points)
            .map(|i| {
                let t = i as f64 / n_points as f64 * 4.0 * std::f64::consts::PI;
                let y = t.sin();
                liveplot::PlotPoint { x: t, y }
            })
            .collect();
        sink.send_points(&trace, points).ok();

        // Flush the channel so traces has the data
        traces.update();

        let panel = liveplot::LiveplotPanel::new_with_id(id, 0);

        Self { panel, traces }
    }

    /// Render the miniature scope into the given UI region.
    pub fn show(&mut self, ui: &mut Ui) {
        self.panel
            .render_panel(ui, |_plot_ui, _scope_data, _traces| {}, &mut self.traces);
    }
}

/// Draw a simple waveform glyph (using raw painter strokes) inside the given
/// rectangle.  This is a lightweight fallback that does not depend on the full
/// [`liveplot`] panel infrastructure.
pub fn draw_scope_glyph(ui: &mut Ui, rect: Rect) {
    let inner = rect.shrink(6.0);
    if inner.width() < 10.0 || inner.height() < 10.0 {
        return;
    }

    let painter = ui.painter();
    let color = Color32::from_rgb(50, 200, 50);
    let stroke = egui::Stroke::new(1.5, color);

    // Draw a stylized sine wave
    let n = 60;
    let mut points = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        let x = inner.left() + t * inner.width();
        let y =
            inner.center().y - (t * 2.0 * std::f32::consts::PI * 2.0).sin() * inner.height() * 0.35;
        points.push(egui::pos2(x, y));
    }

    // Draw background
    painter.rect_filled(inner, 2.0, Color32::from_rgb(30, 30, 30));

    // Draw the waveform line
    for w in points.windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }
}
