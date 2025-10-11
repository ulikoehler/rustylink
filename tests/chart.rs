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
fn parse_chart_and_link_from_system_ref() {
    // Minimal system containing a subsystem referencing system_18 which doesn't exist as XML in MemSource
    let sys_root = r#"<?xml version=\"1.0\" encoding=\"utf-8\"?>
<System>
  <Block BlockType=\"SubSystem\" Name=\"Wall clock\" SID=\"18\">
    <System Ref=\"system_18\"/>
  </Block>
</System>
"#;

    // Chart XML based on provided sample
    let chart_18 = r#"<?xml version=\"1.0\" encoding=\"utf-8\"?>
<chart id=\"18\">
  <P Name=\"name\">Logic/MATLAB Function</P>
  <eml>
    <P Name=\"name\">generateSine</P>
  </eml>
  <Children>
    <state SSID=\"1\">
      <P Name=\"labelString\">eML_blk_kernel()</P>
      <eml>
        <P Name=\"isEML\">1</P>
        <P Name=\"script\">function y = generateSine(phaseDeg, freq, amp, t)\n% comment\ny = amp * sin(2*pi*freq*t + deg2rad(phaseDeg));\nend</P>
      </eml>
    </state>
    <data SSID=\"4\" name=\"phaseDeg\">
      <P Name=\"scope\">INPUT_DATA</P>
      <props>
        <array><P Name=\"size\">-1</P></array>
        <type>
          <P Name=\"method\">SF_INHERITED_TYPE</P>
          <P Name=\"primitive\">SF_DOUBLE_TYPE</P>
        </type>
        <P Name=\"complexity\">SF_COMPLEX_INHERITED</P>
        <unit><P Name=\"name\">inherit</P></unit>
      </props>
      <P Name=\"dataType\">Inherit: Same as Simulink</P>
    </data>
    <data SSID=\"5\" name=\"y\">
      <P Name=\"scope\">OUTPUT_DATA</P>
      <props>
        <array><P Name=\"size\">-1</P></array>
        <type>
          <P Name=\"method\">SF_INHERITED_TYPE</P>
          <P Name=\"primitive\">SF_DOUBLE_TYPE</P>
        </type>
        <P Name=\"complexity\">SF_COMPLEX_INHERITED</P>
        <unit><P Name=\"name\">inherit</P></unit>
      </props>
      <P Name=\"dataType\">Inherit: Same as Simulink</P>
    </data>
  </Children>
</chart>
"#;

    let base = Utf8PathBuf::from("/simulink/systems");
    let mut files = HashMap::new();
    files.insert(base.join("system_root.xml").as_str().to_string(), sys_root.to_string());
    files.insert("/simulink/stateflow/chart_18.xml".to_string(), chart_18.to_string());

    let source = MemSource { files };
    let mut parser = SimulinkParser::new("/", source);

    let system = parser.parse_system_file(base.join("system_root.xml")).expect("parse system");
    assert_eq!(system.blocks.len(), 1);
    let blk = &system.blocks[0];
    let sub = blk.subsystem.as_ref().expect("subsystem resolved");
    let chart = sub.chart.as_ref().expect("chart attached to system");
    assert_eq!(chart.id, Some(18));
    assert_eq!(chart.eml_name.as_deref(), Some("generateSine"));
    assert!(chart.script.as_ref().map(|s| s.contains("generateSine")).unwrap_or(false));
    assert!(chart.inputs.iter().any(|p| p.name == "phaseDeg"));
    assert!(chart.outputs.iter().any(|p| p.name == "y"));
}
