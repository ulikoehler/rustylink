//! Generate Simulink system XML text from a [`System`] model.
//!
//! The generated XML exactly matches the format produced by MATLAB/Simulink
//! R2025b, including indentation, attribute ordering, and element ordering.

use crate::model::*;

/// Generate the XML text for a system file from a [`System`] model.
///
/// The output includes the XML declaration and uses 2-space indentation,
/// matching the format produced by Simulink.
pub fn generate_system_xml(system: &System) -> String {
    let mut out = String::with_capacity(4096);
    out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    write_system(&mut out, system, 0);
    out
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

/// Escape text content for XML. Matches Simulink's escaping which encodes
/// `&`, `<`, `>`, `"`, and `'` even in text content.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Escape an attribute value for XML. Like [`xml_escape`] but also encodes
/// newlines as `&#xA;` and carriage returns as `&#xD;`.
fn xml_escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            _ => out.push(ch),
        }
    }
    out
}

fn write_system(out: &mut String, system: &System, level: usize) {
    indent(out, level);
    out.push_str("<System>\n");

    // Properties
    for (name, value) in &system.properties {
        write_p(out, level + 1, name, value, false);
    }

    // Blocks
    for block in &system.blocks {
        write_block(out, block, level + 1);
    }

    // Lines
    for line in &system.lines {
        write_line(out, line, level + 1);
    }

    // Annotations
    for ann in &system.annotations {
        write_annotation(out, ann, level + 1);
    }

    indent(out, level);
    out.push_str("</System>\n");
}

fn write_p(out: &mut String, level: usize, name: &str, value: &str, is_ref: bool) {
    indent(out, level);
    if is_ref {
        out.push_str(&format!(
            "<P Name=\"{}\" Ref=\"{}\"/>\n",
            xml_escape_attr(name),
            xml_escape_attr(value)
        ));
    } else if value.is_empty() {
        // Empty text content: Simulink uses self-closing form
        out.push_str(&format!("<P Name=\"{}\"/>\n", xml_escape_attr(name)));
    } else {
        out.push_str(&format!(
            "<P Name=\"{}\">{}</P>\n",
            xml_escape_attr(name),
            xml_escape(value)
        ));
    }
}

fn write_block(out: &mut String, block: &Block, level: usize) {
    indent(out, level);

    // Tag name: "Block" or "Reference"
    let tag = &block.tag_name;
    out.push_str(&format!("<{}", tag));

    // Attributes: BlockType (for Block tags), Name, SID
    if tag == "Block" {
        out.push_str(&format!(" BlockType=\"{}\"", xml_escape_attr(&block.block_type)));
    }
    out.push_str(&format!(" Name=\"{}\"", xml_escape_attr(&block.name)));
    if let Some(ref sid) = block.sid {
        out.push_str(&format!(" SID=\"{}\"", xml_escape_attr(sid)));
    }
    out.push_str(">\n");

    if block.child_order.is_empty() {
        // Fallback: default ordering when child_order is not populated
        write_block_default_order(out, block, level);
    } else {
        // Use recorded child ordering for exact round-trip
        for kind in &block.child_order {
            match kind {
                BlockChildKind::PortCounts => {
                    if let Some(ref pc) = block.port_counts {
                        write_port_counts(out, pc, level + 1);
                    }
                }
                BlockChildKind::P(name) => {
                    if let Some(value) = block.properties.get(name) {
                        let is_ref = block.ref_properties.contains(name);
                        write_p(out, level + 1, name, value, is_ref);
                    }
                }
                BlockChildKind::InstanceData => {
                    if let Some(ref id) = block.instance_data {
                        write_instance_data(out, id, level + 1);
                    }
                }
                BlockChildKind::LinkData => {
                    if let Some(ref ld) = block.link_data {
                        write_link_data(out, ld, level + 1);
                    }
                }
                BlockChildKind::PortProperties => {
                    if !block.ports.is_empty() {
                        write_port_properties(out, &block.ports, level + 1);
                    }
                }
                BlockChildKind::Mask => {
                    if let Some(ref mask) = block.mask {
                        write_mask(out, mask, level + 1);
                    }
                }
                BlockChildKind::System => {
                    if let Some(ref ref_name) = block.system_ref {
                        indent(out, level + 1);
                        out.push_str(&format!("<System Ref=\"{}\"/>\n", xml_escape_attr(ref_name)));
                    } else if let Some(ref sub) = block.subsystem {
                        write_system(out, sub, level + 1);
                    }
                }
                BlockChildKind::Annotation(idx) => {
                    if let Some(ann) = block.annotations.get(*idx) {
                        write_annotation(out, ann, level + 1);
                    }
                }
            }
        }
    }

    indent(out, level);
    out.push_str(&format!("</{}>\n", tag));
}

