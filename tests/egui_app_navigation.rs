#![cfg(feature = "egui")]

use rustylink::egui_app::{collect_subsystems_paths, resolve_subsystem_by_path, resolve_subsystem_by_vec};
use rustylink::model::{Block, System};

fn simple_system() -> System {
    let sub_child = System { properties: Default::default(), blocks: vec![], lines: vec![], chart: None };
    let sub_block = Block {
        block_type: "SubSystem".into(),
        name: "Child".into(),
        sid: Some("2".to_string()),
        position: Some("[100, 100, 160, 140]".into()),
        zorder: None,
        commented: false,
        is_matlab_function: false,
        properties: Default::default(),
        ports: vec![],
        c_function: None,
        subsystem: Some(Box::new(sub_child)),
    };
    System { properties: Default::default(), blocks: vec![sub_block], lines: vec![], chart: None }
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
