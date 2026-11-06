//! Source blocks: constants, waveforms, signal generators and readers.
//!
//! Part of the block catalog: one file per Simulink library group.

#![cfg(feature = "egui")]

use super::prelude::{NOISE_TRACE, SAWTOOTH, STAIRCASE, icon, plot};
use crate::simulink_libraries::labels;
use crate::simulink_libraries::types::{
    BlockLabelPolicy, IOPorts, MetadataKey, SimulinkBlockDefinition,
};

#[rustfmt::skip]
pub static BLOCKS: &[SimulinkBlockDefinition] = &[
    // ── Sources / sinks ────────────────────────────────────────────────
    SimulinkBlockDefinition::new("Constant", "Sources")
        .with_description("Output a constant value")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_metadata_keys(&[MetadataKey::with_default("Value", "1")])
        .with_block_label(BlockLabelPolicy::MetadataDependent(labels::constant_value)),
    SimulinkBlockDefinition::new("DiscretePulseGenerator", "Sources").with_aliases(&["PulseGenerator"])
        .with_aliases(&["PulseGenerator", "PulseGenerator"])
        .with_description("Generate discrete square-pulse signal")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "p 0.06,0.82 0.20,0.82 0.20,0.20 0.38,0.20 0.38,0.82 0.56,0.82",
            " 0.56,0.20 0.74,0.20 0.74,0.82 0.94,0.82"
        ))),
    SimulinkBlockDefinition::new("Sin", "Sources")
        .with_aliases(&["Sine Wave", "SineWave"])
        .with_description("Generate sine wave using internal time source")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "a 0.06,0.50 0.94,0.50;",
            "p 0.08,0.50 0.19,0.20 0.30,0.12 0.41,0.20 0.52,0.50",
            " 0.63,0.80 0.74,0.88 0.85,0.80 0.94,0.56"
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
    SimulinkBlockDefinition::new("Chirp", "Sources").with_aliases(&["ChirpSignal", "Chirp Signal"])
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
    SimulinkBlockDefinition::new("Repeating", "Sources").with_aliases(&["RepeatingSequence", "RepeatingSequenceRamp"])
        .with_aliases(&["RepeatingSequence", "RepeatingSequenceRamp", "RepeatingSequence", "RepeatingSequenceRamp"])
        .with_description("Generate repeating arbitrary signal")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(SAWTOOTH)),
    SimulinkBlockDefinition::new("RepeatingInterp", "Sources").with_aliases(&["RepeatingSequenceInterpolated"])
        .with_aliases(&["RepeatingSequenceInterpolated", "RepeatingSequenceInterpolated"])
        .with_description("Repeating sequence with interpolation")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(
            "p 0.06,0.20 0.22,0.62 0.34,0.30 0.50,0.78 0.68,0.86 0.94,0.80"
        )),
    SimulinkBlockDefinition::new("RepeatingStair", "Sources").with_aliases(&["RepeatingSequenceStair"])
        .with_aliases(&["RepeatingSequenceStair", "RepeatingSequenceStair"])
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
    SimulinkBlockDefinition::new("Video", "Sources")
        .with_description("Video/media input source")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(icon("\u{25B6}")),

    SimulinkBlockDefinition::new("FromSpreadsheet", "Sources")
        .with_aliases(&["From Spreadsheet"])
        .with_description("Load data from a spreadsheet file")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(icon("\u{1F4CA}")),
    SimulinkBlockDefinition::new("EnumeratedConstant", "Sources")
        .with_aliases(&["Enumerated Constant", "EnumeratedConst"])
        .with_description("Output an enumerated constant")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(icon("enum")),

    SimulinkBlockDefinition::new("FromFile", "Sources")
        .with_aliases(&["From File"])
        .with_description("Read data from a MAT file")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(icon("\u{2192}file")),
    SimulinkBlockDefinition::new("FromWorkspace", "Sources")
        .with_aliases(&["From Workspace"])
        .with_description("Load data from a workspace variable")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(icon("\u{2192}ws")),

    SimulinkBlockDefinition::new("SignalBuilder", "Sources")
        .with_aliases(&["Signal Builder"])
        .with_description("Interactive waveform builder")
        .with_ports(IOPorts::None, IOPorts::Fixed(1))
        .with_icon(plot(concat!(
            "r 0.10,0.15 0.80,0.70; p 0.16,0.75 0.30,0.40 0.50,0.65 0.70,0.30 0.84,0.55"
        ))),
    SimulinkBlockDefinition::new("TriggeredFromWorkspace", "Sources")
        .with_description("Workspace source gated by a trigger")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{2192}ws")),
    SimulinkBlockDefinition::new("TriggeredReadFromFile", "Sources")
        .with_description("File source gated by a trigger")
        .with_ports(IOPorts::Fixed(1), IOPorts::Fixed(1))
        .with_icon(icon("\u{2192}file")),
];