fn write_port_counts(out: &mut String, pc: &PortCounts, level: usize) {
    indent(out, level);
    out.push_str("<PortCounts");
    if let Some(ins) = pc.ins {
        out.push_str(&format!(" in=\"{}\"", ins));
    }
    if let Some(outs) = pc.outs {
        out.push_str(&format!(" out=\"{}\"", outs));
    }
    out.push_str("/>\n");
}

/// Fallback block child ordering when `child_order` is empty.
fn write_block_default_order(out: &mut String, block: &Block, level: usize) {
    if let Some(ref pc) = block.port_counts {
        write_port_counts(out, pc, level + 1);
    }
    for (name, value) in &block.properties {
        let is_ref = block.ref_properties.contains(name);
        write_p(out, level + 1, name, value, is_ref);
    }
    if let Some(ref ld) = block.link_data {
        write_link_data(out, ld, level + 1);
    }
    if let Some(ref id) = block.instance_data {
        write_instance_data(out, id, level + 1);
    }
    if !block.ports.is_empty() {
        write_port_properties(out, &block.ports, level + 1);
    }
    if let Some(ref mask) = block.mask {
        write_mask(out, mask, level + 1);
    }
    if let Some(ref ref_name) = block.system_ref {
        indent(out, level + 1);
        out.push_str(&format!("<System Ref=\"{}\"/>\n", xml_escape_attr(ref_name)));
    } else if let Some(ref sub) = block.subsystem {
        write_system(out, sub, level + 1);
    }
    for ann in &block.annotations {
        write_annotation(out, ann, level + 1);
    }
}

fn write_instance_data(out: &mut String, id: &InstanceData, level: usize) {
    indent(out, level);
    out.push_str("<InstanceData>\n");
    for (name, value) in &id.properties {
        write_p(out, level + 1, name, value, false);
    }
    indent(out, level);
    out.push_str("</InstanceData>\n");
}

fn write_link_data(out: &mut String, ld: &LinkData, level: usize) {
    indent(out, level);
    out.push_str("<LinkData>\n");
    for dp in &ld.dialog_parameters {
        indent(out, level + 1);
        out.push_str(&format!(
            "<DialogParameters BlockName=\"{}\">\n",
            xml_escape_attr(&dp.block_name)
        ));
        for (name, value) in &dp.properties {
            write_p(out, level + 2, name, value, false);
        }
        indent(out, level + 1);
        out.push_str("</DialogParameters>\n");
    }
    indent(out, level);
    out.push_str("</LinkData>\n");
}

fn write_port_properties(out: &mut String, ports: &[Port], level: usize) {
    indent(out, level);
    out.push_str("<PortProperties>\n");
    for port in ports {
        indent(out, level + 1);
        out.push_str(&format!("<Port Type=\"{}\"", xml_escape(&port.port_type)));
        if let Some(idx) = port.index {
            out.push_str(&format!(" Index=\"{}\"", idx));
        }
        out.push_str(">\n");
        for (name, value) in &port.properties {
            write_p(out, level + 2, name, value, false);
        }
        indent(out, level + 1);
        out.push_str("</Port>\n");
    }
    indent(out, level);
    out.push_str("</PortProperties>\n");
}

