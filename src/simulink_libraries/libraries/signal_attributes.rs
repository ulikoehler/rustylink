//! Signal attribute blocks: data type, width and rate conversions.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::{icon, plot};
use crate::simulink_libraries::renderers;
use crate::simulink_libraries::types::{IOPorts, MetadataKey, SimulinkBlockDefinition};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // ═══════════════════════════════════════════════════════════════════════
    //  Signal Attributes
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("DataTypeConversion", "Signal Attributes")
        .with_aliases(&["Data Type Conversion"])
        .with_description("Convert signal to specified data type")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("OutDataTypeStr", "Inherit: Inherit via back propagation")])
        .with_static_renderer(renderers::static_data_type_conversion),
    // Simulink draws the propagated width above the signal line running from
    // the input to the output port, tapped by a short diagonal stroke.
    SimulinkBlockDefinition::new("Width", "Signal Attributes")
        .with_description("Output width (number of elements) of input signal")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(
            "t 0.45,0.28,0.36 -1; p 0.00,0.50 1.00,0.50; p 0.32,0.78 0.58,0.36",
        )),
    SimulinkBlockDefinition::new("BusToVector", "Signal Attributes")
        .with_aliases(&["Bus to Vector"])
        .with_description("Convert bus to a vector signal")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "p 0.05,0.30 0.40,0.30; p 0.05,0.50 0.40,0.50; p 0.05,0.70 0.40,0.70;",
            "f 0.40,0.10 0.56,0.90;",
            "p 0.56,0.50 0.95,0.50"
        ))),
    SimulinkBlockDefinition::new("DataTypeConversionInherited", "Signal Attributes")
        .with_aliases(&["Data Type Conversion Inherited"])
        .with_description("Convert to inherited data type")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("conv")),
    SimulinkBlockDefinition::new("DataTypeDuplicate", "Signal Attributes")
        .with_description("Enforce same data type on all inputs")
        .with_ports(IOPorts::Variable(1), IOPorts::None)
        .with_icon(icon("dup")),
    SimulinkBlockDefinition::new("DataTypePropagation", "Signal Attributes")
        .with_description("Propagate data type from reference input")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(icon("prop")),
    SimulinkBlockDefinition::new("DataTypeScalingStrip", "Signal Attributes")
        .with_description("Remove scaling from a fixed-point signal")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("strip")),
    SimulinkBlockDefinition::new("InitialCondition", "Signal Attributes")
        .with_aliases(&["IC"])
        .with_description("Set initial condition of a signal")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("IC")),
    SimulinkBlockDefinition::new("Probe", "Signal Attributes")
        .with_description("Output signal attributes (width, type, etc.)")
        .with_ports(IOPorts::Fixed(1), IOPorts::Variable(1))
        .with_icon(icon("probe")),
    SimulinkBlockDefinition::new("SignalSpecification", "Signal Attributes")
        .with_aliases(&["Signal Specification"])
        .with_description("Require specified signal attributes")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("spec")),
    SimulinkBlockDefinition::new("UnitConversion", "Signal Attributes")
        .with_description("Convert signal between physical units")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("unit")),
    SimulinkBlockDefinition::new("UnitSystemConfig", "Signal Attributes")
        .with_description("Configure allowed unit systems")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_icon(icon("units")),
    SimulinkBlockDefinition::new("IsFixed", "Signal Attributes")
        .with_description("Output whether input is fixed-point")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("isfix")),
    SimulinkBlockDefinition::new("IsNonBus", "Signal Attributes")
        .with_description("Output whether input is not a bus")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("!bus")),
    SimulinkBlockDefinition::new("VectorToBus", "Signal Attributes")
        .with_aliases(&["Vector to Bus"])
        .with_description("Convert vector signal to a bus")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("v\u{2192}bus")),
    SimulinkBlockDefinition::new("WeightedSampleTime", "Signal Attributes")
        .with_description("Adjust signal by weighted sample time")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("Ts")),
    SimulinkBlockDefinition::new("RateTransition", "Signal Attributes")
        .with_description("Handle transfer between rate groups")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("RT")),
    SimulinkBlockDefinition::new("SignalConversion", "Signal Attributes")
        .with_description("Convert signal while preserving attributes")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("conv")),
];
