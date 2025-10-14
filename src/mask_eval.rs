//! Minimal mask evaluation (feature-gated) for simple Display scripts like `disp(mytab{control})`.
//! We intentionally implement a tiny parser instead of embedding a full MATLAB engine.
//! Supported subset:
//!  - Initialization lines of the form: <var>={ 'A','B','C' }; or using double quotes or &apos; entities already decoded.
//!  - Popup MaskParameter with Name="control" (any name) and TypeOptions <Option>1. Position Control</Option> (ignored) and Value like "1. Position Control".
//!  - Display string of the form `disp(var{param})` where var and param are identifiers.
//! We map popup parameter to an index (1-based). We then select the corresponding element from var cell array and return it.
//! If anything fails, we return None.
use crate::model::{Block, MaskParameter, MaskParamType};

pub fn evaluate_mask_display(block: &mut Block) {
    let Some(mask) = block.mask.as_ref() else { return; };
    let Some(display) = mask.display.as_ref() else { return; };
    let Some(init) = mask.initialization.as_ref() else { return; };
    // Parse initialization for a single cell array assignment
    // e.g., mytab={'Position','Zero Torque','OFF','J-Impedance', 'WBC torque'};
    let table = parse_cell_array_assignment(init);
    if table.is_empty() { return; }
    // Find popup parameter(s)
    let mut param_map = std::collections::HashMap::new();
    for p in &mask.parameters {
        if matches!(p.param_type, MaskParamType::Popup) {
            // Value like "1. Position Control" -> take prefix number as index, else try find in options
            if let Some(val) = p.value.as_ref() {
                if let Some(idx) = parse_leading_index(val) { param_map.insert(p.name.clone(), idx); }
            }
        }
    }
    // Parse display pattern disp(var{param})
    if let Some((var, param)) = parse_disp_pattern(display) {
        if let Some(idx) = param_map.get(param) {
            if let Some(selected) = table.get(idx.saturating_sub(1)) {
                // Use cleaned selection (stop at first space or punctuation?) We take first token before space if contains a dot prefix like "1. Position Control" not here though.
                block.mask_display_text = Some(selected.clone());
            }
        } else if let Ok(i) = param.parse::<usize>() { // allow numeric constant
            if let Some(selected) = table.get(i.saturating_sub(1)) { block.mask_display_text = Some(selected.clone()); }
        }
    }
}

fn parse_cell_array_assignment(init: &str) -> Vec<String> {
    // Find pattern <ident>={...}; take first {...}
    if let Some(open) = init.find('{') {
        if let Some(close) = init[open+1..].find('}') { // relative index
            let inner = &init[open+1..open+1+close];
            // Split on commas not within quotes (we keep it simple assuming no escaped quotes)
            let mut out = Vec::new();
            for part in inner.split(',') {
                let s = part.trim().trim_matches('\'').trim_matches('"');
                if !s.is_empty() { out.push(s.to_string()); }
            }
            return out;
        }
    }
    Vec::new()
}

fn parse_leading_index(s: &str) -> Option<usize> {
    let mut digits = String::new();
    for c in s.chars() { if c.is_ascii_digit() { digits.push(c); } else { break; } }
    if digits.is_empty() { None } else { digits.parse().ok() }
}

fn parse_disp_pattern(s: &str) -> Option<(&str, &str)> {
    // Expect disp(name{param}) possibly with spaces
    let s = s.trim();
    if !s.starts_with("disp(") || !s.ends_with(')') { return None; }
    let inner = &s[5..s.len()-1];
    // inner like mytab{control}
    let parts: Vec<&str> = inner.split('{').collect();
    if parts.len() != 2 { return None; }
    let var = parts[0].trim();
    let rest = parts[1];
    if let Some(end) = rest.find('}') { let param = rest[..end].trim(); return Some((var, param)); }
    None
}

