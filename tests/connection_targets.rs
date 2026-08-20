use indexmap::IndexMap;
use rustylink::connection_targets::{
    ConnectionTarget, ConnectionTargetOrigin, ConnectionTargetResolve, ConnectionTargetResolver,
};
use rustylink::model::{
    Block, Branch, EndpointRef, Line, NameLocation, Point, Port, System, ValueKind,
};

fn block(
    block_type: &str,
    name: &str,
    sid: &str,
    ports: Vec<Port>,
    subsystem: Option<System>,
    properties: &[(&str, &str)],
) -> Block {
    Block {
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
        properties: props(properties),
        ref_properties: Default::default(),
        port_counts: None,
        ports,
        subsystem: subsystem.map(Box::new),
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
        dashboard_binding: None,
        child_order: Vec::new(),
    }
}

fn port(port_type: &str, index: u32, name: Option<&str>) -> Port {
    let mut properties = IndexMap::new();
    if let Some(name) = name {
        properties.insert("Name".to_string(), name.to_string());
    }
    Port {
        port_type: port_type.to_string(),
        index: Some(index),
        properties,
    }
}

fn propagated_port(port_type: &str, index: u32, propagated_signal: &str) -> Port {
    Port {
        port_type: port_type.to_string(),
        index: Some(index),
        properties: IndexMap::from_iter([(
            "PropagatedSignals".to_string(),
            propagated_signal.to_string(),
        )]),
    }
}

fn testpoint_port(port_type: &str, index: u32, name: Option<&str>) -> Port {
    let mut port = port(port_type, index, name);
    port.properties
        .insert("TestPoint".to_string(), "on".to_string());
    port
}

fn line(src_sid: &str, src_port: u32, dst_sid: &str, dst_port: u32, name: Option<&str>) -> Line {
    Line {
        name: name.map(str::to_string),
        zorder: None,
        src: Some(endpoint(src_sid, "out", src_port)),
        dst: Some(endpoint(dst_sid, "in", dst_port)),
        points: vec![Point { x: 0, y: 0 }, Point { x: 10, y: 0 }],
        labels: None,
        branches: Vec::new(),
        properties: IndexMap::new(),
    }
}

/// A line that fans out to several destinations, the way Simulink stores a
/// branched signal: the line itself has no `dst`, every leg is a branch.
fn branched_line(src_sid: &str, src_port: u32, dsts: &[(&str, &str, u32)]) -> Line {
    Line {
        name: None,
        zorder: None,
        src: Some(endpoint(src_sid, "out", src_port)),
        dst: None,
        points: vec![Point { x: 0, y: 0 }, Point { x: 10, y: 0 }],
        labels: None,
        branches: dsts
            .iter()
            .map(|(sid, port_type, port_index)| Branch {
                name: None,
                zorder: None,
                dst: Some(endpoint(sid, port_type, *port_index)),
                points: Vec::new(),
                labels: None,
                branches: Vec::new(),
                properties: IndexMap::new(),
            })
            .collect(),
        properties: IndexMap::new(),
    }
}

fn endpoint(sid: &str, port_type: &str, port_index: u32) -> EndpointRef {
    EndpointRef {
        sid: sid.to_string(),
        port_type: port_type.to_string(),
        port_index,
    }
}

fn props(entries: &[(&str, &str)]) -> IndexMap<String, String> {
    let mut properties = IndexMap::new();
    for (key, value) in entries {
        properties.insert((*key).to_string(), (*value).to_string());
    }
    properties
}

#[test]
fn bus_selector_uses_named_bus_creator_input() {
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block("Constant", "B", "2", vec![port("out", 1, None)], None, &[]),
            block(
                "BusCreator",
                "BusCreator",
                "3",
                vec![
                    port("in", 1, None),
                    port("in", 2, None),
                    port("out", 1, None),
                ],
                None,
                &[],
            ),
            block(
                "BusSelector",
                "BusSelector",
                "4",
                vec![port("in", 1, None), port("out", 1, Some("beta"))],
                None,
                &[],
            ),
            block("Display", "Sink", "5", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![
            line("1", 1, "3", 1, Some("alpha")),
            line("2", 1, "3", 2, Some("beta")),
            line("3", 1, "4", 1, None),
            line("4", 1, "5", 1, None),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[3]);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, "model/B");
    assert_eq!(targets[0].signal_name.as_deref(), Some("beta"));
    assert_eq!(targets[0].origin, ConnectionTargetOrigin::BusSelector);
}

#[test]
fn demux_uses_matching_mux_input_index() {
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block("Constant", "B", "2", vec![port("out", 1, None)], None, &[]),
            block(
                "Mux",
                "Mux",
                "3",
                vec![
                    port("in", 1, None),
                    port("in", 2, None),
                    port("out", 1, None),
                ],
                None,
                &[],
            ),
            block(
                "Demux",
                "Demux",
                "4",
                vec![
                    port("in", 1, None),
                    port("out", 1, None),
                    port("out", 2, None),
                ],
                None,
                &[],
            ),
            block(
                "Display",
                "Sink1",
                "5",
                vec![port("in", 1, None)],
                None,
                &[],
            ),
            block(
                "Display",
                "Sink2",
                "6",
                vec![port("in", 1, None)],
                None,
                &[],
            ),
        ],
        lines: vec![
            line("1", 1, "3", 1, Some("alpha")),
            line("2", 1, "3", 2, Some("beta")),
            line("3", 1, "4", 1, None),
            line("4", 1, "5", 1, None),
            line("4", 2, "6", 1, None),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[4]);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, "model/B");
    assert_eq!(targets[0].signal_name.as_deref(), Some("beta"));
    assert_eq!(targets[0].origin, ConnectionTargetOrigin::Demux);
}

