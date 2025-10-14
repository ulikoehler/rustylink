use crate::model::*;
use anyhow::Result;
use camino::Utf8Path;
use roxmltree::Node;
use std::collections::BTreeMap;

pub fn parse_annotation_node(node: Node) -> Result<Annotation> {
    let sid = node.attribute("SID").map(|s| s.to_string());
    let mut position: Option<String> = None;
    let mut zorder: Option<String> = None;
    let mut interpreter: Option<String> = None;
    let mut text: Option<String> = None;
    let mut properties: BTreeMap<String, String> = BTreeMap::new();

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

pub fn parse_mask_node(node: Node) -> Result<Mask> {
    let mut display: Option<String> = None;
    let mut description: Option<String> = None;
    let mut initialization: Option<String> = None;
    let mut help: Option<String> = None;
    let mut parameters: Vec<MaskParameter> = Vec::new();
    let mut dialog: Vec<DialogControl> = Vec::new();

    for child in node.children().filter(|c| c.is_element()) {
        match child.tag_name().name() {
            "Display" => display = child.text().map(|s| s.to_string()),
            "Description" => description = child.text().map(|s| s.to_string()),
            "Initialization" => initialization = child.text().map(|s| s.to_string()),
            "MaskParameter" => {
                parameters.push(parse_mask_parameter_node(child));
            }
            "DialogControl" => {
                dialog.push(parse_dialog_control_node(child));
            }
            "Help" => help = child.text().map(|s| s.to_string()),
            other => {
                println!("Unknown tag in Mask: {}", other);
            }
        }
    }

    Ok(Mask {
        display,
        description,
        initialization,
        help,
        parameters,
        dialog,
    })
}

pub fn parse_instance_data_node(node: Node) -> Result<InstanceData> {
    // <InstanceData> contains multiple <P Name="...">value</P>
    let mut props: BTreeMap<String, String> = BTreeMap::new();
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

pub fn parse_mask_parameter_node(node: Node) -> MaskParameter {
    let name = node.attribute("Name").unwrap_or("").to_string();
    let tattr = node.attribute("Type").unwrap_or("");
    let param_type = match tattr {
        t if t.eq_ignore_ascii_case("popup") => MaskParamType::Popup,
        t if t.eq_ignore_ascii_case("edit") => MaskParamType::Edit,
        t if t.eq_ignore_ascii_case("checkbox") => MaskParamType::Checkbox,
        other => {
            println!("Unknown MaskParameter Type: {} (Name='{}')", other, name);
            MaskParamType::Unknown(other.to_string())
        }
    };
    let tunable = node
        .attribute("Tunable")
        .map(|v| matches_ignore_case(v, "on") || v == "1");
    let visible = node
        .attribute("Visible")
        .map(|v| matches_ignore_case(v, "on") || v == "1");

    // Report unexpected attributes
    for attr in node.attributes() {
        let key = attr.name();
        if key != "Name" && key != "Type" && key != "Tunable" && key != "Visible" {
            println!(
                "Unknown attribute in MaskParameter(Name='{}'): {}='{}'",
                name,
                key,
                attr.value()
            );
        }
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
                    } else {
                        println!(
                            "Unknown tag in MaskParameter TypeOptions: {}",
                            to.tag_name().name()
                        );
                    }
                }
            }
            "Callback" => callback = child.text().map(|s| s.to_string()),
            other => {
                println!("Unknown tag in MaskParameter(Name='{}'): {}", name, other);
            }
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
    }
}

pub fn parse_dialog_control_node(node: Node) -> DialogControl {
    let tattr = node.attribute("Type").unwrap_or("");
    let control_type = match tattr {
        t if t.eq_ignore_ascii_case("Group") => DialogControlType::Group,
        t if t.eq_ignore_ascii_case("Text") => DialogControlType::Text,
        t if t.eq_ignore_ascii_case("Edit") => DialogControlType::Edit,
        t if t.eq_ignore_ascii_case("CheckBox") => DialogControlType::CheckBox,
        t if t.eq_ignore_ascii_case("Popup") => DialogControlType::Popup,
        other => {
            println!("Unknown DialogControl Type: {}", other);
            DialogControlType::Unknown(other.to_string())
        }
    };
    let name = node.attribute("Name").map(|s| s.to_string());

    // Report unexpected attributes
    for attr in node.attributes() {
        let key = attr.name();
        if key != "Type" && key != "Name" {
            println!(
                "Unknown attribute in DialogControl(Name='{}'): {}='{}'",
                name.clone().unwrap_or_default(),
                key,
                attr.value()
            );
        }
    }

    let mut prompt: Option<String> = None;
    let mut children: Vec<DialogControl> = Vec::new();

    for child in node.children().filter(|c| c.is_element()) {
        match child.tag_name().name() {
            "Prompt" => prompt = child.text().map(|s| s.to_string()),
            "DialogControl" => children.push(parse_dialog_control_node(child)),
            other => println!(
                "Unknown tag in DialogControl(Name='{}'): {}",
                name.clone().unwrap_or_default(),
                other
            ),
        }
    }

    DialogControl {
        control_type,
        name,
        prompt,
        children,
    }
}

