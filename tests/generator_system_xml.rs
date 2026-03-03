use indexmap::IndexMap;
use rustylink::generator::system_xml::generate_system_xml;
use rustylink::model::{Block, NameLocation, PortCounts, System, ValueKind};

#[test]
fn test_simple_system_roundtrip() {
    let mut props = IndexMap::new();
    props.insert("Location".to_string(), "[0, 0, 1920, 1036]".to_string());
    props.insert("Open".to_string(), "on".to_string());

    let system = System {
        properties: props,
        blocks: vec![],
        lines: vec![],
        annotations: vec![],
        chart: None,
    };

    let xml = generate_system_xml(&system);
    assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
    assert!(xml.contains("<P Name=\"Location\">[0, 0, 1920, 1036]</P>"));
    assert!(xml.contains("<P Name=\"Open\">on</P>"));
}

#[test]
fn test_block_with_port_counts() {
    let system = System {
        properties: IndexMap::new(),
        blocks: vec![Block {
            block_type: "Gain".into(),
            name: "G1".into(),
            sid: Some("5".into()),
            tag_name: "Block".into(),
            position: Some("[10, 20, 50, 60]".into()),
            zorder: Some("1".into()),
            commented: false,
            name_location: NameLocation::Bottom,
            is_matlab_function: false,
            value: None,
            value_kind: ValueKind::Unknown,
            value_rows: None,
            value_cols: None,
            properties: {
                let mut m = IndexMap::new();
                m.insert("Position".into(), "[10, 20, 50, 60]".into());
                m.insert("ZOrder".into(), "1".into());
                m
            },
            ref_properties: Default::default(),
            port_counts: Some(PortCounts {
                ins: Some(1),
                outs: Some(1),
            }),
            ports: vec![],
            subsystem: None,
            system_ref: None,
            c_function: None,
            instance_data: None,
            link_data: None,
            mask: None,
            annotations: vec![],
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
            child_order: vec![],
        }],
        lines: vec![],
        annotations: vec![],
        chart: None,
    };

    let xml = generate_system_xml(&system);
    assert!(xml.contains("<PortCounts in=\"1\" out=\"1\"/>"));
    assert!(xml.contains("BlockType=\"Gain\""));
    assert!(xml.contains("Name=\"G1\""));
    assert!(xml.contains("SID=\"5\""));
}
