//! String operation blocks.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use crate::simulink_libraries::labels;
use crate::simulink_libraries::libraries::prelude::icon;
use crate::simulink_libraries::types::{
    BlockLabelPolicy, IOPorts, MetadataKey, SimulinkBlockDefinition,
};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // ═══════════════════════════════════════════════════════════════════════
    //  String Operations
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("ASCIIToString", "String Operations")
        .with_aliases(&["ASCII to String", "AsciiToString"])
        .with_description("Convert ASCII codes to string")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_block_label(BlockLabelPolicy::Fixed("ASCII \u{27F6} string")),
    SimulinkBlockDefinition::new("ToString", "String Operations")
        .with_aliases(&["To String"])
        .with_description("Convert input to string representation")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_block_label(BlockLabelPolicy::Fixed("\u{27F6} string")),
    // Simulink shows the configured literal, defaulting to `"Hello!"`.
    SimulinkBlockDefinition::new("StringConstant", "String Operations")
        .with_aliases(&["String Constant"])
        .with_description("Output a constant string value")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("String", "\"Hello!\"")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::string_constant)),
    SimulinkBlockDefinition::new("StringConcatenate", "String Operations")
        .with_aliases(&["String Concatenate"])
        .with_description("Concatenate input strings")
        .with_ports(IOPorts::Variable(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{2237}str")),
    SimulinkBlockDefinition::new("StringJoin", "String Operations")
        .with_aliases(&["String Join"])
        .with_description("Join strings with a delimiter")
        .with_ports(IOPorts::Variable(1), IOPorts::Fixed(1))
        .with_icon(icon("join")),
    SimulinkBlockDefinition::new("StringFind", "String Operations")
        .with_aliases(&["String Find"])
        .with_description("Find a substring position")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("find")),
    SimulinkBlockDefinition::new("StringReplace", "String Operations")
        .with_aliases(&["String Replace"])
        .with_description("Replace occurrences of a substring")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("repl")),
    SimulinkBlockDefinition::new("StringLength", "String Operations")
        .with_aliases(&["String Length"])
        .with_description("Return the number of characters")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("len")),
    SimulinkBlockDefinition::new("StringToDouble", "String Operations")
        .with_aliases(&["String to Double"])
        .with_description("Parse string as double")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{2192}f64")),
    SimulinkBlockDefinition::new("StringToSingle", "String Operations")
        .with_aliases(&["String to Single"])
        .with_description("Parse string as single")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{2192}f32")),
    SimulinkBlockDefinition::new("StringToEnum", "String Operations")
        .with_aliases(&["String to Enum"])
        .with_description("Convert string to enumeration value")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{2192}enum")),
    SimulinkBlockDefinition::new("EnumToString", "String Operations")
        .with_aliases(&["Enum to String"])
        .with_description("Convert enumeration to string")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("enum\u{2192}")),
    SimulinkBlockDefinition::new("StringToASCII", "String Operations")
        .with_aliases(&["String to ASCII"])
        .with_description("Convert string to ASCII codes")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("str\u{2192}asc")),
    SimulinkBlockDefinition::new("CompareStrings", "String Operations")
        .with_aliases(&["Compare Strings"])
        .with_description("Compare two strings")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(icon("s==s")),
    SimulinkBlockDefinition::new("CompareStringConstants", "String Operations")
        .with_aliases(&["Compare String to Constant"])
        .with_description("Compare string against a constant")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("s==\"x\"")),
    SimulinkBlockDefinition::new("ComposeString", "String Operations")
        .with_aliases(&["Compose String"])
        .with_description("Build a string from a format spec")
        .with_ports(IOPorts::Variable(1), IOPorts::Fixed(1))
        .with_icon(icon("fmt")),
    SimulinkBlockDefinition::new("Substring", "String Operations")
        .with_description("Extract a substring")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("sub")),
    SimulinkBlockDefinition::new("SplitString", "String Operations")
        .with_aliases(&["Split String"])
        .with_description("Split a string by a delimiter")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(2))
        .with_icon(icon("split")),
    SimulinkBlockDefinition::new("StringEndsWith", "String Operations")
        .with_aliases(&["String Ends With"])
        .with_description("Test whether string ends with a suffix")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("ends")),
    SimulinkBlockDefinition::new("ScanString", "String Operations")
        .with_aliases(&["Scan String"])
        .with_description("Parse a string per a format spec")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("scan")),
    SimulinkBlockDefinition::new("IsString", "String Operations")
        .with_description("Test whether input is a string")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("isstr")),
];