#[test]
fn subsystem_outport_forwards_parent_input_target() {
    let child_system = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Inport",
                "In1",
                "10",
                vec![port("out", 1, None)],
                None,
                &[("Port", "1")],
            ),
            block(
                "Outport",
                "Out1",
                "11",
                vec![port("in", 1, None)],
                None,
                &[("Port", "1")],
            ),
        ],
        lines: vec![line("10", 1, "11", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block(
                "Constant",
                "Src",
                "1",
                vec![port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "SubSystem",
                "Sub",
                "2",
                vec![port("in", 1, None), port("out", 1, None)],
                Some(child_system),
                &[],
            ),
            block("Display", "Sink", "3", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![
            line("1", 1, "2", 1, Some("input")),
            line("2", 1, "3", 1, None),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[1]);

    assert!(targets.iter().any(|target| {
        target.path == "model/Src"
            && target.signal_name.as_deref() == Some("input")
            && target.signals_only
    }));
    assert!(targets.iter().any(|target| {
        target.path == "model/Sub/Out1" && target.signal_name.as_deref() == Some("input")
    }));
    assert!(!targets.iter().any(|target| target.path == "model/Sub/In1"));
}

#[test]
fn mux_does_not_append_explicit_names_to_forwarded_boundary_paths() {
    let child_system = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Inport",
                "In1",
                "10",
                vec![port("out", 1, None)],
                None,
                &[("Port", "1")],
            ),
            block(
                "Outport",
                "Out1",
                "11",
                vec![port("in", 1, None)],
                None,
                &[("Port", "1")],
            ),
        ],
        lines: vec![line("10", 1, "11", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block(
                "Constant",
                "Src",
                "1",
                vec![port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "SubSystem",
                "Sub",
                "2",
                vec![port("in", 1, None), port("out", 1, None)],
                Some(child_system),
                &[],
            ),
            block(
                "Constant",
                "Other",
                "3",
                vec![port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "Mux",
                "Mux",
                "4",
                vec![
                    port("in", 1, None),
                    port("in", 2, None),
                    port("out", 1, None),
                ],
                None,
                &[],
            ),
            block("Display", "Sink", "5", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![
            line("1", 1, "2", 1, Some("input")),
            line("2", 1, "4", 1, Some("alpha")),
            line("3", 1, "4", 2, Some("beta")),
            line("4", 1, "5", 1, None),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[3]);

    assert!(
        targets.iter().any(|target| {
            target.path == "model/Sub/Out1"
                && target.signal_name.as_deref() == Some("alpha")
                && target.origin == ConnectionTargetOrigin::Mux
        }),
        "targets: {targets:?}"
    );
    assert!(
        !targets
            .iter()
            .any(|target| target.path == "model/Sub/Out1/alpha")
    );
}

#[test]
fn subsystem_block_includes_direct_internal_block_targets_only() {
    let nested_system = System {
        properties: IndexMap::new(),
        blocks: vec![block(
            "Constant",
            "Deep",
            "20",
            vec![port("out", 1, None)],
            None,
            &[],
        )],
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    };

    let child_system = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Constant",
                "InnerDirect",
                "10",
                vec![port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "SubSystem",
                "Nested",
                "11",
                vec![port("in", 1, None), port("out", 1, None)],
                Some(nested_system),
                &[],
            ),
        ],
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    };

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![block(
            "SubSystem",
            "Sub",
            "1",
            vec![port("in", 1, None), port("out", 1, None)],
            Some(child_system),
            &[],
        )],
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.block_targets_for_block(&[], &system.blocks[0]);

    assert!(targets.iter().any(|target| {
        target.path == "model/Sub/InnerDirect" && target.origin == ConnectionTargetOrigin::Internal
    }));
    assert!(targets.iter().any(|target| {
        target.path == "model/Sub/Nested" && target.origin == ConnectionTargetOrigin::Internal
    }));
    assert!(
        !targets
            .iter()
            .any(|target| target.path == "model/Sub/Nested/Deep")
    );
}

#[test]
fn dashboard_signal_bindings_are_marked_signals_only() {
    let mut signal_block = block("DisplayBlock", "Gauge", "1", vec![], None, &[]);
    signal_block.dashboard_binding = Some(rustylink::model::DashboardBinding::SignalSpec {
        block_path: "Source".to_string(),
        signal_name: "sig".to_string(),
        target_path: rustylink::model::DashboardTargetPath::default(),
        uuid: "uuid-1".to_string(),
    });

    let mut param_block = block("KnobBlock", "Knob", "2", vec![], None, &[]);
    param_block.dashboard_binding = Some(rustylink::model::DashboardBinding::ParamSource {
        block_path: "ParamBlock".to_string(),
        param_name: "Value".to_string(),
        target_path: rustylink::model::DashboardTargetPath::default(),
        uuid: "uuid-2".to_string(),
    });

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![signal_block, param_block],
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let signal_targets = resolver.block_targets_for_block(&[], &system.blocks[0]);
    let param_targets = resolver.block_targets_for_block(&[], &system.blocks[1]);

    assert!(signal_targets.iter().any(|target| {
        target.origin == ConnectionTargetOrigin::DashboardBinding
            && target.path == "model/Source"
            && target.signals_only
    }));
    assert!(param_targets.iter().any(|target| {
        target.origin == ConnectionTargetOrigin::DashboardBinding
            && target.path == "model/ParamBlock"
            && !target.signals_only
    }));
}

#[test]
fn dashboard_binding_target_uses_binding_payload() {
    let source = block(
        "Gain",
        "Source",
        "1",
        vec![
            port("out", 1, Some("other")),
            port("out", 2, Some("src_signal")),
        ],
        None,
        &[],
    );
    let sink = block(
        "Terminator",
        "Sink",
        "2",
        vec![port("in", 1, None)],
        None,
        &[],
    );
    let mut dashboard = block("DisplayBlock", "Gauge", "3", vec![], None, &[]);
    dashboard.dashboard_binding = Some(rustylink::model::DashboardBinding::SignalSpec {
        block_path: "Source".to_string(),
        signal_name: "src_signal".to_string(),
        target_path: rustylink::model::DashboardTargetPath {
            port_index: Some(2),
            ..Default::default()
        },
        uuid: "uuid-propagate".to_string(),
    });

    let mut line = line("1", 2, "2", 1, Some("src_signal"));
    line.properties
        .insert("TestPoint".to_string(), "on".to_string());

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![source, sink, dashboard],
        lines: vec![line],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let dashboard_targets = resolver.block_targets_for_block(&[], &system.blocks[2]);
    let dashboard_target = dashboard_targets
        .iter()
        .find(|target| target.origin == ConnectionTargetOrigin::DashboardBinding)
        .expect("dashboard target");

    assert_eq!(dashboard_target.path, "model/Source");
    assert_eq!(dashboard_target.signal_name.as_deref(), Some("src_signal"));
    assert_eq!(
        dashboard_target.resolve,
        Some(ConnectionTargetResolve::TargetPath(
            rustylink::model::DashboardTargetPath {
                port_index: Some(2),
                ..Default::default()
            }
        ))
    );
    assert_eq!(dashboard_target.element_index, Some(2));
    assert!(!dashboard_target.testpoint);
    assert!(dashboard_target.signals_only);
}

#[test]
fn dashboard_binding_target_propagates_source_index_without_incoming_line() {
    let source = block(
        "ComplexToRealImag",
        "Complex to Real-Imag1",
        "1",
        vec![port("out", 1, Some("re")), port("out", 2, Some("im"))],
        None,
        &[],
    );
    let mut dashboard = block("DisplayBlock", "Gauge", "2", vec![], None, &[]);
    dashboard.dashboard_binding = Some(rustylink::model::DashboardBinding::SignalSpec {
        block_path: "Complex to Real-Imag1".to_string(),
        signal_name: "im".to_string(),
        target_path: rustylink::model::DashboardTargetPath {
            port_index: Some(2),
            ..Default::default()
        },
        uuid: "uuid-dashboard-only".to_string(),
    });

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![source, dashboard],
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let dashboard_targets = resolver.block_targets_for_block(&[], &system.blocks[1]);
    let dashboard_target = dashboard_targets
        .iter()
        .find(|target| target.origin == ConnectionTargetOrigin::DashboardBinding)
        .expect("dashboard target");

    assert_eq!(dashboard_target.path, "model/Complex to Real-Imag1");
    assert_eq!(dashboard_target.signal_name.as_deref(), Some("im"));
    assert_eq!(
        dashboard_target.resolve,
        Some(ConnectionTargetResolve::TargetPath(
            rustylink::model::DashboardTargetPath {
                port_index: Some(2),
                ..Default::default()
            }
        ))
    );
    assert_eq!(dashboard_target.element_index, Some(2));
    assert!(dashboard_target.signals_only);
}

#[test]
fn dashboard_binding_target_path_index_wins_over_same_path_signal_name_match() {
    let source = block(
        "Demux",
        "Source",
        "1",
        vec![
            port("out", 1, Some("requested_signal")),
            port("out", 2, None),
        ],
        None,
        &[],
    );
    let mut dashboard = block("DisplayBlock", "Gauge", "2", vec![], None, &[]);
    dashboard.dashboard_binding = Some(rustylink::model::DashboardBinding::SignalSpec {
        block_path: "Source".to_string(),
        signal_name: "requested_signal".to_string(),
        target_path: rustylink::model::DashboardTargetPath {
            port_index: Some(2),
            ..Default::default()
        },
        uuid: "uuid-index-wins".to_string(),
    });

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![source, dashboard],
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let dashboard_targets = resolver.block_targets_for_block(&[], &system.blocks[1]);
    let dashboard_target = dashboard_targets
        .iter()
        .find(|target| target.origin == ConnectionTargetOrigin::DashboardBinding)
        .expect("dashboard target");

    assert_eq!(dashboard_target.path, "model/Source");
    assert_eq!(dashboard_target.element_index, Some(2));
    assert_eq!(
        dashboard_target.signal_name.as_deref(),
        Some("requested_signal")
    );
    assert_eq!(
        dashboard_target.resolve,
        Some(ConnectionTargetResolve::TargetPath(
            rustylink::model::DashboardTargetPath {
                port_index: Some(2),
                ..Default::default()
            }
        ))
    );
    assert!(dashboard_target.signals_only);
}

#[test]
fn base_line_targets_preserve_source_port_testpoint() {
    let mut source = block(
        "Constant",
        "Source",
        "1",
        vec![port("out", 1, Some("sig"))],
        None,
        &[],
    );
    source.ports[0]
        .properties
        .insert("TestPoint".to_string(), "on".to_string());

    let sink = block("Display", "Sink", "2", vec![port("in", 1, None)], None, &[]);
    let wire = line("1", 1, "2", 1, Some("sig"));
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![source, sink],
        lines: vec![wire.clone()],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &wire);

    assert!(targets.iter().any(|target| target.testpoint));
    assert!(targets.iter().any(|target| {
        target.path == "model/Source" && target.signal_name.as_deref() == Some("sig")
    }));
}

#[test]
fn base_line_targets_use_line_testpoint_property() {
    let source = block(
        "Constant",
        "Source",
        "1",
        vec![port("out", 1, None)],
        None,
        &[],
    );
    let sink = block("Display", "Sink", "2", vec![port("in", 1, None)], None, &[]);
    let mut wire = line("1", 1, "2", 1, None);
    wire.properties
        .insert("TestPoint".to_string(), "on".to_string());
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![source, sink],
        lines: vec![wire.clone()],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &wire);

    assert!(targets.iter().any(|target| target.testpoint));
}

#[test]
fn canonical_signal_paths_strip_newlines_and_merge_testpoints() {
    let source = block(
        "Constant",
        "Source\nBlock",
        "1",
        vec![port("out", 1, Some("sig\nname"))],
        None,
        &[],
    );
    let sink = block("Display", "Sink", "2", vec![port("in", 1, None)], None, &[]);
    let wire = line("1", 1, "2", 1, None);
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![source, sink],
        lines: vec![wire.clone()],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let mut targets = resolver.line_targets_for_line(&[], &wire);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, "model/Source Block");
    assert_eq!(targets[0].signal_name, None);

    targets.push(ConnectionTarget {
        path: "model/Source Block".to_string(),
        signal_name: None,
        signal_names: Vec::new(),
        resolve: None,
        element_index: None,
        origin: ConnectionTargetOrigin::SourceBlock,
        signals_only: true,
        testpoint: true,
        block_type: None,
    });
    let deduped = rustylink::connection_targets::dedup_targets(targets);
    assert_eq!(deduped.len(), 1);
    assert!(deduped[0].testpoint);
}

#[test]
fn base_line_targets_do_not_invent_signal_names_without_explicit_line_name() {
    let source = block(
        "Constant",
        "Source",
        "1",
        vec![port("out", 1, Some("sig name"))],
        None,
        &[],
    );
    let sink = block("Display", "Sink", "2", vec![port("in", 1, None)], None, &[]);
    let wire = line("1", 1, "2", 1, None);
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![source, sink],
        lines: vec![wire.clone()],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &wire);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, "model/Source");
    assert_eq!(targets[0].signal_name, None);
}

