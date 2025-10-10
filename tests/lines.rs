use rustylink::parser::{ContentSource, SimulinkParser};
use camino::Utf8PathBuf;
use anyhow::Result;
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
}

#[test]
fn parse_lines_and_branches_points_and_endpoints() {
    // Minimal system XML containing a few lines and branches based on the provided example
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <Line>
    <P Name="Name">frequency</P>
    <P Name="ZOrder">4</P>
    <P Name="Src">5#out:1</P>
    <P Name="Points">[50, 0]</P>
    <Branch>
      <P Name="ZOrder">3</P>
      <P Name="Points">[0, -105]</P>
      <P Name="Dst">11#in:2</P>
    </Branch>
    <Branch>
      <P Name="ZOrder">2</P>
      <P Name="Labels">[1, 1]</P>
      <P Name="Dst">2#in:2</P>
    </Branch>
  </Line>
</System>
"#;

    let path = Utf8PathBuf::from("mem://system_22.xml");
    let mut files = HashMap::new();
    files.insert(path.as_str().to_string(), xml.to_string());
    let source = MemSource { files };
    let mut parser = SimulinkParser::new("/", source);
    let system = parser.parse_system_file(&path).expect("parse system XML");

    // Ensure we got some lines
    assert!(!system.lines.is_empty(), "expected at least one <Line>");

    // Find the frequency line example
    let freq = system
        .lines
        .iter()
        .find(|l| l.name.as_deref() == Some("frequency"))
        .expect("frequency line present");

    // Points parsed
    assert_eq!(freq.points.len(), 1);
    assert_eq!((freq.points[0].x, freq.points[0].y), (50, 0));

    // Src endpoint parsed
    let src = freq.src.as_ref().expect("src present");
    assert_eq!(src.sid, 5);
    assert_eq!(src.port_type, "out");
    assert_eq!(src.port_index, 1);

    // Branches parsed, with nested points and dst endpoint
    assert!(freq.branches.len() >= 2);
    // One branch should have Dst 11#in:2 and a single point [0, -105]
    let b = freq
        .branches
        .iter()
        .find(|b| b.dst.as_ref().map(|d| (d.sid, d.port_type.as_str(), d.port_index)) == Some((11, "in", 2)))
        .expect("branch to 11#in:2");
    assert_eq!(b.points.len(), 1);
    assert_eq!((b.points[0].x, b.points[0].y), (0, -105));
}
