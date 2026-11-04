//! Feature-independent structural facts about Simulink block types.
//!
//! This is the core (non-`egui`) counterpart to the rich rendering definitions
//! in [`super::libraries`]: the parser, the model and the signal resolver need
//! to know *what a block does* without pulling in a painter.  Every such fact
//! is declared here, so the code outside `simulink_libraries` never has to name
//! a block type.
//!
//! Adding behaviour for a new block type means extending the tables below – no
//! `block.block_type == "…"` anywhere else.

use crate::model::Block;

/// What a block does to the signals that pass through it.
///
/// The signal resolver ([`crate::connection_targets`]) dispatches on this role
/// instead of on the block type, so the type → behaviour mapping stays in the
/// catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum SignalRole {
    /// Nothing special: the block's outputs are its own.
    #[default]
    Plain,
    /// Bundles its inputs into a bus (`BusCreator`).
    BusCreator,
    /// Picks elements out of a bus (`BusSelector`).
    BusSelector,
    /// Replaces elements of a bus (`BusAssignment`).
    BusAssignment,
    /// Bundles its inputs into a vector (`Mux`).
    Mux,
    /// Splits a vector into its elements (`Demux`).
    Demux,
    /// The inside end of a parent system's input port.
    BoundaryInput,
    /// The inside end of a parent system's output port.
    BoundaryOutput,
    /// Holds a child system whose boundaries continue the signal.
    Container,
    /// Receives the signal of the matching [`SignalRole::Goto`] tag.
    From,
    /// Sends its input to the matching [`SignalRole::From`] tags.
    Goto,
    /// Enable/trigger/reset port: fed by the parent's control signal.
    ControlPort,
    /// Passes the active variant's input through (`VariantStart`,
    /// `VariantSink`).
    VariantSelect,
    /// Emits the active variant's branch (`VariantEnd`, `VariantSource`).
    VariantMerge,
}

impl SignalRole {
    /// Whether a line leaving this block carries the line's own metadata
    /// (signal name, test point) on top of the propagated targets.
    pub const fn propagates_local_metadata(self) -> bool {
        !matches!(self, SignalRole::Plain | SignalRole::Goto)
    }

    /// Whether a line *ending* at this block leaves the current system, so
    /// metadata travelling back upstream crosses a system boundary.
    pub const fn crosses_system_boundary(self) -> bool {
        matches!(self, SignalRole::Container | SignalRole::BoundaryOutput)
    }
}

/// The kind of source code a block carries, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodeKind {
    /// MATLAB code (`MATLAB Function`, `MATLABSystem`, `MATLABFcn`).
    Matlab,
    /// A single-line expression (`Fcn`).
    Expression,
    /// C code (`CFunction`).
    C,
}

/// How a block presents live simulation data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LiveRole {
    /// Prints the current value as text (`Display`).
    Value,
    /// Plots the signal over time (`Scope`, `DashboardScope`).
    Trace,
    /// The user edits the value on the canvas (`Constant`).
    Input,
    /// The user flips the block by clicking it (`ManualSwitch`).
    Toggle,
}

/// The structural facts about one block type.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlockTraits {
    /// What the block does to signals passing through it.
    pub signal_role: SignalRole,
    /// Whether the block owns a child system the user can navigate into.
    pub container: bool,
    /// Whether paths below this block are matched by prefix rather than
    /// exactly (library references resolve into a file of their own).
    pub path_prefix_matched: bool,
    /// The source code the block carries, if any.
    pub code: Option<CodeKind>,
    /// Whether the block's MATLAB code lives in a Stateflow chart (the
    /// `MATLAB Function` block, which the editors open as a script).
    pub stateflow_backed: bool,
    /// Whether creating the block gives it an empty child system.
    pub owns_child_system: bool,
    /// How the block shows live data, if at all.
    pub live: Option<LiveRole>,
    /// Whether the block takes its live data from the signal line reaching it
    /// rather than from a dashboard binding.
    pub reads_signal_line: bool,
    /// Value assumed when the model omits the block's `Value` property.
    pub implicit_value: Option<&'static str>,
    /// Port counts a dashboard block gets when the model states none.
    pub dashboard_ports: Option<(u32, u32)>,
}

impl BlockTraits {
    const PLAIN: Self = Self {
        signal_role: SignalRole::Plain,
        container: false,
        path_prefix_matched: false,
        code: None,
        stateflow_backed: false,
        owns_child_system: false,
        live: None,
        reads_signal_line: false,
        implicit_value: None,
        dashboard_ports: None,
    };