#[test]
fn base_line_targets_set_output_index_for_multi_output_source() {
    let source = block(
        "ComplexToRealImag",
        "ComplexToRealImag",
        "1",
        vec![
            port("in", 1, None),
            port("out", 1, None),
            port("out", 2, None),
        ],
        None,
        &[],
    );
    let sink = block("Display", "Sink", "2", vec![port("in", 1, None)], None, &[]);
    let wire = line("1", 2, "2", 1, None);
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![source, sink],
        lines: vec![wire.clone()],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &wire);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, "model/ComplexToRealImag");
    assert_eq!(targets[0].element_index, Some(2));
}

#[test]
fn bus_selector_ignores_propagated_signal_fallbacks() {
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block("Constant", "B", "2", vec![port("out", 1, None)], None, &[]),
            block(
                "BusCreator",
                "BusCreator",
                "3",
                vec![
                    port("in", 1, None),
                    port("in", 2, None),
                    port("out", 1, None),
                ],
                None,
                &[],
            ),
            block(
                "BusSelector",
                "BusSelector",
                "4",
                vec![port("in", 1, None), propagated_port("out", 1, "beta")],
                None,
                &[],
            ),
            block("Display", "Sink", "5", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![
            line("1", 1, "3", 1, Some("alpha")),
            line("2", 1, "3", 2, Some("beta")),
            line("3", 1, "4", 1, None),
            line("4", 1, "5", 1, None),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[3]);

    assert!(targets.is_empty(), "targets: {targets:?}");
}

#[test]
fn bus_selector_uses_explicit_output_line_name_for_target_paths() {
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block("Constant", "B", "2", vec![port("out", 1, None)], None, &[]),
            block(
                "BusCreator",
                "BusCreator",
                "3",
                vec![
                    port("in", 1, None),
                    port("in", 2, None),
                    port("out", 1, None),
                ],
                None,
                &[],
            ),
            block(
                "BusSelector",
                "BusSelector",
                "4",
                vec![port("in", 1, None), port("out", 1, None)],
                None,
                &[],
            ),
            block("Display", "Sink", "5", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![
            line("1", 1, "3", 1, Some("alpha")),
            line("2", 1, "3", 2, Some("beta")),
            line("3", 1, "4", 1, None),
            line("4", 1, "5", 1, Some("beta")),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[3]);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, "model/B");
    assert_eq!(targets[0].signal_name.as_deref(), Some("beta"));
    assert_eq!(targets[0].origin, ConnectionTargetOrigin::BusSelector);
    assert_eq!(
        targets[0].resolve,
        Some(ConnectionTargetResolve::Signal("beta".to_string()))
    );
}

#[test]
fn bus_selector_matches_angled_line_names_against_bus_creator_inputs() {
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block("Constant", "B", "2", vec![port("out", 1, None)], None, &[]),
            block(
                "BusCreator",
                "BusCreator",
                "3",
                vec![
                    port("in", 1, None),
                    port("in", 2, None),
                    port("out", 1, None),
                ],
                None,
                &[],
            ),
            block(
                "BusSelector",
                "BusSelector",
                "4",
                vec![port("in", 1, None), port("out", 1, None)],
                None,
                &[],
            ),
            block("Display", "Sink", "5", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![
            line("1", 1, "3", 1, Some("a")),
            line("2", 1, "3", 2, Some("signal1")),
            line("3", 1, "4", 1, None),
            line("4", 1, "5", 1, Some("<signal1>")),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[3]);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, "model/B");
    assert_eq!(
        targets[0].resolve,
        Some(ConnectionTargetResolve::Signal("signal1".to_string()))
    );
}

#[test]
fn demux_propagates_names_and_testpoints_back_upstream_and_strips_indices_after_split() {
    let source = block(
        "Constant",
        "Source",
        "1",
        vec![port("out", 1, None)],
        None,
        &[],
    );
    let mux = block(
        "Mux",
        "Mux",
        "2",
        vec![port("in", 1, None), port("out", 1, None)],
        None,
        &[],
    );
    let demux = block(
        "Demux",
        "Demux",
        "3",
        vec![port("in", 1, None), port("out", 1, None)],
        None,
        &[],
    );
    let sink = block("Display", "Sink", "4", vec![port("in", 1, None)], None, &[]);

    let upstream = line("1", 1, "2", 1, None);
    let middle = line("2", 1, "3", 1, None);
    let mut downstream = line("3", 1, "4", 1, Some("selected"));
    downstream
        .properties
        .insert("TestPoint".to_string(), "on".to_string());

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![source, mux, demux, sink],
        lines: vec![upstream.clone(), middle, downstream.clone()],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let upstream_targets = resolver.line_targets_for_line(&[], &upstream);
    let downstream_targets = resolver.line_targets_for_line(&[], &downstream);

    assert_eq!(upstream_targets.len(), 1);
    assert_eq!(upstream_targets[0].path, "model/Source");
    assert_eq!(upstream_targets[0].signal_name.as_deref(), Some("selected"));
    assert!(upstream_targets[0].testpoint);

    assert_eq!(downstream_targets.len(), 1);
    assert_eq!(downstream_targets[0].path, "model/Source");
    assert_eq!(
        downstream_targets[0].signal_name.as_deref(),
        Some("selected")
    );
    assert_eq!(downstream_targets[0].origin, ConnectionTargetOrigin::Demux);
    assert_eq!(downstream_targets[0].element_index, None);
    assert!(downstream_targets[0].testpoint);
}

#[test]
fn upstream_propagation_preserves_explicit_local_line_names() {
    let source = block(
        "Constant",
        "Source",
        "1",
        vec![port("out", 1, None)],
        None,
        &[],
    );
    let mux = block(
        "Mux",
        "Mux",
        "2",
        vec![port("in", 1, None), port("out", 1, None)],
        None,
        &[],
    );
    let demux = block(
        "Demux",
        "Demux",
        "3",
        vec![port("in", 1, None), port("out", 1, None)],
        None,
        &[],
    );
    let sink = block("Display", "Sink", "4", vec![port("in", 1, None)], None, &[]);

    let upstream = line("1", 1, "2", 1, Some("local"));
    let middle = line("2", 1, "3", 1, None);
    let downstream = line("3", 1, "4", 1, Some("remote"));

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![source, mux, demux, sink],
        lines: vec![upstream.clone(), middle, downstream],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let upstream_targets = resolver.line_targets_for_line(&[], &upstream);

    assert_eq!(upstream_targets.len(), 1);
    assert_eq!(upstream_targets[0].signal_name.as_deref(), Some("local"));
}

#[test]
fn subsystem_input_line_receives_child_metadata_upstream() {
    let mut child_wire = line("10", 1, "11", 1, Some("child_name"));
    child_wire
        .properties
        .insert("TestPoint".to_string(), "on".to_string());
    let child_system = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Inport",
                "In1",
                "10",
                vec![port("out", 1, None)],
                None,
                &[("Port", "1")],
            ),
            block(
                "Outport",
                "Out1",
                "11",
                vec![port("in", 1, None)],
                None,
                &[("Port", "1")],
            ),
        ],
        lines: vec![child_wire],
        annotations: Vec::new(),
        chart: None,
    };

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block(
                "Constant",
                "Src",
                "1",
                vec![port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "SubSystem",
                "Sub",
                "2",
                vec![port("in", 1, None), port("out", 1, None)],
                Some(child_system),
                &[],
            ),
        ],
        lines: vec![line("1", 1, "2", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[0]);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].signal_name.as_deref(), Some("child_name"));
    assert!(targets[0].testpoint);
}

