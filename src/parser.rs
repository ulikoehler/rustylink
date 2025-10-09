use crate::model::*;
use anyhow::{anyhow, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use roxmltree::{Document, Node};
use std::collections::BTreeMap;
use std::io::Read;

pub trait ContentSource {
    fn read_to_string(&mut self, path: &Utf8Path) -> Result<String>;
}

pub struct FsSource;

impl ContentSource for FsSource {
    fn read_to_string(&mut self, path: &Utf8Path) -> Result<String> {
        Ok(std::fs::read_to_string(path.as_str())
            .with_context(|| format!("Failed to read {}", path))?)
    }
}

pub struct ZipSource<R: Read + std::io::Seek> {
    zip: zip::ZipArchive<R>,
}

impl<R: Read + std::io::Seek> ZipSource<R> {
    pub fn new(reader: R) -> Result<Self> {
        let zip = zip::ZipArchive::new(reader).context("Failed to open zip archive")?;
        Ok(Self { zip })
    }
}

impl<R: Read + std::io::Seek> ContentSource for ZipSource<R> {
    fn read_to_string(&mut self, path: &Utf8Path) -> Result<String> {
        let p = path
            .as_str()
            .trim_start_matches("./")
            .trim_start_matches('/')
            .to_string();
        let mut f = self
            .zip
            .by_name(&p)
            .with_context(|| format!("File {} not found in zip", p))?;
        let mut s = String::new();
        f.read_to_string(&mut s)
            .with_context(|| format!("Failed to read {} from zip", p))?;
        Ok(s)
    }
}

pub struct SimulinkParser<S: ContentSource> {
    root_dir: Utf8PathBuf,
    source: S,
}

impl<S: ContentSource> SimulinkParser<S> {
    pub fn new(root_dir: impl AsRef<Utf8Path>, source: S) -> Self {
        Self { root_dir: root_dir.as_ref().to_path_buf(), source }
    }

    pub fn parse_system_file(&mut self, path: impl AsRef<Utf8Path>) -> Result<System> {
        let path = path.as_ref();
        let text = self.source.read_to_string(path)?;
        let doc = Document::parse(&text).with_context(|| format!("Failed to parse XML {}", path))?;
        let system_node = doc
            .descendants()
            .find(|n| n.has_tag_name("System"))
            .ok_or_else(|| anyhow!("No <System> root in {}", path))?;
        let base_dir_owned: Utf8PathBuf = path
            .parent()
            .map(|p| p.to_owned())
            .unwrap_or_else(|| self.root_dir.clone());
        self.parse_system(system_node, base_dir_owned.as_path())
    }

    fn parse_system(&mut self, node: Node, base_dir: &Utf8Path) -> Result<System> {
        let mut properties = BTreeMap::new();
        let mut blocks = Vec::new();
        let mut lines = Vec::new();

        for child in node.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "P" => {
                    if let Some(name) = child.attribute("Name") {
                        properties.insert(name.to_string(), child.text().unwrap_or("").to_string());
                    }
                }
                "Block" => {
                    blocks.push(self.parse_block(child, base_dir)?);
                }
                "Line" => {
                    lines.push(self.parse_line(child)?);
                }
                _ => {}
            }
        }

        Ok(System { properties, blocks, lines })
    }

    fn parse_block(&mut self, node: Node, base_dir: &Utf8Path) -> Result<Block> {
        let block_type = node.attribute("BlockType").unwrap_or("").to_string();
        let name = node.attribute("Name").unwrap_or("").to_string();
        let sid = node.attribute("SID").map(|s| s.to_string());
        let mut properties = BTreeMap::new();
        let mut ports = Vec::new();
        let mut position = None;
        let mut zorder = None;
        let mut subsystem: Option<Box<System>> = None;

        for child in node.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "P" => {
                    if let Some(name_attr) = child.attribute("Name") {
                        // Prefer Ref attribute if present, else text
                        let value = child
                            .attribute("Ref")
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| child.text().unwrap_or("").to_string());
                        match name_attr {
                            "Position" => position = Some(value),
                            "ZOrder" => zorder = Some(value),
                            _ => {
                                properties.insert(name_attr.to_string(), value);
                            }
                        }
                    }
                }
                "PortCounts" => {
                    // We won't store counts separately, ports are parsed from PortProperties
                    let _ = child;
                }
                "PortProperties" => {
                    for pnode in child.children().filter(|c| c.is_element() && c.has_tag_name("Port")) {
                        let mut pprops = BTreeMap::new();
                        let port_type = pnode.attribute("Type").unwrap_or("").to_string();
                        let index = pnode.attribute("Index").and_then(|s| s.parse::<u32>().ok());
                        for pp in pnode.children().filter(|c| c.is_element() && c.has_tag_name("P")) {
                            if let Some(nm) = pp.attribute("Name") {
                                pprops.insert(nm.to_string(), pp.text().unwrap_or("").to_string());
                            }
                        }
                        ports.push(Port { port_type, index, properties: pprops });
                    }
                }
                "System" => {
                    // Prefer resolving by Ref attribute when present
                    if let Some(reference) = child.attribute("Ref") {
                        let resolved = resolve_system_reference(reference, base_dir);
                        let sys = self.parse_system_file(&resolved)?;
                        subsystem = Some(Box::new(sys));
                    } else {
                        // Embedded system content
                        let sys = self.parse_system(child, base_dir)?;
                        subsystem = Some(Box::new(sys));
                    }
                }
                _ => {
                    // If there is a System Ref attribute on this element or node, handle it
                }
            }
        }

        // Already handled in the loop; no second pass needed

        Ok(Block { block_type, name, sid, position, zorder, properties, ports, subsystem })
    }

    fn parse_line(&self, node: Node) -> Result<Line> {
        let mut name = None;
        let mut zorder = None;
        let mut src = None;
        let mut dst = None;
        let mut labels = None;
        let mut points_list: Vec<String> = Vec::new();
        let mut branches: Vec<Branch> = Vec::new();

        for child in node.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "P" => {
                    if let Some(nm) = child.attribute("Name") {
                        let val = child.text().unwrap_or("").to_string();
                        match nm {
                            "Name" => name = Some(val),
                            "ZOrder" => zorder = Some(val),
                            "Src" => src = Some(val),
                            "Dst" => dst = Some(val),
                            "Labels" => labels = Some(val),
                            "Points" => {
                                points_list.push(val);
                            }
                            _ => {}
                        }
                    }
                }
                "Branch" => {
                    branches.push(self.parse_branch(child)?);
                }
                _ => {}
            }
        }

        Ok(Line { name, zorder, src, dst, points: points_list, labels, branches })
    }

    fn parse_branch(&self, node: Node) -> Result<Branch> {
        let mut name = None;
        let mut zorder = None;
        let mut dst = None;
        let mut labels = None;
        let mut points_list: Vec<String> = Vec::new();
        let mut branches: Vec<Branch> = Vec::new();

        for child in node.children().filter(|c| c.is_element()) {
            match child.tag_name().name() {
                "P" => {
                    if let Some(nm) = child.attribute("Name") {
                        let val = child.text().unwrap_or("").to_string();
                        match nm {
                            "Name" => name = Some(val),
                            "ZOrder" => zorder = Some(val),
                            "Dst" => dst = Some(val),
                            "Labels" => labels = Some(val),
                            "Points" => points_list.push(val),
                            _ => {}
                        }
                    }
                }
                "Branch" => branches.push(self.parse_branch(child)?),
                _ => {}
            }
        }

        Ok(Branch { name, zorder, dst, points: points_list, labels, branches })
    }
}

fn resolve_system_reference(reference: &str, base_dir: &Utf8Path) -> Utf8PathBuf {
    // The XML uses values like "system_22" or "system_22.xml"; files are in sibling directory or same base.
    let mut candidate = Utf8PathBuf::from(reference);
    if !candidate.extension().is_some_and(|e| e == "xml") {
        candidate.set_extension("xml");
    }
    // If not absolute, join with base_dir
    let path = if candidate.is_absolute() { candidate } else { base_dir.join(candidate) };
    path
}
