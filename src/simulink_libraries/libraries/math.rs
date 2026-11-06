//! Math operation blocks: arithmetic, trigonometry, complex numbers.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::icon;
use crate::simulink_libraries::labels;
use crate::simulink_libraries::renderers;
use crate::simulink_libraries::types::{
    BlockLabelPolicy, IOPorts, MetadataKey, PortLabelPolicy, SimulinkBlockDefinition, SimulinkShape,
};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // ── Math operations ────────────────────────────────────────────────
    SimulinkBlockDefinition::new("Product", "Math Operations")
        .with_description("Multiply or divide inputs")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_icon(icon("×"))
        .with_metadata_keys(&[
            MetadataKey::with_default("Inputs", "**"),
            MetadataKey::with_default("Multiplication", "Element-wise(.*)"),
        ])
        .with_static_renderer(renderers::static_product),
    SimulinkBlockDefinition::new("Sum", "Math Operations")
        .with_description("Add or subtract inputs")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_shape(SimulinkShape::None)
        .with_metadata_keys(&[
            MetadataKey::with_default("IconShape", "round"),
            MetadataKey::with_default("Inputs", "++"),
        ])
        .with_port_overrides_fn(renderers::sum_port_overrides)
        .with_static_renderer(renderers::static_sum),
    SimulinkBlockDefinition::new("Gain", "Math Operations")
        .with_description("Multiply input by a constant")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_shape(SimulinkShape::Triangle)
        .with_metadata_keys(&[MetadataKey::with_default("Gain", "1")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::gain_value)),
    SimulinkBlockDefinition::new("ComplexToRealImag", "Math Operations")
        .with_description("Split a complex signal into real and imaginary parts")
        .with_ports(IOPorts::Fixed(1), IOPorts::Variable(2))
        .with_metadata_keys(&[MetadataKey::with_default("Output", "Real and imag")])
        .with_static_renderer(renderers::static_complex_to_real_imag),
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
    // ═══════════════════════════════════════════════════════════════════════
    //  Matrix Operations  (bridged virtual library fills most; hermitian gap)
    // ═══════════════════════════════════════════════════════════════════════
    SimulinkBlockDefinition::new("IsHermitian", "Matrix Operations")
        .with_aliases(&["Is Hermitian"])
        .with_description("Test whether matrix is Hermitian")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Mode", "Hermitian")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::is_hermitian_mode)),
    SimulinkBlockDefinition::new("Sign", "Math Operations")
        .with_description("Sign of input: -1, 0, or 1")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("sgn")),
    SimulinkBlockDefinition::new("Rounding", "Math Operations")
        .with_aliases(&["Rounding Function"])
        .with_description("Round to a specified integer rule")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("round")),
    SimulinkBlockDefinition::new("UnaryMinus", "Math Operations")
        .with_description("Negate the input")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("-u")),
    SimulinkBlockDefinition::new("SignedSqrt", "Math Operations")
        .with_description("Square root of absolute value, signed")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{221A}|u|")),
    SimulinkBlockDefinition::new("Sqrt", "Math Operations")
        .with_description("Square root")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{221A}")),
    SimulinkBlockDefinition::new("Abs2", "Math Operations")
        .with_description("Squared magnitude")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("|u|\u{00B2}")),
    SimulinkBlockDefinition::new("ProductOfElements", "Math Operations")
        .with_description("Product of all input elements")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{220F}")),
    SimulinkBlockDefinition::new("Reshape", "Math Operations")
        .with_description("Reshape input to a new dimensionality")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{2B0D}")),
    SimulinkBlockDefinition::new("PermuteDimensions", "Math Operations")
        .with_description("Permute input dimensions")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{22A4}d")),
    SimulinkBlockDefinition::new("VectorConcatenate", "Math Operations")
        .with_description("Concatenate inputs into a vector")
        .with_ports(IOPorts::Variable(1), IOPorts::Fixed(1))
        .with_icon(icon("[u1;u2]")),
    SimulinkBlockDefinition::new("Assignment", "Math Operations")
        .with_description("Assign values at specified indices")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_icon(icon("u[i]=y")),
    SimulinkBlockDefinition::new("Polynomial", "Math Operations")
        .with_description("Evaluate a polynomial")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("P(u)")),
    SimulinkBlockDefinition::new("MatrixGain", "Math Operations")
        .with_description("Multiply by a matrix gain")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("K")),
    SimulinkBlockDefinition::new("WeightedSampleTime", "Math Operations")
        .with_description("Multiply by weighted sample time")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("w Ts")),
    SimulinkBlockDefinition::new("Find", "Math Operations")
        .with_description("Find nonzero elements and their indices")
        .with_ports(IOPorts::Fixed(1), IOPorts::Variable(1))
        .with_icon(icon("find")),
    SimulinkBlockDefinition::new("ReciprocalSqrt", "Math Operations")
        .with_description("Reciprocal square root")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("1/\u{221A}")),
];