#[test]
fn subsystem_output_metadata_flows_back_into_child_outport_line() {
    let child_system = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Inport",
                "In1",
                "10",
                vec![port("out", 1, None)],
                None,
                &[("Port", "1")],
            ),
            block(
                "Outport",
                "Out1",
                "11",
                vec![port("in", 1, None)],
                None,
                &[("Port", "1")],
            ),
        ],
        lines: vec![line("10", 1, "11", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let mut parent_output = line("2", 1, "3", 1, Some("outer_name"));
    parent_output
        .properties
        .insert("TestPoint".to_string(), "on".to_string());
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block(
                "SubSystem",
                "Sub",
                "2",
                vec![port("in", 1, None), port("out", 1, None)],
                Some(child_system),
                &[],
            ),
            block("Display", "Sink", "3", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![parent_output],
        annotations: Vec::new(),
        chart: None,
    };

    let child_line = system.blocks[0]
        .subsystem
        .as_ref()
        .and_then(|child| child.lines.first())
        .cloned()
        .expect("child line");
    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&["Sub".to_string()], &child_line);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].signal_name.as_deref(), Some("outer_name"));
    assert!(targets[0].testpoint);
}

#[test]
fn subsystem_named_child_line_keeps_name_and_inherits_outer_testpoint() {
    let child_system = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Inport",
                "In1",
                "10",
                vec![port("out", 1, None)],
                None,
                &[("Port", "1")],
            ),
            block(
                "Outport",
                "Out1",
                "11",
                vec![port("in", 1, None)],
                None,
                &[("Port", "1")],
            ),
        ],
        lines: vec![line("10", 1, "11", 1, Some("inner_name"))],
        annotations: Vec::new(),
        chart: None,
    };

    let mut parent_output = line("2", 1, "3", 1, Some("outer_name"));
    parent_output
        .properties
        .insert("TestPoint".to_string(), "on".to_string());
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block(
                "SubSystem",
                "Sub",
                "2",
                vec![port("in", 1, None), port("out", 1, None)],
                Some(child_system),
                &[],
            ),
            block("Display", "Sink", "3", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![parent_output],
        annotations: Vec::new(),
        chart: None,
    };

    let child_line = system.blocks[0]
        .subsystem
        .as_ref()
        .and_then(|child| child.lines.first())
        .cloned()
        .expect("child line");
    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&["Sub".to_string()], &child_line);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].signal_name.as_deref(), Some("inner_name"));
    assert!(targets[0].testpoint);
}

