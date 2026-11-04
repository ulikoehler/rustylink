//! E2E: parser behavior — real XML → System, malformed/missing inputs (§6/§17).

#![cfg(feature = "egui")]

#[path = "e2e/harness/mod.rs"]
mod harness;

use anyhow::Result;
use camino::Utf8PathBuf;
use rustylink::model::SlxArchive;
use rustylink::parser::{ContentSource, SimulinkParser};
use std::collections::HashMap;
use tempfile::TempDir;

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
    fn list_dir(&mut self, _path: &camino::Utf8Path) -> Result<Vec<Utf8PathBuf>> {
        Ok(vec![])
    }
}

fn parse_xml(xml: &str) -> Result<rustylink::model::System> {
    let path = Utf8PathBuf::from("mem://sys.xml");
    let mut files = HashMap::new();
    files.insert(path.as_str().to_string(), xml.to_string());
    let mut p = SimulinkParser::new("/", MemSource { files });
    p.parse_system_file(&path)
}

const MINIMAL: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <Block BlockType="Constant" Name="C" SID="1">
    <P Name="Position">[0, 0, 40, 24]</P>
  </Block>
  <Block BlockType="Display" Name="D" SID="2">
    <P Name="Position">[100, 0, 140, 24]</P>
  </Block>
  <Line>
    <P Name="Src">1#out:1</P>
    <P Name="Dst">2#in:1</P>
  </Line>
</System>
"#;

#[test]
fn par_001_minimal_system_parses() {
    let sys = parse_xml(MINIMAL).unwrap();
    assert_eq!(sys.blocks.len(), 2);
    assert_eq!(sys.lines.len(), 1);
    let l = &sys.lines[0];
    assert_eq!(l.src.as_ref().unwrap().sid, "1");
    assert_eq!(l.dst.as_ref().unwrap().sid, "2");
    harness::debug_validate(&sys);
}

#[test]
fn par_010_no_system_root_errors() {
    assert!(parse_xml("<Foo><Bar/></Foo>").is_err());
}

#[test]
fn par_011_malformed_xml_errors() {
    assert!(parse_xml("<System><Block").is_err());
    assert!(parse_xml("not xml at all").is_err());
}

#[test]
fn par_012_empty_xml_errors() {
    assert!(parse_xml("").is_err());
}

#[test]
fn par_013_non_utf8_safe() {
    // Parser takes &str — invalid UTF-8 can't even reach it; ensure a file
    // full of NULs inside an .slx errors rather than panics.
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nul.slx");
    harness::slx_factory::raw_zip(
        &dir.path().join("nul.slx"),
        &[("simulink/systems/system_root.xml", &[0xff, 0xfe, 0x00][..])],
    )
    .unwrap();
    let _ = SlxArchive::from_file(&path); // must not panic
}

#[test]
fn par_020_block_positions_parsed() {
    let sys = parse_xml(MINIMAL).unwrap();
    let c = &sys.blocks[0];
    assert_eq!(
        harness::parse_pos(c.position.as_deref().unwrap()),
        Some((0, 0, 40, 24))
    );
}

#[test]
fn par_021_missing_position_block_filtered() {
    // Blocks without Position parse but are filtered from canvas layout.
    let xml = r#"<?xml version="1.0"?>
<System>
  <Block BlockType="Constant" Name="NoPos" SID="9"/>
  <Block BlockType="Gain" Name="G" SID="10"><P Name="Position">[0,0,40,24]</P></Block>
</System>"#;
    let sys = parse_xml(xml).unwrap();
    assert_eq!(sys.blocks.len(), 2);
    assert!(sys.blocks[0].position.is_none());
    harness::debug_validate(&sys);
}

#[test]
fn par_030_nested_system_element_parses() {
    let xml = r#"<?xml version="1.0"?>
<System>
  <Block BlockType="SubSystem" Name="SS" SID="5">
    <P Name="Position">[0,0,80,40]</P>
    <System>
      <Block BlockType="Gain" Name="Inner" SID="6"><P Name="Position">[0,0,40,24]</P></Block>
    </System>
  </Block>
</System>"#;
    let sys = parse_xml(xml).unwrap();
    let ss = &sys.blocks[0];
    assert!(ss.subsystem.is_some(), "nested <System> parsed");
    assert_eq!(ss.subsystem.as_ref().unwrap().blocks[0].name, "Inner");
    harness::debug_validate(&sys);
}

#[test]
fn par_040_xml_entity_escapes() {
    let xml = r#"<?xml version="1.0"?>
<System>
  <Block BlockType="Constant" Name="A &amp; B &quot;quoted&quot;" SID="1">
    <P Name="Position">[0,0,40,24]</P>
  </Block>
</System>"#;
    let sys = parse_xml(xml).unwrap();
    assert_eq!(sys.blocks[0].name, "A & B \"quoted\"");
}

#[test]
fn par_041_survives_large_attribute_values() {
    let big = "x".repeat(64 * 1024);
    let xml = format!(
        r#"<?xml version="1.0"?>
<System>
  <Block BlockType="Constant" Name="{big}" SID="1"><P Name="Position">[0,0,40,24]</P></Block>
</System>"#
    );
    let sys = parse_xml(&xml).unwrap();
    assert_eq!(sys.blocks[0].name.len(), 64 * 1024);
}
