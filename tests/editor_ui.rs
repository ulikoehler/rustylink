use eframe::egui::Color32;
use indexmap::IndexMap;
use rustylink::editor::{
    compute_line_colors, contrast_color, get_block_code, hash_color, is_code_block,
    is_subsystem_block, set_block_code,
};
use rustylink::model::{Block, NameLocation, ValueKind};
use std::collections::HashMap;

#[test]
fn test_hash_color_deterministic() {
    let c1 = hash_color("Gain", 0.35, 0.90);
    let c2 = hash_color("Gain", 0.35, 0.90);
    assert_eq!(c1, c2);
}

#[test]
fn test_hash_color_different_inputs() {
    let c1 = hash_color("Gain", 0.35, 0.90);
    let c2 = hash_color("Sum", 0.35, 0.90);
    // Different inputs should produce different colors (with high probability)
    assert_ne!(c1, c2);
}

#[test]
fn test_contrast_color_light_bg() {
    let c = contrast_color(Color32::WHITE);
    assert_eq!(c, Color32::from_rgb(25, 35, 45));
}

#[test]
fn test_contrast_color_dark_bg() {
    let c = contrast_color(Color32::BLACK);
    assert_eq!(c, Color32::from_rgb(235, 245, 245));
}

#[test]
fn test_is_code_block() {
    let mut block = Block {
        block_type: "SubSystem".to_string(),
        name: "test".to_string(),
        is_matlab_function: true,
        sid: None,
        tag_name: "Block".to_string(),
        position: None,
        zorder: None,
        commented: false,
        name_location: NameLocation::Bottom,
        value: None,
        value_kind: ValueKind::Unknown,
        value_rows: None,
        value_cols: None,
        properties: IndexMap::new(),
        ref_properties: std::collections::BTreeSet::new(),
        system_ref: None,
        mask: None,
        ports: Vec::new(),
        port_counts: None,
        subsystem: None,
        annotations: Vec::new(),
        child_order: Vec::new(),
        block_mirror: None,
        background_color: None,
        instance_data: None,
        c_function: None,
        link_data: None,
        show_name: None,
        font_size: None,
        font_weight: None,
        mask_display_text: None,
        current_setting: None,
        library_source: None,
        library_block_path: None,
    };
    assert!(is_code_block(&block));
    block.is_matlab_function = false;
    assert!(!is_code_block(&block));
    block.block_type = "CFunction".to_string();
    assert!(is_code_block(&block));
}

#[test]
fn test_is_subsystem_block() {
    let mut block = Block {
        block_type: "SubSystem".to_string(),
        name: "test".to_string(),
        is_matlab_function: false,
        sid: None,
        tag_name: "Block".to_string(),
        position: None,
        zorder: None,
        commented: false,
        name_location: NameLocation::Bottom,
        value: None,
        value_kind: ValueKind::Unknown,
        value_rows: None,
        value_cols: None,
        properties: IndexMap::new(),
        ref_properties: std::collections::BTreeSet::new(),
        system_ref: None,
        mask: None,
        ports: Vec::new(),
        port_counts: None,
        subsystem: Some(Box::new(rustylink::model::System {
            properties: IndexMap::new(),
            blocks: Vec::new(),
            lines: Vec::new(),
            annotations: Vec::new(),
            chart: None,
        })),
        annotations: Vec::new(),
        child_order: Vec::new(),
        block_mirror: None,
        background_color: None,
        instance_data: None,
        c_function: None,
        link_data: None,
        show_name: None,
        font_size: None,
        font_weight: None,
        mask_display_text: None,
        current_setting: None,
        library_source: None,
        library_block_path: None,
    };
    assert!(is_subsystem_block(&block));
    block.subsystem = None;
    assert!(!is_subsystem_block(&block));
}

#[test]
fn test_get_set_block_code() {
    let mut block =
        rustylink::editor::operations::create_default_block("SubSystem", "Test", 0, 0, 1, 1);
    block.properties.insert(
        "Script".to_string(),
        "function y = f(x)\n  y = x;\nend".to_string(),
    );

    assert_eq!(get_block_code(&block), "function y = f(x)\n  y = x;\nend");

    set_block_code(&mut block, "function y = g(x)\n  y = 2*x;\nend");
    assert_eq!(
        block.properties.get("Script").unwrap(),
        "function y = g(x)\n  y = 2*x;\nend"
    );
}

#[test]
fn test_compute_line_colors_empty() {
    let colors = compute_line_colors(&[], &HashMap::new());
    assert!(colors.is_empty());
}

#[test]
fn test_compute_line_colors_single() {
    let line = rustylink::model::Line {
        name: None,
        zorder: None,
        src: Some(rustylink::model::EndpointRef {
            sid: "1".to_string(),
            port_type: "out".to_string(),
            port_index: 1,
        }),
        dst: Some(rustylink::model::EndpointRef {
            sid: "2".to_string(),
            port_type: "in".to_string(),
            port_index: 1,
        }),
        points: Vec::new(),
        labels: None,
        branches: Vec::new(),
        properties: IndexMap::new(),
    };
    let colors = compute_line_colors(&[line], &HashMap::new());
    assert_eq!(colors.len(), 1);
}
