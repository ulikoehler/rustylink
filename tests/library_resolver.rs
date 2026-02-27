use camino::Utf8PathBuf;
use rustylink::parser::{
    ExternalFileReference, ExternalFileReferenceType, GraphicalInterface, LibraryResolver,
};
use std::fs::{self, File};
use tempfile::tempdir;

#[test]
fn graphical_interface_library_names_collects_unique_libs() {
    let refs = vec![
        ExternalFileReference {
            path: "$bdroot/whatever".to_string(),
            reference: "Regler/Joint_Interpolator".to_string(),
            sid: "1".to_string(),
            r#type: ExternalFileReferenceType::LibraryBlock,
        },
        ExternalFileReference {
            path: "$bdroot/other".to_string(),
            reference: "simulink/Logic and Bit Operations/Compare To Constant".to_string(),
            sid: "2".to_string(),
            r#type: ExternalFileReferenceType::LibraryBlock,
        },
        // duplicate Regler should only appear once
        ExternalFileReference {
            path: "$bdroot/dup".to_string(),
            reference: "Regler/AnotherBlock".to_string(),
            sid: "3".to_string(),
            r#type: ExternalFileReferenceType::LibraryBlock,
        },
        // non-library type should be ignored
        ExternalFileReference {
            path: "$bdroot/notlib".to_string(),
            reference: "Ignored/Thing".to_string(),
            sid: "4".to_string(),
            r#type: ExternalFileReferenceType::Other("SOMETHING_ELSE".to_string()),
        },
    ];

    let gi = GraphicalInterface {
        external_file_references: refs,
        precomp_execution_domain_type: None,
        simulink_sub_domain_type: None,
        solver_name: None,
    };

    let libs = gi.library_names();
    assert_eq!(libs, vec!["Regler".to_string(), "simulink".to_string()]);
}

#[test]
fn graphical_interface_library_block_references_by_library_groups_blocks() {
    let refs = vec![
        ExternalFileReference {
            path: "$bdroot/whatever".to_string(),
            reference: "Regler/Joint_Interpolator".to_string(),
            sid: "1".to_string(),
            r#type: ExternalFileReferenceType::LibraryBlock,
        },
        ExternalFileReference {
            path: "$bdroot/other".to_string(),
            reference: "simulink/Logic and Bit Operations/Compare To Constant".to_string(),
            sid: "2".to_string(),
            r#type: ExternalFileReferenceType::LibraryBlock,
        },
        ExternalFileReference {
            path: "$bdroot/dup".to_string(),
            reference: "Regler/AnotherBlock".to_string(),
            sid: "3".to_string(),
            r#type: ExternalFileReferenceType::LibraryBlock,
        },
        ExternalFileReference {
            path: "$bdroot/notlib".to_string(),
            reference: "Ignored/Thing".to_string(),
            sid: "4".to_string(),
            r#type: ExternalFileReferenceType::Other("SOMETHING_ELSE".to_string()),
        },
    ];

    let gi = GraphicalInterface {
        external_file_references: refs,
        precomp_execution_domain_type: None,
        simulink_sub_domain_type: None,
        solver_name: None,
    };

    let grouped = gi.library_block_references_by_library();
    assert_eq!(
        grouped.keys().cloned().collect::<Vec<_>>(),
        vec!["Regler".to_string(), "simulink".to_string()]
    );
    assert_eq!(grouped["Regler"].len(), 2);
    assert_eq!(grouped["Regler"][0].sid, "1");
    assert_eq!(grouped["Regler"][1].sid, "3");
    assert_eq!(grouped["simulink"].len(), 1);
    assert_eq!(grouped["simulink"][0].sid, "2");
}

#[test]
fn library_resolver_finds_and_reports_missing_libraries() {
    let tmp = tempdir().unwrap();
    let dir1 = tmp.path().join("p1");
    let dir2 = tmp.path().join("p2");
    fs::create_dir_all(&dir1).unwrap();
    fs::create_dir_all(&dir2).unwrap();

    // Create Regler.slx in dir1 and OtherLib.slx in dir2
    File::create(dir1.join("Regler.slx")).unwrap();
    File::create(dir2.join("OtherLib.slx")).unwrap();
    // Also create Regler.slx in dir2 to test preference ordering
    File::create(dir2.join("Regler.slx")).unwrap();

    let resolver = LibraryResolver::new(vec![
        Utf8PathBuf::from_path_buf(dir1.clone()).unwrap(),
        Utf8PathBuf::from_path_buf(dir2.clone()).unwrap(),
    ]);

    let names = vec!["Regler", "OtherLib", "MissingLib"];
    let res = resolver.locate(names.iter().map(|s| *s));

    // verify virtual library helper recognizes the new simulink/Logic and Bit entry
    assert!(rustylink::parser::is_virtual_library(
        "simulink/Logic and Bit/Whatever"
    ));
    assert!(rustylink::parser::is_virtual_library("simulink.slx"));
    assert!(rustylink::parser::is_virtual_library("matrix_library"));
    assert!(!rustylink::parser::is_virtual_library("NormalLib"));

    // Regler should be found in dir1 (first preference)
    assert_eq!(res.found.len(), 2);
    assert_eq!(res.not_found.len(), 1);

    let regler_entry = res.found.iter().find(|(n, _)| n == "Regler").unwrap();
    assert_eq!(
        regler_entry.1,
        Utf8PathBuf::from_path_buf(dir1.join("Regler.slx")).unwrap()
    );

    let other_entry = res.found.iter().find(|(n, _)| n == "OtherLib").unwrap();
    assert_eq!(
        other_entry.1,
        Utf8PathBuf::from_path_buf(dir2.join("OtherLib.slx")).unwrap()
    );

    // virtual libraries should not be reported as missing or found
    let res_virtual = resolver.locate(vec!["simulink/Discrete", "SIMULINK", "matrix_library"].iter().map(|s| *s));
    assert!(res_virtual.found.is_empty(), "virtual libs should not appear in found");
    assert!(res_virtual.not_found.is_empty(), "virtual libs should not be listed as missing");

    assert_eq!(res.not_found, vec!["MissingLib".to_string()]);
}

