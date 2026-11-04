//! A block linked to an external library must present its ports like the
//! subsystem it links to.

#![cfg(feature = "egui")]

use anyhow::Result;
use camino::Utf8PathBuf;
use rustylink::parser::{ContentSource, SimulinkParser};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

struct MemSource {
    files: HashMap<String, String>,
}
impl ContentSource for MemSource {
    fn read_to_string(&mut self, path: &camino::Utf8Path) -> Result<String> {
        self.files
            .get(path.as_str())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("not found: {}", path))
    }
    fn list_dir(&mut self, path: &camino::Utf8Path) -> Result<Vec<Utf8PathBuf>> {
        let prefix = path.as_str().trim_end_matches('/').to_string() + "/";
        Ok(self
            .files
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .map(|k| Utf8PathBuf::from(k.clone()))
            .collect())
    }
}

/// Write a minimal `.slx` archive holding the given system XML files.
fn write_slx(path: &std::path::Path, files: &[(&str, &str)]) {
    let file = File::create(path).expect("create slx");
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, contents) in files {
        zip.start_file(*name, options).expect("start file");
        zip.write_all(contents.as_bytes()).expect("write file");
    }
    zip.finish().expect("finish slx");
}

/// The library block is deliberately called `Logic`, the name of a catalog
/// block: a read library link describes itself and must not borrow that
/// definition's icon or port labels.
#[test]
fn library_subsystem_supplies_the_host_block_port_names() {
    let dir = tempdir().expect("tempdir");
    write_slx(
        &dir.path().join("ASXTestLibrary.slx"),
        &[
            (
                "simulink/systems/system_root.xml",
                r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <Block BlockType="SubSystem" Name="Logic" SID="1">
    <P Name="Position">[0, 0, 100, 50]</P>
    <System Ref="system_2"/>
  </Block>
</System>
"#,
            ),
            (
                "simulink/systems/system_2.xml",
                r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <Block BlockType="Inport" Name="u" SID="2">
    <P Name="Position">[20, 20, 40, 40]</P>
    <P Name="Port">1</P>
  </Block>
  <Block BlockType="Outport" Name="result" SID="3">
    <P Name="Position">[120, 20, 140, 40]</P>
    <P Name="Port">1</P>
  </Block>
</System>
"#,
            ),
        ],
    );

    let host = r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <Reference Name="Logic" SID="53">
    <P Name="Position">[325, 264, 425, 306]</P>
    <P Name="SourceBlock">ASXTestLibrary/Logic</P>
    <P Name="SourceType">SubSystem</P>
  </Reference>
</System>
"#;
    let host_path = Utf8PathBuf::from("mem://host.xml");
    let mut files = HashMap::new();
    files.insert(host_path.as_str().to_string(), host.to_string());
    let mut parser = SimulinkParser::new("/", MemSource { files });
    let mut system = parser.parse_system_file(&host_path).expect("parse host");
    let lib_paths = vec![Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap()];
    SimulinkParser::<MemSource>::resolve_library_references(&mut system, &lib_paths)
        .expect("resolve library references");

    let b = &system.blocks[0];
    assert!(!b.library_missing, "library is on the search path");
    assert!(b.subsystem.is_some(), "library contents copied onto host");

    // The model declares no <PortCounts>; the linked subsystem's boundary
    // blocks define them.
    let counts = b.port_counts.as_ref().expect("derived port counts");
    assert_eq!((counts.ins, counts.outs), (Some(1), Some(1)));

    let cfg = rustylink::egui_app::get_block_type_cfg(b);
    // The viewer only asks for a label when the config says the block shows
    // them and the port has a defined name.
    assert!(cfg.show_input_port_labels && cfg.show_output_port_labels);
    assert_eq!(
        rustylink::egui_app::port_label_defined_name(b, 1, true, &cfg).as_deref(),
        Some("u")
    );
    assert_eq!(
        rustylink::egui_app::port_label_defined_name(b, 1, false, &cfg).as_deref(),
        Some("result")
    );
    assert_eq!(
        rustylink::egui_app::port_label_display_name(b, 1, true, &cfg),
        "u"
    );
    assert_eq!(
        rustylink::egui_app::port_label_display_name(b, 1, false, &cfg),
        "result"
    );
}

#[test]
fn a_library_block_inside_a_library_subsystem_is_found_by_its_path() {
    let dir = tempdir().expect("tempdir");
    write_slx(
        &dir.path().join("ASXTestLibrary.slx"),
        &[
            (
                "simulink/systems/system_root.xml",
                r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <Block BlockType="SubSystem" Name="Controls" SID="1">
    <P Name="Position">[0, 0, 100, 50]</P>
    <System Ref="system_2"/>
  </Block>
</System>
"#,
            ),
            (
                "simulink/systems/system_2.xml",
                r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <Block BlockType="SubSystem" Name="Logic" SID="2">
    <P Name="Position">[0, 0, 100, 50]</P>
    <System Ref="system_3"/>
  </Block>
</System>
"#,
            ),
            (
                "simulink/systems/system_3.xml",
                r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <Block BlockType="Inport" Name="setpoint" SID="3">
    <P Name="Position">[20, 20, 40, 40]</P>
    <P Name="Port">1</P>
  </Block>
  <Block BlockType="Outport" Name="command" SID="4">
    <P Name="Position">[120, 20, 140, 40]</P>
    <P Name="Port">1</P>
  </Block>
</System>
"#,
            ),
        ],
    );

    let host = r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <Reference Name="Logic" SID="53">
    <P Name="Position">[325, 264, 425, 306]</P>
    <P Name="SourceBlock">ASXTestLibrary/Controls/Logic</P>
    <P Name="SourceType">SubSystem</P>
  </Reference>
</System>
"#;
    let host_path = Utf8PathBuf::from("mem://host.xml");
    let mut files = HashMap::new();
    files.insert(host_path.as_str().to_string(), host.to_string());
    let mut parser = SimulinkParser::new("/", MemSource { files });
    let mut system = parser.parse_system_file(&host_path).expect("parse host");
    let lib_paths = vec![Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap()];
    SimulinkParser::<MemSource>::resolve_library_references(&mut system, &lib_paths)
        .expect("resolve library references");

    let b = &system.blocks[0];
    assert!(b.subsystem.is_some(), "nested library block resolved");
    let cfg = rustylink::egui_app::get_block_type_cfg(b);
    assert_eq!(
        rustylink::egui_app::port_label_display_name(b, 1, true, &cfg),
        "setpoint"
    );
    assert_eq!(
        rustylink::egui_app::port_label_display_name(b, 1, false, &cfg),
        "command"
    );
}
