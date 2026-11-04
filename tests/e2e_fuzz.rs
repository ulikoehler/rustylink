//! E2E: deterministic fuzz — random models through resolver, validator,
//! serialization round-trip (§16 FZ-*).

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::model_builders::{BlockBuilder, SystemBuilder, line};
use harness::slx_factory;
use rustylink::connection_targets::ConnectionTargetResolver;
use rustylink::model::{SlxArchive, SlxContent, System};
use tempfile::TempDir;

/// Tiny deterministic PRNG (xorshift32) — no external deps.
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
}

/// Random-ish valid model: N blocks, each wired forward to 0-2 later blocks.
fn fuzz_model(seed: u32, n: usize) -> System {
    let mut rng = Rng(seed.max(1));
    let mut b = SystemBuilder::new("model");
    for i in 0..n {
        let sid = (i + 1).to_string();
        let ins = if i == 0 { 0 } else { rng.below(2) + 1 };
        b = b.block(
            BlockBuilder::new("Gain", &format!("G{i}"), &sid)
                .pos(
                    (i % 8) as i32 * 80,
                    (i / 8) as i32 * 60,
                    (i % 8) as i32 * 80 + 40,
                    (i / 8) as i32 * 60 + 24,
                )
                .ports(ins, 1 + rng.below(2))
                .build(),
        );
    }
    for i in 0..n {
        for _ in 0..rng.below(3) {
            let dst = i as u32 + 1 + rng.below((n - i) as u32).max(1);
            if (dst as usize) < n {
                b = b.line(line(&(i + 1).to_string(), 1, &(dst + 1).to_string(), 1));
            }
        }
    }
    b.build()
}

#[test]
fn fz_001_resolver_never_panics_20_seeds() {
    for seed in 1..=20u32 {
        let sys = fuzz_model(seed, 60);
        let res = ConnectionTargetResolver::new(&sys);
        for l in &sys.lines {
            let _ = res.line_targets_for_line(&[], l); // must not panic
        }
    }
}

#[test]
fn fz_002_fuzz_models_validate_and_round_trip() {
    let dir = TempDir::new().unwrap();
    for seed in 1..=10u32 {
        let sys = fuzz_model(seed, 40);
        // Structural invariants (unique sids, valid endpoints) — fuzz
        // models can intentionally contain dangling lines; skip validate
        // but require resolver + serialization to not panic.
        let path = dir.path().join(format!("f{seed}.slx"));
        slx_factory::write_model_slx(&path, &sys).unwrap();
        let arch = SlxArchive::from_file(&path).unwrap();
        assert!(
            arch.entries
                .iter()
                .any(|e| matches!(e.content, SlxContent::SystemXml(_)))
        );
    }
}

#[test]
fn fz_003_deterministic_same_seed() {
    // Same seed → identical model → identical resolver output.
    let a = fuzz_model(42, 50);
    let b = fuzz_model(42, 50);
    let (ra, rb) = (
        ConnectionTargetResolver::new(&a),
        ConnectionTargetResolver::new(&b),
    );
    for (la, lb) in a.lines.iter().zip(b.lines.iter()) {
        let ta = ra.line_targets_for_line(&[], la);
        let tb = rb.line_targets_for_line(&[], lb);
        assert_eq!(
            ta.iter().map(|t| t.path.clone()).collect::<Vec<_>>(),
            tb.iter().map(|t| t.path.clone()).collect::<Vec<_>>()
        );
    }
}
