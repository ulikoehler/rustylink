use indexmap::IndexMap;
use rustylink::editor::operations::create_default_block;
use rustylink::editor::selection::{EditorSelection, SelectionRect};
use rustylink::model::{EndpointRef, Line, Point, System};

fn make_test_system() -> System {
    let mut sys = System {
        properties: IndexMap::new(),
        blocks: Vec::new(),
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    };

    let mut b1 = create_default_block("Gain", "Gain1", 100, 100, 1, 1);
    b1.sid = Some("1".to_string());

    let mut b2 = create_default_block("Sum", "Sum1", 200, 100, 2, 1);
    b2.sid = Some("2".to_string());

    let mut b3 = create_default_block("Scope", "Scope1", 300, 200, 1, 0);
    b3.sid = Some("3".to_string());

    sys.blocks.push(b1);
    sys.blocks.push(b2);
    sys.blocks.push(b3);

    sys.lines.push(Line {
        name: None,
        zorder: None,
        src: Some(EndpointRef {
            sid: "1".to_string(),
            port_type: "out".to_string(),
            port_index: 1,
        }),
        dst: Some(EndpointRef {
            sid: "2".to_string(),
            port_type: "in".to_string(),
            port_index: 1,
        }),
        points: vec![Point { x: 130, y: 115 }, Point { x: 200, y: 115 }],
        labels: None,
        branches: Vec::new(),
        properties: IndexMap::new(),
    });

    sys
}

#[test]
fn test_selection_rect_normalized() {
    let rect = SelectionRect {
        start_x: 100.0,
        start_y: 200.0,
        end_x: 50.0,
        end_y: 150.0,
    };
    let (min_x, min_y, max_x, max_y) = rect.normalized();
    assert_eq!(min_x, 50.0);
    assert_eq!(min_y, 150.0);
    assert_eq!(max_x, 100.0);
    assert_eq!(max_y, 200.0);
}

#[test]
fn test_selection_rect_contains() {
    let rect = SelectionRect {
        start_x: 10.0,
        start_y: 10.0,
        end_x: 100.0,
        end_y: 100.0,
    };
    assert!(rect.contains(50.0, 50.0));
    assert!(!rect.contains(150.0, 50.0));
}

#[test]
fn test_selection_rect_overlaps() {
    let rect = SelectionRect {
        start_x: 10.0,
        start_y: 10.0,
        end_x: 100.0,
        end_y: 100.0,
    };
    assert!(rect.overlaps_rect(50.0, 50.0, 150.0, 150.0));
    assert!(!rect.overlaps_rect(200.0, 200.0, 300.0, 300.0));
}

#[test]
fn test_selection_new_is_empty() {
    let sel = EditorSelection::new();
    assert!(sel.is_empty());
    assert_eq!(sel.count(), 0);
}

#[test]
fn test_selection_toggle_block() {
    let mut sel = EditorSelection::new();
    sel.toggle_block(0);
    assert!(sel.is_block_selected(0));
    assert_eq!(sel.count(), 1);

    sel.toggle_block(0);
    assert!(!sel.is_block_selected(0));
    assert!(sel.is_empty());
}

#[test]
fn test_selection_toggle_line() {
    let mut sel = EditorSelection::new();
    sel.toggle_line(0);
    assert!(sel.is_line_selected(0));

    sel.toggle_line(0);
    assert!(!sel.is_line_selected(0));
}

#[test]
fn test_selection_select_block() {
    let mut sel = EditorSelection::new();
    sel.toggle_block(0);
    sel.toggle_block(1);
    assert_eq!(sel.count(), 2);

    sel.select_block(2);
    assert_eq!(sel.count(), 1);
    assert!(sel.is_block_selected(2));
    assert!(!sel.is_block_selected(0));
}

#[test]
fn test_selection_clear() {
    let mut sel = EditorSelection::new();
    sel.toggle_block(0);
    sel.toggle_line(1);
    sel.clear();
    assert!(sel.is_empty());
}

#[test]
fn test_selection_rect_finish_selects_blocks() {
    let sys = make_test_system();
    let mut sel = EditorSelection::new();

    // Create a selection rect that encompasses blocks at [100,100] and [200,100]
    // With zoom=1.0 and pan=(0,0)
    sel.start_rect(90.0, 90.0);
    sel.update_rect(240.0, 140.0);
    sel.finish_rect(&sys, 1.0, 0.0, 0.0, 0.0, 0.0);

    assert!(sel.is_block_selected(0), "Block 0 should be selected");
    assert!(sel.is_block_selected(1), "Block 1 should be selected");
    assert!(
        !sel.is_block_selected(2),
        "Block 2 should not be selected (at 300,200)"
    );
}

#[test]
fn test_selection_rect_too_small_ignored() {
    let sys = make_test_system();
    let mut sel = EditorSelection::new();

    sel.start_rect(100.0, 100.0);
    sel.update_rect(101.0, 101.0); // Less than 3px
    sel.finish_rect(&sys, 1.0, 0.0, 0.0, 0.0, 0.0);

    assert!(sel.is_empty(), "Tiny rect should not select anything");
}

#[test]
fn test_selection_width_height() {
    let rect = SelectionRect {
        start_x: 10.0,
        start_y: 20.0,
        end_x: 110.0,
        end_y: 80.0,
    };
    assert_eq!(rect.width(), 100.0);
    assert_eq!(rect.height(), 60.0);
}
