//! E2E: full serialization round-trips — write .slx via the real generator,
//! re-parse via the real archive reader (§7).

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use harness::model_builders::{BlockBuilder, SystemBuilder, line};
use harness::{gain_chain, slx_factory, subsystem_wrap};
use rustylink::model::{SlxArchive, SlxContent};
use tempfile::TempDir;

fn slx_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(name)
}

#[test]
fn ser_001_write_then_read_round_trip() {
    let dir = TempDir::new().unwrap();
    let path = slx_path(&dir, "m.slx");
    let sys = gain_chain(3);
    slx_factory::write_model_slx(&path, &sys).unwrap();

    let arch = SlxArchive::from_file(&path).unwrap();
    let parsed = arch
        .entries
        .iter()
        .find_map(|e| match &e.content {
            SlxContent::SystemXml(s) => Some(s.as_ref()),
            _ => None,
        })
        .expect("system xml parsed");

    assert_eq!(parsed.blocks.len(), sys.blocks.len());
    assert_eq!(parsed.lines.len(), sys.lines.len());
    for (a, b) in parsed.blocks.iter().zip(sys.blocks.iter()) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.sid, b.sid);
        assert_eq!(a.block_type, b.block_type);
        assert_eq!(a.position, b.position);
    }
    for (a, b) in parsed.lines.iter().zip(sys.lines.iter()) {
        for (ae, be) in [(&a.src, &b.src), (&a.dst, &b.dst)] {
            assert_eq!(
                ae.as_ref().map(|e| (&e.sid, &e.port_type, e.port_index)),
                be.as_ref().map(|e| (&e.sid, &e.port_type, e.port_index))
            );
        }
    }
}

#[test]
fn ser_002_nested_subsystem_round_trip() {
    let dir = TempDir::new().unwrap();
    let path = slx_path(&dir, "nested.slx");
    let sys = subsystem_wrap(gain_chain(1), "SS", "2");
    slx_factory::write_model_slx(&path, &sys).unwrap();

    let arch = SlxArchive::from_file(&path).unwrap();
    let parsed = arch
        .entries
        .iter()
        .find_map(|e| match &e.content {
            SlxContent::SystemXml(s) => Some(s.as_ref()),
            _ => None,
        })
        .expect("parsed");
    let ss = parsed
        .blocks
        .iter()
        .find(|b| b.name == "SS")
        .expect("subsystem block");
    assert!(ss.subsystem.is_some());
    assert_eq!(ss.subsystem.as_ref().unwrap().blocks.len(), 3);
}

#[test]
fn ser_003_raw_entries_preserved_verbatim() {
    let dir = TempDir::new().unwrap();
    let path = slx_path(&dir, "raw.slx");
    let blob = b"\x89PNG fake binary \x00\x01\x02";
    slx_factory::write_slx(
        &path,
        &[("simulink/systems/system_root.xml", gain_chain(1))],
        &[("simulink/bdproperties.mat", blob)],
        true,
    )
    .unwrap();

    let arch = SlxArchive::from_file(&path).unwrap();
    let raw = arch
        .entries
        .iter()
        .find(|e| e.path == "simulink/bdproperties.mat")
        .expect("raw entry kept");
    match &raw.content {
        SlxContent::Raw(bytes) => assert_eq!(bytes, blob),
        _ => panic!("raw entry must stay raw"),
    }
}

#[test]
fn ser_004_malformed_zip_errors_not_panics() {
    let dir = TempDir::new().unwrap();
    let path = slx_path(&dir, "junk.slx");
    std::fs::write(&path, b"not a zip file at all").unwrap();
    assert!(SlxArchive::from_file(&path).is_err());
}

#[test]
fn ser_005_zip_without_system_xml_tolerated() {
    let dir = TempDir::new().unwrap();
    let path = slx_path(&dir, "empty.slx");
    slx_factory::raw_zip(&path, &[("readme.txt", b"hello")]).unwrap();
    // Should read as archive with zero parsed systems (or error cleanly).
    let res = SlxArchive::from_file(&path);
    if let Ok(arch) = res {
        assert!(
            arch.entries
                .iter()
                .all(|e| matches!(e.content, SlxContent::Raw(_)))
        );
    }
}

#[test]
fn ser_006_properties_survive_round_trip() {
    let dir = TempDir::new().unwrap();
    let path = slx_path(&dir, "props.slx");
    let sys = SystemBuilder::new("model")
        .block(
            BlockBuilder::new("Gain", "G", "1")
                .pos(0, 0, 40, 24)
                .ports(1, 1)
                .prop("Gain", "3.5")
                .prop("CustomProp", "hello")
                .build(),
        )
        .block(
            BlockBuilder::new("Display", "D", "2")
                .pos(100, 0, 140, 24)
                .ports(1, 0)
                .build(),
        )
        .line(line("1", 1, "2", 1))
        .build();
    slx_factory::write_model_slx(&path, &sys).unwrap();
    let arch = SlxArchive::from_file(&path).unwrap();
    let parsed = arch
        .entries
        .iter()
        .find_map(|e| match &e.content {
            SlxContent::SystemXml(s) => Some(s.as_ref()),
            _ => None,
        })
        .unwrap();
    let g = &parsed.blocks[0];
    assert_eq!(g.properties.get("Gain").map(String::as_str), Some("3.5"));
}
