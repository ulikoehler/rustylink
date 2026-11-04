//! Seeded scale/stress model generators with a [`ConnectivityOracle`] of
//! expected resolver output, so tests never reimplement the resolver.

#![allow(dead_code)]

use super::model_builders::{BlockBuilder, SystemBuilder, line};
use rustylink::model::System;
use std::collections::BTreeMap;

/// Expected connectivity, recorded structurally at generation time.
#[derive(Default, Debug)]
pub struct ConnectivityOracle {
    /// line index → expected target paths (model-relative, "/"-separated).
    pub line_targets: BTreeMap<usize, Vec<String>>,
}

/// `n_signals` parallel chains, each Constant fanning out to `fan_out` Gains:
/// `C{s} → G{s}_{f}`. Line order matches `oracle.line_targets`.
pub fn gen_wide_net(n_signals: usize, fan_out: usize) -> (System, ConnectivityOracle) {
    let mut b = SystemBuilder::new("model");
    let mut oracle = ConnectivityOracle::default();
    let mut sid = 1usize;
    let mut line_idx = 0usize;
    for s in 0..n_signals {
        let src = sid.to_string();
        sid += 1;
        b = b.block(
            BlockBuilder::new("Constant", &format!("C{s}"), &src)
                .pos(0, s as i32 * 40, 40, s as i32 * 40 + 24)
                .ports(0, 1)
                .build(),
        );
        for f in 0..fan_out {
            let g = sid.to_string();
            sid += 1;
            b = b.block(
                BlockBuilder::new("Gain", &format!("G{s}_{f}"), &g)
                    .pos(
                        100 + f as i32 * 100,
                        s as i32 * 40,
                        150 + f as i32 * 100,
                        s as i32 * 40 + 24,
                    )
                    .ports(1, 1)
                    .build(),
            );
            b = b.line(line(&src, 1, &g, 1));
            oracle
                .line_targets
                .insert(line_idx, vec![format!("model/C{s}")]);
            line_idx += 1;
        }
    }
    (b.build(), oracle)
}

/// `n_buses` independent bus round-trips: `w` Constants → BusCreator →
/// BusSelector (selects element `i` via `OutputSignals`) → Display.
/// Oracle: each selector-output line resolves to `model/In{b}_{i}`.
///
/// Layout per bus b (b-th row, y offset = b * 80):
///   In{b}_{0..w-1} → Bus{b} (in 1..w, out 1) → Sel{b} (out 1) → Disp{b}
pub fn gen_bus_fabric(n_buses: usize, width: usize) -> (System, ConnectivityOracle) {
    let mut b = SystemBuilder::new("model");
    let mut oracle = ConnectivityOracle::default();
    let mut sid = 1usize;
    let mut line_idx = 0usize;
    for bidx in 0..n_buses {
        let y = bidx as i32 * 80;
        let mut in_sids = Vec::with_capacity(width);
        for i in 0..width {
            let s = sid.to_string();
            sid += 1;
            in_sids.push(s.clone());
            b = b.block(
                BlockBuilder::new("Constant", &format!("In{bidx}_{i}"), &s)
                    .pos(0, y + i as i32 * 8, 30, y + i as i32 * 8 + 7)
                    .ports(0, 1)
                    .value(&i.to_string())
                    .build(),
            );
        }
        let bus = sid.to_string();
        sid += 1;
        b = b.block(
            BlockBuilder::new("BusCreator", &format!("Bus{bidx}"), &bus)
                .pos(80, y, 110, y + 40)
                .ports(width as u32, 1)
                .build(),
        );
        let sel = sid.to_string();
        sid += 1;
        b = b.block(
            BlockBuilder::new("BusSelector", &format!("Sel{bidx}"), &sel)
                .pos(160, y, 190, y + 40)
                .ports(1, width as u32)
                .prop(
                    "OutputSignals",
                    &(0..width)
                        .map(|i| format!("In{bidx}_{i}"))
                        .collect::<Vec<_>>()
                        .join(","),
                )
                .build(),
        );
        let disp = sid.to_string();
        sid += 1;
        b = b.block(
            BlockBuilder::new("Display", &format!("Disp{bidx}"), &disp)
                .pos(240, y, 280, y + 40)
                .ports(1, 0)
                .build(),
        );
        for (i, in_sid) in in_sids.iter().enumerate() {
            b = b.line(line(in_sid, 1, &bus, (i + 1) as u32));
            oracle
                .line_targets
                .insert(line_idx, vec![format!("model/In{bidx}_{i}")]);
            line_idx += 1;
        }
        b = b.line(line(&bus, 1, &sel, 1));
        // Selector outputs per element; the out:i line resolves to the i-th input.
        let mut sel_expect = Vec::new();
        for i in 0..width {
            sel_expect.push(format!("model/In{bidx}_{i}"));
        }
        oracle.line_targets.insert(line_idx, sel_expect);
        line_idx += 1;
        b = b.line(line(&sel, 1, &disp, 1));
        oracle.line_targets.insert(line_idx, vec![]); // resolved per element; checked case-by-case
        line_idx += 1;
    }
    (b.build(), oracle)
}
