//! Scripted live-value producers for LV/PERF tests. All drives are
//! synchronous — rate is an assertion parameter, never a wall-clock sleep.

#![allow(dead_code)]

use rustylink::live_values::{LiveValue, LiveValueEntry, LiveValueList};

/// A scalar `LiveValueEntry` (f64).
pub fn scalar(v: f64) -> LiveValueEntry {
    LiveValueEntry::new(LiveValue::new(vec![1], LiveValueList::Float64(vec![v])))
}

/// A vector `LiveValueEntry` (f64).
pub fn vector(v: Vec<f64>) -> LiveValueEntry {
    let n = v.len() as u32;
    LiveValueEntry::new(LiveValue::new(vec![n], LiveValueList::Float64(v)))
}

/// A boolean entry.
pub fn boolean(v: bool) -> LiveValueEntry {
    LiveValueEntry::new(LiveValue::new(vec![1], LiveValueList::Bool(vec![v])))
}

/// A deterministic sine feed at logical sample index `i`.
pub fn sine(period: f64, amp: f64) -> impl Fn(usize) -> f64 {
    move |i| amp * (i as f64 * std::f64::consts::TAU / period).sin()
}

/// Step: `lo` for i < t0, `hi` afterwards.
pub fn step(t0: usize, lo: f64, hi: f64) -> impl Fn(usize) -> f64 {
    move |i| if i < t0 { lo } else { hi }
}

pub fn constant(v: f64) -> impl Fn(usize) -> f64 {
    move |_| v
}
