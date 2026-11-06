//! Discontinuity blocks: saturation, dead zone, rate limiter, backlash.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::{axes, plot};
use crate::simulink_libraries::types::{IOPorts, SimulinkBlockDefinition};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // ═══════════════════════════════════════════════════════════════════════
    //  Discontinuities
    // ═══════════════════════════════════════════════════════════════════════
    // Simulink draws the discontinuity icons as transfer curves over a faint
    // axis cross: a slanted hysteresis ladder, a friction curve with a jump at
    // the origin, and the flat-rise-flat saturation curve.
    SimulinkBlockDefinition::new("Backlash", "Discontinuities")
        .with_description("Model backlash (dead-zone in a mechanical gear)")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.13,0.90 0.50,0.90 0.87,0.12 0.50,0.12 0.13,0.90;",
            "p 0.25,0.64 0.62,0.64; p 0.38,0.38 0.75,0.38"
        ))),
    SimulinkBlockDefinition::new("Saturate", "Discontinuities")
        .with_aliases(&["Saturation"])
        .with_description("Limit input signal to upper and lower bounds")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.06,0.82 0.32,0.82 0.68,0.16 0.94,0.16"
        ))),
    SimulinkBlockDefinition::new("CoulombViscousFriction", "Discontinuities")
        .with_aliases(&["Coulomb & Viscous Friction", "Coulomb"])
        .with_description("Coulomb and viscous friction model")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.06,0.94 0.50,0.62; p 0.50,0.38 0.94,0.06"
        ))),
    // Dead zone: output stays flat across |x|<deadband.
    SimulinkBlockDefinition::new("DeadZone", "Discontinuities")
        .with_description("Set output to zero within a dead region")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.06,0.06 0.42,0.46 0.58,0.54 0.94,0.94"
        ))),
    // Dynamic dead zone carries `up`/`lo` limit inputs.
    SimulinkBlockDefinition::new("DeadZoneDynamic", "Discontinuities")
        .with_aliases(&["Dead Zone Dynamic"])
        .with_description("Dead zone with dynamic limits")
        .with_ports(IOPorts::Fixed(3), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.06,0.06 0.42,0.46 0.58,0.54 0.94,0.94"
        ))),
    // Quantizer: staircase transfer curve.
    SimulinkBlockDefinition::new("Quantizer", "Discontinuities")
        .with_description("Discretize input at a specified interval")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.08,0.84 0.24,0.84 0.24,0.70 0.38,0.70 0.38,0.56",
            " 0.52,0.56 0.52,0.42 0.66,0.42 0.66,0.28 0.80,0.28 0.80,0.14 0.94,0.14"
        ))),
    // Rate limiter: slope-capped rising edge.
    SimulinkBlockDefinition::new("RateLimiter", "Discontinuities")
        .with_aliases(&["Rate Limiter"])
        .with_description("Limit the rate of change of a signal")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.08,0.88 0.46,0.48 0.78,0.48 0.92,0.14"
        ))),
    // Dynamic rate limiter carries `up`/`lo` inputs.
    SimulinkBlockDefinition::new("RateLimiterDynamic", "Discontinuities")
        .with_aliases(&["Rate Limiter Dynamic"])
        .with_description("Rate limiter with dynamic limits")
        .with_ports(IOPorts::Fixed(3), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.08,0.88 0.46,0.48 0.78,0.48 0.92,0.14"
        ))),
    // Relay: square-wave hysteresis.
    SimulinkBlockDefinition::new("Relay", "Discontinuities")
        .with_description("Switch output between two constants with hysteresis")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.18,0.82 0.66,0.82 0.66,0.58 0.30,0.58;",
            "p 0.34,0.42 0.70,0.42 0.70,0.18 0.82,0.18"
        ))),
    // Dynamic saturation: clip at `up`/`lo` inputs.
    SimulinkBlockDefinition::new("SaturateDynamic", "Discontinuities")
        .with_aliases(&["Saturation Dynamic"])
        .with_description("Limit signal to dynamic upper and lower bounds")
        .with_ports(IOPorts::Fixed(3), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.06,0.82 0.32,0.82 0.68,0.16 0.94,0.16"
        ))),
    // Wrap to zero: output wraps to 0 above a threshold.
    SimulinkBlockDefinition::new("WrapToZero", "Discontinuities")
        .with_aliases(&["Wrap To Zero"])
        .with_description("Set output to zero when input exceeds threshold")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            axes!(),
            "p 0.08,0.16 0.44,0.16 0.44,0.50 0.56,0.50 0.56,0.84 0.92,0.84"
        ))),
];
