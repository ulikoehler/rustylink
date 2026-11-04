//! E2E: CLI binary invocation — `rustylink <file>`, `-j`, error paths (§16).

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::{gain_chain, slx_factory};
use std::process::Command;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rustylink"))
}

#[test]
fn cli_001_help_exits_zero() {
    let out = bin().arg("--help").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("SIMULINK_FILE"));
}

#[test]
fn cli_002_version_prints() {
    let out = bin().arg("--version").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("3.1"));
}

#[test]
fn cli_010_parse_slx_prints_json() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("m.slx");
    slx_factory::write_model_slx(&path, &gain_chain(2)).unwrap();

    let out = bin().arg(&path).output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.is_empty());
}

#[test]
fn cli_011_json_flag_produces_valid_json() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("m.slx");
    slx_factory::write_model_slx(&path, &gain_chain(1)).unwrap();

    let out = bin().args(["-j", path.to_str().unwrap()]).output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is valid JSON");
    assert!(v.is_object() || v.is_array());
}

#[test]
fn cli_020_json_mode_missing_file_errors() {
    // `-j` parses the given file → a missing file must fail.
    let out = bin()
        .args(["-j", "/nonexistent/nope.slx"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "missing file must fail in -j mode");
}

#[test]
fn cli_021_json_mode_malformed_slx_errors() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad.slx");
    std::fs::write(&path, b"garbage bytes").unwrap();
    let out = bin().args(["-j", path.to_str().unwrap()]).output().unwrap();
    assert!(!out.status.success());
}

#[test]
fn cli_022_default_linter_tolerates_missing_dir() {
    // Default mode lints ./simulink/*.xml (arg unused) — empty report, exit 0.
    let dir = TempDir::new().unwrap();
    let out = bin()
        .arg("/nonexistent/nope.slx")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "linter mode is tolerant");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("unknown_block_types"));
}

#[test]
fn cli_030_xml_file_directly() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("sys.xml");
    std::fs::write(
        &path,
        r#"<?xml version="1.0"?>
<System>
  <Block BlockType="Constant" Name="C" SID="1"><P Name="Position">[0,0,40,24]</P></Block>
</System>"#,
    )
    .unwrap();
    let out = bin().arg(&path).output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
