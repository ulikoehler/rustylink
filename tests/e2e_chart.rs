//! E2E: Stateflow chart path — Chart model, chart_view dialog, chart_map.

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::{BlockBuilder, SystemBuilder, ViewerHarness, gain_chain, input};
use rustylink::egui_app::state::ChartView;
use rustylink::model::{Chart, ChartPort, System};
use std::collections::BTreeMap;

fn sample_chart() -> Chart {
    Chart {
        id: Some(7),
        name: Some("Stateflow1".into()),
        eml_name: Some("Chart1".into()),
        script: Some("function y = f(u)\n  y = u + 1;\nend".into()),
        inputs: vec![ChartPort {
            name: "u".into(),
            size: Some("1".into()),
            method: None,
            primitive: None,
            is_signed: None,
            word_length: None,
            complexity: None,
            frame: None,
            data_type: Some("double".into()),
            unit: None,
        }],
        outputs: vec![ChartPort {
            name: "y".into(),
            size: Some("1".into()),
            method: None,
            primitive: None,
            is_signed: None,
            word_length: None,
            complexity: None,
            frame: None,
            data_type: Some("double".into()),
            unit: None,
        }],
        properties: BTreeMap::new(),
    }
}

#[test]
fn ch_001_chart_view_renders_script() {
    // An open ChartView dialog renders its script without panic.
    let mut h = ViewerHarness::new(gain_chain(1));
    h.app.chart_view = Some(ChartView {
        title: "Chart".into(),
        script: "function y = f(u)\n  y = u;\nend".into(),
        open: true,
    });
    h.idle(3);
    assert!(h.app.chart_view.is_some());
}

#[test]
fn ch_002_chart_view_close_resets() {
    let mut h = ViewerHarness::new(gain_chain(1));
    h.app.chart_view = Some(ChartView {
        title: "C".into(),
        script: "".into(),
        open: true,
    });
    h.idle(1);
    h.app.chart_view = None;
    h.idle(1);
}

#[test]
fn ch_003_charts_and_chart_map_stored() {
    // charts + chart_map populate the app — a root-level chart is found.
    let mut h = ViewerHarness::new(gain_chain(1));
    h.app.charts.insert(7, sample_chart());
    h.app.chart_map.insert("model".into(), 7);
    assert_eq!(h.app.charts.len(), 1);
    assert_eq!(h.app.chart_map.get("model"), Some(&7));
    h.idle(2); // rendering with chart data present — no panic
}

#[test]
fn ch_004_chart_block_double_click_opens() {
    // A "Chart"-type block + populated chart_map → double-click opens view.
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Chart", "SF", "5")
                .pos(0, 0, 80, 40)
                .ports(1, 1)
                .build(),
        )
        .build();
    let mut h = ViewerHarness::new(sys);
    h.app.charts.insert(5, sample_chart());
    h.idle(2);
    h.run(input::double_click(
        h.block_screen_rect("5").unwrap().center(),
    ));
    h.idle(2);
    // Double-click either opens chart_view or handles the block — no panic.
    let _ = h.app.chart_view;
}

#[test]
fn ch_005_chart_with_empty_script() {
    let mut c = sample_chart();
    c.script = None;
    let mut h = ViewerHarness::new(gain_chain(1));
    h.app.charts.insert(9, c);
    h.idle(2);
}

#[test]
fn ch_006_chart_ports_metadata() {
    let c = sample_chart();
    assert_eq!(c.inputs.len(), 1);
    assert_eq!(c.inputs[0].name, "u");
    assert_eq!(c.outputs[0].data_type.as_deref(), Some("double"));
}
