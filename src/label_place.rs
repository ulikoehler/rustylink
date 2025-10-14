//! Deterministic label placement for polylines with collision avoidance.
//!
//! This module implements a deterministic algorithm to place text labels along a
//! polyline (list of straight segments) such that:
//! - Labels prefer the longest contiguous segment of the line
//! - Labels can be horizontal (for mostly horizontal segments) or vertical (for mostly vertical segments)
//! - We compute the label's bounding box using a provided measurer and ensure labels do not collide
//!   when expanded by a factor (default 1.5) around their center
//! - In case of potential collisions, we slide the label along the selected segment in a predictable
//!   order (0, +step, -step, +2*step, -2*step, ...) and increase the perpendicular offset in steps
//!   to search for a valid location
//! - We add a strong penalty when a label would exceed the segment extents (spill), which encourages
//!   selecting segments where the label actually fits
//!
//! The algorithm is fully deterministic and uses no randomness.
//!
//! Glossary:
//! - Screen space: all inputs are expected to be in a single 2D space where text sizes are provided
//!   in the same units as coordinates.
//! - Segment spill: a measure of how much longer the label is than the segment extent; this is
//!   discouraged via a large penalty.
//!
//! Suggested usage:
//! 1. Build a list of screen-space positions of the polyline points.
//! 2. Provide a measurer that returns the text size for a string, font, and color (the color usually
//!    doesn't affect size but passes through if you need caching by key).
//! 3. Call `place_label` with the polyline and text. Use the returned rectangle to render.

use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2f {
    pub x: f32,
    pub y: f32,
}

impl Vec2f {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectF {
    pub min: Vec2f,
    pub max: Vec2f,
}

impl RectF {
    pub fn from_min_max(min: Vec2f, max: Vec2f) -> Self {
        Self { min, max }
    }
    pub fn center(&self) -> Vec2f {
        Vec2f::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }
    pub fn intersects(&self, other: RectF) -> bool {
        !(self.max.x <= other.min.x
            || other.max.x <= self.min.x
            || self.max.y <= other.min.y
            || other.max.y <= self.min.y)
    }
}

pub trait Measurer {
    /// Return the size of the rendered text (width, height) in the same units as the polyline.
    fn measure(&self, text: &str) -> (f32, f32);
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub expand_factor: f32,
    pub step_fraction: f32,
    pub perp_offset: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            expand_factor: 1.5,
            step_fraction: 0.25, // as a fraction of measured size along the dominant axis
            perp_offset: 6.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlacementResult {
    pub rect: RectF,
    pub horizontal: bool,
}

fn expanded_rect(r: RectF, factor: f32) -> RectF {
    let c = r.center();
    let hw = r.width() * 0.5 * factor;
    let hh = r.height() * 0.5 * factor;
    RectF::from_min_max(
        Vec2f::new(c.x - hw, c.y - hh),
        Vec2f::new(c.x + hw, c.y + hh),
    )
}

/// Place a label along a polyline (list of points in screen space).
/// Returns the chosen rectangle and orientation.
pub fn place_label(
    polyline: &[Vec2f],
    text: &str,
    measurer: &dyn Measurer,
    cfg: Config,
    placed: &[RectF],
) -> Option<PlacementResult> {
    if polyline.len() < 2 {
        return None;
    }

    // Build segments and sort by length descending
    let mut segs: Vec<(usize, Vec2f, Vec2f, f32)> = Vec::new();
    for (i, w) in polyline.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = (dx * dx + dy * dy).sqrt();
        segs.push((i, a, b, len));
    }
    segs.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(Ordering::Equal));

    let mut best_overall: Option<(RectF, bool, f32)> = None; // (rect, horizontal, score)

