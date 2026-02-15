//! Helper functions for parsing Simulink XML values (points, endpoints, system references).

use crate::model::*;
use anyhow::{Result, anyhow};
use camino::{Utf8Path, Utf8PathBuf};

/// Parse a Simulink points string like `"[x, y]"` or `"[x1, y1; x2, y2]"`.
pub fn parse_points(s: &str) -> Vec<Point> {
    let trimmed = s.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .unwrap_or(trimmed);
    let mut points = Vec::new();
    for pair in inner.split(';') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let mut it = pair.split(',').map(|v| v.trim()).filter(|t| !t.is_empty());
        if let (Some(x), Some(y)) = (it.next(), it.next()) {
            if let (Ok(xv), Ok(yv)) = (x.parse::<i32>(), y.parse::<i32>()) {
                points.push(Point { x: xv, y: yv });
            }
        }
    }
    points
}

/// Parse an endpoint string like `"18#out:1"` into an [`EndpointRef`].
pub fn parse_endpoint(s: &str) -> Result<EndpointRef> {
    let (sid_str, rest) = s
        .split_once('#')
        .ok_or_else(|| anyhow!("Invalid endpoint format: {}", s))?;
    let sid: String = sid_str.trim().to_string();
    let (ptype, pidx_str) = rest
        .split_once(':')
        .ok_or_else(|| anyhow!("Invalid endpoint port format: {}", s))?;
    let port_type = ptype.trim().to_string();
    let port_index: u32 = pidx_str.trim().parse()?;
    Ok(EndpointRef {
        sid,
        port_type,
        port_index,
    })
}

/// Resolve a system reference like `"system_22"` to a full XML path.
pub fn resolve_system_reference(reference: &str, base_dir: &Utf8Path) -> Utf8PathBuf {
    let mut candidate = Utf8PathBuf::from(reference);
    if !candidate.extension().is_some_and(|e| e == "xml") {
        candidate.set_extension("xml");
    }
    if candidate.is_absolute() {
        candidate
    } else {
        base_dir.join(candidate)
    }
}
