use camino::Utf8PathBuf;
use rustylink::parser::{ExternalFileReferenceType, FsSource, SimulinkParser, SolverName};
use tempfile::tempdir;

#[test]
fn parse_graphical_interface_json_from_fs() {
    let cwd = std::env::current_dir().expect("cwd");
    let root_utf8 = Utf8PathBuf::from_path_buf(cwd).unwrap();
    let mut parser = SimulinkParser::new(&root_utf8, FsSource);

    // Create a minimal, self-contained graphicalInterface.json.
    let tmp = tempdir().expect("tempdir");
    let path = Utf8PathBuf::from_path_buf(tmp.path().join("graphicalInterface.json")).unwrap();
    let json = r#"{
    "GraphicalInterface": {
        "ExternalFileReferences": [
            {"Path":"$bdroot/Blocks/Joint_Interpolator_Duatic","Reference":"Regler/Joint_Interpolator","SID":"245474","Type":"LIBRARY_BLOCK"},
            {"Path":"$bdroot/Blocks/Other1","Reference":"Regler/Other","SID":"1","Type":"LIBRARY_BLOCK"},
            {"Path":"$bdroot/Blocks/Other2","Reference":"simulink/Logic and Bit Operations/Compare To Constant","SID":"2","Type":"LIBRARY_BLOCK"},
            {"Path":"$bdroot/Blocks/Other3","Reference":"simulink/Sources/Constant","SID":"3","Type":"LIBRARY_BLOCK"},
            {"Path":"$bdroot/Blocks/Other4","Reference":"simulink/Sinks/Out1","SID":"4","Type":"LIBRARY_BLOCK"},
            {"Path":"$bdroot/Blocks/Other5","Reference":"simulink/Math Operations/Add","SID":"5","Type":"LIBRARY_BLOCK"},
            {"Path":"$bdroot/Blocks/Other6","Reference":"simulink/Math Operations/Subtract","SID":"6","Type":"LIBRARY_BLOCK"},
            {"Path":"$bdroot/Blocks/Other7","Reference":"simulink/Signal Routing/Switch","SID":"7","Type":"LIBRARY_BLOCK"},
            {"Path":"$bdroot/Blocks/Other8","Reference":"simulink/Signal Routing/Multiport Switch","SID":"8","Type":"LIBRARY_BLOCK"},
            {"Path":"$bdroot/Blocks/Other9","Reference":"simulink/Logic and Bit Operations/Relational Operator","SID":"9","Type":"LIBRARY_BLOCK"},
            {"Path":"$bdroot/Blocks/Other10","Reference":"simulink/Discrete/Unit Delay","SID":"10","Type":"LIBRARY_BLOCK"}
        ],
        "PreCompExecutionDomainType": null,
        "SimulinkSubDomainType": null,
        "SolverName": "FixedStepDiscrete"
    }
}"#;
    std::fs::write(path.as_std_path(), json).expect("write graphicalInterface.json");
    let gi = parser
        .parse_graphical_interface_file(&path)
        .expect("parse graphicalInterface.json");

    // Basic expectations from the provided sample
    assert_eq!(gi.external_file_references.len(), 11);
    assert_eq!(gi.solver_name, Some(SolverName::FixedStepDiscrete));

    // Find a known entry that exists in the sample file
    let found = gi
        .external_file_references
        .iter()
        .find(|r| r.path.contains("Joint_Interpolator_Duatic"))
        .expect("expected Joint_Interpolator_Duatic entry");

    assert_eq!(found.reference, "Regler/Joint_Interpolator");
    assert_eq!(found.sid, "245474");
    assert_eq!(found.r#type, ExternalFileReferenceType::LibraryBlock);
}
