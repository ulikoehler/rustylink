//! Continuous-time blocks: integrators, derivatives, transfer functions.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::{math, plot};
use crate::simulink_libraries::renderers;
use crate::simulink_libraries::types::{
    IOPorts, MetadataKey, PortLabelPolicy, SimulinkBlockDefinition,
};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // ═══════════════════════════════════════════════════════════════════════
    //  Continuous
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("Derivative", "Continuous")
        .with_description("Output the time derivative of the input")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:\u{0394}u/\u{0394}t")),
    // Simulink draws the plain Integrator as `1/s`; enabling output limits
    // (`LimitOutput`) or state wrapping (`WrapState`) decorates it, and the
    // external reset / initial-condition sources add labelled input ports.
    SimulinkBlockDefinition::new("Integrator", "Continuous")
        .with_description("Integrate input signal over time")
        .with_ports(IOPorts::Variable(1), IOPorts::Fixed(1))
        .with_icon(math("frac:1/s"))
        .with_metadata_keys(&[
            MetadataKey::with_default("LimitOutput", "off"),
            MetadataKey::with_default("WrapState", "off"),
            MetadataKey::with_default("ExternalReset", "none"),
            MetadataKey::with_default("InitialConditionSource", "internal"),
        ])
        .with_static_renderer(renderers::static_integrator)
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(renderers::integrator_port_labels),
            PortLabelPolicy::None,
        ),
    SimulinkBlockDefinition::new("TransferFcn", "Continuous")
        .with_aliases(&["Transfer Fcn", "Transfer Function"])
        .with_description("Linear transfer function (numerator/denominator in s)")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[
            MetadataKey::with_default("Numerator", "[1]"),
            MetadataKey::with_default("Denominator", "[1 1]"),
        ])
        .with_static_renderer(renderers::static_transfer_fcn),
    SimulinkBlockDefinition::new("SecondOrderIntegrator", "Continuous")
        .with_aliases(&["Second-Order Integrator"])
        .with_description("Integrate twice: acceleration to position")
        .with_ports(IOPorts::Variable(1), IOPorts::Fixed(2))
        .with_metadata_keys(&[
            MetadataKey::with_default("LimitX", "off"),
            MetadataKey::with_default("WrapX", "off"),
            MetadataKey::with_default("LimitDXDT", "off"),
            MetadataKey::with_default("ICSourceX", "internal"),
            MetadataKey::with_default("ICSourceDXDT", "internal"),
        ])
        .with_static_renderer(renderers::static_second_order_integrator)
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(renderers::second_order_integrator_port_labels),
            PortLabelPolicy::MetadataDependent(renderers::second_order_integrator_port_labels),
        ),
    SimulinkBlockDefinition::new("DescriptorStateSpace", "Continuous")
        .with_aliases(&["Descriptor State-Space"])
        .with_description("Descriptor state-space model E*dx = Ax + Bu")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("lines:E\u{1E8B} = Ax + Bu|y = Cx + Du")),
// ═══════════════════════════════════════════════════════════════════════
    //  Continuous — transfer-function & integrator variants
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("IntegratorLimited", "Continuous")
        .with_aliases(&["Integrator, Limited"])
        .with_description("Integrator with output saturation limits")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:1/s")),
    SimulinkBlockDefinition::new("IntegratorSecondOrderLimited", "Continuous")
        .with_aliases(&["Second-Order Integrator, Limited"])
        .with_description("Second-order integrator with limits")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(2))
        .with_icon(math("frac:1/s\u{00B2}")),
    SimulinkBlockDefinition::new("PIDController", "Continuous")
        .with_aliases(&["PID Controller"])
        .with_description("Continuous-time PID controller")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:K(s+a)/s")),
    SimulinkBlockDefinition::new("PIDController2DOF", "Continuous")
        .with_aliases(&["PID Controller (2DOF)"])
        .with_description("Two-degree-of-freedom PID")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(math("frac:K(s+a)/s")),
    SimulinkBlockDefinition::new("TransferFcnFirstOrder", "Continuous")
        .with_aliases(&["Transfer Fcn First Order"])
        .with_description("First-order transfer function K/(Ts+1)")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:K/(Ts+1)")),
    SimulinkBlockDefinition::new("TransferFcnLeadLag", "Continuous")
        .with_aliases(&["Transfer Fcn Lead or Lag"])
        .with_description("Lead/lag compensator")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:(s+z)/(s+p)")),
    SimulinkBlockDefinition::new("TransferFcnRealZero", "Continuous")
        .with_aliases(&["Transfer Fcn Real Zero"])
        .with_description("Transfer function with a real zero")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:(s+z)/s")),
    SimulinkBlockDefinition::new("FirstOrderTransferFunction", "Continuous")
        .with_description("First-order transfer function")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:K/(Ts+1)")),
    SimulinkBlockDefinition::new("ZeroPole", "Continuous")
        .with_description("Transfer function specified by zeros and poles")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:Z(s)/P(s)")),
    SimulinkBlockDefinition::new("TransportDelay", "Continuous")
        .with_description("Delay signal by a fixed time")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:e^(-sT)")),
    SimulinkBlockDefinition::new("TransferDelay", "Continuous")
        .with_description("Transport delay specified by delay time")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:e^(-sT)")),
    SimulinkBlockDefinition::new("VariableTimeDelay", "Continuous")
        .with_aliases(&["Variable Transport Delay"])
        .with_description("Delay by a time-varying amount")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(math("frac:e^(-sT(t))")),
    SimulinkBlockDefinition::new("Continuous-VCO", "Continuous")
        .with_description("Continuous-time voltage-controlled oscillator")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "p 0.06,0.50 0.16,0.16 0.26,0.84 0.36,0.16 0.46,0.84",
            " 0.56,0.50 0.66,0.36 0.76,0.62 0.86,0.42 0.94,0.50"
        ))),
];
