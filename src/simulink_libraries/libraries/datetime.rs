//! DateTime blocks: simulation-clock outputs and conversions.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::{icon, plot};
use crate::simulink_libraries::types::{IOPorts, SimulinkBlockDefinition};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // ═══════════════════════════════════════════════════════════════════════
    //  DateTime
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("DateTimeClock", "DateTime")
        .with_aliases(&["DateTime Clock"])
        .with_description("Output current simulation date and time")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot("c 0.5,0.5 0.42; p 0.5,0.5 0.5,0.22; p 0.5,0.5 0.72,0.60")),
    SimulinkBlockDefinition::new("DateTimeDataTypeConverter", "DateTime")
        .with_aliases(&["DateTime Data Type Converter"])
        .with_description("Convert datetime between representations")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("dt\u{2192}")),
    SimulinkBlockDefinition::new("DateTimeElapsedTime", "DateTime")
        .with_aliases(&["DateTime Elapsed Time", "ElapsedTime"])
        .with_description("Output elapsed simulation time")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(icon("dt")),
    SimulinkBlockDefinition::new("DateTimeExtractor", "DateTime")
        .with_aliases(&["DateTime Extractor"])
        .with_description("Extract year/month/day/hour/min/sec")
        .with_ports(IOPorts::Fixed(1), IOPorts::Variable(1))
        .with_icon(icon("dt\u{2192}ymd")),
    SimulinkBlockDefinition::new("DateTimeInterval", "DateTime")
        .with_description("Interval between two datetimes")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(icon("dt-dt")),
];
