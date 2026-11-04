//! E2E: scale/stress — many signals, resolver throughput, parse timing (§14).

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::{EditorHarness, gen_wide_net, slx_factory};
use rustylink::connection_targets::ConnectionTargetResolver;
use rustylink::model::SlxArchive;
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn perf_001_resolver_1000_signals_bounded() {
    let (sys, oracle) = gen_wide_net(500, 2); // 1000 lines
    harness::debug_validate(&sys);
    let t0 = Instant::now();
    let res = ConnectionTargetResolver::new(&sys);
    let build_ms = t0.elapsed().as_millis();

    let t1 = Instant::now();
    for (idx, _l) in sys.lines.iter().enumerate() {
        let t = res.line_targets_for_line(&[], &sys.lines[idx]);
        assert_eq!(
            t.iter().map(|x| x.path.clone()).collect::<Vec<_>>(),
            oracle.line_targets[&idx]
        );
    }
    let query_ms = t1.elapsed().as_millis();
    eprintln!("resolver: build={build_ms}ms query_all={query_ms}ms");
    assert!(build_ms < 5_000, "resolver build bounded");
    assert!(query_ms < 5_000, "all-queries bounded");
}

#[test]
fn perf_002_slx_write_parse_500_signals() {
    let (sys, _o) = gen_wide_net(200, 2);
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("big.slx");
    slx_factory::write_model_slx(&path, &sys).unwrap();

    let t0 = Instant::now();
    let arch = SlxArchive::from_file(&path).unwrap();
    let parse_ms = t0.elapsed().as_millis();
    let parsed = arch
        .entries
        .iter()
        .find_map(|e| match &e.content {
            rustylink::model::SlxContent::SystemXml(s) => Some(s.as_ref()),
            _ => None,
        })
        .unwrap();
    assert_eq!(parsed.lines.len(), sys.lines.len());
    eprintln!("slx round-trip parse: {parse_ms}ms");
    assert!(parse_ms < 10_000);
}

#[test]
fn perf_003_editor_frames_many_blocks() {
    // 60 sources * 3 + 60 = 240 blocks, 180 lines — frames must complete.
    // Debug-build frame cost scales with block count; keep the model modest
    // so the bound is meaningful on slow CI too.
    let (sys, _o) = gen_wide_net(60, 3);
    let mut h = EditorHarness::new(sys);
    let t0 = Instant::now();
    h.idle(5);
    let ms = t0.elapsed().as_millis();
    eprintln!("5 frames on 240-block model: {ms}ms");
    let _ = h.transform();
    assert!(ms < 20_000, "frame loop bounded");
}
