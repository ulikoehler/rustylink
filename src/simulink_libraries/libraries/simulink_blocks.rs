//! Extended Simulink block definitions with icons, shapes, and metadata labels.
//!
//! Covers the full range of standard Simulink library blocks so they render
//! with meaningful icons instead of the `?` placeholder.  Registered before
//! the metadata-only palette so these richer definitions take priority.

#![cfg(feature = "egui")]

use crate::simulink_libraries::labels;
use crate::simulink_libraries::renderers;
use crate::simulink_libraries::types::{
    BlockLabelPolicy, IOPorts, MetadataKey, PortLabelPolicy, SimulinkBlockDefinition, SimulinkIcon,
    SimulinkShape,
};

const fn icon(glyph: &'static str) -> SimulinkIcon {
    SimulinkIcon::Utf8(glyph)
}

/// Typeset-math icon (fraction bar / superscript / overbar); see
/// [`crate::egui_app::render::draw_math_icon`] for the notation.
const fn math(spec: &'static str) -> SimulinkIcon {
    SimulinkIcon::Math(spec)
}

/// Line-art icon; see [`crate::egui_app::render::draw_plot_icon`] for the
/// notation.  Simulink draws source waveforms, discontinuity curves and
/// verification plots as vector line art rather than as glyphs.
const fn plot(spec: &'static str) -> SimulinkIcon {
    SimulinkIcon::Plot(spec)
}

/// The thin grey axis cross Simulink draws behind most line-art icons.
/// A macro (not a `const`) so it can be `concat!`-ed into an icon spec.
macro_rules! axes {
    () => {
        "a 0.02,0.5 0.98,0.5; a 0.5,0.02 0.5,0.98;"
    };
}

/// A jittery trace – Simulink's icon for the random / noise sources.
const NOISE_TRACE: &str = concat!(
    "p 0.05,0.52 0.12,0.28 0.19,0.66 0.26,0.34 0.33,0.74 0.40,0.22 0.47,0.58",
    " 0.54,0.30 0.61,0.70 0.68,0.40 0.75,0.24 0.82,0.62 0.89,0.36 0.95,0.54"
);
/// A rising staircase – the counter sources.
const STAIRCASE: &str = concat!(
    "p 0.06,0.88 0.24,0.88 0.24,0.68 0.42,0.68 0.42,0.48 0.60,0.48",
    " 0.60,0.28 0.78,0.28 0.78,0.12 0.94,0.12"
);
/// Repeating ramps that reset – the repeating-sequence sources.
const SAWTOOTH: &str =
    "p 0.06,0.86 0.30,0.14 0.30,0.86 0.54,0.14 0.54,0.86 0.78,0.14 0.78,0.86 0.94,0.38";