#[test]
fn resolve_virtual_simulink_logic_and_bit() {
    use camino::Utf8PathBuf;
    use indexmap::IndexMap;
    use rustylink::model::{Block, System};
    use rustylink::parser::{FsSource, SimulinkParser};

    // build a minimal system referencing the special virtual library
    let mut sys = System {
        properties: IndexMap::new(),
        blocks: vec![Block {
            block_type: "Foo".to_string(),
            name: "A".to_string(),
            sid: None,
            tag_name: "Block".to_string(),
            position: None,
            zorder: None,
            commented: false,
            name_location: Default::default(),
            is_matlab_function: false,
            value: None,
            value_kind: Default::default(),
            value_rows: None,
            value_cols: None,
            properties: {
                let mut m = IndexMap::new();
                m.insert(
                    "SourceBlock".to_string(),
                    "simulink/Logic and Bit/SomeBlock".to_string(),
                );
                m
            },
            ref_properties: Default::default(),
            port_counts: None,
            ports: Vec::new(),
            mask: None,
            annotations: Vec::new(),
            subsystem: None,
            system_ref: None,
            c_function: None,
            instance_data: None,
            link_data: None,
            background_color: None,
            show_name: None,
            font_size: None,
            font_weight: None,
            mask_display_text: None,
            current_setting: None,
            block_mirror: None,
            library_source: None,
            library_block_path: None,
            child_order: vec![],
        }],
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    };

    // resolution should succeed (no panic) even though library is virtual/empty
    SimulinkParser::<FsSource>::resolve_library_references(&mut sys, &[]).unwrap();
    // block still unresolved but no crash
    assert!(sys.blocks[0].library_source.is_none());
    assert!(sys.blocks[0].library_block_path.is_none());
}

#[test]
fn resolve_virtual_simulink_discrete_discrete_derivative() {
    use indexmap::IndexMap;
    use rustylink::model::System;
    use rustylink::parser::{FsSource, SimulinkParser};

    let mut blk = rustylink::editor::operations::create_default_block(
        "SubSystem",
        "Discrete Derivative",
        0,
        0,
        0,
        0,
    );
    blk.properties.insert(
        "SourceBlock".to_string(),
        "simulink/Discrete/Discrete Derivative".to_string(),
    );

    let mut sys = System {
        properties: IndexMap::new(),
        blocks: vec![blk],
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    };

    SimulinkParser::<FsSource>::resolve_library_references(&mut sys, &[]).unwrap();

    assert_eq!(sys.blocks[0].library_source.as_deref(), Some("simulink/Discrete"));
    assert_eq!(
        sys.blocks[0].library_block_path.as_deref(),
        Some("simulink/Discrete/Discrete Derivative")
    );
    assert_eq!(sys.blocks[0].port_counts.as_ref().and_then(|p| p.ins), Some(1));
    assert_eq!(sys.blocks[0].port_counts.as_ref().and_then(|p| p.outs), Some(1));
    // 1 input + 1 output
    assert_eq!(sys.blocks[0].ports.len(), 2);
}

