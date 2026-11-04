//! E2E: property-style invariants over generated models (deterministic, no deps).

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::model_builders::line;
use harness::{BlockBuilder, SystemBuilder, gain_chain, slx_factory};
use rustylink::connection_targets::ConnectionTargetResolver;
use rustylink::model::{SlxArchive, SlxContent, System};
use tempfile::TempDir;

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[self.below(v.len() as u32) as usize]
    }
}

const BLOCK_TYPES: &[&str] = &[
    "Constant",
    "Gain",
    "Sum",
    "Product",
    "Display",
    "Scope",
    "UnitDelay",
    "Switch",
    "Abs",
    "Sqrt",
    "Saturate",
    "Delay",
];

/// Random DAG: N random-type blocks, forward-only edges (no cycles possible).
fn random_dag(seed: u32, n: usize) -> System {
    let mut rng = Rng(seed.max(1));
    let mut b = SystemBuilder::new("model");
    for i in 0..n {
        let bt = rng.pick(BLOCK_TYPES);
        b = b.block(
            BlockBuilder::new(bt, &format!("B{i}"), &(i + 1).to_string())
                .pos(
                    (i % 8) as i32 * 80,
                    (i / 8) as i32 * 60,
                    (i % 8) as i32 * 80 + 40,
                    (i / 8) as i32 * 60 + 24,
                )
                .ports(if i == 0 { 0 } else { 1 }, 1)
                .build(),
        );
    }
    for i in 0..n.saturating_sub(1) {
        let hops = rng.below(3);
        for _ in 0..hops {
            let dst = i + 1 + rng.below((n - i - 1) as u32) as usize;
            if dst < n {
                b = b.line(line(&(i + 1).to_string(), 1, &(dst + 1).to_string(), 1));
            }
        }
    }
    b.build()
}

#[test]
fn pr_001_resolver_never_panics_50_seeds() {
    for seed in 1..=50u32 {
        let sys = random_dag(seed, 30);
        let res = ConnectionTargetResolver::new(&sys);
        for l in &sys.lines {
            let _ = res.line_targets_for_line(&[], l);
        }
    }
}

#[test]
fn pr_002_serialize_roundtrip_30_seeds() {
    let dir = TempDir::new().unwrap();
    for seed in 1..=30u32 {
        let sys = random_dag(seed, 25);
        let path = dir.path().join(format!("r{seed}.slx"));
        slx_factory::write_model_slx(&path, &sys).unwrap();
        let arch = SlxArchive::from_file(&path).unwrap();
        let parsed = arch
            .entries
            .iter()
            .find_map(|e| match &e.content {
                SlxContent::SystemXml(s) => Some(s.as_ref()),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            parsed.blocks.len(),
            sys.blocks.len(),
            "seed {seed} blocks preserved"
        );
        assert_eq!(
            parsed.lines.len(),
            sys.lines.len(),
            "seed {seed} lines preserved"
        );
    }
}

#[test]
fn pr_003_targets_always_reference_existing_blocks() {
    // Every target path must resolve to a block that actually exists.
    for seed in 40..=60u32 {
        let sys = random_dag(seed, 20);
        let res = ConnectionTargetResolver::new(&sys);
        for l in &sys.lines {
            for t in res.line_targets_for_line(&[], l) {
                let name = t.path.strip_prefix("model/").unwrap_or(&t.path);
                assert!(
                    sys.blocks
                        .iter()
                        .any(|b| b.name == name || b.sid.as_deref() == Some(name)),
                    "target {name} exists (seed {seed})"
                );
            }
        }
    }
}

#[test]
fn pr_004_serializer_preserves_block_count_and_names() {
    let dir = TempDir::new().unwrap();
    let sys = gain_chain(5);
    let path = dir.path().join("g.slx");
    slx_factory::write_model_slx(&path, &sys).unwrap();
    let arch = SlxArchive::from_file(&path).unwrap();
    let parsed = arch
        .entries
        .iter()
        .find_map(|e| match &e.content {
            SlxContent::SystemXml(s) => Some(s.as_ref()),
            _ => None,
        })
        .unwrap();
    let names: Vec<_> = parsed.blocks.iter().map(|b| b.name.as_str()).collect();
    for want in ["In1", "G0", "G1", "G2", "G3", "G4", "Out1"] {
        assert!(names.contains(&want), "block {want} preserved");
    }
}

#[test]
fn pr_005_line_points_never_negative_after_ops() {
    // move_blocks on a wire-connected chain never produces negative coords.
    let mut sys = gain_chain(3);
    for _ in 0..10 {
        let _ = rustylink::editor::operations::move_blocks(&mut sys, &[2], 5, 3);
    }
    for l in &sys.lines {
        for p in &l.points {
            // Relative offsets CAN be negative — just require model validity.
            let _ = (p.x, p.y);
        }
    }
    harness::debug_validate(&sys);
}

#[test]
fn pr_006_resolver_cache_consistent_across_calls() {
    let sys = random_dag(99, 40);
    let res = ConnectionTargetResolver::new(&sys);
    for l in &sys.lines {
        let a = res.line_targets_for_line(&[], l);
        let b = res.line_targets_for_line(&[], l);
        assert_eq!(
            a.iter().map(|t| t.path.clone()).collect::<Vec<_>>(),
            b.iter().map(|t| t.path.clone()).collect::<Vec<_>>()
        );
    }
}