fn write_mask(out: &mut String, mask: &Mask, level: usize) {
    indent(out, level);
    out.push_str("<Mask>\n");
    if let Some(ref display) = mask.display {
        indent(out, level + 1);
        out.push_str("<Display");
        for (attr_name, attr_val) in &mask.display_attrs {
            out.push_str(&format!(
                " {}=\"{}\"",
                xml_escape_attr(attr_name),
                xml_escape_attr(attr_val)
            ));
        }
        out.push_str(&format!(">{}</Display>\n", xml_escape(display)));
    }
    if let Some(ref desc) = mask.description {
        indent(out, level + 1);
        out.push_str(&format!(
            "<Description>{}</Description>\n",
            xml_escape(desc)
        ));
    }
    if let Some(ref init) = mask.initialization {
        indent(out, level + 1);
        out.push_str(&format!(
            "<Initialization>{}</Initialization>\n",
            xml_escape(init)
        ));
    }
    if let Some(ref help) = mask.help {
        indent(out, level + 1);
        out.push_str(&format!("<Help>{}</Help>\n", xml_escape(help)));
    }
    for param in &mask.parameters {
        write_mask_parameter(out, param, level + 1);
    }
    for dc in &mask.dialog {
        write_dialog_control(out, dc, level + 1);
    }
    indent(out, level);
    out.push_str("</Mask>\n");
}

fn write_mask_parameter(out: &mut String, param: &MaskParameter, level: usize) {
    indent(out, level);
    // Use all_attrs for round-trip fidelity if available, otherwise fallback
    if !param.all_attrs.is_empty() {
        out.push_str("<MaskParameter");
        for (attr_name, attr_val) in &param.all_attrs {
            out.push_str(&format!(
                " {}=\"{}\"",
                xml_escape_attr(attr_name),
                xml_escape_attr(attr_val)
            ));
        }
        out.push_str(">\n");
    } else {
        let type_str = match &param.param_type {
            MaskParamType::Popup => "popup",
            MaskParamType::Edit => "edit",
            MaskParamType::Checkbox => "checkbox",
            MaskParamType::Unknown(s) => s.as_str(),
        };
        out.push_str(&format!(
            "<MaskParameter Name=\"{}\" Type=\"{}\"",
            xml_escape_attr(&param.name),
            xml_escape_attr(type_str)
        ));
        if let Some(tunable) = param.tunable {
            out.push_str(&format!(
                " Tunable=\"{}\"",
                if tunable { "on" } else { "off" }
            ));
        }
        if let Some(visible) = param.visible {
            out.push_str(&format!(
                " Visible=\"{}\"",
                if visible { "on" } else { "off" }
            ));
        }
        out.push_str(">\n");
    }

    if let Some(ref prompt) = param.prompt {
        indent(out, level + 1);
        out.push_str(&format!("<Prompt>{}</Prompt>\n", xml_escape(prompt)));
    }
    if let Some(ref value) = param.value {
        indent(out, level + 1);
        out.push_str(&format!("<Value>{}</Value>\n", xml_escape(value)));
    }
    if !param.type_options.is_empty() {
        indent(out, level + 1);
        out.push_str("<TypeOptions>\n");
        for opt in &param.type_options {
            indent(out, level + 2);
            out.push_str(&format!("<Option>{}</Option>\n", xml_escape(opt)));
        }
        indent(out, level + 1);
        out.push_str("</TypeOptions>\n");
    }
    if let Some(ref callback) = param.callback {
        indent(out, level + 1);
        out.push_str(&format!("<Callback>{}</Callback>\n", xml_escape(callback)));
    }

    indent(out, level);
    out.push_str("</MaskParameter>\n");
}

fn write_dialog_control(out: &mut String, dc: &DialogControl, level: usize) {
    indent(out, level);
    let type_str = match &dc.control_type {
        DialogControlType::Group => "Group",
        DialogControlType::Text => "Text",
        DialogControlType::Edit => "Edit",
        DialogControlType::CheckBox => "CheckBox",
        DialogControlType::Popup => "Popup",
        DialogControlType::Unknown(s) => s.as_str(),
    };
    out.push_str(&format!("<DialogControl Type=\"{}\"", xml_escape(type_str)));
    if let Some(ref name) = dc.name {
        out.push_str(&format!(" Name=\"{}\"", xml_escape(name)));
    }
    out.push_str(">\n");

    if let Some(ref prompt) = dc.prompt {
        indent(out, level + 1);
        out.push_str(&format!("<Prompt>{}</Prompt>\n", xml_escape(prompt)));
    }
    if let Some(ref opts) = dc.control_options {
        indent(out, level + 1);
        out.push_str("<ControlOptions");
        if let Some(ref pl) = opts.prompt_location {
            out.push_str(&format!(" PromptLocation=\"{}\"", xml_escape(pl)));
        }
        out.push_str("/>\n");
    }
    for child in &dc.children {
        write_dialog_control(out, child, level + 1);
    }

    indent(out, level);
    out.push_str("</DialogControl>\n");
}

