#![cfg(feature = "egui")]

use rustylink::egui_app::{
    collect_subsystems_paths, resolve_subsystem_by_path, resolve_subsystem_by_vec,
};
use rustylink::model::{Block, System};

fn simple_system() -> System {
    let sub_child = System {
        properties: Default::default(),
        blocks: vec![],
        lines: vec![],
        annotations: vec![],
        chart: None,
    };
    let sub_block = Block {
        block_type: "SubSystem".into(),
        name: "Child".into(),
        sid: Some("2".to_string()),
        position: Some("[100, 100, 160, 140]".into()),
        zorder: None,
        commented: false,
        name_location: rustylink::model::NameLocation::Bottom,
        is_matlab_function: false,
        properties: Default::default(),
        ports: vec![],
        c_function: None,
        mask: None,
        annotations: vec![],
        subsystem: Some(Box::new(sub_child)),
        instance_data: None,
        background_color: None,
        show_name: None,
        font_size: None,
        font_weight: None,
        mask_display_text: None,
        value: None,
    };
    System {
        properties: Default::default(),
        blocks: vec![sub_block],
        lines: vec![],
        annotations: vec![],
        chart: None,
    }
}

#[test]
fn test_resolve_subsystem_path_and_vec() {
    let root = simple_system();
    assert!(resolve_subsystem_by_path(&root, "/Child").is_some());
    assert!(resolve_subsystem_by_vec(&root, &["Child".to_string()]).is_some());
    assert!(resolve_subsystem_by_path(&root, "/Nope").is_none());
}

#[test]
fn test_collect_paths() {
    let root = simple_system();
    let paths = collect_subsystems_paths(&root);
    assert_eq!(paths, vec![vec!["Child".to_string()]]);
}
