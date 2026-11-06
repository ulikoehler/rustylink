//! User-defined function blocks: MATLAB Function, Fcn, S-Function, C Caller.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::{icon, port_labels_from_model};
use crate::simulink_libraries::labels;
use crate::simulink_libraries::renderers;
use crate::simulink_libraries::types::{
    BlockLabelPolicy, IOPorts, MetadataKey, PortLabelPolicy, SimulinkBlockDefinition,
};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // Simulink captions the block `fcn` (under the MATLAB membrane logo).
    SimulinkBlockDefinition::new("MATLAB Function", "User-Defined Functions")
        .with_description("Author block behaviour in MATLAB")
        .with_ports(IOPorts::Variable(1), IOPorts::Variable(1))
        .with_metadata_keys(&[MetadataKey::new(labels::MATLAB_FUNCTION_NAME_PROPERTY)])
        .with_block_label(BlockLabelPolicy::MetadataDependent(
            labels::matlab_function_name,
        ))
        .with_static_renderer(renderers::static_matlab_function)
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
            PortLabelPolicy::MetadataDependent(port_labels_from_model),
        ),
    // ═══════════════════════════════════════════════════════════════════════
    //  User-Defined Functions
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("Fcn", "User-Defined Functions")
        .with_description("Apply user-specified expression: y = f(u)")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("f(u)")),
    SimulinkBlockDefinition::new("CFunction", "User-Defined Functions")
        .with_aliases(&["C Function", "C Caller"])
        .with_description("Call external C++ code")
        .with_ports(IOPorts::Variable(1), IOPorts::Variable(1))
        .with_static_renderer(renderers::static_c_function),
    SimulinkBlockDefinition::new("MATLABFunction", "User-Defined Functions").with_aliases(&["MATLABFcn"])
        .with_aliases(&["MATLABFcn", "MATLABFcn"])
        .with_description("Embedded MATLAB function")
        .with_ports(IOPorts::Variable(1), IOPorts::Variable(1))
        .with_icon(icon("fcn"))
        .with_port_labels(
            PortLabelPolicy::Fixed(&["u"]),
            PortLabelPolicy::Fixed(&["y"]),
        ),
    // Simulink labels the four function-call subsystems with the event they
    // serve, next to a power/reset pictogram.
    SimulinkBlockDefinition::new("InitializeFunction", "User-Defined Functions")
        .with_aliases(&["Initialize Function"])
        .with_description("Subsystem executed on model initialize events")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_block_label(BlockLabelPolicy::Fixed("\u{23FB} initialize")),
    SimulinkBlockDefinition::new("ResetFunction", "User-Defined Functions")
        .with_aliases(&["Reset Function"])
        .with_description("Subsystem executed on model reset events")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_block_label(BlockLabelPolicy::Fixed("\u{21BB} reset")),
    SimulinkBlockDefinition::new("ReinitializeFunction", "User-Defined Functions")
        .with_aliases(&["Reinitialize Function"])
        .with_description("Subsystem executed on model reinitialize events")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_block_label(BlockLabelPolicy::Fixed("\u{23FB} reinit")),
    SimulinkBlockDefinition::new("TerminateFunction", "User-Defined Functions")
        .with_aliases(&["Terminate Function"])
        .with_description("Subsystem executed on model terminate events")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_block_label(BlockLabelPolicy::Fixed("\u{24D8} terminate")),
    SimulinkBlockDefinition::new("S-Function", "User-Defined Functions")
        .with_aliases(&["S-Function Builder", "Level-2 MATLAB S-Function"])
        .with_description("S-Function (system function) block")
        .with_ports(IOPorts::Variable(1), IOPorts::Variable(1))
        .with_icon(icon("S-fn")),
    SimulinkBlockDefinition::new("C++Function", "User-Defined Functions")
        .with_aliases(&["C++ Function"])
        .with_description("Custom C++ code block")
        .with_ports(IOPorts::Variable(1), IOPorts::Variable(1))
        .with_icon(icon("C++")),
    SimulinkBlockDefinition::new("InterpretedMATLABFunction", "User-Defined Functions")
        .with_aliases(&["Interpreted MATLAB Function"])
        .with_description("Interpreted MATLAB expression")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("f(x)")),
    SimulinkBlockDefinition::new("Level2MATLABSFunction", "User-Defined Functions")
        .with_aliases(&["Level-2 MATLAB S-Function"])
        .with_description("Level-2 MATLAB S-Function")
        .with_ports(IOPorts::Variable(1), IOPorts::Variable(1))
        .with_icon(icon("M-Sfn")),
    SimulinkBlockDefinition::new("Python", "User-Defined Functions")
        .with_aliases(&["Python Module"])
        .with_description("Python function block")
        .with_ports(IOPorts::Variable(1), IOPorts::Variable(1))
        .with_icon(icon("py")),
    SimulinkBlockDefinition::new("S-FunctionBuilder", "User-Defined Functions")
        .with_aliases(&["S-Function Builder"])
        .with_description("Build custom S-Function")
        .with_ports(IOPorts::Variable(1), IOPorts::Variable(1))
        .with_icon(icon("S-bld")),
    SimulinkBlockDefinition::new("SimulinkFunction", "User-Defined Functions")
        .with_description("Simulink function definition")
        .with_ports(IOPorts::Variable(1), IOPorts::Variable(1))
        .with_icon(icon("fn")),
];
