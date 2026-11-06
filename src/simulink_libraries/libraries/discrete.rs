//! Discrete-time blocks: unit delays, discrete integrators and filters.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::{icon, math};
use crate::simulink_libraries::renderers;
use crate::simulink_libraries::types::{
    IOPorts, MetadataKey, PortLabelPolicy, SimulinkBlockDefinition,
};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // ═══════════════════════════════════════════════════════════════════════
    //  Discrete
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("Delay", "Discrete")
        .with_description("Delay input by variable number of sample periods")
        .with_ports(IOPorts::Variable(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[
            MetadataKey::with_default("DelayLength", "2"),
            MetadataKey::with_default("DelayLengthSource", "Dialog"),
            MetadataKey::with_default("InputPortMap", "u0"),
            MetadataKey::with_default("ExternalReset", "None"),
        ])
        .with_static_renderer(renderers::static_delay)
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(renderers::delay_port_labels),
            PortLabelPolicy::None,
        ),
    SimulinkBlockDefinition::new("DiscreteIntegrator", "Discrete")
        .with_aliases(&["Discrete-Time Integrator", "Discrete Time Integrator"])
        .with_description("Discrete-time integrator / accumulator")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("IntegratorMethod", "Integration: Forward Euler")])
        .with_static_renderer(renderers::static_discrete_integrator),
    SimulinkBlockDefinition::new("UnitDelay", "Discrete")
        .with_aliases(&["Unit Delay"])
        .with_description("Delay input by one sample period")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:1/z")),
    SimulinkBlockDefinition::new("Difference", "Discrete")
        .with_description("Compute difference between successive samples")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:z-1/z")),
    SimulinkBlockDefinition::new("Discrete Derivative", "Discrete")
        .with_aliases(&["DiscreteDerivative"])
        .with_description("Discrete-time derivative K(z-1)/(Ts z)")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:K(z-1)/Ts z")),
    SimulinkBlockDefinition::new("DiscreteFilter", "Discrete")
        .with_description("Discrete-time IIR/FIR filter")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:1/(z+0.5)")),
    SimulinkBlockDefinition::new("DiscreteTransferFcn", "Discrete")
        .with_aliases(&["Discrete Transfer Fcn"])
        .with_description("Discrete-time transfer function")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:(z-0.5)/z")),
    SimulinkBlockDefinition::new("DiscreteStateSpace", "Discrete")
        .with_aliases(&["Discrete State-Space"])
        .with_description("Discrete-time state-space model")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("lines:x(k+1) = Ax + Bu|y(k) = Cx + Du")),
    SimulinkBlockDefinition::new("DiscreteZeroPole", "Discrete")
        .with_aliases(&["Discrete Zero-Pole"])
        .with_description("Discrete transfer function by zeros and poles")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:Z(z)/P(z)")),
    SimulinkBlockDefinition::new("DiscreteFIRFilter", "Discrete")
        .with_aliases(&["Discrete FIR Filter"])
        .with_description("Discrete-time FIR filter")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:b0+b1 z^-1+\u{22EF}")),
    SimulinkBlockDefinition::new("DiscretePIDController", "Discrete")
        .with_aliases(&["Discrete PID Controller"])
        .with_description("Discrete-time PID controller")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:K(z+a)/(z-1)")),
    SimulinkBlockDefinition::new("DiscretePIDController2DOF", "Discrete")
        .with_aliases(&["Discrete PID Controller (2DOF)"])
        .with_description("Two-degree-of-freedom discrete PID")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(math("frac:K(z+a)/(z-1)")),
    SimulinkBlockDefinition::new("FirstOrderHold", "Discrete")
        .with_aliases(&["First-Order Hold"])
        .with_description("First-order hold interpolation")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:(1+z^{-1})/2")),
    SimulinkBlockDefinition::new("Memory", "Discrete")
        .with_description("Output input from previous time step")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:u(t-1)")),
    SimulinkBlockDefinition::new("ResettableDelay", "Discrete")
        .with_description("Delay with external reset input")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(math("frac:z^{-d}")),
    SimulinkBlockDefinition::new("TappedDelay", "Discrete")
        .with_description("Delay with intermediate taps")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:z^{-N}")),
    SimulinkBlockDefinition::new("VariableIntegerDelay", "Discrete")
        .with_description("Delay by a variable integer number of samples")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(math("frac:z^{-d(n)}")),
    SimulinkBlockDefinition::new("TransferFcnDirectFormII", "Discrete")
        .with_aliases(&["Transfer Fcn Direct Form II"])
        .with_description("Transfer function in direct-form II")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:b(z)/a(z)")),
    SimulinkBlockDefinition::new("TransferFcnDirectFormIITimeVarying", "Discrete")
        .with_aliases(&["Transfer Fcn Direct Form II Time Varying"])
        .with_description("Time-varying direct-form II transfer function")
        .with_ports(IOPorts::Fixed(3), IOPorts::Fixed(1))
        .with_icon(math("frac:b(z,t)/a(z,t)")),
    SimulinkBlockDefinition::new("CICFilter", "Additional Math and Discrete")
        .with_description("Cascaded integrator-comb filter")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("frac:CIC")),
    SimulinkBlockDefinition::new("IncrementRealWorld", "Additional Math and Discrete")
        .with_description("Increment signal by 1 (real-world)")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("+1")),
    SimulinkBlockDefinition::new("IncrementStoredInteger", "Additional Math and Discrete")
        .with_description("Increment signal by 1 (stored integer)")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("+1i")),
    SimulinkBlockDefinition::new("IncrementDecrementToZero", "Additional Math and Discrete")
        .with_description("Increment or decrement toward zero")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{00B1}1")),
    SimulinkBlockDefinition::new("IncrementDecrementToValue", "Additional Math and Discrete")
        .with_description("Increment or decrement toward a value")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(icon("\u{00B1}1")),
    SimulinkBlockDefinition::new("DecrementRealWorld", "Additional Math and Discrete")
        .with_description("Decrement signal by 1 (real-world)")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("-1")),
    SimulinkBlockDefinition::new("DecrementStoredInteger", "Additional Math and Discrete")
        .with_description("Decrement signal by 1 (stored integer)")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("-1i")),

    SimulinkBlockDefinition::new("ZeroOrderHold", "Discrete")
        .with_aliases(&["Zero-Order Hold"])
        .with_description("Sample and hold at a fixed rate")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("ZOH")),
    SimulinkBlockDefinition::new("TriggeredDelay", "Discrete")
        .with_description("Delay gated by a trigger")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(math("frac:z^{-d}")),
    SimulinkBlockDefinition::new("TriggeredSampleAndHold", "Discrete")
        .with_description("Sample-and-hold gated by a trigger")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(icon("S&H")),
];