    for (_i, a, b, _len) in segs.into_iter() {
        let horizontal = (a.y - b.y).abs() <= (a.x - b.x).abs();
        // Measure oriented text
        let oriented_text = if horizontal {
            text.to_string()
        } else {
            text.chars()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join("\n")
        };
        let (w, h) = measurer.measure(&oriented_text);
        let seg_min_x = a.x.min(b.x);
        let seg_max_x = a.x.max(b.x);
        let seg_min_y = a.y.min(b.y);
        let seg_max_y = a.y.max(b.y);
        // Center point of the segment
        let cx = (a.x + b.x) * 0.5;
        let cy = (a.y + b.y) * 0.5;
        let base_off_x = if horizontal { 0.0 } else { cfg.perp_offset };
        let base_off_y = if horizontal { -cfg.perp_offset } else { 0.0 };

        // Allowed extent for center so label stays within segment bounds
        let (min_t, max_t, base_t, step_t, spill) = if horizontal {
            let seg_len = seg_max_x - seg_min_x;
            let half = 0.5 * w; // slide along x
            let min_x = seg_min_x + half;
            let max_x = seg_max_x - half;
            let (minx, maxx, base) = if min_x <= max_x {
                (min_x, max_x, cx.clamp(min_x, max_x))
            } else {
                // Label wider than segment: collapse range to segment center to avoid clamp panic
                let mid = (seg_min_x + seg_max_x) * 0.5;
                (mid, mid, mid)
            };
            let spill = (w - seg_len).max(0.0);
            (
                minx,
                maxx,
                base,
                (w.max(40.0) * cfg.step_fraction).max(1.0),
                spill,
            )
        } else {
            let seg_len = seg_max_y - seg_min_y;
            let half = 0.5 * h; // slide along y
            let min_y = seg_min_y + half;
            let max_y = seg_max_y - half;
            let (miny, maxy, base) = if min_y <= max_y {
                (min_y, max_y, cy.clamp(min_y, max_y))
            } else {
                let mid = (seg_min_y + seg_max_y) * 0.5;
                (mid, mid, mid)
            };
            let spill = (h - seg_len).max(0.0);
            (
                miny,
                maxy,
                base,
                (h.max(20.0) * cfg.step_fraction).max(1.0),
                spill,
            )
        };

        let mut chosen: Option<(RectF, f32)> = None; // (rect, score)
        let mut best_score = f32::INFINITY;
        let max_perp_mult = 5;
        for k in 0..=max_perp_mult {
            let off_x = base_off_x * (1 + k as i32) as f32;
            let off_y = base_off_y * (1 + k as i32) as f32;
            let max_span = (max_t - min_t).abs();
            let mut m = 0usize;
            loop {
                let delta = (m as f32) * step_t;
                let ds: Vec<f32> = if m == 0 {
                    vec![0.0]
                } else {
                    vec![delta, -delta]
                };
                let mut progressed = false;
                for d in ds {
                    let mut ccx = cx;
                    let mut ccy = cy;
                    if horizontal {
                        ccx = (base_t + d).clamp(min_t, max_t);
                    } else {
                        ccy = (base_t + d).clamp(min_t, max_t);
                    }
                    let tl = Vec2f::new(ccx + off_x - w * 0.5, ccy + off_y - h * 0.5);
                    let br = Vec2f::new(tl.x + w, tl.y + h);
                    let rect = RectF::from_min_max(tl, br);
                    let er = expanded_rect(rect, cfg.expand_factor);
                    let mut intersect = false;
                    let mut overlap = 0.0f32;
                    for o in placed {
                        let eo = expanded_rect(*o, cfg.expand_factor);
                        if er.intersects(eo) {
                            intersect = true;
                            // coarse overlap area proxy
                            let ix = (er.max.x - eo.min.x).min(eo.max.x - er.min.x).max(0.0);
                            let iy = (er.max.y - eo.min.y).min(eo.max.y - er.min.y).max(0.0);
                            overlap = overlap.max(ix * iy);
                        }
                    }
                    let center_bias = if m == 0 && k == 0 { -1.0 } else { 0.0 };
                    let spill_bias = spill * 100.0; // strong penalty for spill
                    let score = (if intersect { overlap } else { 0.0 }) + center_bias + spill_bias;
                    if !intersect {
                        if score < best_score {
                            best_score = score;
                            chosen = Some((rect, score));
                        }
                    } else if score < best_score {
                        best_score = score;
                        chosen = Some((rect, score));
                    }
                    progressed = true;
                }
                if m == 0 && best_score.is_finite() && best_score <= 0.0 {
                    break;
                }
                if best_score <= 0.0 {
                    break;
                }
                m += 1;
                if delta > max_span + 1.0 {
                    break;
                }
                if !progressed {
                    break;
                }
            }
            if best_score <= 0.0 {
                break;
            }
        }

        if let Some((rect, score)) = chosen {
            if best_overall.map(|(_, _, s)| score < s).unwrap_or(true) {
                best_overall = Some((rect, horizontal, score));
            }
            if score <= 0.0 {
                break;
            }
        }
    }

    best_overall.map(|(rect, horizontal, _)| PlacementResult { rect, horizontal })
}

// Unit tests were moved to integration tests in `tests/label_place.rs` to
// exercise the public API from the outside and keep implementation details
// private. See `tests/label_place.rs` for the test coverage and examples.
