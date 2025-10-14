use camino::Utf8PathBuf;
use rustylink::parser::{FsSource, SimulinkParser};
use std::fs;

// Helper to write a temporary XML file structure
#[test]
fn test_mask_display_evaluated() {
    let temp_dir = tempfile::tempdir().unwrap();
    let systems_dir = temp_dir.path().join("simulink/systems");
    std::fs::create_dir_all(&systems_dir).unwrap();
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<System>
  <P Name="Name">Root</P>
  <Block BlockType="SubSystem" Name="Choose Application7" SID="183">
    <P Name="Position">[0,0,100,40]</P>
    <Mask>
      <Display>disp(mytab{control})</Display>
      <Initialization>mytab={'Position','Zero Torque','OFF'};</Initialization>
      <MaskParameter Name="control" Type="popup">
        <Value>1. Position Control</Value>
      </MaskParameter>
    </Mask>
  </Block>
</System>
"#;
    let sys_path = systems_dir.join("system_183.xml");
    fs::write(&sys_path, xml).unwrap();
    // Create another system file referencing the block system (simulate root)
    let root_path = systems_dir.join("system_1.xml");
    fs::write(&root_path, xml).unwrap();
    let root_utf8 = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
    let mut parser = SimulinkParser::new(&root_utf8, FsSource);
    let system = parser
        .parse_system_file(Utf8PathBuf::from_path_buf(root_path.clone()).unwrap())
        .unwrap();
    // Find block
    let blk = system
        .blocks
        .iter()
        .find(|b| b.name == "Choose Application7")
        .expect("block");
    assert_eq!(blk.mask_display_text.as_deref(), Some("Position"));
}
