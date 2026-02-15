//! Block, Line, Annotation, and System XML parsing.
//!
//! All parsing functions produce model types that preserve the full fidelity of
//! the original XML so that system files can be exactly regenerated.

use crate::model::*;
use anyhow::Result;
use camino::Utf8Path;
use indexmap::IndexMap;
use roxmltree::Node;

// ────────────────────────────────────────────────────────────────────────────
// Annotation
// ────────────────────────────────────────────────────────────────────────────

pub fn parse_annotation_node(node: Node) -> Result<Annotation> {
    let sid = node.attribute("SID").map(|s| s.to_string());
    let mut position: Option<String> = None;
    let mut zorder: Option<String> = None;
    let mut interpreter: Option<String> = None;
    let mut text: Option<String> = None;
    let mut properties: IndexMap<String, String> = IndexMap::new();

    for child in node
        .children()
        .filter(|c| c.is_element() && c.has_tag_name("P"))
    {
        if let Some(nm) = child.attribute("Name") {
            let val = child.text().unwrap_or("").to_string();
            match nm {
                "Position" => position = Some(val.clone()),
                "ZOrder" => zorder = Some(val.clone()),
                "Interpreter" => interpreter = Some(val.clone()),
                "Name" => {
                    text = Some(val.clone());
                }
                _ => {}
            }
            properties.insert(nm.to_string(), val);
        }
    }

    Ok(Annotation {
        sid,
        text,
        position,
        zorder,
        interpreter,
        properties,
    })
}

// ────────────────────────────────────────────────────────────────────────────
// Mask
// ────────────────────────────────────────────────────────────────────────────

pub fn parse_mask_node(node: Node) -> Result<Mask> {
    let mut display: Option<String> = None;
    let mut display_attrs: IndexMap<String, String> = IndexMap::new();
    let mut description: Option<String> = None;
    let mut initialization: Option<String> = None;
    let mut help: Option<String> = None;
    let mut parameters: Vec<MaskParameter> = Vec::new();
    let mut dialog: Vec<DialogControl> = Vec::new();

    for child in node.children().filter(|c| c.is_element()) {
        match child.tag_name().name() {
            "Display" => {
                display = child.text().map(|s| s.to_string());
                // Capture all attributes on <Display>
                for attr in child.attributes() {
                    display_attrs.insert(
                        attr.name().to_string(),
                        attr.value().to_string(),
                    );
                }
            }
            "Description" => description = child.text().map(|s| s.to_string()),
            "Initialization" => initialization = child.text().map(|s| s.to_string()),
            "MaskParameter" => {
                parameters.push(parse_mask_parameter_node(child));
            }
            "DialogControl" => {
                dialog.push(parse_dialog_control_node(child));
            }
            "Help" => help = child.text().map(|s| s.to_string()),
            _other => {}
        }
    }

    Ok(Mask {
        display,
        display_attrs,
        description,
        initialization,
        help,
        parameters,
        dialog,
    })
}

// ────────────────────────────────────────────────────────────────────────────
// InstanceData
// ────────────────────────────────────────────────────────────────────────────

pub fn parse_instance_data_node(node: Node) -> Result<InstanceData> {
    let mut props: IndexMap<String, String> = IndexMap::new();
    for p in node
        .children()
        .filter(|c| c.is_element() && c.has_tag_name("P"))
    {
        if let Some(nm) = p.attribute("Name") {
            let val = p.text().unwrap_or("").to_string();
            props.insert(nm.to_string(), val);
        }
    }
    Ok(InstanceData { properties: props })
}

// ────────────────────────────────────────────────────────────────────────────
// MaskParameter
// ────────────────────────────────────────────────────────────────────────────

