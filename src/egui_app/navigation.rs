#![cfg(feature = "egui")]

use crate::model::System;

/// Resolve a subsystem by an absolute path string, e.g. "/Top/Sub".
/// Returns `Some(&System)` when the path resolves within `root`, otherwise `None`.
pub fn resolve_subsystem_by_path<'a>(root: &'a System, path: &str) -> Option<&'a System> {
    let mut cur: &System = root;
    let p = path.trim();
    let mut parts = p
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty());
    for name in parts.by_ref() {
        let mut found = None;
        for b in &cur.blocks {
            if (b.block_type == "SubSystem" || b.block_type == "Reference") && b.name == name {
                if let Some(sub) = &b.subsystem {
                    found = Some(sub.as_ref());
                    break;
                }
            }
        }
        cur = found?;
    }
    Some(cur)
}

/// Resolve a subsystem by a vector of names relative to the `root` system.
pub fn resolve_subsystem_by_vec<'a>(root: &'a System, path: &[String]) -> Option<&'a System> {
    let mut cur: &System = root;
    for name in path {
        let mut found = None;
        for b in &cur.blocks {
            if (b.block_type == "SubSystem" || b.block_type == "Reference") && &b.name == name {
                if let Some(sub) = &b.subsystem {
                    found = Some(sub.as_ref());
                    break;
                }
            }
        }
        cur = found?;
    }
    Some(cur)
}

/// Collect all non-chart subsystem paths for search/autocomplete.
/// Returns a vector of paths, each path is represented as `Vec<String>` of names from root.
pub fn collect_subsystems_paths(root: &System) -> Vec<Vec<String>> {
    fn rec(cur: &System, path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
        for b in &cur.blocks {
            if b.block_type == "SubSystem" || b.block_type == "Reference" {
                if let Some(sub) = &b.subsystem {
                    if sub.chart.is_none() {
                        path.push(b.name.clone());
                        out.push(path.clone());
                        rec(sub, path, out);
                        path.pop();
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut p = Vec::new();
    rec(root, &mut p, &mut out);
    out
}

// tests moved to tests/ module
