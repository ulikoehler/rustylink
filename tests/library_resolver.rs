use camino::Utf8PathBuf;
use rustylink::parser::{ExternalFileReference, ExternalFileReferenceType, GraphicalInterface, LibraryResolver};
use tempfile::tempdir;
use std::fs::{self, File};

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

    // Regler should be found in dir1 (first preference)
    assert_eq!(res.found.len(), 2);
    assert_eq!(res.not_found.len(), 1);

    let regler_entry = res.found.iter().find(|(n, _)| n == "Regler").unwrap();
    assert_eq!(regler_entry.1, Utf8PathBuf::from_path_buf(dir1.join("Regler.slx")).unwrap());

    let other_entry = res.found.iter().find(|(n, _)| n == "OtherLib").unwrap();
    assert_eq!(other_entry.1, Utf8PathBuf::from_path_buf(dir2.join("OtherLib.slx")).unwrap());

    assert_eq!(res.not_found, vec!["MissingLib".to_string()]);
}
