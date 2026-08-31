//! Stateflow chart XML parsing.

use crate::model::*;
use anyhow::{Context, Result, anyhow};
use roxmltree::Document;
use std::collections::BTreeMap;

/// Parse a Stateflow chart from its XML text.
pub fn parse_chart_from_text(text: &str, path_hint: Option<&str>) -> Result<Chart> {
    let doc = Document::parse(text)
        .with_context(|| format!("Failed to parse XML {}", path_hint.unwrap_or("<chart>")))?;
    let chart_node = doc
        .descendants()
        .find(|n| n.is_element() && n.has_tag_name("chart"))
        .ok_or_else(|| anyhow!("No <chart> root in {}", path_hint.unwrap_or("<chart>")))?;

    let mut properties = BTreeMap::new();
    for p in chart_node
        .children()
        .filter(|c| c.is_element() && c.has_tag_name("P"))
    {
        if let Some(nm) = p.attribute("Name") {
            properties.insert(nm.to_string(), p.text().unwrap_or("").to_string());
        }
    }

    let id = chart_node
        .attribute("id")
        .and_then(|s| s.parse::<u32>().ok());
    let name = properties.get("name").cloned();

    let eml_name = chart_node
        .children()
        .find(|c| c.is_element() && c.has_tag_name("eml"))
        .and_then(|eml| {
            eml.children().find(|c| {
                c.is_element() && c.has_tag_name("P") && c.attribute("Name") == Some("name")
            })
        })
        .and_then(|p| p.text())
        .map(|s| s.to_string());

    let mut script: Option<String> = None;
    for st in chart_node
        .descendants()
        .filter(|c| c.is_element() && c.has_tag_name("state"))
    {
        if let Some(eml) = st
            .children()
            .find(|c| c.is_element() && c.has_tag_name("eml"))
            && let Some(scr) = eml
                .children()
                .find(|c| {
                    c.is_element() && c.has_tag_name("P") && c.attribute("Name") == Some("script")
                })
                .and_then(|p| p.text())
        {
            script = Some(scr.to_string());
            break;
        }
    }

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    for data in chart_node
        .descendants()
        .filter(|c| c.is_element() && c.has_tag_name("data"))
    {
        let port_name = data.attribute("name").unwrap_or("").to_string();
        if port_name.is_empty() {
            continue;
        }
        let mut scope: Option<String> = None;
        let mut size: Option<String> = None;
        let mut method: Option<String> = None;
        let mut primitive: Option<String> = None;
        let mut is_signed: Option<bool> = None;
        let mut word_length: Option<u32> = None;
        let mut complexity: Option<String> = None;
        let mut frame: Option<String> = None;
        let mut unit: Option<String> = None;
        let mut data_type: Option<String> = None;

        for child in data.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "P" => {
                    if let Some(nm) = child.attribute("Name") {
                        let val = child.text().unwrap_or("").to_string();
                        match nm {
                            "scope" => scope = Some(val),
                            "dataType" => data_type = Some(val),
                            _ => {}
                        }
                    }
                }
                "props" => {
                    for pp in child.children().filter(|c| c.is_element()) {
                        match pp.tag_name().name() {
                            "array" => {
                                if let Some(szp) = pp.children().find(|c| {
                                    c.is_element()
                                        && c.has_tag_name("P")
                                        && c.attribute("Name") == Some("size")
                                }) {
                                    size = szp.text().map(|s| s.to_string());
                                }
                            }
                            "type" => {
                                for tprop in pp
                                    .children()
                                    .filter(|c| c.is_element() && c.has_tag_name("P"))
                                {
                                    if let Some(nm) = tprop.attribute("Name") {
                                        let val = tprop.text().unwrap_or("").to_string();
                                        match nm {
                                            "method" => method = Some(val),
                                            "primitive" => primitive = Some(val),
                                            "isSigned" => {
                                                is_signed = val.parse::<i32>().ok().map(|v| v != 0)
                                            }
                                            "wordLength" => word_length = val.parse::<u32>().ok(),
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            "unit" => {
                                if let Some(up) = pp.children().find(|c| {
                                    c.is_element()
                                        && c.has_tag_name("P")
                                        && c.attribute("Name") == Some("name")
                                }) {
                                    unit = up.text().map(|s| s.to_string());
                                }
                            }
                            _ => {
                                if pp.has_tag_name("P")
                                    && let Some(nm) = pp.attribute("Name")
                                {
                                    let val = pp.text().unwrap_or("").to_string();
                                    match nm {
                                        "complexity" => complexity = Some(val),
                                        "frame" => frame = Some(val),
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let port = ChartPort {
            name: port_name,
            size,
            method,
            primitive,
            is_signed,
            word_length,
            complexity,
            frame,
            data_type,
            unit,
        };
        match scope.as_deref() {
            Some("INPUT_DATA") => inputs.push(port),
            Some("OUTPUT_DATA") => outputs.push(port),
            _ => {}
        }
    }

    Ok(Chart {
        id,
        name,
        eml_name,
        script,
        inputs,
        outputs,
        properties,
    })
}

/// Record the MATLAB function each MATLAB Function block runs on the block
/// itself.
///
/// The name (`fcn`, `test`, …) lives in the block's Stateflow chart, which the
/// renderers cannot reach; copying it into the block's properties lets the
/// catalog caption the block with it the way Simulink does.  Charts are keyed
/// by SID and by block name, and the `function y = fcn(u)` header of the
/// script is used when the chart carries no `eml` name.
pub fn annotate_matlab_function_names(
    system: &mut System,
    charts: &BTreeMap<u32, Chart>,
    chart_map: &BTreeMap<String, u32>,
) {
    // A chart's own `name` is the path of the block that owns it, which is a
    // bare block name at the model root but a `Sub/Inner/MATLAB Function` path
    // deeper down.  Index the charts by their last path segment as well, so a
    // nested block is still matched; an ambiguous segment (the same block name
    // in two subsystems) is dropped rather than guessed.
    let mut by_leaf: BTreeMap<String, Option<u32>> = BTreeMap::new();
    for (name, id) in chart_map {
        let leaf = chart_leaf_name(name);
        by_leaf
            .entry(leaf)
            .and_modify(|slot| {
                if *slot != Some(*id) {
                    *slot = None;
                }
            })
            .or_insert(Some(*id));
    }
    annotate_matlab_function_names_in(system, charts, chart_map, &by_leaf, "");
}

/// The block name a chart path refers to: `Sub/Inner/MATLAB Function` → the
/// last segment, with the SLX line breaks in block names folded to spaces.
fn chart_leaf_name(path: &str) -> String {
    let leaf = path.rsplit('/').next().unwrap_or(path);
    leaf.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn annotate_matlab_function_names_in(
    system: &mut System,
    charts: &BTreeMap<u32, Chart>,
    chart_map: &BTreeMap<String, u32>,
    by_leaf: &BTreeMap<String, Option<u32>>,
    path: &str,
) {
    for block in &mut system.blocks {
        let block_path = if path.is_empty() {
            block.name.clone()
        } else {
            format!("{path}/{}", block.name)
        };
        if block.is_matlab_function || block.block_type == "MATLAB Function" {
            let chart = block
                .sid
                .as_ref()
                .and_then(|sid| chart_map.get(sid))
                .or_else(|| chart_map.get(&block_path))
                .or_else(|| chart_map.get(&block.name))
                .copied()
                .or_else(|| {
                    by_leaf
                        .get(&chart_leaf_name(&block.name))
                        .copied()
                        .flatten()
                })
                .and_then(|id| charts.get(&id));
            let name = chart
                .and_then(|chart| {
                    // Prefer the script's function declaration (authoritative)
                    // over `eml_name`, which may carry the chart name instead.
                    chart
                        .script
                        .as_deref()
                        .and_then(script_function_name)
                        .or_else(|| chart.eml_name.clone())
                })
                .filter(|name| !name.trim().is_empty());
            if let Some(name) = name {
                block.properties.insert(
                    crate::simulink_libraries::stubs::MATLAB_FUNCTION_NAME_PROPERTY.to_string(),
                    name,
                );
            }
        }
        if let Some(subsystem) = block.subsystem.as_deref_mut() {
            annotate_matlab_function_names_in(subsystem, charts, chart_map, by_leaf, &block_path);
        }
    }
}

/// The function name of a MATLAB script: `function [x,y] = test(u,v)` → `test`.
///
/// Handles MATLAB line continuation (`...`): when the `function` declaration
/// spans multiple lines, the continuation lines are joined before the name is
/// extracted, so a header like
/// ```text
/// function [out] = ...
///     myFunc(u)
/// ```
/// yields `myFunc`.
fn script_function_name(script: &str) -> Option<String> {
    let mut lines = script.lines().map(strip_comment).map(str::trim);
    // Find the first line of the function declaration.  The declaration can sit
    // anywhere in the script – behind a comment header, a blank line, or a
    // `%%` cell marker – so every line is considered, and `function` must be a
    // keyword of its own rather than the start of an identifier.
    let first = lines.find(|line| starts_with_function_keyword(line))?;
    // Join continuation lines (lines ending with `...`) into a single header.
    let mut header = first.to_string();
    while header.trim_end().ends_with("...") {
        // Drop the trailing `...` (and any whitespace before it).
        let cut = header.trim_end().len() - 3;
        header.truncate(cut);
        if let Some(next) = lines.next() {
            header.push(' ');
            header.push_str(next);
        } else {
            break;
        }
    }
    let header = header.trim();
    let after_keyword = header.get("function".len()..).unwrap_or("");
    // `function [a,b] = name(args)` – the name follows the last `=` in front of
    // the argument list; `function name(args)` has no `=` at all.
    let arg_list = after_keyword.find('(').unwrap_or(after_keyword.len());
    let after_outputs = match after_keyword[..arg_list].rfind('=') {
        Some(eq) => &after_keyword[eq + 1..],
        None => after_keyword,
    };
    let name = after_outputs
        .split(['(', ';', ',', ' ', '\t'])
        .find(|part| !part.is_empty())?;
    (!name.is_empty()).then(|| name.to_string())
}

/// Whether the line opens with the MATLAB `function` keyword (and not with an
/// identifier that merely starts with those letters, such as `functions`).
fn starts_with_function_keyword(line: &str) -> bool {
    match line.strip_prefix("function") {
        Some(rest) => rest
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_'),
        None => false,
    }
}

/// Drop a trailing `% …` comment from a MATLAB source line.
fn strip_comment(line: &str) -> &str {
    match line.find('%') {
        Some(at) => &line[..at],
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::script_function_name;

    #[test]
    fn single_line_function_header() {
        assert_eq!(
            script_function_name("function y = fcn(u)\ny = u;"),
            Some("fcn".to_string())
        );
    }

    #[test]
    fn multi_output_function_header() {
        assert_eq!(
            script_function_name("function [x,y] = test(u,v)\ny = u;\nx=v;"),
            Some("test".to_string())
        );
    }

    #[test]
    fn function_header_with_continuation_line() {
        let script = "function [out] = ...\n    myFunc(u)\ny = u;";
        assert_eq!(script_function_name(script), Some("myFunc".to_string()));
    }

    #[test]
    fn function_header_with_multiple_continuation_lines() {
        let script = "function ...\n  result ...\n  = ...\n  compute(x)\ny = x;";
        assert_eq!(script_function_name(script), Some("compute".to_string()));
    }

    #[test]
    fn function_header_no_outputs_with_continuation() {
        let script = "function ...\n  doit(u)\ny = u;";
        assert_eq!(script_function_name(script), Some("doit".to_string()));
    }

    #[test]
    fn function_header_after_comment_lines() {
        let script = "% Copyright\n%% cell marker\n\nfunction y = later(u)\ny = u;";
        assert_eq!(script_function_name(script), Some("later".to_string()));
    }

    #[test]
    fn commented_out_header_is_ignored() {
        let script = "% function y = wrong(u)\nfunction y = right(u)\ny = u;";
        assert_eq!(script_function_name(script), Some("right".to_string()));
    }

    #[test]
    fn header_without_outputs() {
        assert_eq!(
            script_function_name("function noOut(u)\ndisp(u);"),
            Some("noOut".to_string())
        );
    }

    #[test]
    fn identifier_starting_with_function_is_not_a_header() {
        assert_eq!(script_function_name("functions = 3;\ny = functions;"), None);
    }

    #[test]
    fn no_function_header_returns_none() {
        assert_eq!(script_function_name("y = u;"), None);
    }

    #[test]
    fn function_header_with_continuation_in_arguments() {
        let script = concat!(
            "function [q_des, dq_des, ddq_des, running_1__ready_0, q_ref_out] = ...\n",
            "    JojoJointInterpolator(dq_ref, ddq_ref, eigenvalues, dt, ...\n",
            "    consider_position_constraints, consider_velocity_constraints, ...\n",
            "    q_lowerlimit, q_upperlimit, dq_lowerlimit, dq_upperlimit, follow_q_meas, ...\n",
            "    q_target, update_q_target, shortcut_update, reset, dq_measured, q_measured)\n",
            "y = u;"
        );
        assert_eq!(
            script_function_name(script),
            Some("JojoJointInterpolator".to_string())
        );
    }

    #[test]
    fn function_header_with_long_name_on_continuation_line() {
        let script = concat!(
            "function [follow_q_meas, control_mode] = ...\n",
            "    Control_Mode_Preprocessor(desired_control_mode, collision_1OK_0_danger, new_collision_trigger, internal_simulation_enabled, is_real_experiment, Robot_DEF) ...\n",
            "%% body\n",
            "follow_q_meas = 0;"
        );
        assert_eq!(
            script_function_name(script),
            Some("Control_Mode_Preprocessor".to_string())
        );
    }
}
