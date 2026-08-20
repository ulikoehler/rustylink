#![cfg(feature = "egui")]

use indexmap::IndexMap;
use rustylink::egui_app::{get_block_type_cfg, port_label_display_name};
use rustylink::model::System;

#[test]
fn port_labels_do_not_fall_back_to_propagated_signals() {
    let mut block =
        rustylink::editor::operations::create_default_block("SubSystem", "SubSystem", 0, 0, 1, 1);
    block.ports = vec![rustylink::model::Port {
        port_type: "out".to_string(),
        index: Some(1),
        properties: IndexMap::from_iter([(
            "PropagatedSignals".to_string(),
            "ConnectedSignal".to_string(),
        )]),
    }];

    let cfg = get_block_type_cfg(&block);
    assert_eq!(port_label_display_name(&block, 1, false, &cfg), "Out1");
}

#[test]
fn fixed_port_labels_ignore_port_name_overrides() {
    // AlgebraicConstraint names its ports `f(z)` and `z` in the catalog; a
    // per-port `Name` in the model must not override that.
    let mut block = rustylink::editor::operations::create_default_block(
        "AlgebraicConstraint",
        "AlgebraicConstraint",
        0,
        0,
        1,
        1,
    );
    block.ports = vec![
        rustylink::model::Port {
            port_type: "in".to_string(),
            index: Some(1),
            properties: IndexMap::from_iter([("Name".to_string(), "OtherSignal".to_string())]),
        },
        rustylink::model::Port {
            port_type: "out".to_string(),
            index: Some(1),
            properties: IndexMap::from_iter([("Name".to_string(), "VisibleSignal".to_string())]),
        },
    ];

    let cfg = get_block_type_cfg(&block);
    assert_eq!(port_label_display_name(&block, 1, true, &cfg), "f(z)");
    assert_eq!(port_label_display_name(&block, 1, false, &cfg), "z");
}

#[test]
fn subsystem_port_labels_use_internal_boundary_block_names() {
    let mut block =
        rustylink::editor::operations::create_default_block("SubSystem", "SubSystem", 0, 0, 1, 1);
    block.subsystem = Some(Box::new(System {
        properties: IndexMap::new(),
        blocks: vec![
            rustylink::model::Block {
                name: "SubsystemInput".to_string(),
                properties: IndexMap::from_iter([("Port".to_string(), "1".to_string())]),
                ports: vec![],
                block_type: "Inport".to_string(),
                sid: Some("10".to_string()),
                tag_name: "Block".to_string(),
                position: None,
                zorder: None,
                commented: false,
                name_location: rustylink::model::NameLocation::Bottom,
                is_matlab_function: false,
                value: None,
                value_kind: rustylink::model::ValueKind::default(),
                value_rows: None,
                value_cols: None,
                ref_properties: Default::default(),
                port_counts: None,
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
                dashboard_binding: None,
                child_order: Vec::new(),
            },
            rustylink::model::Block {
                name: "SubsystemOutput".to_string(),
                properties: IndexMap::from_iter([("Port".to_string(), "1".to_string())]),
                ports: vec![],
                block_type: "Outport".to_string(),
                sid: Some("11".to_string()),
                tag_name: "Block".to_string(),
                position: None,
                zorder: None,
                commented: false,
                name_location: rustylink::model::NameLocation::Bottom,
                is_matlab_function: false,
                value: None,
                value_kind: rustylink::model::ValueKind::default(),
                value_rows: None,
                value_cols: None,
                ref_properties: Default::default(),
                port_counts: None,
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
                dashboard_binding: None,
                child_order: Vec::new(),
            },
        ],
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    }));

    let cfg = get_block_type_cfg(&block);
    assert_eq!(
        port_label_display_name(&block, 1, true, &cfg),
        "SubsystemInput"
    );
    assert_eq!(
        port_label_display_name(&block, 1, false, &cfg),
        "SubsystemOutput"
    );
}

#[test]
fn subsystem_port_numbers_follow_the_port_property_not_the_block_name() {
    // Reordering a subsystem's ports renumbers them while the boundary blocks
    // keep the default names they were created with, so `In2` can be port 1.
    let boundary = |block_type: &str, name: &str, port: &str| {
        let mut child =
            rustylink::editor::operations::create_default_block(block_type, name, 0, 0, 0, 0);
        child
            .properties
            .insert("Port".to_string(), port.to_string());
        child
    };

    let mut block =
        rustylink::editor::operations::create_default_block("SubSystem", "SubSystem", 0, 0, 2, 2);
    block.subsystem = Some(Box::new(System {
        properties: IndexMap::new(),
        blocks: vec![
            boundary("Inport", "In2", "1"),
            boundary("Inport", "In1", "2"),
            boundary("Outport", "Out2", "1"),
            boundary("Outport", "Out1", "2"),
        ],
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    }));

    let cfg = get_block_type_cfg(&block);
    assert_eq!(port_label_display_name(&block, 1, true, &cfg), "1");
    assert_eq!(port_label_display_name(&block, 2, true, &cfg), "2");
    assert_eq!(port_label_display_name(&block, 1, false, &cfg), "1");
    assert_eq!(port_label_display_name(&block, 2, false, &cfg), "2");
}
