//! Shared E2E test harness for rustylink.
//!
//! This module is included into each `tests/e2e_*.rs` binary via
//! `#[path = "e2e/harness/mod.rs"] mod harness;` and provides:
//!
//! - [`model_builders`] — fluent `System`/`Block`/`Line` construction.
//! - [`slx_factory`] — synthetic `.slx` archive writer (round-trips through
//!   the real `SlxArchive` writer) plus a raw-zip escape hatch for malformed
//!   archives.
//! - [`input`] — `egui::RawInput` gesture synthesizers (press/drag/release,
//!   clicks, keys, scroll) where every gesture is a frame sequence.
//! - [`gui`] — headless `EditorHarness`/`ViewerHarness` drivers that run real
//!   `editor_update`/`ui::update` frames inside an `egui::Context`.
//! - [`gen_models`] — seeded scale/stress model generators that also return a
//!   [`gen_models::ConnectivityOracle`] of expected resolver output.
//! - Shared assertions ([`assert_targets_eq`], [`assert_line_orthogonal`],
//!   [`debug_validate`]).

#![allow(dead_code)]
#![allow(unused_imports)]

pub mod gen_models;
pub mod gui;
pub mod input;
pub mod live_feed;
pub mod model_builders;
pub mod slx_factory;

pub use gen_models::{ConnectivityOracle, gen_bus_fabric, gen_wide_net};
pub use gui::{EditorHarness, ViewerHarness};
pub use model_builders::{BlockBuilder, SystemBuilder, gain_chain, line, port, subsystem_wrap};

use indexmap::IndexMap;
use rustylink::connection_targets::ConnectionTarget;
use rustylink::model::{Line, Point, System};
use std::collections::HashSet;

// ────────────────────────────────────────────────────────────────────────────
// Shared assertions
// ────────────────────────────────────────────────────────────────────────────

fn norm_path(p: &str) -> String {
    p.replace('.', "/").trim_matches('/').to_string()
}

/// Order-insensitive, path-normalized comparison of resolved targets against
/// expected `(path, signal_name)` pairs — `signal_name` may be `None`.
pub fn assert_targets_eq(actual: &[ConnectionTarget], expected: &[(&str, Option<&str>)]) {
    let mut a: Vec<(String, Option<String>)> = actual
        .iter()
        .map(|t| (norm_path(&t.path), t.signal_name.clone()))
        .collect();
    let mut e: Vec<(String, Option<String>)> = expected
        .iter()
        .map(|(p, s)| (norm_path(p), s.map(str::to_string)))
        .collect();
    a.sort();
    e.sort();
    assert_eq!(a, e, "resolved target mismatch");
}

/// Order-insensitive comparison on paths only (ignores signal_name).
pub fn assert_target_paths_eq(actual: &[ConnectionTarget], expected: &[&str]) {
    let mut a: Vec<String> = actual.iter().map(|t| norm_path(&t.path)).collect();
    let mut e: Vec<String> = expected.iter().map(|p| norm_path(p)).collect();
    a.sort();
    e.sort();
    assert_eq!(a, e, "resolved target path mismatch");
}

/// Every consecutive pair of routing points shares x or y (trunk + branches).
pub fn assert_line_orthogonal(line: &Line) -> bool {
    fn pts_ortho(points: &[Point]) -> bool {
        points
            .windows(2)
            .all(|w| w[0].x == w[1].x || w[0].y == w[1].y)
    }
    fn branch_ortho(branches: &[rustylink::model::Branch]) -> bool {
        branches
            .iter()
            .all(|b| pts_ortho(&b.points) && branch_ortho(&b.branches))
    }
    pts_ortho(&line.points) && branch_ortho(&line.branches)
}

/// Structural sanity every generated/edited model must satisfy:
/// unique SIDs (within this system), valid line endpoints, 1-based port
/// indices, and recursive validation of nested subsystems.
pub fn debug_validate(sys: &System) {
    let mut sids = HashSet::new();
    for b in &sys.blocks {
        if let Some(s) = &b.sid {
            assert!(sids.insert(s.clone()), "duplicate sid {s} ({})", b.name);
        }
    }
    for l in &sys.lines {
        for ep in [&l.src, &l.dst].into_iter().flatten() {
            assert!(
                sids.contains(&ep.sid),
                "line endpoint references missing sid {}",
                ep.sid
            );
            assert!(ep.port_index >= 1, "port index must be 1-based");
        }
        fn check_branch(b: &rustylink::model::Branch, sids: &HashSet<String>) {
            if let Some(ep) = &b.dst {
                assert!(sids.contains(&ep.sid), "branch dst missing sid {}", ep.sid);
            }
            for c in &b.branches {
                check_branch(c, sids);
            }
        }
        for br in &l.branches {
            check_branch(br, &sids);
        }
    }
    for b in &sys.blocks {
        if let Some(sub) = &b.subsystem {
            debug_validate(sub);
        }
    }
}

/// Parse a block's `Position` string "(l, t, r, b)" — mirrors
/// `editor::operations::parse_position` for non-egui contexts.
pub fn parse_pos(pos: &str) -> Option<(i32, i32, i32, i32)> {
    let inner = pos.trim().trim_start_matches('[').trim_end_matches(']');
    let nums: Vec<i32> = inner
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();
    (nums.len() == 4).then(|| (nums[0], nums[1], nums[2], nums[3]))
}

/// Empty `properties` map helper.
pub fn no_props() -> IndexMap<String, String> {
    IndexMap::new()
}