#[test]
fn resolve_virtual_matrix_library_blocks() {
    use indexmap::IndexMap;
    use rustylink::model::{Block, System};
    use rustylink::parser::{FsSource, SimulinkParser};

    // three different matrix blocks
    let mut sys = System {
        properties: IndexMap::new(),
        blocks: vec![
            Block {
                block_type: "Foo".to_string(),
                name: "A".to_string(),
                sid: None,
                tag_name: "Block".to_string(),
                position: None,
                zorder: None,
                commented: false,
                name_location: Default::default(),
                is_matlab_function: false,
                value: None,
                value_kind: Default::default(),
                value_rows: None,
                value_cols: None,
                properties: {
                    let mut m = IndexMap::new();
                    m.insert(
                        "SourceBlock".to_string(),
                        "matrix_library/IsTriangular".to_string(),
                    );
                    m
                },
                ref_properties: Default::default(),
                port_counts: None,
                ports: Vec::new(),
                mask: None,
                annotations: Vec::new(),
                subsystem: None,
                system_ref: None,
                c_function: None,
                instance_data: None,
                link_data: None,
                background_color: None,
                show_name: None,
                font_size: None,
                font_weight: None,
                mask_display_text: None,
                current_setting: None,
                block_mirror: None,
                library_source: None,
                library_block_path: None,
                child_order: vec![],
            },
            Block {
                block_type: "Bar".to_string(),
                name: "B".to_string(),
                sid: None,
                tag_name: "Block".to_string(),
                position: None,
                zorder: None,
                commented: false,
                name_location: Default::default(),
                is_matlab_function: false,
                value: None,
                value_kind: Default::default(),
                value_rows: None,
                value_cols: None,
                properties: {
                    let mut m = IndexMap::new();
                    m.insert(
                        "SourceBlock".to_string(),
                        "matrix_library/IdentityMatrix".to_string(),
                    );
                    m
                },
                ref_properties: Default::default(),
                port_counts: None,
                ports: Vec::new(),
                mask: None,
                annotations: Vec::new(),
                subsystem: None,
                system_ref: None,
                c_function: None,
                instance_data: None,
                link_data: None,
                background_color: None,
                show_name: None,
                font_size: None,
                font_weight: None,
                mask_display_text: None,
                current_setting: None,
                block_mirror: None,
                library_source: None,
                library_block_path: None,
                child_order: vec![],
            },
            Block {
                block_type: "Baz".to_string(),
                name: "C".to_string(),
                sid: None,
                tag_name: "Block".to_string(),
                position: None,
                zorder: None,
                commented: false,
                name_location: Default::default(),
                is_matlab_function: false,
                value: None,
                value_kind: Default::default(),
                value_rows: None,
                value_cols: None,
                properties: {
                    let mut m = IndexMap::new();
                    m.insert(
                        "SourceBlock".to_string(),
                        "matrix_library/PermuteColumns".to_string(),
                    );
                    m
                },
                ref_properties: Default::default(),
                port_counts: None,
                ports: Vec::new(),
                mask: None,
                annotations: Vec::new(),
                subsystem: None,
                system_ref: None,
                c_function: None,
                instance_data: None,
                link_data: None,
                background_color: None,
                show_name: None,
                font_size: None,
                font_weight: None,
                mask_display_text: None,
                current_setting: None,
                block_mirror: None,
                library_source: None,
                library_block_path: None,
                child_order: vec![],
            },
        ],
        lines: Vec::new(),
        annotations: Vec::new(),
        chart: None,
    };

    SimulinkParser::<FsSource>::resolve_library_references(&mut sys, &[]).unwrap();

    assert_eq!(
        sys.blocks[0].library_source.as_deref(),
        Some("matrix_library")
    );
    assert_eq!(
        sys.blocks[0].library_block_path.as_deref(),
        Some("matrix_library/IsTriangular")
    );
    assert_eq!(
        sys.blocks[0].port_counts.as_ref().and_then(|p| p.ins),
        Some(1)
    );
    // port labels should exist but be empty strings
    assert_eq!(
        sys.blocks[0].ports[0]
            .properties
            .get("Name")
            .map(|s| s.as_str()),
        Some("")
    );
    assert_eq!(
        sys.blocks[1].library_source.as_deref(),
        Some("matrix_library")
    );
    assert_eq!(
        sys.blocks[1].library_block_path.as_deref(),
        Some("matrix_library/IdentityMatrix")
    );
    assert_eq!(
        sys.blocks[1].port_counts.as_ref().and_then(|p| p.ins),
        Some(0)
    );
    // IdentityMatrix has a single output port index 1
    assert_eq!(
        sys.blocks[1].ports[0]
            .properties
            .get("Name")
            .map(|s| s.as_str()),
        Some("")
    );

    assert_eq!(
        sys.blocks[2].library_source.as_deref(),
        Some("matrix_library")
    );
    assert_eq!(
        sys.blocks[2].library_block_path.as_deref(),
        Some("matrix_library/PermuteColumns")
    );
    assert_eq!(
        sys.blocks[2].port_counts.as_ref().and_then(|p| p.ins),
        Some(2)
    );
    assert_eq!(
        sys.blocks[2].port_counts.as_ref().and_then(|p| p.outs),
        Some(1)
    );
    // port labels should exist but be empty strings
    assert_eq!(sys.blocks[2].ports.len(), 3);
    assert_eq!(
        sys.blocks[2].ports[0]
            .properties
            .get("Name")
            .map(|s| s.as_str()),
        Some("")
    );
}
