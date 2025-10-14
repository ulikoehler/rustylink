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
fn parse_block_sid_as_string_and_endpoint() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <Block BlockType="Product" Name="Product1" SID="52">
    <P Name="Position">[10, 10, 40, 40]</P>
  </Block>
  <Block BlockType="Inport" Name="freq" SID="2::28">
    <P Name="Position">[10, 50, 40, 70]</P>
  </Block>
  <Line>
    <P Name="Src">2::28#out:1</P>
    <P Name="Dst">52#in:1</P>
  </Line>
</System>
"#;

    let path = Utf8PathBuf::from("mem://system_test.xml");
    let mut files = HashMap::new();
    files.insert(path.as_str().to_string(), xml.to_string());
    let source = MemSource { files };
    let mut parser = SimulinkParser::new("/", source);
    let system = parser.parse_system_file(&path).expect("parse system XML");

  assert_eq!(system.blocks.len(), 2);
  let b0 = &system.blocks[0];
  assert_eq!(b0.name, "Product1");
  assert_eq!(b0.sid.as_deref(), Some("52"));
  let b1 = &system.blocks[1];
  assert_eq!(b1.name, "freq");
  assert_eq!(b1.sid.as_deref(), Some("2::28"));
  // Endpoints should parse as strings too
  assert!(system.lines.len() >= 1);
  let l = &system.lines[0];
  assert_eq!(l.src.as_ref().map(|e| e.sid.as_str()), Some("2::28"));
  assert_eq!(l.dst.as_ref().map(|e| e.sid.as_str()), Some("52"));
}
