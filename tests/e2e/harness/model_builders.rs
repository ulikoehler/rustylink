//! Fluent builders for `model::{System, Block, Line, Port, …}`.
//!
//! Builders always produce structurally valid models (unique SIDs, 1-based
//! port indices, `port_counts` consistent with `ports`). Call
//! [`super::debug_validate`] in tests that compose them.

#![allow(dead_code)]

use indexmap::IndexMap;
use rustylink::model::*;
use std::collections::BTreeSet;

pub fn props(entries: &[(&str, &str)]) -> IndexMap<String, String> {
    entries
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

pub fn port(port_type: &str, index: u32) -> Port {
    Port {
        port_type: port_type.to_string(),
        index: Some(index),
        properties: IndexMap::new(),
    }
}

pub fn endpoint(sid: &str, port_type: &str, port_index: u32) -> EndpointRef {
    EndpointRef {
        sid: sid.to_string(),
        port_type: port_type.to_string(),
        port_index,
    }
}

/// Position string in the Simulink `"[l, t, r, b]"` form.
pub fn pos_str(l: i32, t: i32, r: i32, b: i32) -> String {
    format!("[{l}, {t}, {r}, {b}]")
}

// ────────────────────────────────────────────────────────────────────────────
// BlockBuilder
// ────────────────────────────────────────────────────────────────────────────

pub struct BlockBuilder {
    block: Block,
}

impl BlockBuilder {
    pub fn new(block_type: &str, name: &str, sid: &str) -> Self {
        Self {
            block: Block {
                block_type: block_type.to_string(),
                name: name.to_string(),
                sid: Some(sid.to_string()),
                tag_name: "Block".to_string(),
                position: None,
                zorder: None,
                commented: false,
                name_location: NameLocation::Bottom,
                is_matlab_function: false,
                value: None,
                value_kind: ValueKind::default(),
                value_rows: None,
                value_cols: None,
                properties: IndexMap::new(),
                ref_properties: BTreeSet::new(),
                port_counts: None,
                ports: Vec::new(),
                subsystem: None,
                system_ref: None,
                c_function: None,
                instance_data: None,
                link_data: None,
                mask: None,
                annotations: Vec::new(),
                background_color: None,
                show_name: None,
                font_size: None,
                font_weight: None,
                mask_display_text: None,
                current_setting: None,
                block_mirror: None,
                library_source: None,
                library_block_path: None,
                library_missing: false,
                dashboard_binding: None,
                child_order: Vec::new(),
            },
        }
    }

    /// Set `position` (also recorded in `properties["Position"]`).
    pub fn pos(mut self, l: i32, t: i32, r: i32, b: i32) -> Self {
        let s = pos_str(l, t, r, b);
        self.block.position = Some(s.clone());
        self.block.properties.insert("Position".to_string(), s);
        self
    }

    /// Declare `ins`/`outs` via `PortCounts` *and* matching `ports` entries,
    /// so both the parser-facing and canvas-facing views agree.
    pub fn ports(mut self, ins: u32, outs: u32) -> Self {
        self.block.port_counts = Some(PortCounts {
            ins: (ins > 0).then_some(ins),
            outs: (outs > 0).then_some(outs),
            ..Default::default()
        });
        self.block.ports = (1..=ins)
            .map(|i| port("in", i))
            .chain((1..=outs).map(|i| port("out", i)))
            .collect();
        self
    }

    /// Declare control (top-edge) ports.
    pub fn control_ports(mut self, enable: u32, trigger: u32) -> Self {
        let pc = self.block.port_counts.get_or_insert_with(Default::default);
        pc.enable = (enable > 0).then_some(enable);
        pc.trigger = (trigger > 0).then_some(trigger);
        self
    }

    pub fn prop(mut self, k: &str, v: &str) -> Self {
        self.block.properties.insert(k.to_string(), v.to_string());
        self
    }

    pub fn value(mut self, v: &str) -> Self {
        self.block.value = Some(v.to_string());
        self
    }

    pub fn commented(mut self) -> Self {
        self.block.commented = true;
        self.block
            .properties
            .insert("Commented".to_string(), "on".to_string());
        self
    }

    pub fn mirrored(mut self) -> Self {
        self.block.block_mirror = Some(true);
        self
    }

    /// Inline subsystem content (nested `<System>` child).
    pub fn subsystem(mut self, sys: System) -> Self {
        self.block.subsystem = Some(Box::new(sys));
        self
    }

    pub fn build(self) -> Block {
        self.block
    }
}

// ────────────────────────────────────────────────────────────────────────────
// LineBuilder
// ────────────────────────────────────────────────────────────────────────────

pub struct LineBuilder {
    line: Line,
}

impl LineBuilder {
    pub fn new() -> Self {
        Self {
            line: Line {
                name: None,
                zorder: None,
                src: None,
                dst: None,
                points: Vec::new(),
                labels: None,
                branches: Vec::new(),
                properties: IndexMap::new(),
            },
        }
    }

    pub fn name(mut self, n: &str) -> Self {
        self.line.name = Some(n.to_string());
        self
    }

    pub fn from(mut self, sid: &str, port_index: u32) -> Self {
        self.line.src = Some(endpoint(sid, "out", port_index));
        self
    }

    /// Set the primary (trunk) destination.
    pub fn to(mut self, sid: &str, port_index: u32) -> Self {
        self.line.dst = Some(endpoint(sid, "in", port_index));
        self
    }

    /// Append a first-level branch destination.
    pub fn branch_to(mut self, sid: &str, port_index: u32) -> Self {
        self.line.branches.push(Branch {
            name: None,
            zorder: None,
            dst: Some(endpoint(sid, "in", port_index)),
            points: Vec::new(),
            labels: None,
            branches: Vec::new(),
            properties: IndexMap::new(),
        });
        self
    }

    /// Trunk routing points.
    pub fn points(mut self, pts: &[(i32, i32)]) -> Self {
        self.line.points = pts.iter().map(|&(x, y)| Point { x, y }).collect();
        self
    }

    pub fn testpoint(mut self) -> Self {
        self.line
            .properties
            .insert("TestPoint".to_string(), "on".to_string());
        self
    }

    pub fn build(self) -> Line {
        self.line
    }
}

impl Default for LineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick src→dst line: `line("1", 1, "2", 1)`.
pub fn line(src_sid: &str, src_port: u32, dst_sid: &str, dst_port: u32) -> Line {
    LineBuilder::new()
        .from(src_sid, src_port)
        .to(dst_sid, dst_port)
        .build()
}

// ────────────────────────────────────────────────────────────────────────────
// SystemBuilder
// ────────────────────────────────────────────────────────────────────────────

pub struct SystemBuilder {
    sys: System,
}

impl SystemBuilder {
    /// `model_name` feeds `properties["Name"]` — the prefix used in resolved
    /// `ConnectionTarget.path` ("model/Block").
    pub fn new(model_name: &str) -> Self {
        let mut sys = System {
            properties: IndexMap::new(),
            blocks: Vec::new(),
            lines: Vec::new(),
            annotations: Vec::new(),
            chart: None,
        };
        if !model_name.is_empty() {
            sys.properties
                .insert("Name".to_string(), model_name.to_string());
        }
        Self { sys }
    }

    pub fn block(mut self, b: Block) -> Self {
        self.sys.blocks.push(b);
        self
    }

    pub fn line(mut self, l: Line) -> Self {
        self.sys.lines.push(l);
        self
    }

    pub fn build(self) -> System {
        self.sys
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Canonical fixtures
// ────────────────────────────────────────────────────────────────────────────

/// `In1 → G0 → … → G(n-1) → Out1` (model name "model").
/// Block sids: Inport=1, G_i=i+2, Outport=n+2. All blocks ~40×24 at y=0 rows.
pub fn gain_chain(n: usize) -> System {
    let mut b = SystemBuilder::new("model").block(
        BlockBuilder::new("Inport", "In1", "1")
            .pos(0, 0, 30, 24)
            .ports(0, 1)
            .build(),
    );
    for i in 0..n {
        let x = 80 + i as i32 * 100;
        b = b.block(
            BlockBuilder::new("Gain", &format!("G{i}"), &(i + 2).to_string())
                .pos(x, 0, x + 40, 24)
                .ports(1, 1)
                .prop("Gain", "2")
                .build(),
        );
    }
    let out_x = 80 + n as i32 * 100;
    let out_sid = (n + 2).to_string();
    b = b.block(
        BlockBuilder::new("Outport", "Out1", &out_sid)
            .pos(out_x, 0, out_x + 30, 24)
            .ports(1, 0)
            .build(),
    );
    let mut lines = vec![line("1", 1, "2", 1)];
    for i in 0..n.saturating_sub(1) {
        lines.push(line(&(i + 2).to_string(), 1, &(i + 3).to_string(), 1));
    }
    lines.push(line(&(n + 1).to_string(), 1, &out_sid, 1));
    for l in lines {
        b = b.line(l);
    }
    b.build()
}

/// Wrap `inner` in a single SubSystem block at the parent level, wiring the
/// parent's `In`→SS→`Out` chain. The child system gets Inport/Outport
/// boundary blocks automatically when `inner` lacks them.
pub fn subsystem_wrap(inner: System, name: &str, sid: &str) -> System {
    let mut child = inner;
    // Ensure boundary ports exist
    let has_in = child.blocks.iter().any(|b| b.block_type == "Inport");
    let has_out = child.blocks.iter().any(|b| b.block_type == "Outport");
    if !has_in {
        child.blocks.insert(
            0,
            BlockBuilder::new("Inport", "In1", "child_in")
                .pos(0, 0, 30, 24)
                .ports(0, 1)
                .prop("Port", "1")
                .build(),
        );
    }
    if !has_out {
        child.blocks.push(
            BlockBuilder::new("Outport", "Out1", "child_out")
                .pos(300, 0, 330, 24)
                .ports(1, 0)
                .prop("Port", "1")
                .build(),
        );
    }
    SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Constant", "C", "1")
                .pos(0, 0, 40, 24)
                .ports(0, 1)
                .value("1")
                .build(),
        )
        .block(
            BlockBuilder::new("SubSystem", name, sid)
                .pos(100, -10, 200, 34)
                .ports(1, 1)
                .subsystem(child)
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "Sink", "3")
                .pos(260, 0, 300, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, sid, 1))
        .line(line(sid, 1, "3", 1))
        .build()
}
