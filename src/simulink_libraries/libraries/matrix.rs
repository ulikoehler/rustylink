//! Matrix-operations library (`matrix_library`).
//!
//! These blocks carry dedicated SVG icons shipped under `icons/matrix/`.  Port
//! counts mirror [`crate::simulink_libraries::stubs::MATRIX_BLOCKS`], which the
//! core parser uses to synthesise stubs when the `.slx` library file is absent.

#![cfg(feature = "egui")]

use crate::simulink_libraries::labels;
use crate::simulink_libraries::renderers;
use crate::simulink_libraries::types::{
    BlockLabelPolicy, IOPorts, MetadataKey, SimulinkBlockDefinition, SimulinkIcon,
};

const CAT: &str = "Matrix Operations";

/// Typeset-math icon (superscript); see [`crate::egui_app::render::draw_math_icon`].
const fn math(spec: &'static str) -> SimulinkIcon {
    SimulinkIcon::Math(spec)
}

/// Line-art icon; see [`crate::egui_app::render::draw_plot_icon`].
const fn plot(spec: &'static str) -> SimulinkIcon {
    SimulinkIcon::Plot(spec)
}

/// A 3×3 grid of dots inside square brackets – Simulink's "a matrix" pictogram,
/// occupying the right-hand half of the icon.
macro_rules! dot_matrix {
    () => {
        concat!(
            "p 0.62,0.16 0.55,0.16 0.55,0.84 0.62,0.84;",
            "p 0.90,0.16 0.97,0.16 0.97,0.84 0.90,0.84;",
            "d 0.63,0.28 0.035; d 0.76,0.28 0.035; d 0.89,0.28 0.035;",
            "d 0.63,0.50 0.035; d 0.76,0.50 0.035; d 0.89,0.50 0.035;",
            "d 0.63,0.72 0.035; d 0.76,0.72 0.035; d 0.89,0.72 0.035"
        )
    };
}

pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // Simulink labels most of these blocks with the MATLAB function they wrap
    // (`eye`, `cross`, `symmetric`, `hermitian`) rather than with a pictogram.
    SimulinkBlockDefinition::new("Identity Matrix", CAT)
        .with_aliases(&["IdentityMatrix"])
        .with_description("Generate an identity matrix")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_block_label(BlockLabelPolicy::Fixed("eye")),
    // The letter is the tested triangularity (`Upper` → `U`, `Lower` → `L`),
    // drawn beside the diagonal of a square.
    SimulinkBlockDefinition::new("Is Triangular", CAT)
        .with_aliases(&["IsTriangular"])
        .with_description("Test whether a matrix is triangular")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Mode", "Upper")])
        .with_static_renderer(renderers::static_is_triangular),
    SimulinkBlockDefinition::new("Is Symmetric", CAT)
        .with_aliases(&["IsSymmetric"])
        .with_description("Test whether a matrix is symmetric")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Mode", "Symmetric")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(
            labels::is_symmetric_mode,
        )),
    SimulinkBlockDefinition::new("Is Hermitian", CAT)
        .with_aliases(&["IsHermitian"])
        .with_description("Test whether a matrix is Hermitian")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Mode", "Hermitian")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(
            labels::is_hermitian_mode,
        )),
    SimulinkBlockDefinition::new("Cross Product", CAT)
        .with_description("Cross product of two vectors")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_block_label(BlockLabelPolicy::Fixed("cross")),
    SimulinkBlockDefinition::new("Matrix Multiply", CAT)
        .with_aliases(&["MatrixMultiply"])
        .with_description("Matrix multiplication")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_block_label(BlockLabelPolicy::Fixed("Matrix\nMultiply")),
    // A dotted matrix with one element highlighted by a selection box.
    SimulinkBlockDefinition::new("Submatrix", CAT)
        .with_description("Select a submatrix")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "p 0.18,0.10 0.10,0.10 0.10,0.90 0.18,0.90;",
            "p 0.82,0.10 0.90,0.10 0.90,0.90 0.82,0.90;",
            "d 0.26,0.26 0.045; d 0.50,0.26 0.045; d 0.74,0.26 0.045;",
            "d 0.26,0.50 0.045; d 0.50,0.50 0.045; d 0.74,0.50 0.045;",
            "d 0.26,0.74 0.045; d 0.50,0.74 0.045; d 0.74,0.74 0.045;",
            "r 0.38,0.14 0.86,0.62"
        ))),
    SimulinkBlockDefinition::new("Transpose", CAT)
        .with_description("Transpose a matrix")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("sup:A^T")),
    SimulinkBlockDefinition::new("Hermitian Transpose", CAT)
        .with_description("Complex-conjugate (Hermitian) transpose")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("sup:A^H")),
    // Simulink writes the operation the block performs: `AᴴA`.
    SimulinkBlockDefinition::new("Matrix Square", CAT)
        .with_aliases(&["Square"])
        .with_description("Square a matrix (A*A)")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(math("sup:A^H A")),
    SimulinkBlockDefinition::new("Permute Matrix", CAT)
        .with_aliases(&["Permute Columns", "PermuteMatrix", "PermuteColumns"])
        .with_description("Permute rows or columns of a matrix")
        .with_ports(IOPorts::Fixed(2), IOPorts::Fixed(1))
        .with_block_label(BlockLabelPolicy::Fixed("permute")),
    // Matrix in, diagonal out: a square with its diagonal drawn beside a
    // labelled arrow (`A ⇒ D` for extract, `D ⇒ A` for create).
    SimulinkBlockDefinition::new("Extract Diagonal", CAT)
        .with_aliases(&["ExtractDiag"])
        .with_description("Extract the main diagonal of a matrix")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "t 0.06,0.50,0.30 A;",
            "r 0.16,0.22 0.44,0.78;",
            "p 0.50,0.50 0.64,0.50; p 0.58,0.42 0.64,0.50 0.58,0.58;",
            "p 0.70,0.22 0.88,0.78; t 0.96,0.50,0.30 D"
        ))),
    SimulinkBlockDefinition::new("Create Diagonal Matrix", CAT)
        .with_aliases(&["DiagonalMatrix"])
        .with_description("Create a diagonal matrix from a vector")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "t 0.05,0.50,0.30 D;",
            "p 0.14,0.22 0.32,0.78;",
            "p 0.40,0.50 0.54,0.50; p 0.48,0.42 0.54,0.50 0.48,0.58;",
            "r 0.60,0.22 0.88,0.78; t 0.96,0.50,0.30 A"
        ))),
    // A scalar (single dot) fanning out into a full matrix of elements.
    SimulinkBlockDefinition::new("Expand Scalar", CAT)
        .with_aliases(&["ExpandScalar"])
        .with_description("Expand a scalar to a matrix")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "d 0.06,0.50 0.05; p 0.11,0.50 0.44,0.50;",
            "p 0.38,0.42 0.44,0.50 0.38,0.58;",
            dot_matrix!()
        ))),
    SimulinkBlockDefinition::new("Matrix Concatenate", CAT)
        .with_description("Concatenate matrices")
        .with_ports(IOPorts::Variable(2), IOPorts::Fixed(1))
        .with_metadata_keys(&[
            MetadataKey::with_default("Mode", "Multidimensional array"),
            MetadataKey::with_default("ConcatenateDimension", "2"),
        ])
        .with_static_renderer(renderers::static_concatenate),
];
