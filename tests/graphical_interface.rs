use camino::Utf8PathBuf;
use rustylink::parser::{FsSource, SimulinkParser, ExternalFileReferenceType, SolverName};

#[test]
fn parse_graphical_interface_json_from_fs() {
    let cwd = std::env::current_dir().expect("cwd");
    let root_utf8 = Utf8PathBuf::from_path_buf(cwd).unwrap();
    let mut parser = SimulinkParser::new(&root_utf8, FsSource);

    // Use the simulink JSON file from the other workspace folder available in the environment
    let path = camino::Utf8PathBuf::from("/ram/2025/simulink/graphicalInterface.json");
    let gi = parser
        .parse_graphical_interface_file(&path)
        .expect("parse graphicalInterface.json");

    // Basic expectations from the provided sample
    assert!(gi.external_file_references.len() > 10, "unexpectedly small list");
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