    const fn signal(role: SignalRole) -> Self {
        Self {
            signal_role: role,
            ..Self::PLAIN
        }
    }
}

/// The structural facts about `block_type`.
///
/// Unknown types are [`BlockTraits::PLAIN`]: a block with no special
/// behaviour, which is what the great majority of Simulink blocks are.
pub fn block_traits(block_type: &str) -> BlockTraits {
    match block_type {
        "BusCreator" => BlockTraits::signal(SignalRole::BusCreator),
        "BusSelector" => BlockTraits::signal(SignalRole::BusSelector),
        "BusAssignment" => BlockTraits::signal(SignalRole::BusAssignment),
        "Mux" => BlockTraits::signal(SignalRole::Mux),
        "Demux" => BlockTraits::signal(SignalRole::Demux),
        "Inport" | "InportShadow" => BlockTraits::signal(SignalRole::BoundaryInput),
        "Outport" => BlockTraits::signal(SignalRole::BoundaryOutput),
        "From" => BlockTraits::signal(SignalRole::From),
        "Goto" => BlockTraits::signal(SignalRole::Goto),
        "EnablePort" | "TriggerPort" | "ResetPort" => BlockTraits::signal(SignalRole::ControlPort),
        "VariantStart" | "VariantSink" => BlockTraits::signal(SignalRole::VariantSelect),
        "VariantEnd" | "VariantSource" => BlockTraits::signal(SignalRole::VariantMerge),
        "SubSystem" | "AtomicSubSystem" | "EnabledSubSystem" | "TriggeredSubSystem"
        | "ForEachSubSystem" | "ForIterator" | "WhileIterator" | "MaskedSubSystem"
        | "ConfigSubSystem" | "VariantSubSystem" => BlockTraits {
            signal_role: SignalRole::Container,
            container: true,
            owns_child_system: true,
            ..BlockTraits::PLAIN
        },
        "Reference" => BlockTraits {
            signal_role: SignalRole::Container,
            container: true,
            path_prefix_matched: true,
            ..BlockTraits::PLAIN
        },
        "MATLAB Function" => BlockTraits {
            code: Some(CodeKind::Matlab),
            stateflow_backed: true,
            ..BlockTraits::PLAIN
        },
        "MATLABSystem" | "MATLABFcn" => BlockTraits {
            code: Some(CodeKind::Matlab),
            ..BlockTraits::PLAIN
        },
        "Fcn" => BlockTraits {
            code: Some(CodeKind::Expression),
            ..BlockTraits::PLAIN
        },
        "CFunction" => BlockTraits {
            code: Some(CodeKind::C),
            ..BlockTraits::PLAIN
        },
        "Constant" => BlockTraits {
            live: Some(LiveRole::Input),
            implicit_value: Some("1"),
            ..BlockTraits::PLAIN
        },
        "ManualSwitch" => BlockTraits {
            live: Some(LiveRole::Toggle),
            ..BlockTraits::PLAIN
        },
        "Display" => BlockTraits {
            live: Some(LiveRole::Value),
            reads_signal_line: true,
            dashboard_ports: Some((1, 0)),
            ..BlockTraits::PLAIN
        },
        "Scope" => BlockTraits {
            live: Some(LiveRole::Trace),
            reads_signal_line: true,
            dashboard_ports: Some((0, 0)),
            ..BlockTraits::PLAIN
        },
        "DashboardScope" => BlockTraits {
            live: Some(LiveRole::Trace),
            dashboard_ports: Some((0, 0)),
            ..BlockTraits::PLAIN
        },
        // ── subsystem containers (deeper Ports & Subsystems coverage) ──
        "EnabledAndTriggeredSubsystem" | "ForIteratorSubsystem"
        | "FunctionCallSubsystem" | "ResettableSubsystem"
        | "SwitchCaseActionSubsystem" | "InitializeFunction" | "ResetFunction"
        | "TerminateFunction" | "SimulinkFunction" => BlockTraits {
            signal_role: SignalRole::Container,
            container: true,
            owns_child_system: true,
            ..BlockTraits::PLAIN
        },
        "SwitchCase" | "ElseIf" | "Else" => BlockTraits {
            signal_role: SignalRole::ControlPort,
            ..BlockTraits::PLAIN
        },
        // ── sources ──
        "Ground" => BlockTraits {
            live: Some(LiveRole::Input),
            implicit_value: Some("0"),
            ..BlockTraits::PLAIN
        },
        "Clock" | "DigitalClock" | "Chirp" | "ChirpSignal" | "SignalBuilder"
        | "SignalGenerator" | "BandLimitedWhiteNoise" | "FromWorkspace"
        | "FromFile" | "FromSpreadsheet" | "EnumeratedConstant" | "Counter"
        | "CounterLimited" | "RepeatingSequenceInterpolated" | "DateTimeClock" => BlockTraits {
            live: Some(LiveRole::Input),
            ..BlockTraits::PLAIN
        },
        "Step" | "Ramp" | "PulseGenerator" | "SineWave" | "RandomNumber"
        | "UniformRandomNumber" | "RepeatingSequence" | "RepeatingSequenceStair"
        | "RepeatingSequenceRamp" => BlockTraits {
            live: Some(LiveRole::Input),
            ..BlockTraits::PLAIN
        },
        // ── sinks ──
        "Terminator" | "ToWorkspace" | "ToFile" | "XYGraph" | "FloatingScope"
        | "Record" | "Stop" | "Assertion" => BlockTraits {
            live: Some(LiveRole::Value),
            reads_signal_line: true,
            ..BlockTraits::PLAIN
        },
        // ── dashboard widgets (input / output roles) ──
        "ToggleSwitch" | "ToggleSwitchBlock" | "ToggleSwitchCustom"
        | "PushButton" | "PushButtonCustom" | "Checkbox" | "CheckboxCustom"
        | "RockerSwitch" | "RockerSwitchCustom" | "RotarySwitch"
        | "RotarySwitchCustom" | "RadioButton" | "RadioButtonCustom"
        | "RadioButtonGroup" | "ComboBox" | "SliderSwitch" | "SliderSwitchCustom"
        | "CallbackButton" => BlockTraits {
            live: Some(LiveRole::Toggle),
            dashboard_ports: Some((0, 0)),
            ..BlockTraits::PLAIN
        },
        "Knob" | "KnobCustom" | "Slider" | "SliderCustom" | "Edit"
        | "EditCustom" | "TextArea" | "SliderGain" => BlockTraits {
            live: Some(LiveRole::Input),
            dashboard_ports: Some((0, 0)),
            ..BlockTraits::PLAIN
        },
        "Lamp" | "LampCustom" | "Gauge" | "GaugeCustom" | "CircularGauge"
        | "CircularGaugeCustom" | "LinearGauge" | "LinearGaugeCustom"
        | "HalfGauge" | "HalfGaugeCustom" | "QuarterGauge" | "QuarterGaugeCustom"
        | "Stopwatch" | "DisplayCustom" | "DashboardScopeCustom" => BlockTraits {
            live: Some(LiveRole::Value),
            dashboard_ports: Some((0, 0)),
            ..BlockTraits::PLAIN
        },
        "Playback" | "RealTimeDashboard" => BlockTraits {
            live: Some(LiveRole::Trace),
            dashboard_ports: Some((0, 0)),
            ..BlockTraits::PLAIN
        },
        // ── pass-through conversions (resolver-transparent) ──
        "DataTypeConversion" | "DataTypeConversionInherited" | "UnitConversion"
        | "InitialCondition" | "SignalSpecification" | "WeightedSampleTime"
        | "IsFixed" | "IsNonBus" | "Probe" | "Width" | "GotoTagVisibility"
        | "DataTypeDuplicate" | "DataTypePropagation" | "DataTypeScalingStrip"
        | "VectorToBus" | "BusToVector" | "BusElement" | "BusElementPort"
        | "SignalConversion" | "UnitSystemConfig" | "EnvironmentController" => {
            BlockTraits::PLAIN
        }
        _ => BlockTraits::PLAIN,
    }
}

