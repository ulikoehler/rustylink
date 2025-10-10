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
    pub sid: Option<String>,
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
