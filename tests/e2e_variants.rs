//! E2E: LiveValueList variants + navigation/subsystem edge cases.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::{ViewerHarness, gain_chain, live_feed};
use rustylink::live_values::{LiveValue, LiveValueList};

fn fmt(v: LiveValue) -> String {
    rustylink::live_values::LiveValueEntry::new(v).formatted_text()
}

#[test]
fn vr_001_float_variants_format() {
    assert!(!fmt(LiveValue::new(vec![1], LiveValueList::Float64(vec![2.5]))).is_empty());
    assert!(!fmt(LiveValue::new(vec![1], LiveValueList::Float32(vec![1.5]))).is_empty());
}

#[test]
fn vr_002_int_variants_format() {
    for list in [
        LiveValueList::Int8(vec![-5]),
        LiveValueList::Int16(vec![-300]),
        LiveValueList::Int32(vec![-70000]),
        LiveValueList::Int64(vec![-9_000_000_000]),
        LiveValueList::UInt8(vec![200]),
        LiveValueList::UInt16(vec![60000]),
        LiveValueList::UInt32(vec![4_000_000_000]),
        LiveValueList::UInt64(vec![18_000_000_000]),
    ] {
        assert!(!fmt(LiveValue::new(vec![1], list)).is_empty());
    }
}

#[test]
fn vr_003_bool_and_string_format() {
    assert!(!fmt(LiveValue::new(vec![1], LiveValueList::Bool(vec![true]))).is_empty());
    assert!(
        !fmt(LiveValue::new(
            vec![1],
            LiveValueList::String(vec!["hello".into()])
        ))
        .is_empty()
    );
    assert!(!fmt(LiveValue::new(vec![1], LiveValueList::Enum(vec![0u64]))).is_empty());
}

#[test]
fn vr_004_bytes_and_empty_format() {
    let b = fmt(LiveValue::new(
        vec![2],
        LiveValueList::Bytes(vec![vec![0xde, 0xad]]),
    ));
    assert!(!b.is_empty());
    let e = fmt(LiveValue::new(vec![0], LiveValueList::Empty));
    let _ = e; // Empty is allowed to render as anything — must not panic
}

#[test]
fn vr_005_complex_formats() {
    let c = fmt(LiveValue::new(
        vec![1],
        LiveValueList::Complex64(vec![rustylink::live_values::LiveComplex64 {
            re: 1.0,
            im: -2.0,
        }]),
    ));
    assert!(!c.is_empty());
}

#[test]
fn vr_010_navigate_to_unknown_path_safe() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.app.navigate_to_path(vec!["no_such".to_string()]);
    h.idle(2); // must not panic — either resets or tolerates
}

#[test]
fn vr_011_live_feed_pushes_multiple_types() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.app
        .live_values
        .insert("k1".into(), live_feed::scalar(1.0));
    h.app
        .live_values
        .insert("k2".into(), live_feed::boolean(true));
    h.app
        .live_values
        .insert("k3".into(), live_feed::scalar(2.0));
    assert_eq!(h.app.live_values.len(), 3, "all three keys stored");
}

#[test]
fn vr_012_live_value_overwrite_replaces() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.app
        .live_values
        .insert("same".into(), live_feed::scalar(1.0));
    h.app
        .live_values
        .insert("same".into(), live_feed::scalar(9.9));
    assert_eq!(h.app.live_values.len(), 1, "same key replaces");
    let v = h.app.live_values.get("same").unwrap();
    assert!(v.formatted_text().contains("9.9"));
}
