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
        {
            if let Some(scr) = eml
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
                                                is_signed =
                                                    val.parse::<i32>().ok().map(|v| v != 0)
                                            }
                                            "wordLength" => {
                                                word_length = val.parse::<u32>().ok()
                                            }
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
                                if pp.has_tag_name("P") {
                                    if let Some(nm) = pp.attribute("Name") {
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