/// The plot frame (left/bottom rules) behind every Model Verification icon.
macro_rules! check_axes {
    () => {
        "a 0.10,0.10 0.10,0.92; a 0.04,0.80 0.96,0.80;"
    };
}
/// One period of a sine, spanning the plot area.
macro_rules! sine_wave {
    () => {
        concat!(
            "p 0.12,0.50 0.22,0.29 0.33,0.20 0.43,0.29 0.53,0.50",
            " 0.64,0.71 0.74,0.80 0.84,0.71 0.94,0.50;"
        )
    };
}
/// A triangular wave – the signal Simulink shows inside a range band.
macro_rules! triangle_wave {
    () => {
        "p 0.12,0.62 0.24,0.34 0.36,0.62 0.48,0.34 0.60,0.62 0.72,0.34 0.84,0.62 0.94,0.44;"
    };
}
/// A rising staircase – the quantised signal of Check Input Resolution.
macro_rules! check_stairs {
    () => {
        "p 0.12,0.78 0.30,0.78 0.30,0.60 0.50,0.60 0.50,0.42 0.70,0.42 0.70,0.26 0.94,0.26;"
    };
}

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

    SimulinkBlockDefinition::new("DiscretePulseGenerator", "Sources")
        .with_aliases(&["Discrete Pulse Generator", "Pulse Generator"])
        .with_description("Generate discrete square-pulse signal")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "p 0.06,0.82 0.20,0.82 0.20,0.20 0.38,0.20 0.38,0.82 0.56,0.82",
            " 0.56,0.20 0.74,0.20 0.74,0.82 0.94,0.82"
        ))),

    // ═══════════════════════════════════════════════════════════════════════
    //  Logic and Bit Operations
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("Logic", "Logic and Bit Operations")
        .with_aliases(&["Logical Operator"])
        .with_description("Perform logical operation (AND, OR, NOT, ...)")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_shape(SimulinkShape::None)
        .with_metadata_keys(&[
            MetadataKey::with_default("Operator", "AND"),
            MetadataKey::with_default("IconShape", "rectangular"),
        ])
        .with_static_renderer(renderers::static_logic),

    SimulinkBlockDefinition::new("RelationalOperator", "Logic and Bit Operations")
        .with_aliases(&["Relational Operator"])
        .with_description("Compare two inputs (<=, >=, ==, ~=)")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Operator", "<=")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::relational_operator)),

    SimulinkBlockDefinition::new("BitClear", "Logic and Bit Operations")
        .with_aliases(&["Bit Clear"])
        .with_description("Clear specified bit of stored integer")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("iBit", "0")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::bit_clear)),

    SimulinkBlockDefinition::new("BitSet", "Logic and Bit Operations")
        .with_aliases(&["Bit Set"])
        .with_description("Set specified bit of stored integer")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("iBit", "0")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::bit_set)),

    SimulinkBlockDefinition::new("CompareToZero", "Logic and Bit Operations")
        .with_aliases(&["Compare To Zero", "Compare"])
        .with_description("Compare input signal to zero")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("<= 0"))
        .with_instance_label(labels::compare_to_zero),

    SimulinkBlockDefinition::new("CompareToConstant", "Logic and Bit Operations")
        .with_aliases(&["Compare To Constant"])
        .with_description("Compare input signal to a constant")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("<= K"))
        .with_instance_label(labels::compare_to_constant),

    // Simulink labels these with the comparison they perform against the
    // previous sample (`U/z`), not with an arrow.
    SimulinkBlockDefinition::new("DetectDecrease", "Logic and Bit Operations")
        .with_aliases(&["Detect Decrease"])
        .with_description("Detect decrease in signal value")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("U < U/z")),

    SimulinkBlockDefinition::new("DetectIncrease", "Logic and Bit Operations")
        .with_aliases(&["Detect Increase"])
        .with_description("Detect increase in signal value")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("U > U/z")),

    // ═══════════════════════════════════════════════════════════════════════
    //  Lookup Tables
    // ═══════════════════════════════════════════════════════════════════════
    // Simulink captions the table with its dimensionality and plots the
    // breakpoint curve underneath.
    SimulinkBlockDefinition::new("Lookup_n-D", "Lookup Tables")
        .with_aliases(&["1-D Lookup Table", "2-D Lookup Table", "n-D Lookup Table"])
        .with_description("n-dimensional lookup table interpolation")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("NumberOfTableDimensions", "1")])
        .with_static_renderer(renderers::static_lookup_table),

    SimulinkBlockDefinition::new("Cosine", "Lookup Tables")
        .with_description("Cosine function via lookup table")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_block_label(BlockLabelPolicy::Fixed("cos(2*pi*u)"))
        .with_port_labels(PortLabelPolicy::Fixed(&["u"]), PortLabelPolicy::None),

    SimulinkBlockDefinition::new("Sine", "Lookup Tables")
        .with_description("Sine function via lookup table")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_block_label(BlockLabelPolicy::Fixed("sin(2*pi*u)"))
        .with_port_labels(PortLabelPolicy::Fixed(&["u"]), PortLabelPolicy::None),

    // ═══════════════════════════════════════════════════════════════════════
    //  Math Operations
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("Abs", "Math Operations")
        .with_description("Output absolute value of input")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("|u|")),

    SimulinkBlockDefinition::new("Bias", "Math Operations")
        .with_aliases(&["Add Constant"])
        .with_description("Add a bias (constant) to the input")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Bias", "0")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::bias_value)),

    SimulinkBlockDefinition::new("DotProduct", "Math Operations")
        .with_aliases(&["Dot Product"])
        .with_description("Compute dot product of two vectors")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(icon("\u{2022}")),

    SimulinkBlockDefinition::new("Math", "Math Operations")
        .with_aliases(&["Math Function"])
        .with_description("Apply mathematical function (exp, log, sqrt, ...)")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Operator", "exp")])
        .with_static_renderer(renderers::static_math_function),

    SimulinkBlockDefinition::new("Trigonometry", "Math Operations")
        .with_aliases(&["Trigonometric Function"])
        .with_description("Trigonometric function (sin, cos, tan, acos, ...)")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Operator", "sin")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::trig_function))
        // `sincos` is captionless and identified by its two named outputs.
        .with_static_renderer(renderers::static_trigonometry)
        .with_port_labels(
            PortLabelPolicy::None,
            PortLabelPolicy::MetadataDependent(renderers::trigonometry_port_labels),
        ),

    SimulinkBlockDefinition::new("MinMax", "Math Operations")
        .with_description("Output minimum or maximum of inputs")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Function", "min")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::minmax_function)),

    // Simulink identifies the complex converters purely by their port labels
    // (`|u|`/`∠u`, `Re`/`Im`) drawn beside a small splitter, not by a glyph.
    SimulinkBlockDefinition::new("ComplexToMagnitudeAngle", "Math Operations")
        .with_aliases(&["Complex to Magnitude-Angle"])
        .with_description("Split complex signal to magnitude and angle")
        .with_ports(IOPorts::Fixed(1), IOPorts::Variable(2))
        .with_metadata_keys(&[MetadataKey::with_default("Output", "Magnitude and angle")])
        .with_static_renderer(renderers::static_complex_to_magnitude_angle),

    SimulinkBlockDefinition::new("MagnitudeAngleToComplex", "Math Operations")
        .with_aliases(&["Magnitude-Angle to Complex"])
        .with_description("Combine magnitude and angle into complex signal")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Input", "Magnitude and angle")])
        .with_static_renderer(renderers::static_magnitude_angle_to_complex),

    SimulinkBlockDefinition::new("RealImagToComplex", "Math Operations")
        .with_aliases(&["Real-Imag to Complex"])
        .with_description("Combine real and imaginary into complex signal")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Input", "Real and imag")])
        .with_static_renderer(renderers::static_real_imag_to_complex),

    SimulinkBlockDefinition::new("AlgebraicConstraint", "Math Operations")
        .with_aliases(&["Algebraic Constraint"])
        .with_description("Solve algebraic loop: f(z) = 0")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Constraint", "f(z) = 0")])
        .with_static_renderer(renderers::static_algebraic_constraint)
        .with_port_labels(
            PortLabelPolicy::Fixed(&["f(z)"]),
            PortLabelPolicy::Fixed(&["z"]),
        ),

    SimulinkBlockDefinition::new("MinMaxRunningResettable", "Math Operations")
        .with_aliases(&["MinMax Running Resettable"])
        .with_description("Running min/max with external reset")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Function", "min")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(
            labels::minmax_running_function,
        ))
        .with_port_labels(
            PortLabelPolicy::Fixed(&["u", "R"]),
            PortLabelPolicy::Fixed(&["y"]),
        ),

    SimulinkBlockDefinition::new("Sin", "Sources")
        .with_aliases(&["Sine Wave"])
        .with_description("Generate sine wave using internal time source")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "a 0.06,0.50 0.94,0.50;",
            "p 0.08,0.50 0.19,0.20 0.30,0.12 0.41,0.20 0.52,0.50",
            " 0.63,0.80 0.74,0.88 0.85,0.80 0.94,0.56"
        ))),

    // ═══════════════════════════════════════════════════════════════════════
    //  Matrix Operations  (bridged virtual library fills most; hermitian gap)
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("IsHermitian", "Matrix Operations")
        .with_aliases(&["Is Hermitian"])
        .with_description("Test whether matrix is Hermitian")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Mode", "Hermitian")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::is_hermitian_mode)),

    // ═══════════════════════════════════════════════════════════════════════
    //  Model Verification / Testing
    // ═══════════════════════════════════════════════════════════════════════
    // Simulink draws each verification block as a miniature plot: the signal
    // under test against the grey band of the region it is checked against,
    // with the bound inputs labelled `max`/`min` and the signal `u`.
    SimulinkBlockDefinition::new("Assertion", "Testing & Verification")
        .with_description("Assert that input is nonzero")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(plot(concat!(
            "p 0.02,0.50 0.30,0.50;",
            "c 0.62,0.50,0.30;",
            "p 0.50,0.52 0.59,0.64 0.75,0.34"
        ))),

    SimulinkBlockDefinition::new("CheckDynamicRange", "Testing & Verification")
        .with_aliases(&["Check Dynamic Range"])
        .with_description("Verify signal stays within dynamic range")
        .with_ports(IOPorts::Fixed(3), IOPorts::None)
        .with_port_labels(
            PortLabelPolicy::Fixed(&["max", "min", "u"]),
            PortLabelPolicy::None,
        )
        .with_icon(plot(concat!(
            check_axes!(),
            "b 0.12,0.30 0.94,0.66;",
            triangle_wave!()
        ))),

    SimulinkBlockDefinition::new("CheckStaticGap", "Testing & Verification")
        .with_aliases(&["Check Static Gap"])
        .with_description("Verify no static gap in signal")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(plot(concat!(
            check_axes!(),
            "b 0.12,0.42 0.94,0.58;",
            sine_wave!()
        ))),

    SimulinkBlockDefinition::new("CheckStaticRange", "Testing & Verification")
        .with_aliases(&["Check Static Range"])
        .with_description("Verify signal stays within static range")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(plot(concat!(
            check_axes!(),
            "b 0.12,0.28 0.94,0.68;",
            triangle_wave!()
        ))),

    SimulinkBlockDefinition::new("CheckDynamicGap", "Testing & Verification")
        .with_aliases(&["Check Dynamic Gap"])
        .with_description("Verify no dynamic gap in signal")
        .with_ports(IOPorts::Fixed(3), IOPorts::None)
        .with_port_labels(
            PortLabelPolicy::Fixed(&["max", "min", "u"]),
            PortLabelPolicy::None,
        )
        .with_icon(plot(concat!(
            check_axes!(),
            "b 0.12,0.44 0.94,0.58;",
            sine_wave!()
        ))),

    SimulinkBlockDefinition::new("CheckDiscreteGradient", "Testing & Verification")
        .with_aliases(&["Check Discrete Gradient"])
        .with_description("Verify discrete gradient within bounds")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(plot(concat!(
            check_axes!(),
            sine_wave!(),
            "p 0.44,0.86 0.66,0.18"
        ))),

    SimulinkBlockDefinition::new("CheckDynamicLowerBound", "Testing & Verification")
        .with_aliases(&["Check Dynamic Lower Bound"])
        .with_description("Verify signal above dynamic lower bound")
        .with_ports(IOPorts::Fixed(2), IOPorts::None)
        .with_port_labels(PortLabelPolicy::Fixed(&["min", "u"]), PortLabelPolicy::None)
        .with_icon(plot(concat!(
            check_axes!(),
            "b 0.40,0.60 0.94,0.74;",
            "p 0.12,0.62 0.28,0.30 0.44,0.24 0.60,0.46 0.76,0.66 0.94,0.34"
        ))),

    SimulinkBlockDefinition::new("CheckDynamicUpperBound", "Testing & Verification")
        .with_aliases(&["Check Dynamic Upper Bound"])
        .with_description("Verify signal below dynamic upper bound")
        .with_ports(IOPorts::Fixed(2), IOPorts::None)
        .with_port_labels(PortLabelPolicy::Fixed(&["max", "u"]), PortLabelPolicy::None)
        .with_icon(plot(concat!(
            check_axes!(),
            "b 0.40,0.16 0.94,0.30;",
            "p 0.12,0.72 0.26,0.30 0.44,0.24 0.58,0.52 0.72,0.56 0.94,0.68"
        ))),

    SimulinkBlockDefinition::new("CheckInputResolution", "Testing & Verification")
        .with_aliases(&["Check Input Resolution"])
        .with_description("Verify signal resolution meets requirement")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(plot(concat!(check_axes!(), check_stairs!()))),

    SimulinkBlockDefinition::new("CheckStaticLowerBound", "Testing & Verification")
        .with_aliases(&["Check Static Lower Bound"])
        .with_description("Verify signal above static lower bound")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(plot(concat!(
            check_axes!(),
            "b 0.12,0.62 0.94,0.76;",
            sine_wave!()
        ))),

    SimulinkBlockDefinition::new("CheckStaticUpperBound", "Testing & Verification")
        .with_aliases(&["Check Static Upper Bound"])
        .with_description("Verify signal below static upper bound")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_icon(plot(concat!(
            check_axes!(),
            "b 0.12,0.14 0.94,0.28;",
            sine_wave!()
        ))),

    // ═══════════════════════════════════════════════════════════════════════
    //  Ports & Subsystems
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("EnablePort", "Ports & Subsystems")
        .with_aliases(&["Enable"])
        .with_description("Add enable port to subsystem")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_icon(icon("EN")),

    SimulinkBlockDefinition::new("ForIterator", "Ports & Subsystems")
        .with_aliases(&["For Iterator"])
        .with_description("Repeat subsystem execution a specified number of times")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(icon("for")),

    SimulinkBlockDefinition::new("ForEach", "Ports & Subsystems")
        .with_aliases(&["For Each"])
        .with_description("Partition input and apply subsystem to each element")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{2200}")),

    SimulinkBlockDefinition::new("TriggerPort", "Ports & Subsystems")
        .with_aliases(&["Trigger"])
        .with_description("Add trigger port to subsystem")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_icon(icon("\u{2191}")),

    SimulinkBlockDefinition::new("ResetPort", "Ports & Subsystems")
        .with_aliases(&["Reset"])
        .with_description("Add reset port to subsystem")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_metadata_keys(&[MetadataKey::with_default("ResetTriggerType", "rising")])
        .with_static_renderer(renderers::static_reset_port),

    SimulinkBlockDefinition::new("PMIOPort", "Ports & Subsystems")
        .with_aliases(&["Connection Port", "Simscape Port"])
        .with_description("Physical modeling connection port")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_shape(SimulinkShape::Obround)
        .with_metadata_keys(&[MetadataKey::with_default("Port", "1")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::port_number)),

    // ═══════════════════════════════════════════════════════════════════════
    //  Signal Attributes
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("DataTypeConversion", "Signal Attributes")
        .with_aliases(&["Data Type Conversion"])
        .with_description("Convert signal to specified data type")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("OutDataTypeStr", "Inherit: Inherit via back propagation")])
        .with_static_renderer(renderers::static_data_type_conversion),

    // Simulink draws the propagated width above a diagonal probe line.
    SimulinkBlockDefinition::new("Width", "Signal Attributes")
        .with_description("Output width (number of elements) of input signal")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot("t 0.50,0.30,0.32 -1; p 0.12,0.82 0.88,0.44")),

    SimulinkBlockDefinition::new("SignalConversion", "Signal Routing")
        .with_aliases(&["Signal Conversion"])
        .with_description("Convert between signal types")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("ConversionOutput", "Signal copy")])
        .with_static_renderer(renderers::static_signal_conversion),

    SimulinkBlockDefinition::new("BusToVector", "Signal Attributes")
        .with_aliases(&["Bus to Vector"])
        .with_description("Convert bus to a vector signal")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "p 0.05,0.30 0.40,0.30; p 0.05,0.50 0.40,0.50; p 0.05,0.70 0.40,0.70;",
            "f 0.40,0.10 0.56,0.90;",
            "p 0.56,0.50 0.95,0.50"
        ))),

    // ═══════════════════════════════════════════════════════════════════════
    //  Signal Routing
    // ═══════════════════════════════════════════════════════════════════════
    // Simulink labels this with the signals being replaced, e.g.
    // `Bus / Bus := signal1`; it is not one of the solid black bus bars.
    SimulinkBlockDefinition::new("BusAssignment", "Signal Routing")
        .with_aliases(&["Bus Assignment"])
        .with_description("Assign signals to a bus")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("AssignedSignals", "")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::bus_assignment)),

    SimulinkBlockDefinition::new("GotoTagVisibility", "Signal Routing")
        .with_aliases(&["Goto Tag Visibility"])
        .with_description("Define scope of Goto tag visibility")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_metadata_keys(&[MetadataKey::with_default("GotoTag", "A")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::goto_tag_braced)),

    SimulinkBlockDefinition::new("Merge", "Signal Routing")
        .with_description("Merge multiple signals into single output")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_block_label(BlockLabelPolicy::Fixed("merge")),

    // Both switches are drawn as a schematic: fixed contacts on the left and a
    // lever swinging to the selected one.
    SimulinkBlockDefinition::new("MultiPortSwitch", "Signal Routing")
        .with_aliases(&["Multiport Switch"])
        .with_description("Select one of N inputs based on control signal")
        .with_ports(IOPorts::Variable(4), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Inputs", "")])
        .with_static_renderer(renderers::static_multiport_switch)
        .with_port_labels(
            PortLabelPolicy::MetadataDependent(renderers::multiport_switch_port_labels),
            PortLabelPolicy::None,
        ),

    SimulinkBlockDefinition::new("Selector", "Signal Routing")
        .with_description("Select input elements from a vector/matrix")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[
            MetadataKey::with_default("NumberOfDimensions", "1"),
            MetadataKey::with_default("InputPortWidth", "3"),
            MetadataKey::with_default("Indices", "[1]"),
        ])
        .with_static_renderer(renderers::static_selector),

    SimulinkBlockDefinition::new("Switch", "Signal Routing")
        .with_description("Switch between two inputs based on threshold")
        .with_ports(IOPorts::Fixed(3), IOPorts::Fixed(1))
        .with_metadata_keys(&[
            MetadataKey::with_default("Criteria", "u2 >= Threshold"),
            MetadataKey::with_default("Threshold", "0"),
        ])
        .with_static_renderer(renderers::static_switch),

    // ═══════════════════════════════════════════════════════════════════════
    //  Sinks
    // ═══════════════════════════════════════════════════════════════════════
    // Simulink's XY Graph icon is a scatter of samples along a rising trend.
    SimulinkBlockDefinition::new("Record", "Sinks")
        .with_aliases(&["XY Graph", "To Workspace"])
        .with_description("Record signal data")
        .with_ports(IOPorts::Variable(1), IOPorts::None)
        .with_icon(plot(concat!(
            "r 0.04,0.04 0.96,0.96; p 0.10,0.90 0.90,0.10;",
            "d 0.22,0.72 0.045; d 0.34,0.70 0.045; d 0.44,0.54 0.045;",
            "d 0.58,0.46 0.045; d 0.68,0.30 0.045; d 0.80,0.24 0.045"
        ))),

    // ═══════════════════════════════════════════════════════════════════════
    //  Sources
    // ═══════════════════════════════════════════════════════════════════════
    // Simulink's Sources icons are miniature plots of the signal each block
    // produces, drawn as line art rather than as a text abbreviation.
    SimulinkBlockDefinition::new("Clock", "Sources")
        .with_description("Output continuous simulation time")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot("c 0.5,0.5 0.44; p 0.5,0.5 0.5,0.18; p 0.5,0.5 0.74,0.62")),

    SimulinkBlockDefinition::new("DigitalClock", "Sources")
        .with_aliases(&["Digital Clock"])
        .with_description("Output simulation time at specified sample rate")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(icon("12:34")),

    SimulinkBlockDefinition::new("Ground", "Sources")
        .with_description("Output zero-valued signal")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "p 0.20,0.40 0.80,0.40; p 0.30,0.60 0.70,0.60; p 0.42,0.80 0.58,0.80;",
            "p 0.50,0.15 0.50,0.40"
        ))),

    SimulinkBlockDefinition::new("RandomNumber", "Sources")
        .with_aliases(&["Random Number"])
        .with_description("Generate normally distributed random numbers")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(NOISE_TRACE)),

    SimulinkBlockDefinition::new("UniformRandomNumber", "Sources")
        .with_aliases(&["Uniform Random Number"])
        .with_description("Generate uniformly distributed random numbers")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(NOISE_TRACE)),

    SimulinkBlockDefinition::new("SignalGenerator", "Sources")
        .with_aliases(&["Signal Generator"])
        .with_description("Generate various waveforms (sine, square, sawtooth)")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "r 0.06,0.10 0.94,0.90;",
            "r 0.16,0.24 0.30,0.42; r 0.36,0.24 0.50,0.42;",
            "r 0.56,0.24 0.70,0.42; r 0.76,0.24 0.90,0.42;",
            "c 0.28,0.68 0.09; c 0.62,0.68 0.09"
        ))),

    SimulinkBlockDefinition::new("Step", "Sources")
        .with_description("Generate step function signal")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot("p 0.08,0.82 0.45,0.82 0.45,0.16 0.92,0.16")),

    SimulinkBlockDefinition::new("Ramp", "Sources")
        .with_description("Generate ramp signal")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot("p 0.08,0.88 0.40,0.88 0.92,0.12")),

    SimulinkBlockDefinition::new("BandLimitedWhiteNoise", "Sources")
        .with_aliases(&["Band-Limited White Noise"])
        .with_description("White noise with specified bandwidth")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(NOISE_TRACE)),

    SimulinkBlockDefinition::new("Chirp", "Sources")
        .with_aliases(&["Chirp Signal"])
        .with_description("Generate frequency-swept sinusoidal signal")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "p 0.06,0.50 0.14,0.14 0.24,0.86 0.34,0.14 0.42,0.86 0.49,0.14",
            " 0.55,0.86 0.61,0.14 0.66,0.86 0.71,0.14 0.76,0.86 0.80,0.14",
            " 0.84,0.86 0.88,0.14 0.92,0.86"
        ))),

    SimulinkBlockDefinition::new("Counter", "Sources")
        .with_aliases(&["Counter Free-Running"])
        .with_description("Free-running counter output")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(STAIRCASE)),

    SimulinkBlockDefinition::new("CounterLimited", "Sources")
        .with_aliases(&["Counter Limited"])
        .with_description("Counter with configurable upper limit")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "t 0.20,0.12,0.22 lim;",
            "p 0.06,0.92 0.24,0.92 0.24,0.74 0.42,0.74 0.42,0.56 0.60,0.56",
            " 0.60,0.38 0.78,0.38 0.78,0.92 0.94,0.92"
        ))),

    SimulinkBlockDefinition::new("Repeating", "Sources")
        .with_aliases(&["Repeating Sequence"])
        .with_description("Generate repeating arbitrary signal")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(SAWTOOTH)),

    SimulinkBlockDefinition::new("RepeatingInterp", "Sources")
        .with_aliases(&["Repeating Sequence Interpolated"])
        .with_description("Repeating sequence with interpolation")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(
            "p 0.06,0.20 0.22,0.62 0.34,0.30 0.50,0.78 0.68,0.86 0.94,0.80"
        )),

    SimulinkBlockDefinition::new("RepeatingStair", "Sources")
        .with_aliases(&["Repeating Sequence Stair"])
        .with_description("Generate repeating staircase signal")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "p 0.06,0.80 0.24,0.80 0.24,0.56 0.42,0.56 0.42,0.30 0.60,0.30",
            " 0.60,0.80 0.78,0.80 0.78,0.56 0.94,0.56"
        ))),

    SimulinkBlockDefinition::new("WaveformGenerator", "Sources")
        .with_aliases(&["Waveform Generator"])
        .with_description("Generate waveform from stored table")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "p 0.06,0.50 0.14,0.24 0.22,0.62 0.30,0.20 0.38,0.70 0.46,0.34",
            " 0.54,0.66 0.62,0.22 0.70,0.60 0.78,0.28 0.86,0.56 0.94,0.40"
        ))),

    // ═══════════════════════════════════════════════════════════════════════
    //  String Operations
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("ASCIIToString", "String Operations")
        .with_aliases(&["ASCII to String"])
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

    SimulinkBlockDefinition::new("MATLABFunction", "User-Defined Functions")
        .with_aliases(&["MATLAB Function", "MATLAB Fcn", "Interpreted MATLAB Function"])
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

    SimulinkBlockDefinition::new("CustomCallbackButton", "Dashboard")
        .with_aliases(&["Callback Button"])
        .with_description("Dashboard callback button")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_block_label(BlockLabelPolicy::Fixed("Button")),

    // ═══════════════════════════════════════════════════════════════════════
    //  Timing & Scheduling / Advanced
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("EventListener", "Ports & Subsystems")
        .with_aliases(&["Event Listener"])
        .with_description("Listen for simulation events")
        .with_ports(IOPorts::None, IOPorts::None)
        .with_icon(icon("evt")),

    SimulinkBlockDefinition::new("StateReader", "Ports & Subsystems")
        .with_aliases(&["State Reader"])
        .with_description("Read block state for logging or initialisation")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_shape(SimulinkShape::None)
        .with_static_renderer(renderers::static_state_parameter_access),

    SimulinkBlockDefinition::new("StateWriter", "Ports & Subsystems")
        .with_aliases(&["State Writer"])
        .with_description("Write values into block state")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_shape(SimulinkShape::None)
        .with_static_renderer(renderers::static_state_parameter_access),

    SimulinkBlockDefinition::new("ParameterWriter", "Ports & Subsystems")
        .with_aliases(&["Parameter Writer"])
        .with_description("Write values into another block's parameters")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_shape(SimulinkShape::None)
        .with_static_renderer(renderers::static_state_parameter_access),

    SimulinkBlockDefinition::new("DataStoreRead", "Signal Routing")
        .with_aliases(&["Data Store Read"])
        .with_description("Read from a data store")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("DataStoreName", "A")])
        .with_static_renderer(renderers::static_data_store_access),

    SimulinkBlockDefinition::new("DataStoreWrite", "Signal Routing")
        .with_aliases(&["Data Store Write"])
        .with_description("Write to a data store")
        .with_ports(IOPorts::Fixed(1), IOPorts::None)
        .with_metadata_keys(&[MetadataKey::with_default("DataStoreName", "A")])
        .with_static_renderer(renderers::static_data_store_access),
];