#[test]
fn subsystem_child_line_inherits_outer_testpoint_when_outer_line_is_unnamed() {
    let child_system = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Inport",
                "In1",
                "10",
                vec![port("out", 1, None)],
                None,
                &[("Port", "1")],
            ),
            block(
                "Outport",
                "Out1",
                "11",
                vec![port("in", 1, None)],
                None,
                &[("Port", "1")],
            ),
        ],
        lines: vec![line("10", 1, "11", 1, Some("inner_name"))],
        annotations: Vec::new(),
        chart: None,
    };

    let mut parent_output = line("2", 1, "3", 1, None);
    parent_output
        .properties
        .insert("TestPoint".to_string(), "on".to_string());
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block(
                "SubSystem",
                "Sub",
                "2",
                vec![port("in", 1, None), port("out", 1, None)],
                Some(child_system),
                &[],
            ),
            block("Display", "Sink", "3", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![parent_output],
        annotations: Vec::new(),
        chart: None,
    };

    let child_line = system.blocks[0]
        .subsystem
        .as_ref()
        .and_then(|child| child.lines.first())
        .cloned()
        .expect("child line");
    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&["Sub".to_string()], &child_line);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].signal_name.as_deref(), Some("inner_name"));
    assert!(targets[0].testpoint);
}

#[test]
fn bus_selector_uses_default_signal_name_for_unnamed_bus_input() {
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block(
                "BusCreator",
                "BusCreator",
                "2",
                vec![port("in", 1, None), port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "BusSelector",
                "BusSelector",
                "3",
                vec![port("in", 1, None), port("out", 1, None)],
                None,
                &[],
            ),
            block("Display", "Sink", "4", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![
            line("1", 1, "2", 1, None),
            line("2", 1, "3", 1, None),
            line("3", 1, "4", 1, None),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[2]);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].path, "model/A");
    assert_eq!(
        targets[0].resolve,
        Some(ConnectionTargetResolve::Signal("signal1".to_string()))
    );
}

