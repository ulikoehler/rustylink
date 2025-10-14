/// Utility for parsing Simulink background color strings (named or [r,g,b] arrays)
pub fn parse_color(val: &str) -> Option<String> {
    let val = val.trim();
    if val.starts_with('[') && val.ends_with(']') {
        // Parse [r,g,b] array, e.g. [1.0, 0.411765, 0.380392]
        let inner = &val[1..val.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        if parts.len() == 3 {
            let r = parts[0].parse::<f32>().unwrap_or(0.0);
            let g = parts[1].parse::<f32>().unwrap_or(0.0);
            let b = parts[2].parse::<f32>().unwrap_or(0.0);
            Some(format!("rgb({:.3},{:.3},{:.3})", r, g, b))
        } else {
            None
        }
    } else {
        // Accept extended named colors
        let named = match val.to_ascii_lowercase().as_str() {
            "white" => Some("#ffffff".to_string()),
            "black" => Some("#000000".to_string()),
            "red" => Some("#ff0000".to_string()),
            "green" => Some("#00ff00".to_string()),
            "blue" => Some("#0000ff".to_string()),
            "yellow" => Some("#ffff00".to_string()),
            "orange" => Some("#ffa500".to_string()),
            "cyan" => Some("#00ffff".to_string()),
            "magenta" => Some("#ff00ff".to_string()),
            "lightblue" => Some("#add8e6".to_string()),
            "darkgreen" => Some("#006400".to_string()),
            "gray" | "grey" => Some("#808080".to_string()),
            "lightgray" | "lightgrey" => Some("#d3d3d3".to_string()),
            "darkgray" | "darkgrey" => Some("#a9a9a9".to_string()),
            "brown" => Some("#a52a2a".to_string()),
            "purple" => Some("#800080".to_string()),
            "pink" => Some("#ffc0cb".to_string()),
            "lime" => Some("#00ff00".to_string()),
            "navy" => Some("#000080".to_string()),
            "teal" => Some("#008080".to_string()),
            "olive" => Some("#808000".to_string()),
            "maroon" => Some("#800000".to_string()),
            "silver" => Some("#c0c0c0".to_string()),
            _ => None,
        };
        named.or_else(|| Some(val.to_string()))
    }
}
