#![cfg(feature = "egui")]

use rustylink::egui_app::{parse_block_rect, port_anchor_pos};
use rustylink::model::Block;

#[test]
fn test_ports_and_rect() {
    let b = Block {
        block_type: "Gain".into(),
        name: "G".into(),
        sid: None,
        position: Some("[10, 20, 50, 60]".into()),
        zorder: None,
        commented: false,
        is_matlab_function: false,
        properties: Default::default(),
        ports: vec![],
        c_function: None,
        mask: None,
        subsystem: None,
    };
    let r = parse_block_rect(&b).unwrap();
    let p_in = port_anchor_pos(r, "in", 1, Some(2));
    let p_out = port_anchor_pos(r, "out", 2, Some(2));
    assert!(p_in.y < p_out.y && p_in.x < p_out.x);
}
