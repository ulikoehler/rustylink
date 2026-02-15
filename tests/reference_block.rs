use anyhow::Result;
use camino::Utf8PathBuf;
use rustylink::parser::{ContentSource, SimulinkParser};
use std::collections::HashMap;

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
        let mut out = Vec::new();
        for k in self.files.keys() {
            if k.starts_with(&prefix) {
                out.push(Utf8PathBuf::from(k.clone()));
            }
        }
        Ok(out)
    }
}

#[test]
fn parse_reference_tag_as_block() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <Reference Name="Logic" SID="53">
    <P Name="Position">[325, 264, 425, 306]</P>
    <P Name="ZOrder">377</P>
    <P Name="LibraryVersion">1.1</P>
    <P Name="SourceBlock">ASXTestLibrary/Logic</P>
    <P Name="SourceType">SubSystem</P>
    <PortProperties>
      <Port Type="out" Index="1">
        <P Name="Name">result</P>
        <P Name="TestPoint">on</P>
      </Port>
    </PortProperties>
  </Reference>
</System>
"#;

    let path = Utf8PathBuf::from("mem://reference_test.xml");
    let mut files = HashMap::new();
    files.insert(path.as_str().to_string(), xml.to_string());
    let source = MemSource { files };
    let mut parser = SimulinkParser::new("/", source);
    let system = parser.parse_system_file(&path).expect("parse system XML");

    assert_eq!(system.blocks.len(), 1);
    let b = &system.blocks[0];
    assert_eq!(b.name, "Logic");
    assert_eq!(b.sid.as_deref(), Some("53"));
    // Tag <Reference> should be treated as a Reference block
    assert_eq!(b.block_type, "Reference");
    assert_eq!(b.ports.len(), 1);
    assert_eq!(b.ports[0].port_type, "out");
}