/// The facts about this block instance, including the ones that depend on its
/// properties rather than only on its type.
pub fn traits_of(block: &Block) -> BlockTraits {
    let mut traits = block_traits(&block.block_type);
    if block.is_matlab_function {
        traits.code = Some(CodeKind::Matlab);
    }
    traits
}

/// Whether the block owns a child system the user can navigate into.
pub fn is_container(block: &Block) -> bool {
    block_traits(&block.block_type).container
}

/// Whether the block is a MATLAB Function block: either the dedicated block
/// type or a subsystem flagged as one by its `SFBlockType`.
pub fn is_matlab_function(block: &Block) -> bool {
    block.is_matlab_function || block_traits(&block.block_type).stateflow_backed
}

/// Whether the block links into a library whose contents were read.
///
/// Such a link defines its own ports, port names and interior through the
/// blocks it links to, so catalog guesses made from its *name* (a library
/// block may well be called `Logic` or `Gain`) must not override them.
pub fn links_to_read_library_contents(block: &Block) -> bool {
    block.library_block_path.is_some()
        && block
            .subsystem
            .as_ref()
            .is_some_and(|contents| !contents.blocks.is_empty())
}

/// Whether a freshly created block of this type starts with a child system.
pub fn owns_child_system(block_type: &str) -> bool {
    block_traits(block_type).owns_child_system
}