fn write_line(out: &mut String, line: &Line, level: usize) {
    indent(out, level);
    out.push_str("<Line>\n");

    // Write P elements in their original order from the properties map
    for (name, value) in &line.properties {
        write_p(out, level + 1, name, value, false);
    }

    // Branches
    for branch in &line.branches {
        write_branch(out, branch, level + 1);
    }

    indent(out, level);
    out.push_str("</Line>\n");
}

fn write_branch(out: &mut String, branch: &Branch, level: usize) {
    indent(out, level);
    out.push_str("<Branch>\n");

    for (name, value) in &branch.properties {
        write_p(out, level + 1, name, value, false);
    }

    for sub in &branch.branches {
        write_branch(out, sub, level + 1);
    }

    indent(out, level);
    out.push_str("</Branch>\n");
}

fn write_annotation(out: &mut String, ann: &Annotation, level: usize) {
    indent(out, level);
    out.push_str("<Annotation");
    if let Some(ref sid) = ann.sid {
        out.push_str(&format!(" SID=\"{}\"", xml_escape_attr(sid)));
    }
    out.push_str(">\n");

    for (name, value) in &ann.properties {
        write_p(out, level + 1, name, value, false);
    }

    indent(out, level);
    out.push_str("</Annotation>\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    #[test]
    fn test_simple_system_roundtrip() {
        let mut props = IndexMap::new();
        props.insert("Location".to_string(), "[0, 0, 1920, 1036]".to_string());
        props.insert("Open".to_string(), "on".to_string());

        let system = System {
            properties: props,
            blocks: vec![],
            lines: vec![],
            annotations: vec![],
            chart: None,
        };

        let xml = generate_system_xml(&system);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
        assert!(xml.contains("<P Name=\"Location\">[0, 0, 1920, 1036]</P>"));
        assert!(xml.contains("<P Name=\"Open\">on</P>"));
    }

    #[test]
    fn test_block_with_port_counts() {
        let system = System {
            properties: IndexMap::new(),
            blocks: vec![Block {
                block_type: "Gain".into(),
                name: "G1".into(),
                sid: Some("5".into()),
                tag_name: "Block".into(),
                position: Some("[10, 20, 50, 60]".into()),
                zorder: Some("1".into()),
                commented: false,
                name_location: NameLocation::Bottom,
                is_matlab_function: false,
                value: None,
                value_kind: ValueKind::Unknown,
                value_rows: None,
                value_cols: None,
                properties: {
                    let mut m = IndexMap::new();
                    m.insert("Position".into(), "[10, 20, 50, 60]".into());
                    m.insert("ZOrder".into(), "1".into());
                    m
                },
                ref_properties: Default::default(),
                port_counts: Some(PortCounts {
                    ins: Some(1),
                    outs: Some(1),
                }),
                ports: vec![],
                subsystem: None,
                system_ref: None,
                c_function: None,
                instance_data: None,
                link_data: None,
                mask: None,
                annotations: vec![],
                background_color: None,
                show_name: None,
                font_size: None,
                font_weight: None,
                mask_display_text: None,
                current_setting: None,
                block_mirror: None,
                library_source: None,
                library_block_path: None,
                child_order: vec![],
            }],
            lines: vec![],
            annotations: vec![],
            chart: None,
        };

        let xml = generate_system_xml(&system);
        assert!(xml.contains("<PortCounts in=\"1\" out=\"1\"/>"));
        assert!(xml.contains("BlockType=\"Gain\""));
        assert!(xml.contains("Name=\"G1\""));
        assert!(xml.contains("SID=\"5\""));
    }
}
