//! GraphicalInterface JSON types and parsing.

use serde::{Deserialize, Serialize};

/// Type of external file reference in `graphicalInterface.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalFileReferenceType {
    LibraryBlock,
    Other(String),
}

impl<'de> Deserialize<'de> for ExternalFileReferenceType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "LIBRARY_BLOCK" => Ok(ExternalFileReferenceType::LibraryBlock),
            other => Ok(ExternalFileReferenceType::Other(other.to_string())),
        }
    }
}

impl Serialize for ExternalFileReferenceType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ExternalFileReferenceType::LibraryBlock => serializer.serialize_str("LIBRARY_BLOCK"),
            ExternalFileReferenceType::Other(s) => serializer.serialize_str(s),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ExternalFileReference {
    #[serde(rename = "Path")]
    pub path: String,
    #[serde(rename = "Reference")]
    pub reference: String,
    #[serde(rename = "SID")]
    pub sid: String,
    #[serde(rename = "Type")]
    pub r#type: ExternalFileReferenceType,
}

/// Solver name from `graphicalInterface.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolverName {
    FixedStepDiscrete,
    Other(String),
}

impl<'de> Deserialize<'de> for SolverName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "FixedStepDiscrete" => Ok(SolverName::FixedStepDiscrete),
            other => Ok(SolverName::Other(other.to_string())),
        }
    }
}

impl Serialize for SolverName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            SolverName::FixedStepDiscrete => serializer.serialize_str("FixedStepDiscrete"),
            SolverName::Other(s) => serializer.serialize_str(s),
        }
    }
}

/// Parsed `graphicalInterface.json` structure.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct GraphicalInterface {
    #[serde(rename = "ExternalFileReferences")]
    pub external_file_references: Vec<ExternalFileReference>,
    #[serde(rename = "PreCompExecutionDomainType")]
    pub precomp_execution_domain_type: Option<String>,
    #[serde(rename = "SimulinkSubDomainType")]
    pub simulink_sub_domain_type: Option<String>,
    #[serde(rename = "SolverName")]
    pub solver_name: Option<SolverName>,
}

impl GraphicalInterface {
    /// Return unique library names referenced by `ExternalFileReferences`.
    pub fn library_names(&self) -> Vec<String> {
        use std::collections::HashSet;
        let mut out: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for r in &self.external_file_references {
            if r.r#type != ExternalFileReferenceType::LibraryBlock {
                continue;
            }
            let lib = r
                .reference
                .split_once('/')
                .map(|(a, _)| a.trim().to_string())
                .unwrap_or_else(|| r.reference.trim().to_string());
            if lib.is_empty() {
                continue;
            }
            if seen.insert(lib.clone()) {
                out.push(lib);
            }
        }
        out
    }
}