/// Whether the block owns a child system the user can descend into.
///
/// A MATLAB Function block is technically a subsystem wrapping a Stateflow
/// S-function, but Simulink shows its MATLAB source rather than that wiring,
/// so it is not navigable.
pub fn is_navigable_subsystem(block: &Block) -> bool {
    is_container(block)
        && !is_matlab_function(block)
        && block
            .subsystem
            .as_ref()
            .is_some_and(|sub| sub.chart.is_none())
}

/// Whether the block carries source code the user can open in an editor.
pub fn carries_code(block: &Block) -> bool {
    traits_of(block).code.is_some()
}

/// Whether the block carries C code (`CFunction`).
pub fn carries_c_code(block: &Block) -> bool {
    traits_of(block).code == Some(CodeKind::C)
}

/// How the block presents live simulation data, if at all.
pub fn live_role(block_type: &str) -> Option<LiveRole> {
    block_traits(block_type).live
}

/// Whether the block plots live data over time rather than printing it.
pub fn shows_live_trace(block_type: &str) -> bool {
    live_role(block_type) == Some(LiveRole::Trace)
}

/// Whether the block prints a live value as text.
pub fn shows_live_value(block_type: &str) -> bool {
    live_role(block_type) == Some(LiveRole::Value)
}

/// Whether the block shows its value as centred text on the canvas, either
/// live (`Display`) or as the value the user typed (`Constant`).
pub fn shows_value_text(block_type: &str) -> bool {
    matches!(
        live_role(block_type),
        Some(LiveRole::Value | LiveRole::Input)
    )
}

/// Whether the user can edit this block's value on the canvas.
pub fn has_editable_value(block_type: &str) -> bool {
    live_role(block_type) == Some(LiveRole::Input)
}

/// Whether clicking the block flips it (`ManualSwitch`).
pub fn is_click_toggle(block_type: &str) -> bool {
    live_role(block_type) == Some(LiveRole::Toggle)
}

/// Whether the block reads its live data from the signal line reaching it.
pub fn reads_signal_line(block_type: &str) -> bool {
    block_traits(block_type).reads_signal_line
}

/// Whether the block is a link into another library or model file.
pub fn is_library_reference(block_type: &str) -> bool {
    block_traits(block_type).path_prefix_matched
}

/// The value the canvas shows when the model states none.
pub fn implicit_value(block_type: &str) -> Option<&'static str> {
    block_traits(block_type).implicit_value
}

/// Catalog key selected by a block's own properties rather than by its type
/// or its library path, for the block types Simulink draws differently
/// depending on how they are configured.
pub fn property_variant_key(block: &Block) -> Option<&'static str> {
    match block.block_type.as_str() {
        "Product"
            if block
                .properties
                .get("Multiplication")
                .map(|v| v.trim())
                .is_some_and(|v| v == "Matrix(*)") =>
        {
            Some("matrix multiply")
        }
        _ => None,
    }
}

/// Default port counts for a dashboard block the model gives no ports for.
pub fn dashboard_port_counts(block_type: &str) -> (u32, u32) {
    block_traits(block_type).dashboard_ports.unwrap_or((0, 0))
}

