use anyhow::Result;
use camino::Utf8PathBuf;
use rustylink::parser::{ContentSource, SimulinkParser};
use rustylink::model::SystemDoc;
use std::collections::HashMap;
use tempfile::NamedTempFile;

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

#[test]
fn test_binary_serialization() -> Result<()> {
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
    
    let doc = SystemDoc { system };

    // Create a temporary file
    let temp_file = NamedTempFile::new()?;
    let temp_path = temp_file.path();

    // Save to binary
    doc.save_to_binary(temp_path)?;

    // Load from binary
    let loaded_doc = SystemDoc::load_from_binary(temp_path)?;

    // Verify content
    assert_eq!(loaded_doc.system.blocks.len(), 2);
    assert_eq!(loaded_doc.system.blocks[0].name, "Product1");
    assert_eq!(loaded_doc.system.blocks[0].sid.as_deref(), Some("52"));
    assert_eq!(loaded_doc.system.blocks[1].name, "freq");
    assert_eq!(loaded_doc.system.blocks[1].sid.as_deref(), Some("2::28"));
    
    assert_eq!(loaded_doc.system.lines.len(), 1);
    let l = &loaded_doc.system.lines[0];
    assert_eq!(l.src.as_ref().map(|e| e.sid.as_str()), Some("2::28"));
    assert_eq!(l.dst.as_ref().map(|e| e.sid.as_str()), Some("52"));

    Ok(())
}