fn matches_ignore_case(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

pub fn parse_mask_parameter_node(node: Node) -> MaskParameter {
    let name = node.attribute("Name").unwrap_or("").to_string();
    let tattr = node.attribute("Type").unwrap_or("");
    let param_type = match tattr {
        t if t.eq_ignore_ascii_case("popup") => MaskParamType::Popup,
        t if t.eq_ignore_ascii_case("edit") => MaskParamType::Edit,
        t if t.eq_ignore_ascii_case("checkbox") => MaskParamType::Checkbox,
        other => MaskParamType::Unknown(other.to_string()),
    };
    let tunable = node
        .attribute("Tunable")
        .map(|v| matches_ignore_case(v, "on") || v == "1");
    let visible = node
        .attribute("Visible")
        .map(|v| matches_ignore_case(v, "on") || v == "1");

    // Capture ALL attributes in their document order for round-trip generation
    let mut all_attrs = IndexMap::new();
    for attr in node.attributes() {
        all_attrs.insert(attr.name().to_string(), attr.value().to_string());
    }

    let mut prompt: Option<String> = None;
    let mut value: Option<String> = None;
    let mut callback: Option<String> = None;
    let mut type_options: Vec<String> = Vec::new();

    for child in node.children().filter(|c| c.is_element()) {
        match child.tag_name().name() {
            "Prompt" => prompt = child.text().map(|s| s.to_string()),
            "Value" => value = child.text().map(|s| s.to_string()),
            "TypeOptions" => {
                for to in child.children().filter(|c| c.is_element()) {
                    if to.has_tag_name("Option") {
                        if let Some(t) = to.text() {
                            type_options.push(t.to_string());
                        }
                    }
                }
            }
            "Callback" => callback = child.text().map(|s| s.to_string()),
            _ => {}
        }
    }

    MaskParameter {
        name,
        param_type,
        prompt,
        value,
        callback,
        tunable,
        visible,
        type_options,
        all_attrs,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// DialogControl
// ────────────────────────────────────────────────────────────────────────────

pub fn parse_dialog_control_node(node: Node) -> DialogControl {
    let tattr = node.attribute("Type").unwrap_or("");
    let control_type = match tattr {
        t if t.eq_ignore_ascii_case("Group") => DialogControlType::Group,
        t if t.eq_ignore_ascii_case("Text") => DialogControlType::Text,
        t if t.eq_ignore_ascii_case("Edit") => DialogControlType::Edit,
        t if t.eq_ignore_ascii_case("CheckBox") => DialogControlType::CheckBox,
        t if t.eq_ignore_ascii_case("Popup") => DialogControlType::Popup,
        other => DialogControlType::Unknown(other.to_string()),
    };
    let name = node.attribute("Name").map(|s| s.to_string());

    let mut prompt: Option<String> = None;
    let mut control_options: Option<ControlOptions> = None;
    let mut children: Vec<DialogControl> = Vec::new();

    for child in node.children().filter(|c| c.is_element()) {
        match child.tag_name().name() {
            "Prompt" => prompt = child.text().map(|s| s.to_string()),
            "ControlOptions" => {
                let mut opts = ControlOptions::default();
                if let Some(pl) = child.attribute("PromptLocation") {
                    opts.prompt_location = Some(pl.to_string());
                }
                control_options = Some(opts);
            }
            "DialogControl" => children.push(parse_dialog_control_node(child)),
            _ => {}
        }
    }

    DialogControl {
        control_type,
        name,
        prompt,
        control_options,
        children,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Value shape analysis
// ────────────────────────────────────────────────────────────────────────────

fn parse_value_shape(val: &str) -> (ValueKind, Option<u32>, Option<u32>) {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return (ValueKind::Unknown, None, None);
    }
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return (ValueKind::Scalar, Some(1), Some(1));
    }
    let inner = &trimmed[1..trimmed.len().saturating_sub(1)];
    if inner.trim().is_empty() {
        return (ValueKind::Unknown, None, None);
    }
    let rows: Vec<&str> = inner.split(';').collect();
    let row_count = rows.len();
    let mut col_count: Option<usize> = None;
    for row in &rows {
        let cols: Vec<&str> = row
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if cols.is_empty() {
            return (ValueKind::Unknown, None, None);
        }
        match col_count {
            None => col_count = Some(cols.len()),
            Some(c) if c != cols.len() => return (ValueKind::Unknown, None, None),
            _ => {}
        }
    }
    let cols_final = col_count.unwrap_or(0);
    if row_count == 1 {
        if cols_final == 1 {
            (ValueKind::Scalar, Some(1), Some(1))
        } else {
            (ValueKind::Vector, Some(1), Some(cols_final as u32))
        }
    } else {
        (
            ValueKind::Matrix,
            Some(row_count as u32),
            Some(cols_final as u32),
        )
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Branch
// ────────────────────────────────────────────────────────────────────────────

pub fn parse_branch_node(node: Node) -> Result<Branch> {
    let mut name = None;
    let mut zorder = None;
    let mut dst: Option<EndpointRef> = None;
    let mut labels = None;
    let mut points_list: Vec<Point> = Vec::new();
    let mut branches: Vec<Branch> = Vec::new();
    let mut properties: IndexMap<String, String> = IndexMap::new();

    for child in node.children().filter(|c| c.is_element()) {
        match child.tag_name().name() {
            "P" => {
                if let Some(nm) = child.attribute("Name") {
                    let val = child.text().unwrap_or("").to_string();
                    properties.insert(nm.to_string(), val.clone());
                    match nm {
                        "Name" => name = Some(val),
                        "ZOrder" => zorder = Some(val),
                        "Dst" => dst = crate::parser::parse_endpoint(&val).ok(),
                        "Labels" => labels = Some(val),
                        "Points" => points_list.extend(crate::parser::parse_points(&val)),
                        _ => {}
                    }
                }
            }
            "Branch" => branches.push(parse_branch_node(child)?),
            _ => {}
        }
    }

    Ok(Branch {
        name,
        zorder,
        dst,
        points: points_list,
        labels,
        branches,
        properties,
    })
}

// ────────────────────────────────────────────────────────────────────────────
// Line
// ────────────────────────────────────────────────────────────────────────────

pub fn parse_line_node(node: Node) -> Result<Line> {
    let mut name = None;
    let mut zorder = None;
    let mut src: Option<EndpointRef> = None;
    let mut dst: Option<EndpointRef> = None;
    let mut labels = None;
    let mut points_list: Vec<Point> = Vec::new();
    let mut branches: Vec<Branch> = Vec::new();
    let mut properties: IndexMap<String, String> = IndexMap::new();

    for child in node.children().filter(|c| c.is_element()) {
        match child.tag_name().name() {
            "P" => {
                if let Some(nm) = child.attribute("Name") {
                    let val = child.text().unwrap_or("").to_string();
                    properties.insert(nm.to_string(), val.clone());
                    match nm {
                        "Name" => name = Some(val),
                        "ZOrder" => zorder = Some(val),
                        "Src" => src = crate::parser::parse_endpoint(&val).ok(),
                        "Dst" => dst = crate::parser::parse_endpoint(&val).ok(),
                        "Labels" => labels = Some(val),
                        "Points" => points_list.extend(crate::parser::parse_points(&val)),
                        _ => {}
                    }
                }
            }
            "Branch" => {
                branches.push(parse_branch_node(child)?);
            }
            _ => {}
        }
    }

    Ok(Line {
        name,
        zorder,
        src,
        dst,
        points: points_list,
        labels,
        branches,
        properties,
    })
}

// ────────────────────────────────────────────────────────────────────────────
// Block (shallow parse)
// ────────────────────────────────────────────────────────────────────────────

/// Parse a `<Block>` or `<Reference>` element without cross-file recursion.
///
/// All `<P>` values are stored in the `properties` map in their original
/// insertion order so that the XML can be exactly regenerated.
pub fn parse_block_shallow(node: Node, base_dir: &Utf8Path) -> Result<Block> {
    let tag_name = node.tag_name().name().to_string();
    let mut block_type = node.attribute("BlockType").unwrap_or("").to_string();
    if block_type.is_empty() && tag_name == "Reference" {
        block_type = "Reference".to_string();
    }
    let name = node.attribute("Name").unwrap_or("").to_string();
    let sid = node.attribute("SID").map(|s| s.to_string());

    let mut properties: IndexMap<String, String> = IndexMap::new();
    let mut ref_properties = std::collections::BTreeSet::new();
    let mut ports = Vec::new();
    let mut position = None;
    let mut zorder = None;
    let mut port_counts: Option<PortCounts> = None;
    let mut subsystem: Option<Box<System>> = None;
    let mut system_ref: Option<String> = None;
    let mut commented = false;
    let mut is_matlab_function = false;
    let mut c_output_code: Option<String> = None;
    let mut c_start_code: Option<String> = None;
    let mut c_term_code: Option<String> = None;
    let mut c_codegen_output: Option<String> = None;
    let mut c_codegen_start: Option<String> = None;
    let mut c_codegen_term: Option<String> = None;
    let mut mask: Option<Mask> = None;
    let mut instance_data: Option<InstanceData> = None;
    let mut link_data: Option<LinkData> = None;
    let mut annotations: Vec<Annotation> = Vec::new();
    let mut background_color: Option<String> = None;
    let mut show_name: Option<bool> = None;
    let mut font_size: Option<u32> = None;
    let mut font_weight: Option<String> = None;
    let mut block_value: Option<String> = None;
    let mut name_location: NameLocation = NameLocation::Bottom;
    let mut current_setting: Option<String> = None;
    let mut block_mirror: Option<bool> = None;
    let mut value_kind = ValueKind::Unknown;
    let mut value_rows: Option<u32> = None;
    let mut value_cols: Option<u32> = None;
    let mut child_order: Vec<BlockChildKind> = Vec::new();

    for child in node.children().filter(|c| c.is_element()) {
        match child.tag_name().name() {
            "P" => {
                if let Some(name_attr) = child.attribute("Name") {
                    // Determine value: from Ref attribute or text content
                    let is_ref = child.attribute("Ref").is_some();
                    let value = if let Some(ref_val) = child.attribute("Ref") {
                        ref_val.to_string()
                    } else {
                        child.text().unwrap_or("").to_string()
                    };

                    // Always store in properties map (preserving insertion order)
                    properties.insert(name_attr.to_string(), value.clone());
                    if is_ref {
                        ref_properties.insert(name_attr.to_string());
                    }
                    child_order.push(BlockChildKind::P(name_attr.to_string()));

                    // Derive convenience typed fields
                    match name_attr {
                        "Position" => position = Some(value),
                        "ZOrder" => zorder = Some(value),
                        "Commented" => {
                            commented = value.eq_ignore_ascii_case("on");
                        }
                        "SFBlockType" => {
                            if value == "MATLAB Function" {
                                is_matlab_function = true;
                            }
                        }
                        "OutputCode" => c_output_code = Some(value),
                        "StartCode" => c_start_code = Some(value),
                        "TerminateCode" => c_term_code = Some(value),
                        "CodegenOutputCode" => c_codegen_output = Some(value),
                        "CodegenStartCode" => c_codegen_start = Some(value),
                        "CodegenTerminateCode" => c_codegen_term = Some(value),
                        "BackgroundColor" => {
                            background_color = crate::color::parse_color(&value);
                        }
                        "ShowName" => {
                            show_name = Some(!value.eq_ignore_ascii_case("off"));
                        }
                        "BlockMirror" => {
                            let on = value.eq_ignore_ascii_case("on")
                                || value == "1"
                                || value.eq_ignore_ascii_case("true");
                            block_mirror = Some(on);
                        }
                        "FontSize" => {
                            font_size = value.parse::<u32>().ok();
                        }
                        "FontWeight" => {
                            font_weight = Some(value);
                        }
                        "NameLocation" => {
                            name_location =
                                match value.trim().to_ascii_lowercase().as_str() {
                                    "top" => NameLocation::Top,
                                    "bottom" => NameLocation::Bottom,
                                    "left" => NameLocation::Left,
                                    "right" => NameLocation::Right,
                                    _ => NameLocation::Bottom,
                                };
                        }
                        "Value" => {
                            block_value = Some(value);
                        }
                        "CurrentSetting" => {
                            current_setting = Some(value);
                        }
                        _ => {}
                    }
                }
            }
            "PortCounts" => {
                let ins = child
                    .attribute("in")
                    .and_then(|s| s.parse::<u32>().ok());
                let outs = child
                    .attribute("out")
                    .and_then(|s| s.parse::<u32>().ok());
                port_counts = Some(PortCounts { ins, outs });
                child_order.push(BlockChildKind::PortCounts);
            }
            "PortProperties" => {
                for pnode in child
                    .children()
                    .filter(|c| c.is_element() && c.has_tag_name("Port"))
                {
                    let mut pprops = IndexMap::new();
                    let port_type = pnode.attribute("Type").unwrap_or("").to_string();
                    let index = pnode.attribute("Index").and_then(|s| s.parse::<u32>().ok());
                    for pp in pnode
                        .children()
                        .filter(|c| c.is_element() && c.has_tag_name("P"))
                    {
                        if let Some(nm) = pp.attribute("Name") {
                            pprops.insert(nm.to_string(), pp.text().unwrap_or("").to_string());
                        }
                    }
                    ports.push(Port {
                        port_type,
                        index,
                        properties: pprops,
                    });
                }
                child_order.push(BlockChildKind::PortProperties);
            }
            "LinkData" => {
                let mut dp_entries = Vec::new();
                for dp in child
                    .children()
                    .filter(|c| c.is_element() && c.has_tag_name("DialogParameters"))
                {
                    let block_name = dp.attribute("BlockName").unwrap_or("").to_string();
                    let mut dp_props = IndexMap::new();
                    for p in dp
                        .children()
                        .filter(|c| c.is_element() && c.has_tag_name("P"))
                    {
                        if let Some(nm) = p.attribute("Name") {
                            dp_props.insert(nm.to_string(), p.text().unwrap_or("").to_string());
                        }
                    }
                    dp_entries.push(DialogParametersEntry {
                        block_name,
                        properties: dp_props,
                    });
                }
                link_data = Some(LinkData {
                    dialog_parameters: dp_entries,
                });
                child_order.push(BlockChildKind::LinkData);
            }
            "System" => {
                if let Some(reference) = child.attribute("Ref") {
                    // Store just the reference name (e.g., "system_18")
                    system_ref = Some(reference.to_string());
                } else {
                    match parse_system_shallow(child, base_dir) {
                        Ok(sys) => subsystem = Some(Box::new(sys)),
                        Err(err) => eprintln!(
                            "[rustylink] Warning: failed to parse inline system: {}",
                            err
                        ),
                    }
                }
                child_order.push(BlockChildKind::System);
            }
            "Mask" => match parse_mask_node(child) {
                Ok(m) => {
                    mask = Some(m);
                    child_order.push(BlockChildKind::Mask);
                }
                Err(err) => eprintln!(
                    "[rustylink] Error parsing <Mask> in block '{}': {}",
                    name, err
                ),
            },
            "InstanceData" => match parse_instance_data_node(child) {
                Ok(id) => {
                    instance_data = Some(id);
                    child_order.push(BlockChildKind::InstanceData);
                }
                Err(err) => eprintln!(
                    "[rustylink] Warning: failed to parse <InstanceData> in block '{}': {}",
                    name, err
                ),
            },
            "Annotation" => match parse_annotation_node(child) {
                Ok(a) => {
                    let idx = annotations.len();
                    annotations.push(a);
                    child_order.push(BlockChildKind::Annotation(idx));
                }
                Err(err) => eprintln!(
                    "[rustylink] Warning: failed to parse <Annotation> in block '{}': {}",
                    name, err
                ),
            },
            _ => {}
        }
    }

    // Derive value shape from block_value (for API convenience only)
    if let Some(v) = block_value.as_ref() {
        let (kind, rows, cols) = parse_value_shape(v);
        value_kind = kind;
        value_rows = rows;
        value_cols = cols;
    }

    // Simulink omits <P Name="Value"> for Constant blocks with default value "1".
    // Set the convenience field but do NOT synthesize it in properties.
    if block_type == "Constant" && block_value.is_none() {
        block_value = Some("1".to_string());
    }

    // Note: we do NOT mutate block_type for MATLAB Function blocks.
    // The is_matlab_function flag indicates this status without changing
    // the block_type, which is needed for round-trip XML fidelity.

    let c_function = if block_type == "CFunction" {
        Some(CFunctionCode {
            output_code: c_output_code,
            start_code: c_start_code,
            terminate_code: c_term_code,
            codegen_output_code: c_codegen_output,
            codegen_start_code: c_codegen_start,
            codegen_terminate_code: c_codegen_term,
        })
    } else {
        None
    };

    let mut blk = Block {
        block_type,
        name,
        sid,
        tag_name,
        position,
        zorder,
        commented,
        name_location,
        is_matlab_function,
        properties,
        ref_properties,
        port_counts,
        ports,
        subsystem,
        system_ref,
        c_function,
        instance_data,
        link_data,
        mask,
        annotations,
        background_color,
        show_name,
        font_size,
        font_weight,
        mask_display_text: None,
        value: block_value,
        value_kind,
        value_rows,
        value_cols,
        current_setting,
        block_mirror,
        library_source: None,
        library_block_path: None,
        child_order,
    };

    if blk.mask_display_text.is_none()
        && blk.mask.as_ref().and_then(|m| m.display.as_ref()).is_some()
    {
        crate::mask_eval::evaluate_mask_display(&mut blk);
    }
    Ok(blk)
}

/// Alias for backward compatibility.
pub fn parse_block(node: Node, base_dir: &Utf8Path) -> Result<Block> {
    parse_block_shallow(node, base_dir)
}

// ────────────────────────────────────────────────────────────────────────────
// System (shallow parse)
// ────────────────────────────────────────────────────────────────────────────

/// Parse a `<System>` element without cross-file recursion.
pub fn parse_system_shallow(node: Node, base_dir: &Utf8Path) -> Result<System> {
    let mut properties = IndexMap::new();
    let mut blocks = Vec::new();
    let mut lines = Vec::new();
    let mut annotations: Vec<Annotation> = Vec::new();
    for child in node.children().filter(|c| c.is_element()) {
        match child.tag_name().name() {
            "P" => {
                if let Some(name) = child.attribute("Name") {
                    properties.insert(name.to_string(), child.text().unwrap_or("").to_string());
                }
            }
            "Block" => {
                blocks.push(parse_block_shallow(child, base_dir)?);
            }
            "Reference" => {
                blocks.push(parse_block_shallow(child, base_dir)?);
            }
            "Line" => {
                lines.push(parse_line_node(child)?);
            }
            "Annotation" => match parse_annotation_node(child) {
                Ok(a) => annotations.push(a),
                Err(err) => {
                    eprintln!("[rustylink] Warning: failed to parse <Annotation>: {}", err)
                }
            },
            _ => {}
        }
    }
    Ok(System {
        properties,
        blocks,
        lines,
        annotations,
        chart: None,
    })
}