fn matches_ignore_case(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

pub fn parse_branch_node(node: Node) -> Result<Branch> {
    let mut name = None;
    let mut zorder = None;
    let mut dst: Option<EndpointRef> = None;
    let mut labels = None;
    let mut points_list: Vec<Point> = Vec::new();
    let mut branches: Vec<Branch> = Vec::new();

    for child in node.children().filter(|c| c.is_element()) {
        match child.tag_name().name() {
            "P" => {
                if let Some(nm) = child.attribute("Name") {
                    let val = child.text().unwrap_or("").to_string();
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
            unknown => {
                println!("Unknown tag in Branch: {}", unknown);
            }
        }
    }

    Ok(Branch {
        name,
        zorder,
        dst,
        points: points_list,
        labels,
        branches,
    })
}

pub fn parse_line_node(node: Node) -> Result<Line> {
    let mut name = None;
    let mut zorder = None;
    let mut src: Option<EndpointRef> = None;
    let mut dst: Option<EndpointRef> = None;
    let mut labels = None;
    let mut points_list: Vec<Point> = Vec::new();
    let mut branches: Vec<Branch> = Vec::new();

    for child in node.children().filter(|c| c.is_element()) {
        match child.tag_name().name() {
            "P" => {
                if let Some(nm) = child.attribute("Name") {
                    let val = child.text().unwrap_or("").to_string();
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
            unknown => {
                println!("Unknown tag in Line: {}", unknown);
            }
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
    })
}

pub fn parse_block_shallow(node: Node, base_dir: &Utf8Path) -> Result<Block> {
    // Use the same logic as parse_block but without cross-file recursion; also use free helpers
    // Start with defaults
    let mut block_type = node.attribute("BlockType").unwrap_or("").to_string();
    let name = node.attribute("Name").unwrap_or("").to_string();
    let sid = node.attribute("SID").map(|s| s.to_string());
    let mut properties = BTreeMap::new();
    let mut ports = Vec::new();
    let mut position = None;
    let mut zorder = None;
    let mut subsystem: Option<Box<System>> = None;
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
    let mut annotations: Vec<Annotation> = Vec::new();
    let mut background_color: Option<String> = None;
    let mut show_name: Option<bool> = None;
    let mut font_size: Option<u32> = None;
    let mut font_weight: Option<String> = None;
    let mut block_value: Option<String> = None;

    for child in node.children().filter(|c| c.is_element()) {
        match child.tag_name().name() {
            "P" => {
                if let Some(name_attr) = child.attribute("Name") {
                    let value = child
                        .attribute("Ref")
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| child.text().unwrap_or("").to_string());
                    match name_attr {
                        "Position" => position = Some(value),
                        "ZOrder" => zorder = Some(value),
                        "Commented" => {
                            commented = value.eq_ignore_ascii_case("on");
                            properties.insert(name_attr.to_string(), value);
                        }
                        "SFBlockType" => {
                            if value == "MATLAB Function" {
                                is_matlab_function = true;
                            }
                            properties.insert(name_attr.to_string(), value);
                        }
                        // Capture CFunction code snippets
                        "OutputCode" => {
                            c_output_code = Some(value.clone());
                            properties.insert(name_attr.to_string(), value);
                        }
                        "StartCode" => {
                            c_start_code = Some(value.clone());
                            properties.insert(name_attr.to_string(), value);
                        }
                        "TerminateCode" => {
                            c_term_code = Some(value.clone());
                            properties.insert(name_attr.to_string(), value);
                        }
                        "CodegenOutputCode" => {
                            c_codegen_output = Some(value.clone());
                            properties.insert(name_attr.to_string(), value);
                        }
                        "CodegenStartCode" => {
                            c_codegen_start = Some(value.clone());
                            properties.insert(name_attr.to_string(), value);
                        }
                        "CodegenTerminateCode" => {
                            c_codegen_term = Some(value.clone());
                            properties.insert(name_attr.to_string(), value);
                        }
                        "BackgroundColor" => {
                            background_color = crate::color::parse_color(&value);
                        }
                        "ShowName" => {
                            show_name = Some(!value.eq_ignore_ascii_case("off"));
                        }
                        "FontSize" => {
                            font_size = value.parse::<u32>().ok();
                        }
                        "FontWeight" => {
                            font_weight = Some(value);
                        }
                        "Value" => {
                            // Keep raw textual value; also store into properties
                            block_value = Some(value.clone());
                            properties.insert(name_attr.to_string(), value);
                        }
                        _ => {
                            properties.insert(name_attr.to_string(), value);
                        }
                    }
                }
            }
            "PortCounts" => {
                let _ = child;
            }
            "PortProperties" => {
                for pnode in child
                    .children()
                    .filter(|c| c.is_element() && c.has_tag_name("Port"))
                {
                    let mut pprops = BTreeMap::new();
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
            }
            "System" => {
                if let Some(reference) = child.attribute("Ref") {
                    let resolved = crate::parser::resolve_system_reference(reference, base_dir);
                    properties.insert("__SystemRef".to_string(), resolved.as_str().to_string());
                } else {
                    // Inline nested system: parse shallow
                    match parse_system_shallow(child, base_dir) {
                        Ok(sys) => subsystem = Some(Box::new(sys)),
                        Err(err) => eprintln!(
                            "[rustylink] Warning: failed to parse inline system: {}",
                            err
                        ),
                    }
                }
            }
            "Mask" => match parse_mask_node(child) {
                Ok(m) => mask = Some(m),
                Err(err) => eprintln!(
                    "[rustylink] Error parsing <Mask> in block '{}': {}",
                    name, err
                ),
            },
            "InstanceData" => match parse_instance_data_node(child) {
                Ok(id) => instance_data = Some(id),
                Err(err) => eprintln!(
                    "[rustylink] Warning: failed to parse <InstanceData> in block '{}': {}",
                    name, err
                ),
            },
            "Annotation" => match parse_annotation_node(child) {
                Ok(a) => annotations.push(a),
                Err(err) => eprintln!(
                    "[rustylink] Warning: failed to parse <Annotation> in block '{}': {}",
                    name, err
                ),
            },
            unknown => {
                println!("Unknown tag in Block: {}", unknown);
            }
        }
    }

    if block_type == "SubSystem" && is_matlab_function {
        block_type = "MATLAB Function".to_string();
    }
    let c_function = if block_type == "CFunction" {
        Some(crate::model::CFunctionCode {
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
        position,
        zorder,
        commented,
        is_matlab_function,
        properties,
        ports,
        subsystem,
        c_function,
        instance_data,
        mask,
        annotations,
        background_color,
        show_name,
        font_size,
        font_weight,
        mask_display_text: None,
        value: block_value,
    };
    if blk.mask_display_text.is_none()
        && blk.mask.as_ref().and_then(|m| m.display.as_ref()).is_some()
    {
        crate::mask_eval::evaluate_mask_display(&mut blk);
    }
    Ok(blk)
}

pub fn parse_block(node: Node, base_dir: &Utf8Path) -> Result<Block> {
    // The original method belonged to SimulinkParser but didn't use `self` state.
    // Reuse the same logic as the original implementation by delegating to the shallow parser
    // and then performing any linking externally if needed.
    // For now keep identical behavior to the old method (shallow parse semantics + mask eval).
    // Note: deeper linking of referenced systems is handled by the caller (SimulinkParser).
    parse_block_shallow(node, base_dir)
}

pub fn parse_system_shallow(node: Node, base_dir: &Utf8Path) -> Result<System> {
    let mut properties = BTreeMap::new();
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
            "Line" => {
                lines.push(parse_line_node(child)?);
            }
            "Annotation" => match parse_annotation_node(child) {
                Ok(a) => annotations.push(a),
                Err(err) => eprintln!("[rustylink] Warning: failed to parse <Annotation>: {}", err),
            },
            unknown => {
                println!("Unknown tag in System: {}", unknown);
            }
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