#[test]
fn bus_selector_output_port_testpoint_is_visible_on_output_line() {
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block(
                "BusCreator",
                "BusCreator",
                "2",
                vec![port("in", 1, None), port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "BusSelector",
                "BusSelector",
                "3",
                vec![
                    port("in", 1, None),
                    testpoint_port("out", 1, Some("signal1")),
                ],
                None,
                &[],
            ),
            block("Display", "Sink", "4", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![
            line("1", 1, "2", 1, Some("signal1")),
            line("2", 1, "3", 1, None),
            line("3", 1, "4", 1, None),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[2]);

    assert_eq!(targets.len(), 1);
    assert!(targets[0].testpoint);
    assert_eq!(targets[0].origin, ConnectionTargetOrigin::BusSelector);
}

#[test]
fn bus_selector_output_testpoint_propagates_back_to_bus_creator_inputs() {
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block(
                "BusCreator",
                "BusCreator",
                "2",
                vec![port("in", 1, None), port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "BusSelector",
                "BusSelector",
                "3",
                vec![
                    port("in", 1, None),
                    testpoint_port("out", 1, Some("signal1")),
                ],
                None,
                &[],
            ),
            block("Display", "Sink", "4", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![
            line("1", 1, "2", 1, None),
            line("2", 1, "3", 1, None),
            line("3", 1, "4", 1, None),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let upstream_targets = resolver.line_targets_for_line(&[], &system.lines[0]);
    let downstream_targets = resolver.line_targets_for_line(&[], &system.lines[2]);

    assert_eq!(upstream_targets.len(), 1);
    assert!(upstream_targets[0].testpoint);
    assert_eq!(downstream_targets.len(), 1);
    assert!(downstream_targets[0].testpoint);
}

#[test]
fn demux_output_port_testpoint_propagates_back_to_mux_input() {
    let source = block(
        "Constant",
        "Source",
        "1",
        vec![port("out", 1, None)],
        None,
        &[],
    );
    let mux = block(
        "Mux",
        "Mux",
        "2",
        vec![port("in", 1, None), port("out", 1, None)],
        None,
        &[],
    );
    let demux = block(
        "Demux",
        "Demux",
        "3",
        vec![port("in", 1, None), testpoint_port("out", 1, None)],
        None,
        &[],
    );
    let sink = block("Display", "Sink", "4", vec![port("in", 1, None)], None, &[]);

    let upstream = line("1", 1, "2", 1, None);
    let middle = line("2", 1, "3", 1, None);
    let downstream = line("3", 1, "4", 1, None);

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![source, mux, demux, sink],
        lines: vec![upstream.clone(), middle, downstream.clone()],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let upstream_targets = resolver.line_targets_for_line(&[], &upstream);
    let downstream_targets = resolver.line_targets_for_line(&[], &downstream);

    assert!(upstream_targets[0].testpoint);
    assert!(downstream_targets[0].testpoint);
    assert_eq!(downstream_targets[0].origin, ConnectionTargetOrigin::Demux);
}

#[test]
fn mux_targets_use_resolve_without_element_index() {
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block(
                "Mux",
                "Mux",
                "2",
                vec![port("in", 1, None), port("out", 1, None)],
                None,
                &[],
            ),
            block("Display", "Sink", "3", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![line("1", 1, "2", 1, None), line("2", 1, "3", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[1]);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].resolve, Some(ConnectionTargetResolve::Index(1)));
    assert_eq!(targets[0].element_index, None);
    assert_eq!(targets[0].origin, ConnectionTargetOrigin::Mux);
}

#[test]
fn topology_signature_ignores_geometry_but_tracks_topology() {
    use rustylink::connection_targets::model_topology_signature;
    use rustylink::model::Point;

    let make = || System {
        properties: props(&[("Name", "m")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block("Display", "Sink", "2", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![line("1", 1, "2", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let base = make();
    let base_sig = model_topology_signature(&base);

    // Geometry-only edits (block Position/ZOrder, line waypoints) must not
    // change the signature.
    let mut geom = make();
    geom.blocks[0]
        .properties
        .insert("Position".to_string(), "[1, 2, 3, 4]".to_string());
    geom.blocks[0]
        .properties
        .insert("ZOrder".to_string(), "99".to_string());
    geom.lines[0].points = vec![Point { x: 5, y: 5 }, Point { x: 50, y: 50 }];
    assert_eq!(
        base_sig,
        model_topology_signature(&geom),
        "geometry-only edits should not change the topology signature"
    );

    // Topology edits must change the signature.
    let mut renamed = make();
    renamed.blocks[0].name = "A2".to_string();
    assert_ne!(base_sig, model_topology_signature(&renamed));

    let mut rewired = make();
    rewired.lines[0] = line("1", 1, "2", 2, None);
    assert_ne!(base_sig, model_topology_signature(&rewired));

    let mut retyped = make();
    retyped.blocks[0].block_type = "Gain".to_string();
    assert_ne!(base_sig, model_topology_signature(&retyped));
}

#[test]
fn signal_names_accumulate_through_subsystem_boundaries() {
    let child_system = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Inport",
                "In1",
                "10",
                vec![port("out", 1, None)],
                None,
                &[("Port", "1")],
            ),
            block(
                "Outport",
                "Out1",
                "11",
                vec![port("in", 1, None)],
                None,
                &[("Port", "1")],
            ),
        ],
        lines: vec![line("10", 1, "11", 1, Some("sig_inner"))],
        annotations: Vec::new(),
        chart: None,
    };

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block(
                "Constant",
                "Src",
                "1",
                vec![port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "SubSystem",
                "Sub",
                "2",
                vec![port("in", 1, None), port("out", 1, None)],
                Some(child_system),
                &[],
            ),
            block("Display", "Sink", "3", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![
            line("1", 1, "2", 1, Some("sig_outer")),
            line("2", 1, "3", 1, Some("sig_result")),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[1]);

    let src = targets
        .iter()
        .find(|target| target.path == "model/Src")
        .unwrap_or_else(|| panic!("no model/Src target: {targets:?}"));
    for name in ["sig_outer", "sig_inner", "sig_result"] {
        assert!(
            src.signal_names.iter().any(|n| n == name),
            "expected alias {name:?} in {:?}",
            src.signal_names
        );
    }
}

#[test]
fn signal_names_accumulate_through_bus_creator() {
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block("Constant", "B", "2", vec![port("out", 1, None)], None, &[]),
            block(
                "BusCreator",
                "BusCreator",
                "3",
                vec![
                    port("in", 1, None),
                    port("in", 2, None),
                    port("out", 1, None),
                ],
                None,
                &[],
            ),
            block("Display", "Sink", "4", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![
            line("1", 1, "3", 1, Some("alpha")),
            line("2", 1, "3", 2, Some("beta")),
            line("3", 1, "4", 1, Some("bus")),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[2]);

    let alpha = targets
        .iter()
        .find(|target| target.path == "model/A")
        .unwrap_or_else(|| panic!("no model/A target: {targets:?}"));
    assert!(
        alpha.signal_names.iter().any(|n| n == "alpha"),
        "expected element name in {:?}",
        alpha.signal_names
    );
    assert!(
        alpha.signal_names.iter().any(|n| n == "bus"),
        "expected bus line name in {:?}",
        alpha.signal_names
    );
}

#[test]
fn signal_names_accumulate_through_mux() {
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block("Constant", "B", "2", vec![port("out", 1, None)], None, &[]),
            block(
                "Mux",
                "Mux",
                "3",
                vec![
                    port("in", 1, None),
                    port("in", 2, None),
                    port("out", 1, None),
                ],
                None,
                &[],
            ),
            block("Display", "Sink", "4", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![
            line("1", 1, "3", 1, Some("alpha")),
            line("2", 1, "3", 2, Some("beta")),
            line("3", 1, "4", 1, Some("muxed")),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[2]);

    let alpha = targets
        .iter()
        .find(|target| target.path == "model/A")
        .unwrap_or_else(|| panic!("no model/A target: {targets:?}"));
    assert!(
        alpha.signal_names.iter().any(|n| n == "alpha"),
        "expected mux element name in {:?}",
        alpha.signal_names
    );
    assert!(
        alpha.signal_names.iter().any(|n| n == "muxed"),
        "expected mux output line name in {:?}",
        alpha.signal_names
    );
}

#[test]
fn subsystem_input_sees_signal_produced_by_another_subsystem() {
    // Producer: a constant driving the subsystem's Outport.
    let producer = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Constant",
                "Src",
                "20",
                vec![port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "Outport",
                "Out1",
                "21",
                vec![port("in", 1, None)],
                None,
                &[("Port", "1")],
            ),
        ],
        lines: vec![line("20", 1, "21", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    // Consumer: the subsystem's Inport driving a sink.
    let consumer = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Inport",
                "In1",
                "30",
                vec![port("out", 1, None)],
                None,
                &[("Port", "1")],
            ),
            block(
                "Display",
                "Sink",
                "31",
                vec![port("in", 1, None)],
                None,
                &[],
            ),
        ],
        lines: vec![line("30", 1, "31", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block(
                "SubSystem",
                "Producer",
                "1",
                vec![port("out", 1, None)],
                Some(producer),
                &[],
            ),
            block(
                "SubSystem",
                "Consumer",
                "2",
                vec![port("in", 1, None)],
                Some(consumer),
                &[],
            ),
        ],
        lines: vec![line("1", 1, "2", 1, Some("bus"))],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    // Inside the consumer the signal must resolve back to the producing
    // subsystem, exactly as it does when the line starts at a root-level block.
    let inner = resolver.line_targets_for_line(
        &["Consumer".to_string()],
        &system.blocks[1].subsystem.as_ref().unwrap().lines[0],
    );
    assert!(
        inner
            .iter()
            .any(|target| target.path == "model/Producer/Out1"),
        "consumer inport lost the producing subsystem: {inner:#?}"
    );
    assert!(
        inner
            .iter()
            .any(|target| target.signal_name.as_deref() == Some("bus")),
        "consumer inport lost the signal name: {inner:#?}"
    );
}

#[test]
fn bus_built_in_one_subsystem_is_selectable_in_the_next() {
    // Producer: two constants joined into a bus behind the subsystem boundary.
    let producer = System {
        properties: IndexMap::new(),
        blocks: vec![
            block("Constant", "A", "20", vec![port("out", 1, None)], None, &[]),
            block("Constant", "B", "21", vec![port("out", 1, None)], None, &[]),
            block(
                "BusCreator",
                "BusCreator",
                "22",
                vec![
                    port("in", 1, None),
                    port("in", 2, None),
                    port("out", 1, None),
                ],
                None,
                &[],
            ),
            block(
                "Outport",
                "Out1",
                "23",
                vec![port("in", 1, None)],
                None,
                &[("Port", "1")],
            ),
        ],
        lines: vec![
            line("20", 1, "22", 1, Some("alpha")),
            line("21", 1, "22", 2, Some("beta")),
            line("22", 1, "23", 1, None),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    // Consumer: pick `beta` back out of the bus that crossed the boundary.
    let consumer = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Inport",
                "In1",
                "30",
                vec![port("out", 1, None)],
                None,
                &[("Port", "1")],
            ),
            block(
                "BusSelector",
                "BusSelector",
                "31",
                vec![port("in", 1, None), port("out", 1, Some("beta"))],
                None,
                &[],
            ),
            block(
                "Display",
                "Sink",
                "32",
                vec![port("in", 1, None)],
                None,
                &[],
            ),
        ],
        lines: vec![line("30", 1, "31", 1, None), line("31", 1, "32", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block(
                "SubSystem",
                "Producer",
                "1",
                vec![port("out", 1, None)],
                Some(producer),
                &[],
            ),
            block(
                "SubSystem",
                "Consumer",
                "2",
                vec![port("in", 1, None)],
                Some(consumer),
                &[],
            ),
        ],
        lines: vec![line("1", 1, "2", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let selected = resolver.line_targets_for_line(
        &["Consumer".to_string()],
        &system.blocks[1].subsystem.as_ref().unwrap().lines[1],
    );
    assert!(
        selected
            .iter()
            .any(|target| target.path == "model/Producer/B"),
        "bus element built in the producer was not selectable: {selected:#?}"
    );
}

#[test]
fn branched_line_feeds_the_subsystem_port_it_actually_ends_at() {
    // Two inports, each wired to its own sink so the two paths stay distinct.
    let child = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Inport",
                "In1",
                "30",
                vec![port("out", 1, None)],
                None,
                &[("Port", "1")],
            ),
            block(
                "Inport",
                "In2",
                "31",
                vec![port("out", 1, None)],
                None,
                &[("Port", "2")],
            ),
            block(
                "Display",
                "Sink1",
                "32",
                vec![port("in", 1, None)],
                None,
                &[],
            ),
            block(
                "Display",
                "Sink2",
                "33",
                vec![port("in", 1, None)],
                None,
                &[],
            ),
        ],
        lines: vec![line("30", 1, "32", 1, None), line("31", 1, "33", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block("Constant", "B", "2", vec![port("out", 1, None)], None, &[]),
            block("Display", "Tap", "4", vec![port("in", 1, None)], None, &[]),
            block(
                "SubSystem",
                "Sub",
                "3",
                vec![port("in", 1, None), port("in", 2, None)],
                Some(child),
                &[],
            ),
        ],
        lines: vec![
            line("2", 1, "3", 1, None),
            // `A` branches to the second input and to a tap.
            branched_line("1", 1, &[("3", "in", 2), ("4", "in", 1)]),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let child_system = system.blocks[3].subsystem.as_ref().unwrap();
    let from_in1 = resolver.line_targets_for_line(&["Sub".to_string()], &child_system.lines[0]);
    let from_in2 = resolver.line_targets_for_line(&["Sub".to_string()], &child_system.lines[1]);

    assert!(
        from_in1.iter().any(|t| t.path == "model/B")
            && !from_in1.iter().any(|t| t.path == "model/A"),
        "port 1 must carry only B: {from_in1:#?}"
    );
    assert!(
        from_in2.iter().any(|t| t.path == "model/A")
            && !from_in2.iter().any(|t| t.path == "model/B"),
        "the branch ends at port 2, so it must carry A: {from_in2:#?}"
    );
}

#[test]
fn control_port_signal_does_not_land_on_the_first_data_port() {
    let child = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "EnablePort",
                "Enable",
                "29",
                vec![port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "Inport",
                "In1",
                "30",
                vec![port("out", 1, None)],
                None,
                &[("Port", "1")],
            ),
            block(
                "Display",
                "Sink",
                "31",
                vec![port("in", 1, None)],
                None,
                &[],
            ),
        ],
        lines: vec![line("30", 1, "31", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let mut enable_line = line("2", 1, "3", 1, None);
    enable_line.dst = Some(endpoint("3", "enable", 1));

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block(
                "Constant",
                "Data",
                "1",
                vec![port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "Constant",
                "Switch",
                "2",
                vec![port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "SubSystem",
                "Sub",
                "3",
                vec![port("in", 1, None)],
                Some(child),
                &[],
            ),
        ],
        lines: vec![line("1", 1, "3", 1, None), enable_line],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let child_system = system.blocks[2].subsystem.as_ref().unwrap();
    let from_in1 = resolver.line_targets_for_line(&["Sub".to_string()], &child_system.lines[0]);

    // The enable signal is not data port 1.
    assert!(
        from_in1.iter().any(|t| t.path == "model/Data")
            && !from_in1.iter().any(|t| t.path == "model/Switch"),
        "data port 1 picked up the enable signal: {from_in1:#?}"
    );
}

#[test]
fn branched_line_into_a_mux_keeps_its_element_index() {
    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block("Constant", "B", "2", vec![port("out", 1, None)], None, &[]),
            block("Display", "Tap", "5", vec![port("in", 1, None)], None, &[]),
            block(
                "Mux",
                "Mux",
                "3",
                vec![
                    port("in", 1, None),
                    port("in", 2, None),
                    port("out", 1, None),
                ],
                None,
                &[],
            ),
            block(
                "Demux",
                "Demux",
                "4",
                vec![
                    port("in", 1, None),
                    port("out", 1, None),
                    port("out", 2, None),
                ],
                None,
                &[],
            ),
            block("Display", "S1", "6", vec![port("in", 1, None)], None, &[]),
            block("Display", "S2", "7", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![
            line("1", 1, "3", 1, None),
            // `B` branches into the *second* mux input and into a tap.
            branched_line("2", 1, &[("3", "in", 2), ("5", "in", 1)]),
            line("3", 1, "4", 1, None),
            line("4", 1, "6", 1, None),
            line("4", 2, "7", 1, None),
        ],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let first = resolver.line_targets_for_line(&[], &system.lines[3]);
    let second = resolver.line_targets_for_line(&[], &system.lines[4]);

    assert!(
        first.iter().any(|t| t.path == "model/A") && !first.iter().any(|t| t.path == "model/B"),
        "demux output 1 must be the mux's first input: {first:#?}"
    );
    assert!(
        second.iter().any(|t| t.path == "model/B") && !second.iter().any(|t| t.path == "model/A"),
        "demux output 2 must be the branch that ends at mux input 2: {second:#?}"
    );
}

#[test]
fn reordered_subsystem_inports_route_by_their_port_property() {
    // The boundary blocks are stored in the reverse of their port order, and
    // their names no longer match their numbers – only `Port` is authoritative.
    let child = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Inport",
                "In2",
                "30",
                vec![port("out", 1, None)],
                None,
                &[("Port", "2")],
            ),
            block(
                "Inport",
                "In1",
                "31",
                vec![port("out", 1, None)],
                None,
                &[("Port", "1")],
            ),
            block(
                "Display",
                "Sink1",
                "32",
                vec![port("in", 1, None)],
                None,
                &[],
            ),
            block(
                "Display",
                "Sink2",
                "33",
                vec![port("in", 1, None)],
                None,
                &[],
            ),
        ],
        // `In2` (port 2) feeds Sink2, `In1` (port 1) feeds Sink1.
        lines: vec![line("30", 1, "33", 1, None), line("31", 1, "32", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block("Constant", "A", "1", vec![port("out", 1, None)], None, &[]),
            block("Constant", "B", "2", vec![port("out", 1, None)], None, &[]),
            block(
                "SubSystem",
                "Sub",
                "3",
                vec![port("in", 1, None), port("in", 2, None)],
                Some(child),
                &[],
            ),
        ],
        lines: vec![line("1", 1, "3", 1, None), line("2", 1, "3", 2, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let child_system = system.blocks[2].subsystem.as_ref().unwrap();
    let from_port2 = resolver.line_targets_for_line(&["Sub".to_string()], &child_system.lines[0]);
    let from_port1 = resolver.line_targets_for_line(&["Sub".to_string()], &child_system.lines[1]);

    assert!(
        from_port1.iter().any(|t| t.path == "model/A")
            && !from_port1.iter().any(|t| t.path == "model/B"),
        "the Inport with Port=1 must carry A: {from_port1:#?}"
    );
    assert!(
        from_port2.iter().any(|t| t.path == "model/B")
            && !from_port2.iter().any(|t| t.path == "model/A"),
        "the Inport with Port=2 must carry B: {from_port2:#?}"
    );
}

#[test]
fn reordered_subsystem_outports_route_by_their_port_property() {
    let child = System {
        properties: IndexMap::new(),
        blocks: vec![
            block("Constant", "P", "30", vec![port("out", 1, None)], None, &[]),
            block("Constant", "Q", "31", vec![port("out", 1, None)], None, &[]),
            block(
                "Outport",
                "Out2",
                "32",
                vec![port("in", 1, None)],
                None,
                &[("Port", "2")],
            ),
            block(
                "Outport",
                "Out1",
                "33",
                vec![port("in", 1, None)],
                None,
                &[("Port", "1")],
            ),
        ],
        // `P` leaves through port 2, `Q` through port 1.
        lines: vec![line("30", 1, "32", 1, None), line("31", 1, "33", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block(
                "SubSystem",
                "Sub",
                "1",
                vec![port("out", 1, None), port("out", 2, None)],
                Some(child),
                &[],
            ),
            block("Display", "D1", "2", vec![port("in", 1, None)], None, &[]),
            block("Display", "D2", "3", vec![port("in", 1, None)], None, &[]),
        ],
        lines: vec![line("1", 1, "2", 1, None), line("1", 2, "3", 1, None)],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let from_out1 = resolver.line_targets_for_line(&[], &system.lines[0]);
    let from_out2 = resolver.line_targets_for_line(&[], &system.lines[1]);

    assert!(
        from_out1.iter().any(|t| t.path == "model/Sub/Q")
            && !from_out1.iter().any(|t| t.path == "model/Sub/P"),
        "output 1 is fed by the Outport with Port=1: {from_out1:#?}"
    );
    assert!(
        from_out2.iter().any(|t| t.path == "model/Sub/P")
            && !from_out2.iter().any(|t| t.path == "model/Sub/Q"),
        "output 2 is fed by the Outport with Port=2: {from_out2:#?}"
    );
}

#[test]
fn branched_line_picks_up_child_metadata_of_the_subsystem_it_branches_into() {
    let mut child_wire = line("10", 1, "11", 1, Some("child_name"));
    child_wire
        .properties
        .insert("TestPoint".to_string(), "on".to_string());
    let child_system = System {
        properties: IndexMap::new(),
        blocks: vec![
            block(
                "Inport",
                "In1",
                "10",
                vec![port("out", 1, None)],
                None,
                &[("Port", "1")],
            ),
            block(
                "Outport",
                "Out1",
                "11",
                vec![port("in", 1, None)],
                None,
                &[("Port", "1")],
            ),
        ],
        lines: vec![child_wire],
        annotations: Vec::new(),
        chart: None,
    };

    let system = System {
        properties: props(&[("Name", "model")]),
        blocks: vec![
            block(
                "Constant",
                "Src",
                "1",
                vec![port("out", 1, None)],
                None,
                &[],
            ),
            block(
                "SubSystem",
                "Sub",
                "2",
                vec![port("in", 1, None), port("out", 1, None)],
                Some(child_system),
                &[],
            ),
            block("Display", "Tap", "3", vec![port("in", 1, None)], None, &[]),
        ],
        // The line has no `dst` of its own: both ends are branches.
        lines: vec![branched_line("1", 1, &[("2", "in", 1), ("3", "in", 1)])],
        annotations: Vec::new(),
        chart: None,
    };

    let resolver = ConnectionTargetResolver::new(&system);
    let targets = resolver.line_targets_for_line(&[], &system.lines[0]);

    assert!(
        targets
            .iter()
            .any(|t| t.signal_name.as_deref() == Some("child_name") && t.testpoint),
        "metadata of the branched-into subsystem must reach the line: {targets:#?}"
    );
}
