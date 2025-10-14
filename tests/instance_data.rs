use rustylink::parser::{ContentSource, SimulinkParser};
use camino::Utf8PathBuf;
use anyhow::Result;
use std::collections::HashMap;

struct MemSource { files: HashMap<String, String> }
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
            if k.starts_with(&prefix) { out.push(Utf8PathBuf::from(k.clone())); }
        }
        Ok(out)
    }
}

#[test]
fn parse_block_instance_data_kv() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <Block BlockType="Reference" Name="Detect Increase7" SID="1195">
    <PortCounts in="1" out="1"/>
    <P Name="Position">[220, 914, 280, 946]</P>
    <P Name="ZOrder">2540</P>
    <P Name="LibraryVersion">7.11</P>
    <InstanceData>
      <P Name="ContentPreviewEnabled">off</P>
      <P Name="vinit">0.0</P>
      <P Name="OutDataTypeStr">boolean</P>
    </InstanceData>
  </Block>
</System>
"#;

    let path = Utf8PathBuf::from("mem://system_instancedata.xml");
    let mut files = HashMap::new();
    files.insert(path.as_str().to_string(), xml.to_string());
    let source = MemSource { files };
    let mut parser = SimulinkParser::new("/", source);
    let system = parser.parse_system_file(&path).expect("parse system XML");

    assert_eq!(system.blocks.len(), 1);
    let b0 = &system.blocks[0];
    assert_eq!(b0.name, "Detect Increase7");
    let id = b0.instance_data.as_ref().expect("InstanceData present");
    assert_eq!(id.properties.get("ContentPreviewEnabled").map(|s| s.as_str()), Some("off"));
    assert_eq!(id.properties.get("vinit").map(|s| s.as_str()), Some("0.0"));
    assert_eq!(id.properties.get("OutDataTypeStr").map(|s| s.as_str()), Some("boolean"));
}
