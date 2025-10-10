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
}

#[test]
fn parse_block_sid_as_u32() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <Block BlockType="Product" Name="Product1" SID="52">
    <P Name="Position">[10, 10, 40, 40]</P>
  </Block>
</System>
"#;

    let path = Utf8PathBuf::from("mem://system_test.xml");
    let mut files = HashMap::new();
    files.insert(path.as_str().to_string(), xml.to_string());
    let source = MemSource { files };
    let mut parser = SimulinkParser::new("/", source);
    let system = parser.parse_system_file(&path).expect("parse system XML");

    assert_eq!(system.blocks.len(), 1);
    let b = &system.blocks[0];
    assert_eq!(b.name, "Product1");
    assert_eq!(b.sid, Some(52));
}