/// The block types the SLX scanner recognises, used to report the ones this
/// crate does not model yet.
pub const KNOWN_BLOCK_TYPES: &[&str] = &[    "Abs",
    "ActionPort",
    "AsciiToString",
    "Assertion",
    "Assignment",
    "Backlash",
    "BandLimitedWhiteNoise",
    "Bias",
    "BitClear",
    "BitSet",
    "BitwiseOperator",
    "BlockSupportTable",
    "BusAssignment",
    "BusCreator",
    "BusElement",
    "BusSelector",
    "BusToVector",
    "CICFilter",
    "CallbackButton",
    "CheckDiscreteGradient",
    "CheckDynamicGap",
    "CheckDynamicLowerBound",
    "CheckDynamicRange",
    "CheckDynamicUpperBound",
    "CheckInputResolution",
    "CheckStaticGap",
    "CheckStaticLowerBound",
    "CheckStaticRange",
    "CheckStaticUpperBound",
    "Checkbox",
    "CheckboxCustom",
    "Chirp",
    "ChirpSignal",
    "CircularGauge",
    "CircularGaugeCustom",
    "Clock",
    "CombinatorialLogic",
    "ComboBox",
    "CompareStringConstants",
    "CompareStrings",
    "CompareToConstant",
    "CompareToZero",
    "ComplexToMagnitudeAngle",
    "ComplexToRealImag",
    "ComposeString",
    "Constant",
    "Continuous-VCO",
    "Counter",
    "CounterLimited",
    "CrossProduct",
    "DashboardScopeCustom",
    "DataStoreMemory",
    "DataStoreRead",
    "DataStoreWrite",
    "DataTypeConversion",
    "DataTypeConversionInherited",
    "DataTypeDuplicate",
    "DataTypePropagation",
    "DataTypeScalingStrip",
    "DateTimeClock",
    "DateTimeDataTypeConverter",
    "DateTimeElapsedTime",
    "DateTimeExtractor",
    "DateTimeInterval",
    "DeadZone",
    "DeadZoneDynamic",
    "DecrementRealWorld",
    "DecrementStoredInteger",
    "Delay",
    "Demux",
    "Derivative",
    "DetectChange",
    "DetectDecrease",
    "DetectFallNegative",
    "DetectFallNonpositive",
    "DetectIncrease",
    "DetectRiseNonnegative",
    "DetectRisePositive",
    "Diagonal",
    "Difference",
    "DigitalClock",
    "DirectLookup",
    "DiscreteDerivative",
    "DiscreteFIRFilter",
    "DiscreteFilter",
    "DiscreteIntegrator",
    "DiscretePIDController",
    "DiscretePIDController2DOF",
    "DiscreteStateSpace",
    "DiscreteTransferFcn",
    "DiscreteZeroPole",
    "Display",
    "DisplayCustom",
    "DocBlock",
    "DotProduct",
    "Edit",
    "EditCustom",
    "ElapsedTime",
    "ElseIf",
    "EnabledAndTriggeredSubsystem",
    "EnabledSubsystem",
    "EntitySink",
    "EntitySource",
    "EnumToString",
    "EnumeratedConst",
    "EnumeratedConstant",
    "EnvironmentController",
    "EventListener",
    "ExtractBits",
    "Fcn",
    "FirstOrderHold",
    "FirstOrderTransferFunction",
    "FloatingScope",
    "ForEach",
    "ForEachSubsystem",
    "ForIterator",
    "ForIteratorSubsystem",
    "From",
    "FromFile",
    "FromSpreadsheet",
    "FromWorkspace",
    "FunctionCallFeedbackLatch",
    "FunctionCallGenerator",
    "FunctionCallSplit",
    "FunctionCallSubsystem",
    "Gain",
    "Gauge",
    "GaugeCustom",
    "Goto",
    "GotoTagVisibility",
    "Ground",
    "HalfGauge",
    "HalfGaugeCustom",
    "If",
    "IfActionSubsystem",
    "IncrementDecrementToValue",
    "IncrementDecrementToZero",
    "IncrementRealWorld",
    "IncrementStoredInteger",
    "InitialCondition",
    "InitializeFunction",
    "Inport",
    "InportShadow",
    "Integrator",
    "IntegratorLimited",
    "IntegratorSecondOrder",
    "IntegratorSecondOrderLimited",
    "InterpretedMATLABFunction",
    "IntervalTest",
    "IntervalTestDynamic",
    "IsFixed",
    "IsNonBus",
    "IsString",
    "Knob",
    "KnobCustom",
    "Lamp",
    "LampCustom",
    "Level2MATLABSFunction",
    "LinearGauge",
    "LinearGaugeCustom",
    "LogicalOperator",
    "Lookup",
    "Lookup1D",
    "Lookup2D",
    "LookupDynamic",
    "Lookup_n-D",
    "MATLABFcn",
    "Math",
    "MatrixConcatenate",
    "MatrixGain",
    "Max",
    "MaxMin",
    "Merge",
    "MessageDisplay",
    "MessageInput",
    "MessageMerge",
    "MessageOutput",
    "Min",
    "MinMax",
    "MinMaxRunningResettable",
    "ModelInfo",
    "ModelReference",
    "MultiPortSwitch",
    "Mux",
    "Outport",
    "OutportShadow",
    "PIDController",
    "PIDController2DOF",
    "PermuteDimensions",
    "Playback",
    "Polynomial",
    "PreLookup",
    "Probe",
    "Product",
    "ProductOfElements",
    "PulseGenerator",
    "PushButton",
    "PushButtonCustom",
    "Python",
    "Quantizer",
    "RadioButton",
    "RadioButtonCustom",
    "RadioButtonGroup",
    "Ramp",
    "RandomNumber",
    "RateLimiter",
    "RateLimiterDynamic",
    "RateTransition",
    "RealTimeDashboard",
    "ReciprocalSqrt",
    "RelationalOperator",
    "Relay",
    "RepeatingSequence",
    "RepeatingSequenceInterpolated",
    "RepeatingSequenceRamp",
    "RepeatingSequenceStair",
    "ResetFunction",
    "ResettableDelay",
    "ResettableSubsystem",
    "Reshape",
    "RockerSwitch",
    "RockerSwitchCustom",
    "RotarySwitch",
    "RotarySwitchCustom",
    "Rounding",
    "S-Function",
    "S-FunctionBuilder",
    "Saturate",
    "SaturateDynamic",
    "ScanString",
    "Scope",
    "Selector",
    "SelectorPort",
    "SequenceViewer",
    "ShiftArithmetic",
    "Sign",
    "SignalBuilder",
    "SignalConversion",
    "SignalGenerator",
    "SignalSpecification",
    "SignedSqrt",
    "SimulinkConfig",
    "SimulinkFunction",
    "SineWave",
    "Slider",
    "SliderCustom",
    "SliderGain",
    "SliderSwitch",
    "SliderSwitchCustom",
    "SplitString",
    "Sqrt",
    "Sqrt3D",
    "StateSpace",
    "Step",
    "Stop",
    "Stopwatch",
    "StringConcatenate",
    "StringConstant",
    "StringEndsWith",
    "StringFind",
    "StringJoin",
    "StringLength",
    "StringReplace",
    "StringToASCII",
    "StringToDouble",
    "StringToEnum",
    "StringToSingle",
    "SubSystem",
    "Submatrix",
    "Substring",
    "Sum",
    "SumOfElements",
    "Switch",
    "SwitchCase",
    "SwitchCaseActionSubsystem",
    "TappedDelay",
    "TerminateFunction",
    "TextArea",
    "ToFile",
    "ToWorkspace",
    "ToggleSwitch",
    "ToggleSwitchBlock",
    "ToggleSwitchCustom",
    "TransferDelay",
    "TransferFcn",
    "TransferFcnDirectFormII",
    "TransferFcnDirectFormIITimeVarying",
    "TransferFcnFirstOrder",
    "TransferFcnLeadLag",
    "TransferFcnRealZero",
    "TransportDelay",
    "Transpose",
    "Trig",
    "TriggeredDelay",
    "TriggeredFromWorkspace",
    "TriggeredReadFromFile",
    "TriggeredSampleAndHold",
    "TriggeredSubsystem",
    "TriggeredToWorkspace",
    "TriggeredWriteToFile",
    "UnaryMinus",
    "UniformRandomNumber",
    "UnitConversion",
    "UnitDelay",
    "UnitSystemConfig",
    "VariableIntegerDelay",
    "VariableTimeDelay",
    "VariantEnd",
    "VariantSink",
    "VariantSource",
    "VariantStart",
    "VectorConcatenate",
    "VectorToBus",
    "Video",
    "WhileIterator",
    "WhileSubsystem",
    "WrapToZero",
    "XYGraph",
    "ZeroOrderHold",
    "ZeroPole",
];

/// Whether the SLX scanner models `block_type`.
pub fn is_known_block_type(block_type: &str) -> bool {
    KNOWN_BLOCK_TYPES.binary_search(&block_type).is_ok()
}
