use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDoc {
    pub system: System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct System {
    pub properties: BTreeMap<String, String>,
    pub blocks: Vec<Block>,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    #[serde(rename = "type")]
    pub block_type: String,
    pub name: String,
    pub sid: Option<u32>,
    pub position: Option<String>,
    pub zorder: Option<String>,
    pub commented: bool,
    pub properties: BTreeMap<String, String>,
    pub ports: Vec<Port>,
    pub subsystem: Option<Box<System>>, // resolved nested system if present
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortCounts {
    pub ins: Option<u32>,
    pub outs: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub port_type: String, // in/out
    pub index: Option<u32>,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub name: Option<String>,
    pub zorder: Option<String>,
    pub src: Option<EndpointRef>,
    pub dst: Option<EndpointRef>,
    pub points: Vec<Point>,
    pub labels: Option<String>,
    pub branches: Vec<Branch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: Option<String>,
    pub zorder: Option<String>,
    pub dst: Option<EndpointRef>,
    pub points: Vec<Point>,
    pub labels: Option<String>,
    pub branches: Vec<Branch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRef {
    pub sid: u32,
    pub port_type: String, // "in" | "out"
    pub port_index: u32,
}

impl System {
    /// Walk all blocks in this system recursively, calling `cb` for every block.
    ///
    /// The callback receives the path of subsystem names from the root to the
    /// containing subsystem (not including the block name) and a reference to
    /// the block itself. The path is returned as a slice of Strings where each
    /// element is the name of the subsystem block that introduced that level.
    pub fn walk_blocks<F>(&self, path: &mut Vec<String>, cb: &mut F)
    where
        F: FnMut(&[String], &Block),
    {
        for blk in &self.blocks {
            cb(&path, blk);
            if let Some(sub) = &blk.subsystem {
                // descend into subsystem: push the block name as part of path
                path.push(blk.name.clone());
                sub.walk_blocks(path, cb);
                path.pop();
            }
        }
    }

    /// Find all blocks that have `block_type` (case sensitive) and return a
    /// vector of (path, Block) pairs where `path` is the vector of subsystem
    /// names from root down to the containing subsystem.
    pub fn find_blocks_by_type(&self, block_type: &str) -> Vec<(Vec<String>, Block)> {
        let mut result = Vec::new();
        let mut path = Vec::new();
        self.walk_blocks(&mut path, &mut |p, b| {
            if b.block_type == block_type {
                result.push((p.to_vec(), b.clone()));
            }
        });
        result
    }
}
