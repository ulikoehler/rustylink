//! Block catalog with 750+ Simulink-inspired block types organized by category.
//!
//! The catalog provides a searchable, categorized list of block types that can
//! be added to a model. Each entry specifies the block type name, a human-readable
//! display name, the category it belongs to, default port counts, and an optional
//! icon hint.
//!
//! # Usage
//!
//! ```rust,ignore
//! use rustylink::editor::block_catalog::{get_block_catalog, BlockCatalogEntry};
//!
//! let catalog = get_block_catalog();
//! // Search for blocks matching "gain"
//! let matches: Vec<&BlockCatalogEntry> = catalog
//!     .iter()
//!     .filter(|e| e.matches_query("gain"))
//!     .collect();
//! ```

#![cfg(feature = "egui")]

use once_cell::sync::Lazy;

/// A single entry in the block catalog.
#[derive(Debug, Clone)]
pub struct BlockCatalogEntry {
    /// Internal block type name (e.g., `"Gain"`, `"SubSystem"`).
    pub block_type: String,
    /// Human-readable display name shown in the browser.
    pub display_name: String,
    /// Category path (e.g., `"Math Operations"`, `"Signal Routing"`).
    pub category: String,
    /// Default number of input ports.
    pub default_inputs: u32,
    /// Default number of output ports.
    pub default_outputs: u32,
    /// Brief description of the block's function.
    pub description: String,
}

impl BlockCatalogEntry {
    /// Check if this entry matches a search query (case-insensitive substring match
    /// on block type, display name, category, or description).
    pub fn matches_query(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let q = query.to_lowercase();
        self.block_type.to_lowercase().contains(&q)
            || self.display_name.to_lowercase().contains(&q)
            || self.category.to_lowercase().contains(&q)
            || self.description.to_lowercase().contains(&q)
    }
}

/// A category of blocks in the catalog, with a name and list of entries.
#[derive(Debug, Clone)]
pub struct BlockCatalogCategory {
    /// Category display name.
    pub name: String,
    /// Entries belonging to this category.
    pub entries: Vec<BlockCatalogEntry>,
}

/// Helper to create a catalog entry concisely.
fn entry(
    block_type: &str,
    display_name: &str,
    category: &str,
    inputs: u32,
    outputs: u32,
    description: &str,
) -> BlockCatalogEntry {
    BlockCatalogEntry {
        block_type: block_type.to_string(),
        display_name: display_name.to_string(),
        category: category.to_string(),
        default_inputs: inputs,
        default_outputs: outputs,
        description: description.to_string(),
    }
}

/// Returns the complete block catalog with 750+ entries.
///
/// The catalog is lazily initialized on first access and cached for the
/// lifetime of the process.
pub fn get_block_catalog() -> &'static [BlockCatalogEntry] {
    static CATALOG: Lazy<Vec<BlockCatalogEntry>> = Lazy::new(build_catalog);
    &CATALOG
}

/// Returns the catalog organized by category.
pub fn get_block_catalog_by_category() -> &'static [BlockCatalogCategory] {
    static CATEGORIES: Lazy<Vec<BlockCatalogCategory>> = Lazy::new(|| {
        let catalog = get_block_catalog();
        let mut cat_map: indexmap::IndexMap<String, Vec<BlockCatalogEntry>> =
            indexmap::IndexMap::new();
        for e in catalog {
            cat_map
                .entry(e.category.clone())
                .or_default()
                .push(e.clone());
        }
        cat_map
            .into_iter()
            .map(|(name, entries)| BlockCatalogCategory { name, entries })
            .collect()
    });
    &CATEGORIES
}

fn build_catalog() -> Vec<BlockCatalogEntry> {
    let mut c = Vec::with_capacity(800);

    // ── Sources ──────────────────────────────────────────────────────────
    let cat = "Sources";
    c.push(entry("Constant", "Constant", cat, 0, 1, "Output a constant value"));
    c.push(entry("Ground", "Ground", cat, 0, 1, "Ground (zero) signal source"));
    c.push(entry("Inport", "Inport", cat, 0, 1, "External input port"));
    c.push(entry("FromWorkspace", "From Workspace", cat, 0, 1, "Load data from workspace variable"));
    c.push(entry("FromFile", "From File", cat, 0, 1, "Read data from MAT file"));
    c.push(entry("Clock", "Clock", cat, 0, 1, "Output simulation time"));
    c.push(entry("DigitalClock", "Digital Clock", cat, 0, 1, "Output simulation time at specified rate"));
    c.push(entry("Step", "Step", cat, 0, 1, "Step function input"));
    c.push(entry("Ramp", "Ramp", cat, 0, 1, "Ramp function input"));
    c.push(entry("SineWave", "Sine Wave", cat, 0, 1, "Generate sine wave signal"));
    c.push(entry("SignalGenerator", "Signal Generator", cat, 0, 1, "Generate various waveforms"));
    c.push(entry("Pulse", "Pulse Generator", cat, 0, 1, "Generate square pulse signal"));
    c.push(entry("Chirp", "Chirp Signal", cat, 0, 1, "Generate frequency-swept signal"));
    c.push(entry("RandomNumber", "Random Number", cat, 0, 1, "Generate random number using normal distribution"));
    c.push(entry("UniformRandom", "Uniform Random Number", cat, 0, 1, "Generate uniformly distributed random numbers"));
    c.push(entry("BandLimitedWhiteNoise", "Band-Limited White Noise", cat, 0, 1, "White noise with specified bandwidth"));
    c.push(entry("Repeating", "Repeating Sequence", cat, 0, 1, "Generate repeating arbitrary signal"));
    c.push(entry("RepeatingStair", "Repeating Sequence Stair", cat, 0, 1, "Generate repeating staircase signal"));
    c.push(entry("RepeatingInterp", "Repeating Sequence Interpolated", cat, 0, 1, "Repeating sequence with interpolation"));
    c.push(entry("Counter", "Counter Free-Running", cat, 0, 1, "Free running counter output"));
    c.push(entry("CounterLimited", "Counter Limited", cat, 0, 1, "Counter with configurable limit"));
    c.push(entry("EnumConstant", "Enumerated Constant", cat, 0, 1, "Output enumerated type constant"));
    c.push(entry("TimeTableLookup", "Time Table Lookup", cat, 0, 1, "Table-based time signal"));
    c.push(entry("SignalBuilderBlock", "Signal Builder", cat, 0, 1, "Build signals interactively"));
    c.push(entry("WaveformGenerator", "Waveform Generator", cat, 0, 1, "Generate standard waveforms"));
    c.push(entry("MultiPortSwGen", "Multi-port Waveform Generator", cat, 0, 1, "Multi-output waveform gen"));
    c.push(entry("Playback", "Playback", cat, 0, 1, "Play back recorded signal"));

    // ── Sinks ────────────────────────────────────────────────────────────
    let cat = "Sinks";
    c.push(entry("Outport", "Outport", cat, 1, 0, "External output port"));
    c.push(entry("Terminator", "Terminator", cat, 1, 0, "Terminate unconnected signal"));
    c.push(entry("Scope", "Scope", cat, 1, 0, "Display signal in scope window"));
    c.push(entry("FloatingScope", "Floating Scope", cat, 0, 0, "Free-floating scope"));
    c.push(entry("Display", "Display", cat, 1, 0, "Show signal value numerically"));
    c.push(entry("ToWorkspace", "To Workspace", cat, 1, 0, "Write data to workspace variable"));
    c.push(entry("ToFile", "To File", cat, 1, 0, "Write data to MAT file"));
    c.push(entry("XYGraph", "XY Graph", cat, 2, 0, "Display XY plot of two signals"));
    c.push(entry("Stop", "Stop Simulation", cat, 1, 0, "Stop simulation when input is nonzero"));
    c.push(entry("OutBus", "Out Bus Element", cat, 1, 0, "Output bus element"));
    c.push(entry("Record", "Record", cat, 1, 0, "Record signal data"));

    // ── Math Operations ─────────────────────────────────────────────────
    let cat = "Math Operations";
    c.push(entry("Sum", "Sum / Add", cat, 2, 1, "Add or subtract inputs"));
    c.push(entry("Add", "Add", cat, 2, 1, "Add inputs"));
    c.push(entry("Subtract", "Subtract", cat, 2, 1, "Subtract inputs"));
    c.push(entry("Product", "Product", cat, 2, 1, "Multiply or divide inputs"));
    c.push(entry("Gain", "Gain", cat, 1, 1, "Multiply input by constant gain"));
    c.push(entry("Abs", "Abs", cat, 1, 1, "Absolute value"));
    c.push(entry("Sign", "Sign", cat, 1, 1, "Signum function"));
    c.push(entry("Sqrt", "Sqrt", cat, 1, 1, "Square root"));
    c.push(entry("ReciprocalSqrt", "Reciprocal Sqrt", cat, 1, 1, "Reciprocal square root (1/sqrt)"));
    c.push(entry("MathFunction", "Math Function", cat, 1, 1, "Common math function (exp, log, pow, etc.)"));
    c.push(entry("TrigFunction", "Trigonometric Function", cat, 1, 1, "Trig functions (sin, cos, tan, etc.)"));
    c.push(entry("MinMax", "MinMax", cat, 2, 1, "Minimum or maximum of inputs"));
    c.push(entry("DotProduct", "Dot Product", cat, 2, 1, "Vector dot product"));
    c.push(entry("CrossProduct", "Cross Product", cat, 2, 1, "Vector cross product"));
    c.push(entry("Divide", "Divide", cat, 2, 1, "Element-wise division"));
    c.push(entry("Mod", "Mod (Remainder)", cat, 2, 1, "Modulus / remainder"));
    c.push(entry("Round", "Rounding Function", cat, 1, 1, "Round, floor, ceil, fix"));
    c.push(entry("Bias", "Bias", cat, 1, 1, "Add bias value to input"));
    c.push(entry("SliderGain", "Slider Gain", cat, 1, 1, "Variable gain via slider"));
    c.push(entry("Polynomial", "Polynomial", cat, 1, 1, "Evaluate polynomial coefficients"));
    c.push(entry("Magnitude-AngleToComplex", "Magnitude-Angle to Complex", cat, 2, 1, "Convert magnitude and angle to complex"));
    c.push(entry("RealImagToComplex", "Real-Imag to Complex", cat, 2, 1, "Convert real and imag to complex"));
    c.push(entry("ComplexToMagnitudeAngle", "Complex to Magnitude-Angle", cat, 1, 2, "Extract magnitude and angle from complex"));
    c.push(entry("ComplexToRealImag", "Complex to Real-Imag", cat, 1, 2, "Extract real and imag from complex"));
    c.push(entry("Reshape", "Reshape", cat, 1, 1, "Reshape signal dimensions"));
    c.push(entry("WeightedSum", "Weighted Sum", cat, 2, 1, "Weighted sum of inputs"));
    c.push(entry("SumOfElements", "Sum of Elements", cat, 1, 1, "Sum all elements of vector/matrix"));
    c.push(entry("ProductOfElements", "Product of Elements", cat, 1, 1, "Product of all elements"));
    c.push(entry("Norm", "Vector Norm", cat, 1, 1, "Compute vector norm"));
    c.push(entry("Unary Minus", "Unary Minus", cat, 1, 1, "Negate signal"));
    c.push(entry("Power", "Power", cat, 2, 1, "Raise to power"));
    c.push(entry("Exponential", "Exponential", cat, 1, 1, "e^x"));
    c.push(entry("NaturalLog", "Natural Log", cat, 1, 1, "ln(x)"));
    c.push(entry("Log10", "Log10", cat, 1, 1, "log10(x)"));
    c.push(entry("Log2", "Log2", cat, 1, 1, "log2(x)"));
    c.push(entry("Hypot", "Hypot", cat, 2, 1, "Hypotenuse sqrt(x^2+y^2)"));
    c.push(entry("MaxConstraint", "Max Constraint", cat, 2, 1, "Output max of inputs with constraint"));
    c.push(entry("MinConstraint", "Min Constraint", cat, 2, 1, "Output min of inputs with constraint"));

    // ── Logic and Bit Operations ────────────────────────────────────────
    let cat = "Logic and Bit Operations";
    c.push(entry("Logic", "Logical Operator", cat, 2, 1, "AND, OR, NAND, NOR, XOR, NOT"));
    c.push(entry("RelationalOperator", "Relational Operator", cat, 2, 1, "Compare: ==, ~=, <, <=, >, >="));
    c.push(entry("Compare", "Compare To Zero", cat, 1, 1, "Compare signal to zero"));
    c.push(entry("CompareToConstant", "Compare To Constant", cat, 1, 1, "Compare signal to constant"));
    c.push(entry("BitwiseOperator", "Bitwise Operator", cat, 1, 1, "AND, OR, XOR, NOT on bits"));
    c.push(entry("ShiftArithmetic", "Shift Arithmetic", cat, 1, 1, "Bitwise arithmetic shift"));
    c.push(entry("BitClear", "Bit Clear", cat, 1, 1, "Clear specified bit"));
    c.push(entry("BitSet", "Bit Set", cat, 1, 1, "Set specified bit"));
    c.push(entry("ExtractBits", "Extract Bits", cat, 1, 1, "Extract range of bits"));
    c.push(entry("IntervalTest", "Interval Test", cat, 1, 1, "Test if value is in interval"));
    c.push(entry("IntervalTestDynamic", "Interval Test Dynamic", cat, 3, 1, "Dynamic interval test"));
    c.push(entry("CombLogic", "Combinatorial Logic", cat, 1, 1, "Truth-table based logic"));
    c.push(entry("Detect", "Detect Change", cat, 1, 1, "Detect when signal changes"));
    c.push(entry("DetectDecrease", "Detect Decrease", cat, 1, 1, "Detect when signal decreases"));
    c.push(entry("DetectIncrease", "Detect Increase", cat, 1, 1, "Detect when signal increases"));
    c.push(entry("DetectRisePositive", "Detect Rise Positive", cat, 1, 1, "Detect rising edge"));
    c.push(entry("DetectRiseNonneg", "Detect Rise Nonnegative", cat, 1, 1, "Detect nonnegative rising edge"));
    c.push(entry("DetectFallNeg", "Detect Fall Negative", cat, 1, 1, "Detect falling edge"));
    c.push(entry("DetectFallNonpos", "Detect Fall Nonpositive", cat, 1, 1, "Detect nonpositive falling"));

    // ── Continuous ──────────────────────────────────────────────────────
    let cat = "Continuous";
    c.push(entry("Integrator", "Integrator", cat, 1, 1, "Continuous-time integrator"));
    c.push(entry("IntegratorLimited", "Integrator Limited", cat, 1, 1, "Integrator with saturation limits"));
    c.push(entry("IntegratorSecondOrder", "Integrator Second Order", cat, 1, 2, "Second-order integrator (x, dx/dt)"));
    c.push(entry("Derivative", "Derivative", cat, 1, 1, "Time derivative du/dt"));
    c.push(entry("TransferFcn", "Transfer Function", cat, 1, 1, "Transfer function (s-domain)"));
    c.push(entry("StateSpace", "State-Space", cat, 1, 1, "State-space model (A,B,C,D)"));
    c.push(entry("ZeroPole", "Zero-Pole", cat, 1, 1, "Zero-pole-gain transfer function"));
    c.push(entry("TransportDelay", "Transport Delay", cat, 1, 1, "Fixed-time transport delay"));
    c.push(entry("VariableTransportDelay", "Variable Transport Delay", cat, 2, 1, "Variable time delay"));
    c.push(entry("PadeApprox", "Pade Approximation", cat, 1, 1, "Pade approximation of time delay"));
    c.push(entry("PID", "PID Controller", cat, 1, 1, "Continuous PID controller"));
    c.push(entry("PIDAdvanced", "PID Controller (2DOF)", cat, 2, 1, "Two-degree-of-freedom PID"));
    c.push(entry("PIDDiscrete", "Discrete PID Controller", cat, 1, 1, "Discrete-time PID controller"));
    c.push(entry("PIDDiscreteAdvanced", "Discrete PID (2DOF)", cat, 2, 1, "Discrete-time 2DOF PID"));
    c.push(entry("Lead Lag", "Lead-Lag Compensator", cat, 1, 1, "Lead-lag compensator"));

    // ── Discrete ────────────────────────────────────────────────────────
    let cat = "Discrete";
    c.push(entry("UnitDelay", "Unit Delay", cat, 1, 1, "Delay signal by one sample period"));
    c.push(entry("Delay", "Delay", cat, 1, 1, "Delay by N samples"));
    c.push(entry("VariableIntegerDelay", "Variable Integer Delay", cat, 2, 1, "Variable integer sample delay"));
    c.push(entry("TappedDelay", "Tapped Delay", cat, 1, 1, "Multi-tap delay line"));
    c.push(entry("DiscreteIntegrator", "Discrete-Time Integrator", cat, 1, 1, "Discrete integration / accumulator"));
    c.push(entry("DiscreteDerivative", "Discrete Derivative", cat, 1, 1, "Discrete-time derivative"));
    c.push(entry("DiscreteTransferFcn", "Discrete Transfer Fcn", cat, 1, 1, "Discrete transfer function (z-domain)"));
    c.push(entry("DiscreteZeroPole", "Discrete Zero-Pole", cat, 1, 1, "Discrete zero-pole-gain"));
    c.push(entry("DiscreteStateSpace", "Discrete State-Space", cat, 1, 1, "Discrete state space model"));
    c.push(entry("DiscreteFilter", "Discrete Filter", cat, 1, 1, "IIR / FIR discrete filter"));
    c.push(entry("DiscreteFIR", "Discrete FIR Filter", cat, 1, 1, "Finite impulse response filter"));
    c.push(entry("DiscreteIIR", "Discrete IIR Filter", cat, 1, 1, "Infinite impulse response filter"));
    c.push(entry("ZeroOrderHold", "Zero-Order Hold", cat, 1, 1, "Zero-order hold (sample)"));
    c.push(entry("FirstOrderHold", "First-Order Hold", cat, 1, 1, "First-order hold interpolation"));
    c.push(entry("RateTransition", "Rate Transition", cat, 1, 1, "Handle multi-rate transitions"));
    c.push(entry("Downsample", "Downsample", cat, 1, 1, "Reduce sample rate by integer factor"));
    c.push(entry("Upsample", "Upsample", cat, 1, 1, "Increase sample rate by integer factor"));
    c.push(entry("Memory", "Memory", cat, 1, 1, "Output previous sample value"));
    c.push(entry("ResettableDelay", "Resettable Delay", cat, 2, 1, "Unit delay with external reset"));
    c.push(entry("ZeroOrderHoldExact", "Zero-Order Hold (Exact)", cat, 1, 1, "Exact ZOH"));
    c.push(entry("DiscreteMedianFilter", "Discrete Median Filter", cat, 1, 1, "Moving median filter"));
    c.push(entry("DiscreteMovingAverage", "Moving Average", cat, 1, 1, "Moving average filter"));
    c.push(entry("DiscretePIDv2", "PID Controller v2", cat, 1, 1, "Second-generation PID"));

    // ── Signal Routing ──────────────────────────────────────────────────
    let cat = "Signal Routing";
    c.push(entry("Mux", "Mux", cat, 2, 1, "Multiplex scalar signals into vector"));
    c.push(entry("Demux", "Demux", cat, 1, 2, "Demultiplex vector into scalars"));
    c.push(entry("Switch", "Switch", cat, 3, 1, "Switch between two inputs based on threshold"));
    c.push(entry("MultiPortSwitch", "Multiport Switch", cat, 3, 1, "Select from multiple data inputs"));
    c.push(entry("ManualSwitch", "Manual Switch", cat, 2, 1, "Manually toggle between two inputs"));
    c.push(entry("BusCreator", "Bus Creator", cat, 2, 1, "Create bus signal from individual signals"));
    c.push(entry("BusSelector", "Bus Selector", cat, 1, 1, "Select signals from bus"));
    c.push(entry("BusAssignment", "Bus Assignment", cat, 2, 1, "Assign signals to bus elements"));
    c.push(entry("BusToVector", "Bus to Vector", cat, 1, 1, "Convert bus to vector signal"));
    c.push(entry("Goto", "Goto", cat, 1, 0, "Send signal to matching From block"));
    c.push(entry("From", "From", cat, 0, 1, "Receive signal from matching Goto block"));
    c.push(entry("GotoTagVisibility", "Goto Tag Visibility", cat, 0, 0, "Make Goto tag visible"));
    c.push(entry("DataStoreRead", "Data Store Read", cat, 0, 1, "Read from data store"));
    c.push(entry("DataStoreWrite", "Data Store Write", cat, 1, 0, "Write to data store"));
    c.push(entry("DataStoreMemory", "Data Store Memory", cat, 0, 0, "Define data store memory"));
    c.push(entry("Merge", "Merge", cat, 2, 1, "Merge multiple signals"));
    c.push(entry("IndexVector", "Index Vector", cat, 2, 1, "Index into vector"));
    c.push(entry("Selector", "Selector", cat, 1, 1, "Select elements by index"));
    c.push(entry("Assignment", "Assignment", cat, 2, 1, "Assign values to vector/matrix elements"));
    c.push(entry("Concatenate", "Concatenate", cat, 2, 1, "Concatenate signals"));
    c.push(entry("VectorConcatenate", "Vector Concatenate", cat, 2, 1, "Concatenate into vector"));
    c.push(entry("MatrixConcatenate", "Matrix Concatenate", cat, 2, 1, "Concatenate into matrix"));
    c.push(entry("Permute", "Permute Dimensions", cat, 1, 1, "Rearrange signal dimensions"));
    c.push(entry("Squeeze", "Squeeze", cat, 1, 1, "Remove singleton dimensions"));
    c.push(entry("SignalConversion", "Signal Conversion", cat, 1, 1, "Convert signal type"));
    c.push(entry("InBus", "In Bus Element", cat, 0, 1, "Input bus element"));
    c.push(entry("Environment", "Environment Controller", cat, 0, 2, "Switch between sim and codegen"));

    // ── Signal Attributes ───────────────────────────────────────────────
    let cat = "Signal Attributes";
    c.push(entry("DataTypeConversion", "Data Type Conversion", cat, 1, 1, "Convert between data types"));
    c.push(entry("DataTypeDuplicate", "Data Type Duplicate", cat, 2, 0, "Ensure matching data types"));
    c.push(entry("DataTypeScale", "Data Type Scaling Strip", cat, 1, 1, "Strip fixed-point scaling"));
    c.push(entry("DataTypePropagation", "Data Type Propagation", cat, 3, 1, "Propagate data types"));
    c.push(entry("SignalSpecification", "Signal Specification", cat, 1, 1, "Specify signal attributes"));
    c.push(entry("IC", "IC (Initial Condition)", cat, 1, 1, "Set initial condition"));
    c.push(entry("Width", "Width", cat, 1, 1, "Output signal width"));
    c.push(entry("ProbeSignal", "Probe", cat, 1, 4, "Probe signal attributes"));
    c.push(entry("WeightedSample", "Weighted Sample Time", cat, 1, 1, "Scale by sample time"));
    c.push(entry("BusToSignal", "Bus to Signal", cat, 1, 1, "Convert bus to signal"));

    // ── Subsystems ──────────────────────────────────────────────────────
    let cat = "Subsystems";
    c.push(entry("SubSystem", "Subsystem", cat, 1, 1, "Group blocks into subsystem"));
    c.push(entry("AtomicSubSystem", "Atomic Subsystem", cat, 1, 1, "Atomic (non-virtual) subsystem"));
    c.push(entry("EnabledSubSystem", "Enabled Subsystem", cat, 2, 1, "Subsystem with enable port"));
    c.push(entry("TriggeredSubSystem", "Triggered Subsystem", cat, 2, 1, "Subsystem with trigger port"));
    c.push(entry("EnabledTriggeredSubSystem", "Enabled and Triggered", cat, 3, 1, "Subsystem with enable and trigger"));
    c.push(entry("ConditionalSubSystem", "Conditional Subsystem", cat, 2, 1, "Conditional execution subsystem"));
    c.push(entry("ForEachSubSystem", "For Each Subsystem", cat, 1, 1, "Process each element independently"));
    c.push(entry("ForIterator", "For Iterator Subsystem", cat, 1, 1, "Iterate N times"));
    c.push(entry("WhileIterator", "While Iterator Subsystem", cat, 1, 1, "Iterate while condition is true"));
    c.push(entry("IfAction", "If Action Subsystem", cat, 1, 1, "Execute when If condition is true"));
    c.push(entry("SwitchCaseAction", "Switch Case Action", cat, 1, 1, "Execute for switch case match"));
    c.push(entry("FunctionCallSubSystem", "Function-Call Subsystem", cat, 1, 1, "Call subsystem as function"));
    c.push(entry("ConfigSubSystem", "Configurable Subsystem", cat, 1, 1, "Configurable subsystem variant"));
    c.push(entry("VariantSubSystem", "Variant Subsystem", cat, 1, 1, "Compile-time variant selection"));
    c.push(entry("ModelReference", "Model Reference", cat, 1, 1, "Reference external model"));
    c.push(entry("IteratorSubSystem", "Iterator", cat, 1, 1, "Generic iteration subsystem"));
    c.push(entry("CodeReuse", "Code Reuse Subsystem", cat, 1, 1, "Promote reusable code generation"));
    c.push(entry("MaskedSubSystem", "Masked Subsystem", cat, 1, 1, "Subsystem with custom mask"));

    // ── Ports & Subsystem Ports ─────────────────────────────────────────
    let cat = "Ports & Subsystems";
    c.push(entry("EnablePort", "Enable", cat, 0, 0, "Enable port for subsystem"));
    c.push(entry("TriggerPort", "Trigger", cat, 0, 0, "Trigger port for subsystem"));
    c.push(entry("ActionPort", "Action Port", cat, 0, 0, "Action port for if/switch subsystem"));
    c.push(entry("FunctionCallGenerator", "Function-Call Generator", cat, 0, 1, "Generate function calls"));
    c.push(entry("If", "If", cat, 1, 1, "If-else conditional branching"));
    c.push(entry("SwitchCase", "Switch Case", cat, 1, 2, "Switch-case branching"));
    c.push(entry("FunctionCaller", "Function Caller", cat, 1, 1, "Call Simulink Function"));
    c.push(entry("SimulinkFunction", "Simulink Function", cat, 1, 1, "Define Simulink function"));
    c.push(entry("ArgumentInport", "Argument Inport", cat, 0, 1, "Function argument input"));
    c.push(entry("ArgumentOutport", "Argument Outport", cat, 1, 0, "Function argument output"));
    c.push(entry("InitializeFunction", "Initialize Function", cat, 0, 0, "Model initialize event"));
    c.push(entry("TerminateFunction", "Terminate Function", cat, 0, 0, "Model terminate event"));
    c.push(entry("ResetFunction", "Reset Function", cat, 0, 0, "Model reset event"));

    // ── Lookup Tables ───────────────────────────────────────────────────
    let cat = "Lookup Tables";
    c.push(entry("Lookup", "1-D Lookup Table", cat, 1, 1, "One-dimensional lookup table"));
    c.push(entry("Lookup2D", "2-D Lookup Table", cat, 2, 1, "Two-dimensional lookup table"));
    c.push(entry("LookupND", "n-D Lookup Table", cat, 1, 1, "N-dimensional lookup table"));
    c.push(entry("PreLookup", "Prelookup", cat, 1, 2, "Prelookup for interpolation index"));
    c.push(entry("InterpUsingPreLookup", "Interpolation Using Prelookup", cat, 1, 1, "Interpolate using prelookup result"));
    c.push(entry("DirectLookup", "Direct Lookup Table (n-D)", cat, 1, 1, "Direct index into table"));
    c.push(entry("DynamicLookup", "Dynamic Lookup", cat, 3, 1, "Lookup with dynamic breakpoints"));
    c.push(entry("LookupSine", "Sine Lookup", cat, 1, 1, "Sine approximation via table"));
    c.push(entry("LookupCosine", "Cosine Lookup", cat, 1, 1, "Cosine approximation via table"));

    // ── User-Defined Functions ──────────────────────────────────────────
    let cat = "User-Defined Functions";
    c.push(entry("MATLAB Function", "MATLAB Function", cat, 1, 1, "Embedded MATLAB function block"));
    c.push(entry("CFunction", "C Function", cat, 1, 1, "Call C/C++ code"));
    c.push(entry("Fcn", "Fcn (Expression)", cat, 1, 1, "Mathematical expression evaluator"));
    c.push(entry("MATLABSystem", "MATLAB System", cat, 1, 1, "MATLAB System object block"));
    c.push(entry("SFunction", "S-Function", cat, 1, 1, "S-Function (level-2 MEX)"));
    c.push(entry("SFunctionBuilder", "S-Function Builder", cat, 1, 1, "Generate S-Function code"));
    c.push(entry("Level2MATLAB", "Level-2 MATLAB S-Function", cat, 1, 1, "MATLAB-based S-Function"));
    c.push(entry("InterpretedMATLAB", "Interpreted MATLAB Function", cat, 1, 1, "Evaluate MATLAB expression"));
    c.push(entry("EmbeddedMATLAB", "Embedded MATLAB Function", cat, 1, 1, "Embedded MATLAB code block"));
    c.push(entry("PythonFunction", "Python Function", cat, 1, 1, "Call Python code"));

    // ── Discontinuities ─────────────────────────────────────────────────
    let cat = "Discontinuities";
    c.push(entry("Saturation", "Saturation", cat, 1, 1, "Limit signal to bounds"));
    c.push(entry("SaturationDynamic", "Saturation Dynamic", cat, 3, 1, "Saturation with dynamic limits"));
    c.push(entry("DeadZone", "Dead Zone", cat, 1, 1, "Zero output within dead zone band"));
    c.push(entry("DeadZoneDynamic", "Dead Zone Dynamic", cat, 3, 1, "Dead zone with dynamic limits"));
    c.push(entry("RateLimiter", "Rate Limiter", cat, 1, 1, "Limit rate of change"));
    c.push(entry("RateLimiterDynamic", "Rate Limiter Dynamic", cat, 3, 1, "Rate limiter with dynamic limits"));
    c.push(entry("Backlash", "Backlash", cat, 1, 1, "Model backlash / hysteresis"));
    c.push(entry("Coulomb", "Coulomb & Viscous Friction", cat, 1, 1, "Coulomb and viscous friction"));
    c.push(entry("HitCrossing", "Hit Crossing", cat, 1, 1, "Detect zero-crossing event"));
    c.push(entry("Quantizer", "Quantizer", cat, 1, 1, "Quantize signal to fixed levels"));
    c.push(entry("Relay", "Relay", cat, 1, 1, "Switch output on/off with hysteresis"));
    c.push(entry("WrappedToZero", "Wrap To Zero", cat, 1, 1, "Wrap to zero on overflow"));

    // ── Matrix Operations ───────────────────────────────────────────────
    let cat = "Matrix Operations";
    c.push(entry("MatrixGain", "Matrix Gain", cat, 1, 1, "Gain by matrix multiplication"));
    c.push(entry("MatrixMultiply", "Matrix Multiply", cat, 2, 1, "Multiply two matrices"));
    c.push(entry("MatrixInverse", "Matrix Inverse", cat, 1, 1, "Compute matrix inverse"));
    c.push(entry("Transpose", "Transpose", cat, 1, 1, "Transpose matrix"));
    c.push(entry("Hermitian", "Hermitian Transpose", cat, 1, 1, "Conjugate transpose"));
    c.push(entry("MatrixDivide", "Matrix Divide", cat, 2, 1, "Solve AX=B (left divide)"));
    c.push(entry("MatrixConcat", "Matrix Concatenation", cat, 2, 1, "Concatenate matrices"));
    c.push(entry("SingularValues", "Singular Value Decomposition", cat, 1, 3, "SVD decomposition"));
    c.push(entry("Eigenvalue", "Eigenvalue", cat, 1, 2, "Eigenvalue decomposition"));
    c.push(entry("LUFactor", "LU Factorization", cat, 1, 2, "LU matrix factorization"));
    c.push(entry("QRFactor", "QR Factorization", cat, 1, 2, "QR matrix factorization"));
    c.push(entry("CholeskyFactor", "Cholesky Factorization", cat, 1, 1, "Cholesky factorization"));
    c.push(entry("Determinant", "Determinant", cat, 1, 1, "Matrix determinant"));
    c.push(entry("Trace", "Trace", cat, 1, 1, "Matrix trace"));
    c.push(entry("MatrixRank", "Rank", cat, 1, 1, "Matrix rank"));
    c.push(entry("PseudoInverse", "Pseudo Inverse", cat, 1, 1, "Moore-Penrose pseudoinverse"));
    c.push(entry("Kronecker", "Kronecker Product", cat, 2, 1, "Kronecker tensor product"));
    c.push(entry("EyeMatrix", "Identity Matrix", cat, 0, 1, "Generate identity matrix"));
    c.push(entry("DiagonalMatrix", "Create Diagonal Matrix", cat, 1, 1, "Create diagonal from vector"));
    c.push(entry("ExtractDiag", "Extract Diagonal", cat, 1, 1, "Extract diagonal of matrix"));
    c.push(entry("ToeplitzMatrix", "Toeplitz Matrix", cat, 1, 1, "Generate Toeplitz matrix"));

    // ── Signal Processing ───────────────────────────────────────────────
    let cat = "Signal Processing";
    c.push(entry("FFT", "FFT", cat, 1, 1, "Fast Fourier Transform"));
    c.push(entry("IFFT", "IFFT", cat, 1, 1, "Inverse Fast Fourier Transform"));
    c.push(entry("AnalogFilter", "Analog Filter Design", cat, 1, 1, "Butterworth/Chebyshev/Elliptic filter"));
    c.push(entry("DigitalFilter", "Digital Filter Design", cat, 1, 1, "Design and apply digital filter"));
    c.push(entry("BiquadFilter", "Biquad Filter", cat, 1, 1, "Second-order section (biquad) filter"));
    c.push(entry("MedianFilter", "Median Filter", cat, 1, 1, "Median filter"));
    c.push(entry("LMSFilter", "LMS Filter", cat, 2, 2, "Least mean squares adaptive filter"));
    c.push(entry("RLSFilter", "RLS Filter", cat, 2, 2, "Recursive least squares filter"));
    c.push(entry("WindowFunction", "Window Function", cat, 1, 1, "Apply window function"));
    c.push(entry("Convolution", "Convolution", cat, 2, 1, "Signal convolution"));
    c.push(entry("Correlation", "Correlation", cat, 2, 1, "Signal cross-correlation"));
    c.push(entry("Autocorrelation", "Autocorrelation", cat, 1, 1, "Signal autocorrelation"));
    c.push(entry("DCT", "DCT", cat, 1, 1, "Discrete Cosine Transform"));
    c.push(entry("IDCT", "IDCT", cat, 1, 1, "Inverse Discrete Cosine Transform"));
    c.push(entry("Hilbert", "Hilbert Transform", cat, 1, 1, "Hilbert transform"));
    c.push(entry("Spectrum", "Spectrum Analyzer", cat, 1, 0, "Frequency spectrum analysis"));
    c.push(entry("Spectrogram", "Spectrogram", cat, 1, 1, "Time-frequency spectrogram"));
    c.push(entry("PowerSpectrum", "Power Spectrum", cat, 1, 1, "Power spectral density"));
    c.push(entry("SignalEnvelope", "Envelope Detector", cat, 1, 1, "Signal envelope extraction"));
    c.push(entry("ZeroCrossingCount", "Zero-Crossing Counter", cat, 1, 1, "Count zero crossings"));
    c.push(entry("PeakDetector", "Peak Detector", cat, 1, 2, "Find signal peaks"));
    c.push(entry("Interpolation", "Interpolation", cat, 1, 1, "Signal interpolation"));
    c.push(entry("Decimation", "Decimation", cat, 1, 1, "Signal decimation"));
    c.push(entry("Resampler", "Resampler", cat, 1, 1, "Multi-rate resampling"));
    c.push(entry("OverlapAdd", "Overlap-Add", cat, 1, 1, "Block-based signal processing"));

    // ── Control System ──────────────────────────────────────────────────
    let cat = "Control System";
    c.push(entry("LTISystem", "LTI System", cat, 1, 1, "Linear time-invariant system"));
    c.push(entry("PIDCompact", "PID Controller (Compact)", cat, 1, 1, "Compact PID block"));
    c.push(entry("LQR", "LQR Controller", cat, 1, 1, "Linear quadratic regulator"));
    c.push(entry("LQG", "LQG Controller", cat, 1, 1, "Linear quadratic Gaussian"));
    c.push(entry("KalmanFilter", "Kalman Filter", cat, 2, 2, "Standard Kalman filter"));
    c.push(entry("ExtendedKalman", "Extended Kalman Filter", cat, 2, 2, "Extended Kalman filter"));
    c.push(entry("UnscentedKalman", "Unscented Kalman Filter", cat, 2, 2, "Unscented Kalman filter"));
    c.push(entry("ParticleFilter", "Particle Filter", cat, 2, 2, "Particle filter / sequential Monte Carlo"));
    c.push(entry("Observer", "State Observer", cat, 2, 1, "Luenberger state observer"));
    c.push(entry("ModelPredictive", "MPC Controller", cat, 2, 1, "Model predictive controller"));
    c.push(entry("GainSchedule", "Gain Scheduling", cat, 2, 1, "Gain-scheduled controller"));
    c.push(entry("AdaptiveController", "Adaptive Controller", cat, 2, 1, "Adaptive control block"));
    c.push(entry("NotchFilter", "Notch Filter", cat, 1, 1, "Notch (band-stop) filter"));
    c.push(entry("Compensator", "Lead-Lag Compensator", cat, 1, 1, "Frequency-domain compensator"));
    c.push(entry("AntiWindup", "Anti-Windup", cat, 2, 1, "Integrator anti-windup"));

    // ── Stateflow ───────────────────────────────────────────────────────
    let cat = "Stateflow";
    c.push(entry("Chart", "Chart", cat, 1, 1, "Stateflow chart"));
    c.push(entry("StateTransitionTable", "State Transition Table", cat, 1, 1, "State transition table"));
    c.push(entry("TruthTable", "Truth Table", cat, 1, 1, "Truth table"));
    c.push(entry("SequenceViewer", "Sequence Viewer", cat, 1, 0, "Sequence viewer"));

    // ── Physical Modeling ───────────────────────────────────────────────
    let cat = "Physical Modeling";
    c.push(entry("SimscapeBlock", "Simscape Block", cat, 1, 1, "Simscape physical model block"));
    c.push(entry("SimscapeConnection", "Connection Port", cat, 0, 0, "Simscape connection port"));
    c.push(entry("PSConvert", "PS-Simulink Converter", cat, 1, 1, "Physical signal to Simulink"));
    c.push(entry("SPConvert", "Simulink-PS Converter", cat, 1, 1, "Simulink to physical signal"));
    c.push(entry("SimscapeSolver", "Solver Configuration", cat, 0, 0, "Simscape solver config"));
    c.push(entry("SpringDamper", "Spring-Damper", cat, 2, 1, "Spring-damper mechanical element"));
    c.push(entry("RotationalSpring", "Rotational Spring", cat, 1, 1, "Rotational spring"));
    c.push(entry("RotationalDamper", "Rotational Damper", cat, 1, 1, "Rotational damper"));
    c.push(entry("MassBlock", "Mass", cat, 1, 1, "Translational mass"));
    c.push(entry("InertiaBlock", "Inertia", cat, 1, 1, "Rotational inertia"));
    c.push(entry("IdealForceSource", "Ideal Force Source", cat, 1, 1, "Ideal force source"));
    c.push(entry("IdealTorqueSource", "Ideal Torque Source", cat, 1, 1, "Ideal torque source"));
    c.push(entry("MechanicalRef", "Mechanical Translational Reference", cat, 0, 0, "Translational reference"));
    c.push(entry("RotationalRef", "Rotational Reference", cat, 0, 0, "Rotational reference"));
    c.push(entry("Resistor", "Resistor", cat, 0, 0, "Electrical resistor"));
    c.push(entry("Capacitor", "Capacitor", cat, 0, 0, "Electrical capacitor"));
    c.push(entry("Inductor", "Inductor", cat, 0, 0, "Electrical inductor"));
    c.push(entry("Diode", "Diode", cat, 0, 0, "Electrical diode"));
    c.push(entry("VoltageSource", "Voltage Source", cat, 0, 0, "DC voltage source"));
    c.push(entry("CurrentSource", "Current Source", cat, 0, 0, "DC current source"));
    c.push(entry("ElectricalRef", "Electrical Reference", cat, 0, 0, "Ground node"));
    c.push(entry("VoltageSensor", "Voltage Sensor", cat, 0, 1, "Measure voltage"));
    c.push(entry("CurrentSensor", "Current Sensor", cat, 0, 1, "Measure current"));
    c.push(entry("Pipe", "Pipe", cat, 0, 0, "Hydraulic pipe"));
    c.push(entry("HydraulicPump", "Hydraulic Pump", cat, 1, 0, "Hydraulic pump"));
    c.push(entry("HydraulicMotor", "Hydraulic Motor", cat, 0, 1, "Hydraulic motor"));
    c.push(entry("HydraulicCylinder", "Hydraulic Cylinder", cat, 1, 1, "Hydraulic cylinder"));
    c.push(entry("FluidReservoir", "Fluid Reservoir", cat, 0, 0, "Fluid reservoir"));
    c.push(entry("ThermalMass", "Thermal Mass", cat, 0, 0, "Thermal mass"));
    c.push(entry("HeatExchanger", "Heat Exchanger", cat, 0, 0, "Thermal heat exchanger"));

    // ── Robotics ────────────────────────────────────────────────────────
    let cat = "Robotics";
    c.push(entry("RigidBodyTree", "Rigid Body Tree", cat, 1, 1, "Multi-body robot model"));
    c.push(entry("ForwardKinematics", "Forward Kinematics", cat, 1, 1, "Compute forward kinematics"));
    c.push(entry("InverseKinematics", "Inverse Kinematics", cat, 2, 1, "Compute inverse kinematics"));
    c.push(entry("Jacobian", "Geometric Jacobian", cat, 1, 1, "Compute Jacobian matrix"));
    c.push(entry("InverseDynamics", "Inverse Dynamics", cat, 3, 1, "Compute inverse dynamics torques"));
    c.push(entry("ForwardDynamics", "Forward Dynamics", cat, 2, 2, "Compute forward dynamics"));
    c.push(entry("GravityCompensation", "Gravity Compensation", cat, 1, 1, "Compute gravity torques"));
    c.push(entry("MassMatrix", "Mass Matrix", cat, 1, 1, "Compute manipulator inertia matrix"));
    c.push(entry("CoriolisMatrix", "Coriolis Matrix", cat, 2, 1, "Compute Coriolis matrix"));
    c.push(entry("JointSpaceMotion", "Joint Space Motion Model", cat, 2, 2, "Joint space motion model"));
    c.push(entry("TaskSpaceMotion", "Task Space Motion Model", cat, 2, 2, "Task space motion model"));
    c.push(entry("TrajectoryGenerator", "Trajectory Generator", cat, 1, 1, "Generate joint trajectories"));
    c.push(entry("WaypointFollower", "Waypoint Follower", cat, 2, 1, "Follow waypoint path"));
    c.push(entry("PurePursuit", "Pure Pursuit", cat, 2, 2, "Pure pursuit path tracking"));
    c.push(entry("JointActuator", "Joint Actuator", cat, 1, 0, "Actuate robot joints"));
    c.push(entry("JointSensor", "Joint Sensor", cat, 0, 1, "Sense joint states"));
    c.push(entry("BodyActuator", "Body Actuator", cat, 1, 0, "Apply body forces/torques"));
    c.push(entry("TransformSensor", "Transform Sensor", cat, 0, 1, "Sense body transforms"));

    // ── Navigation / Aerospace ──────────────────────────────────────────
    let cat = "Aerospace / Navigation";
    c.push(entry("QuaternionMultiply", "Quaternion Multiplication", cat, 2, 1, "Multiply quaternions"));
    c.push(entry("QuaternionInverse", "Quaternion Inverse", cat, 1, 1, "Inverse of quaternion"));
    c.push(entry("QuaternionNorm", "Quaternion Norm", cat, 1, 1, "Norm of quaternion"));
    c.push(entry("QuaternionNormalize", "Quaternion Normalize", cat, 1, 1, "Normalize quaternion"));
    c.push(entry("QuaternionToEuler", "Quaternion to Euler", cat, 1, 1, "Convert quaternion to Euler angles"));
    c.push(entry("EulerToQuaternion", "Euler to Quaternion", cat, 1, 1, "Convert Euler to quaternion"));
    c.push(entry("QuaternionToRotMatrix", "Quaternion to DCM", cat, 1, 1, "Quaternion to direction cosine matrix"));
    c.push(entry("RotMatrixToQuaternion", "DCM to Quaternion", cat, 1, 1, "DCM to quaternion"));
    c.push(entry("EulerToRotMatrix", "Euler to DCM", cat, 1, 1, "Euler angles to DCM"));
    c.push(entry("RotMatrixToEuler", "DCM to Euler", cat, 1, 1, "DCM to Euler angles"));
    c.push(entry("AxisAngleToQuat", "Axis-Angle to Quaternion", cat, 1, 1, "Convert axis-angle to quaternion"));
    c.push(entry("QuatToAxisAngle", "Quaternion to Axis-Angle", cat, 1, 1, "Convert quaternion to axis-angle"));
    c.push(entry("AngularVelocity", "Angular Velocity", cat, 1, 1, "Compute angular velocity"));
    c.push(entry("CoordinateTransform", "Coordinate Transformation", cat, 1, 1, "Transform coordinates between frames"));
    c.push(entry("FrameRotation", "Frame Rotation", cat, 2, 1, "Rotate frame by angle/quaternion"));
    c.push(entry("FlatEarthDynamics", "Flat Earth Dynamics", cat, 4, 4, "6-DOF flat earth equations of motion"));
    c.push(entry("WindModel", "Wind Model", cat, 1, 1, "Atmospheric wind model"));
    c.push(entry("GravityModel", "Gravity Model", cat, 1, 1, "Gravitational field model"));
    c.push(entry("AtmosphereModel", "Atmosphere Model", cat, 1, 3, "Standard atmosphere properties"));
    c.push(entry("AeroCoefficients", "Aerodynamic Coefficients", cat, 3, 3, "Compute aero forces/moments"));
    c.push(entry("GPSSensor", "GPS Sensor", cat, 1, 3, "GPS position/velocity sensor model"));
    c.push(entry("IMUSensor", "IMU Sensor", cat, 2, 2, "Inertial measurement unit model"));
    c.push(entry("MagnetometerSensor", "Magnetometer", cat, 1, 1, "Magnetic field sensor model"));
    c.push(entry("BarometerSensor", "Barometer", cat, 1, 1, "Barometric pressure sensor model"));
    c.push(entry("AHRS", "AHRS Filter", cat, 3, 1, "Attitude heading reference system"));
    c.push(entry("ComplementaryFilter", "Complementary Filter", cat, 2, 1, "Complementary filter for IMU"));
    c.push(entry("INS", "INS Filter", cat, 3, 2, "Inertial navigation system filter"));

    // ── Communications ──────────────────────────────────────────────────
    let cat = "Communications";
    c.push(entry("UDPSend", "UDP Send", cat, 1, 0, "Send data via UDP"));
    c.push(entry("UDPReceive", "UDP Receive", cat, 0, 1, "Receive data via UDP"));
    c.push(entry("TCPIPSend", "TCP/IP Send", cat, 1, 0, "Send data via TCP/IP"));
    c.push(entry("TCPIPReceive", "TCP/IP Receive", cat, 0, 1, "Receive data via TCP/IP"));
    c.push(entry("SerialSend", "Serial Send", cat, 1, 0, "Send data via serial port"));
    c.push(entry("SerialReceive", "Serial Receive", cat, 0, 1, "Receive data via serial port"));
    c.push(entry("CANTransmit", "CAN Transmit", cat, 1, 0, "Send CAN message"));
    c.push(entry("CANReceive", "CAN Receive", cat, 0, 1, "Receive CAN message"));
    c.push(entry("CANPack", "CAN Pack", cat, 1, 1, "Pack signals into CAN message"));
    c.push(entry("CANUnpack", "CAN Unpack", cat, 1, 1, "Unpack signals from CAN message"));
    c.push(entry("SharedMemoryRead", "Shared Memory Read", cat, 0, 1, "Read from shared memory"));
    c.push(entry("SharedMemoryWrite", "Shared Memory Write", cat, 1, 0, "Write to shared memory"));
    c.push(entry("MQTTPublish", "MQTT Publish", cat, 1, 0, "Publish MQTT message"));
    c.push(entry("MQTTSubscribe", "MQTT Subscribe", cat, 0, 1, "Subscribe to MQTT topic"));
    c.push(entry("ROSPublisher", "ROS Publisher", cat, 1, 0, "Publish to ROS topic"));
    c.push(entry("ROSSubscriber", "ROS Subscriber", cat, 0, 1, "Subscribe to ROS topic"));
    c.push(entry("ROSServiceClient", "ROS Service Client", cat, 1, 1, "Call ROS service"));
    c.push(entry("ROSServiceServer", "ROS Service Server", cat, 1, 1, "Serve ROS service requests"));
    c.push(entry("ROS2Publisher", "ROS 2 Publisher", cat, 1, 0, "Publish to ROS 2 topic"));
    c.push(entry("ROS2Subscriber", "ROS 2 Subscriber", cat, 0, 1, "Subscribe to ROS 2 topic"));
    c.push(entry("EtherCAT", "EtherCAT", cat, 1, 1, "EtherCAT communication"));
    c.push(entry("ProfinetIO", "PROFINET IO", cat, 1, 1, "PROFINET IO communication"));
    c.push(entry("OPCUARead", "OPC UA Read", cat, 0, 1, "Read OPC UA node"));
    c.push(entry("OPCUAWrite", "OPC UA Write", cat, 1, 0, "Write OPC UA node"));
    c.push(entry("Modbus", "Modbus", cat, 1, 1, "Modbus communication"));

    // ── Image / Video Processing ────────────────────────────────────────
    let cat = "Image Processing";
    c.push(entry("ImageResize", "Image Resize", cat, 1, 1, "Resize image"));
    c.push(entry("ImageRotate", "Image Rotate", cat, 1, 1, "Rotate image by angle"));
    c.push(entry("ImageCrop", "Image Crop", cat, 1, 1, "Crop image region"));
    c.push(entry("ColorConversion", "Color Space Conversion", cat, 1, 1, "Convert between RGB/HSV/YCbCr/Grayscale"));
    c.push(entry("EdgeDetection", "Edge Detection", cat, 1, 1, "Detect edges (Sobel, Canny, Prewitt)"));
    c.push(entry("Morphology", "Morphological Operation", cat, 1, 1, "Erosion, dilation, opening, closing"));
    c.push(entry("BlobAnalysis", "Blob Analysis", cat, 1, 3, "Find and analyze connected regions"));
    c.push(entry("TemplateMatch", "Template Matching", cat, 2, 1, "Find template in image"));
    c.push(entry("Threshold", "Image Threshold", cat, 1, 1, "Binary threshold on image"));
    c.push(entry("HistogramEqualize", "Histogram Equalization", cat, 1, 1, "Equalize image histogram"));
    c.push(entry("GaussianBlur", "Gaussian Blur", cat, 1, 1, "Gaussian smoothing filter"));
    c.push(entry("MedianFilterImg", "Median Filter (Image)", cat, 1, 1, "Median filter for images"));
    c.push(entry("HoughTransform", "Hough Transform", cat, 1, 1, "Hough transform for lines/circles"));
    c.push(entry("FeatureExtract", "Feature Extraction", cat, 1, 1, "Extract image features (SURF, ORB)"));
    c.push(entry("OpticalFlow", "Optical Flow", cat, 1, 1, "Compute optical flow"));
    c.push(entry("VideoDisplay", "Video Display", cat, 1, 0, "Display video stream"));
    c.push(entry("VideoCapture", "Video Capture", cat, 0, 1, "Capture from camera"));
    c.push(entry("VideoFileReader", "Video File Reader", cat, 0, 1, "Read video from file"));
    c.push(entry("VideoFileWriter", "Video File Writer", cat, 1, 0, "Write video to file"));
    c.push(entry("DrawShapes", "Draw Shapes", cat, 2, 1, "Draw shapes on image"));
    c.push(entry("InsertText", "Insert Text", cat, 2, 1, "Insert text into image"));
    c.push(entry("ImageComposite", "Image Composite", cat, 2, 1, "Composite two images"));
    c.push(entry("PointCloudDisplay", "Point Cloud Display", cat, 1, 0, "Display 3D point cloud"));
    c.push(entry("DepthEstimation", "Depth Estimation", cat, 2, 1, "Stereo depth estimation"));

    // ── Deep Learning / AI ──────────────────────────────────────────────
    let cat = "AI / Deep Learning";
    c.push(entry("PredictBlock", "Predict", cat, 1, 1, "Run deep learning inference"));
    c.push(entry("ClassificationBlock", "Image Classifier", cat, 1, 1, "Image classification network"));
    c.push(entry("ObjectDetector", "Object Detector", cat, 1, 2, "Object detection (YOLO, SSD, etc.)"));
    c.push(entry("SemanticSegmentation", "Semantic Segmentation", cat, 1, 1, "Pixel-wise segmentation"));
    c.push(entry("InstanceSegmentation", "Instance Segmentation", cat, 1, 2, "Instance segmentation"));
    c.push(entry("PoseEstimation", "Pose Estimation", cat, 1, 1, "Human / object pose estimation"));
    c.push(entry("ReinforcementLearning", "RL Agent", cat, 2, 1, "Reinforcement learning agent"));
    c.push(entry("NeuralNetwork", "Neural Network", cat, 1, 1, "Custom neural network block"));
    c.push(entry("ONNXModel", "ONNX Model", cat, 1, 1, "Import and run ONNX model"));
    c.push(entry("TensorFlowModel", "TensorFlow Model", cat, 1, 1, "Import TensorFlow model"));
    c.push(entry("FuzzyLogicController", "Fuzzy Logic Controller", cat, 1, 1, "Fuzzy inference system"));
    c.push(entry("NeuralFit", "Neural Fitting", cat, 1, 1, "Neural network function fitting"));

    // ── Statistics ──────────────────────────────────────────────────────
    let cat = "Statistics";
    c.push(entry("Mean", "Mean", cat, 1, 1, "Compute running mean"));
    c.push(entry("Variance", "Variance", cat, 1, 1, "Compute running variance"));
    c.push(entry("StandardDeviation", "Standard Deviation", cat, 1, 1, "Compute running std dev"));
    c.push(entry("RMS", "RMS", cat, 1, 1, "Root mean square"));
    c.push(entry("Minimum", "Minimum", cat, 1, 1, "Running minimum"));
    c.push(entry("Maximum", "Maximum", cat, 1, 1, "Running maximum"));
    c.push(entry("MovingMean", "Moving Mean", cat, 1, 1, "Sliding window mean"));
    c.push(entry("MovingVariance", "Moving Variance", cat, 1, 1, "Sliding window variance"));
    c.push(entry("MovingRMS", "Moving RMS", cat, 1, 1, "Sliding window RMS"));
    c.push(entry("Histogram", "Histogram", cat, 1, 1, "Histogram of signal values"));
    c.push(entry("CumulativeSum", "Cumulative Sum", cat, 1, 1, "Running cumulative sum"));
    c.push(entry("CumulativeProd", "Cumulative Product", cat, 1, 1, "Running cumulative product"));
    c.push(entry("Sort", "Sort", cat, 1, 1, "Sort vector elements"));

    // ── Code Generation ─────────────────────────────────────────────────
    let cat = "Code Generation";
    c.push(entry("CodeAnnotation", "Code Annotation", cat, 0, 0, "Annotate generated code"));
    c.push(entry("TargetHardware", "Target Hardware", cat, 0, 0, "Target hardware configuration"));
    c.push(entry("ProcessorInLoop", "Processor-in-the-Loop", cat, 1, 1, "PIL testing block"));
    c.push(entry("SoftwareInLoop", "Software-in-the-Loop", cat, 1, 1, "SIL testing block"));
    c.push(entry("ExternalMode", "External Mode Control", cat, 0, 0, "External mode interface"));
    c.push(entry("SignalLogging", "Signal Logging", cat, 1, 0, "Log signal for code gen"));
    c.push(entry("DataObjectConfig", "Data Object", cat, 0, 0, "Configure data for code gen"));

    // ── Testing & Verification ──────────────────────────────────────────
    let cat = "Testing & Verification";
    c.push(entry("TestAssessment", "Test Assessment", cat, 1, 0, "Assess test criteria"));
    c.push(entry("TestSequence", "Test Sequence", cat, 0, 1, "Define test sequences"));
    c.push(entry("Assertion", "Assertion", cat, 1, 0, "Assert condition during simulation"));
    c.push(entry("CheckStaticRange", "Check Static Range", cat, 1, 1, "Verify static range"));
    c.push(entry("CheckDynamicRange", "Check Dynamic Range", cat, 1, 1, "Verify dynamic range"));
    c.push(entry("CheckStaticGap", "Check Static Gap", cat, 1, 1, "Verify no gap"));
    c.push(entry("CheckStaticLower", "Check Static Lower Bound", cat, 1, 1, "Verify lower bound"));
    c.push(entry("CheckStaticUpper", "Check Static Upper Bound", cat, 1, 1, "Verify upper bound"));
    c.push(entry("CheckGradient", "Check Gradient", cat, 1, 1, "Verify signal gradient"));
    c.push(entry("CheckResolution", "Check Resolution", cat, 1, 1, "Verify signal resolution"));
    c.push(entry("ProofAssumption", "Proof Assumption", cat, 1, 1, "Formal verification assumption"));
    c.push(entry("ProofObjective", "Proof Objective", cat, 1, 0, "Formal verification objective"));

    // ── Commonly Used Blocks (more from Simulink library) ───────────────
    let cat = "Commonly Used";
    c.push(entry("DataTypeConv", "Convert", cat, 1, 1, "Quick data type converter"));
    c.push(entry("SubsystemRef", "Subsystem Reference", cat, 1, 1, "Reference to external subsystem file"));
    c.push(entry("DocBlock", "DocBlock", cat, 0, 0, "Document block"));
    c.push(entry("Model", "Model", cat, 0, 0, "Model reference block"));
    c.push(entry("Abstraction", "Signal Hierarchy", cat, 1, 1, "Signal abstraction layer"));

    // ── Additional Specialized Blocks ───────────────────────────────────
    let cat = "Power Electronics";
    c.push(entry("IGBT", "IGBT", cat, 0, 0, "IGBT power switch"));
    c.push(entry("MOSFET", "MOSFET", cat, 0, 0, "MOSFET power switch"));
    c.push(entry("ThyristorBlock", "Thyristor", cat, 0, 0, "Thyristor"));
    c.push(entry("InverterBlock", "Inverter", cat, 1, 1, "Power inverter"));
    c.push(entry("RectifierBlock", "Rectifier", cat, 1, 1, "Power rectifier"));
    c.push(entry("DCDCConverter", "DC-DC Converter", cat, 1, 1, "DC-DC buck/boost converter"));
    c.push(entry("PWMGenerator", "PWM Generator", cat, 1, 1, "PWM signal generator"));
    c.push(entry("ThreePhaseSource", "Three-Phase Source", cat, 0, 3, "Three-phase AC source"));
    c.push(entry("ThreePhaseMeasure", "Three-Phase Measurement", cat, 3, 3, "Three-phase VI measurement"));
    c.push(entry("ABCToDQ0", "abc to dq0 Transform", cat, 2, 1, "Park transformation"));
    c.push(entry("DQ0ToABC", "dq0 to abc Transform", cat, 2, 1, "Inverse Park transform"));
    c.push(entry("AlphaBetaToABC", "αβ to abc Transform", cat, 1, 1, "Inverse Clarke transform"));
    c.push(entry("ABCToAlphaBeta", "abc to αβ Transform", cat, 1, 1, "Clarke transform"));
    c.push(entry("PLL", "Phase-Locked Loop", cat, 1, 2, "Phase-locked loop"));
    c.push(entry("HBridge", "H-Bridge", cat, 1, 1, "H-bridge motor driver"));
    c.push(entry("BatteryModel", "Battery", cat, 1, 1, "Battery model"));
    c.push(entry("SolarCell", "Solar Cell", cat, 0, 1, "Photovoltaic cell model"));
    c.push(entry("Transformer", "Transformer", cat, 0, 0, "Electrical transformer"));
    c.push(entry("Motor", "Motor", cat, 1, 1, "Electric motor model"));

    // ── HDL / FPGA ──────────────────────────────────────────────────────
    let cat = "HDL / FPGA";
    c.push(entry("HDLCounter", "HDL Counter", cat, 0, 1, "HDL-compatible counter"));
    c.push(entry("HDLDelay", "HDL Delay", cat, 1, 1, "HDL-compatible delay"));
    c.push(entry("HDLFIFORead", "HDL FIFO Read", cat, 1, 2, "HDL FIFO read interface"));
    c.push(entry("HDLFIFOWrite", "HDL FIFO Write", cat, 2, 1, "HDL FIFO write interface"));
    c.push(entry("DualPortRAM", "Dual Port RAM", cat, 2, 2, "Dual port RAM"));
    c.push(entry("SinglePortRAM", "Single Port RAM", cat, 1, 1, "Single port RAM"));
    c.push(entry("Serializer", "Serializer", cat, 1, 1, "Parallel to serial"));
    c.push(entry("Deserializer", "Deserializer", cat, 1, 1, "Serial to parallel"));
    c.push(entry("BitConcat", "Bit Concat", cat, 2, 1, "Concatenate bit fields"));
    c.push(entry("BitSlice", "Bit Slice", cat, 1, 1, "Extract bit slice"));
    c.push(entry("HDLSubsystem", "HDL Subsystem", cat, 1, 1, "Subsystem for HDL code gen"));

    // ── Timing & Scheduling ─────────────────────────────────────────────
    let cat = "Timing & Scheduling";
    c.push(entry("SampleTime", "Specify Sample Time", cat, 1, 1, "Override sample time"));
    c.push(entry("HitCross", "Hit Crossing", cat, 1, 1, "Detect exact crossing events"));
    c.push(entry("EventListener", "Event Listener", cat, 0, 1, "Listen for events"));
    c.push(entry("EventBroadcast", "Event Broadcast", cat, 1, 0, "Broadcast event"));
    c.push(entry("Timer", "Timer", cat, 0, 1, "Generate timer events"));
    c.push(entry("TaskSync", "Task Synchronizer", cat, 1, 1, "Synchronize concurrent tasks"));
    c.push(entry("RateSelector", "Rate Selector", cat, 1, 1, "Select execution rate"));

    // ── Automotive ──────────────────────────────────────────────────────
    let cat = "Automotive";
    c.push(entry("EngineModel", "Engine Model", cat, 2, 2, "IC engine model"));
    c.push(entry("TransmissionModel", "Transmission", cat, 2, 2, "Gear transmission model"));
    c.push(entry("VehicleDynamics", "Vehicle Body", cat, 3, 3, "Vehicle body dynamics"));
    c.push(entry("TireModel", "Tire Model", cat, 2, 2, "Tire force model"));
    c.push(entry("BrakeModel", "Brake System", cat, 1, 1, "Brake actuator model"));
    c.push(entry("SteeringModel", "Steering System", cat, 1, 1, "Steering dynamics"));
    c.push(entry("DriverModel", "Driver Model", cat, 2, 2, "Human driver model"));
    c.push(entry("ABSController", "ABS Controller", cat, 2, 1, "Anti-lock braking system"));
    c.push(entry("ESCController", "ESC Controller", cat, 3, 1, "Electronic stability control"));
    c.push(entry("CruiseControl", "Cruise Control", cat, 2, 1, "Cruise control logic"));
    c.push(entry("AdaptiveCruise", "Adaptive Cruise Control", cat, 3, 1, "ACC with radar input"));
    c.push(entry("LaneKeep", "Lane Keep Assist", cat, 2, 1, "Lane keeping controller"));
    c.push(entry("ParkingAssist", "Parking Assist", cat, 3, 1, "Parking assistant"));
    c.push(entry("RadarModel", "Radar Sensor", cat, 1, 2, "Automotive radar model"));
    c.push(entry("LidarModel", "Lidar Sensor", cat, 1, 2, "Lidar point cloud model"));
    c.push(entry("CameraModel", "Camera Sensor", cat, 1, 1, "Vision camera model"));
    c.push(entry("UltrasonicSensor", "Ultrasonic Sensor", cat, 1, 1, "Proximity sensor model"));

    // ── Audio ───────────────────────────────────────────────────────────
    let cat = "Audio";
    c.push(entry("AudioSource", "Audio File Source", cat, 0, 1, "Read audio from file"));
    c.push(entry("AudioSink", "Audio File Sink", cat, 1, 0, "Write audio to file"));
    c.push(entry("AudioPlayer", "Audio Device Writer", cat, 1, 0, "Play audio through device"));
    c.push(entry("AudioCapture", "Audio Device Reader", cat, 0, 1, "Record from microphone"));
    c.push(entry("AudioEqualizer", "Parametric EQ Filter", cat, 1, 1, "Parametric equalizer"));
    c.push(entry("AudioCompressor", "Dynamic Range Compressor", cat, 1, 1, "Audio compressor"));
    c.push(entry("AudioLimiter", "Dynamic Range Limiter", cat, 1, 1, "Audio limiter"));
    c.push(entry("AudioGate", "Noise Gate", cat, 1, 1, "Audio noise gate"));
    c.push(entry("AudioReverb", "Reverb", cat, 1, 1, "Reverberation effect"));
    c.push(entry("AudioDelay", "Audio Delay", cat, 1, 1, "Time delay effect"));
    c.push(entry("AudioMixer", "Audio Mixer", cat, 2, 1, "Mix audio channels"));
    c.push(entry("AudioOscillator", "Audio Oscillator", cat, 0, 1, "Generate audio waveform"));
    c.push(entry("AudioPitch", "Pitch Shift", cat, 1, 1, "Shift audio pitch"));
    c.push(entry("AudioTimeStretch", "Time Stretch", cat, 1, 1, "Time-stretch audio"));

    // ── Machine Learning ────────────────────────────────────────────────
    let cat = "Machine Learning";
    c.push(entry("ClassificationTree", "Classification Tree", cat, 1, 1, "Decision tree classifier"));
    c.push(entry("RegressionTree", "Regression Tree", cat, 1, 1, "Decision tree regressor"));
    c.push(entry("SVM", "SVM Classifier", cat, 1, 1, "Support vector machine classifier"));
    c.push(entry("SVMRegressor", "SVM Regressor", cat, 1, 1, "Support vector machine regressor"));
    c.push(entry("KNN", "KNN Classifier", cat, 1, 1, "K-nearest neighbors classifier"));
    c.push(entry("RandomForest", "Random Forest", cat, 1, 1, "Random forest ensemble"));
    c.push(entry("GradientBoosting", "Gradient Boosting", cat, 1, 1, "Gradient boosting ensemble"));
    c.push(entry("NaiveBayes", "Naive Bayes", cat, 1, 1, "Naive Bayes classifier"));
    c.push(entry("LinearRegression", "Linear Regression", cat, 1, 1, "Linear regression model"));
    c.push(entry("LogisticRegression", "Logistic Regression", cat, 1, 1, "Logistic regression classifier"));
    c.push(entry("KMeans", "K-Means Clustering", cat, 1, 1, "K-means clusterer"));
    c.push(entry("PCA", "PCA", cat, 1, 1, "Principal component analysis"));
    c.push(entry("Normalization", "Feature Normalization", cat, 1, 1, "Normalize feature vector"));

    // ── More Specialized / Utility ──────────────────────────────────────
    let cat = "Utility";
    c.push(entry("Reference", "Library Reference", cat, 1, 1, "Reference to library block"));
    c.push(entry("SimpleAnnotation", "Text Annotation", cat, 0, 0, "Free text annotation (non-block)"));
    c.push(entry("ConfigurationBlock", "Configuration", cat, 0, 0, "Model configuration block"));
    c.push(entry("ModelInfo", "Model Info", cat, 0, 0, "Display model information"));
    c.push(entry("SignalLabel", "Signal Label", cat, 1, 1, "Label a signal"));
    c.push(entry("NoteBlock", "Note Block", cat, 0, 0, "Add note to diagram"));
    c.push(entry("BoundaryDiagram", "Boundary Diagram", cat, 0, 0, "Boundary diagram block"));
    c.push(entry("RequirementBlock", "Requirement", cat, 0, 0, "Link to requirement"));

    // ── Optimization ────────────────────────────────────────────────────
    let cat = "Optimization";
    c.push(entry("QPSolver", "QP Solver", cat, 2, 1, "Quadratic programming solver"));
    c.push(entry("LPSolver", "LP Solver", cat, 2, 1, "Linear programming solver"));
    c.push(entry("NonlinearSolver", "Nonlinear Solver", cat, 1, 1, "Nonlinear equation solver"));
    c.push(entry("GeneticAlgorithm", "Genetic Algorithm", cat, 1, 1, "Genetic optimization"));
    c.push(entry("ParticleSwarm", "Particle Swarm", cat, 1, 1, "Particle swarm optimization"));
    c.push(entry("SimulatedAnnealing", "Simulated Annealing", cat, 1, 1, "Simulated annealing optimizer"));
    c.push(entry("GradientDescent", "Gradient Descent", cat, 1, 1, "Gradient descent optimizer"));
    c.push(entry("NewtonRaphson", "Newton-Raphson", cat, 1, 1, "Newton-Raphson root finder"));

    // ── Fixed-Point ─────────────────────────────────────────────────────
    let cat = "Fixed-Point";
    c.push(entry("FixedPointConvert", "Fixed-Point Conversion", cat, 1, 1, "Convert to/from fixed-point"));
    c.push(entry("FixedPointGain", "Fixed-Point Gain", cat, 1, 1, "Fixed-point gain"));
    c.push(entry("FixedPointSum", "Fixed-Point Sum", cat, 2, 1, "Fixed-point addition"));
    c.push(entry("FixedPointProduct", "Fixed-Point Product", cat, 2, 1, "Fixed-point multiplication"));
    c.push(entry("FixedPointLookup", "Fixed-Point Lookup Table", cat, 1, 1, "Fixed-point lookup table"));
    c.push(entry("FixedPointRelay", "Fixed-Point Relay", cat, 1, 1, "Fixed-point relay"));
    c.push(entry("FixedPointSaturation", "Fixed-Point Saturation", cat, 1, 1, "Fixed-point saturation"));
    c.push(entry("FixedPointSubtract", "Fixed-Point Subtract", cat, 2, 1, "Fixed-point subtraction"));
    c.push(entry("FixedPointDivide", "Fixed-Point Divide", cat, 2, 1, "Fixed-point division"));
    c.push(entry("FixedPointAbs", "Fixed-Point Abs", cat, 1, 1, "Fixed-point absolute value"));

    // ── Hardware I/O ────────────────────────────────────────────────────
    let cat = "Hardware I/O";
    c.push(entry("AnalogInput", "Analog Input", cat, 0, 1, "Read analog input pin"));
    c.push(entry("AnalogOutput", "Analog Output", cat, 1, 0, "Write analog output pin"));
    c.push(entry("DigitalInput", "Digital Input", cat, 0, 1, "Read digital input pin"));
    c.push(entry("DigitalOutput", "Digital Output", cat, 1, 0, "Write digital output pin"));
    c.push(entry("PWMOutput", "PWM Output", cat, 1, 0, "Generate PWM output"));
    c.push(entry("PWMInput", "PWM Input", cat, 0, 1, "Read PWM duty cycle"));
    c.push(entry("I2CRead", "I2C Read", cat, 0, 1, "Read from I2C bus"));
    c.push(entry("I2CWrite", "I2C Write", cat, 1, 0, "Write to I2C bus"));
    c.push(entry("SPIRead", "SPI Read", cat, 0, 1, "Read from SPI bus"));
    c.push(entry("SPIWrite", "SPI Write", cat, 1, 0, "Write to SPI bus"));
    c.push(entry("EncoderRead", "Encoder Read", cat, 0, 1, "Read quadrature encoder"));
    c.push(entry("CounterInput", "Counter Input", cat, 0, 1, "Count pulses on input"));
    c.push(entry("GPIORead", "GPIO Read", cat, 0, 1, "General purpose I/O read"));
    c.push(entry("GPIOWrite", "GPIO Write", cat, 1, 0, "General purpose I/O write"));
    c.push(entry("InterruptCallback", "External Interrupt", cat, 0, 1, "Trigger on external interrupt"));
    c.push(entry("DACOutput", "DAC Output", cat, 1, 0, "Digital-to-analog output"));
    c.push(entry("ADCInput", "ADC Input", cat, 0, 1, "Analog-to-digital input"));

    // ── Additional Math / Special ───────────────────────────────────────
    let cat = "Special Math";
    c.push(entry("Gamma", "Gamma Function", cat, 1, 1, "Gamma function Γ(x)"));
    c.push(entry("Beta", "Beta Function", cat, 2, 1, "Beta function B(a,b)"));
    c.push(entry("Bessel", "Bessel Function", cat, 1, 1, "Bessel function of first kind"));
    c.push(entry("Erf", "Error Function", cat, 1, 1, "Error function erf(x)"));
    c.push(entry("Erfc", "Complementary Error", cat, 1, 1, "Complementary error function erfc(x)"));
    c.push(entry("Factorial", "Factorial", cat, 1, 1, "Factorial n!"));
    c.push(entry("Binomial", "Binomial Coefficient", cat, 2, 1, "Binomial coefficient C(n,k)"));
    c.push(entry("Signum", "Signum", cat, 1, 1, "Sign function (-1, 0, +1)"));
    c.push(entry("Sinc", "Sinc", cat, 1, 1, "Sinc function sin(x)/x"));
    c.push(entry("HyperbolicTrig", "Hyperbolic Trig", cat, 1, 1, "sinh, cosh, tanh, etc."));
    c.push(entry("InverseHyperbolic", "Inverse Hyperbolic", cat, 1, 1, "asinh, acosh, atanh"));
    c.push(entry("InverseTrig", "Inverse Trig", cat, 1, 1, "asin, acos, atan"));
    c.push(entry("Atan2", "atan2", cat, 2, 1, "Four-quadrant inverse tangent"));
    c.push(entry("ComplexConjugate", "Complex Conjugate", cat, 1, 1, "Conjugate of complex number"));
    c.push(entry("ComplexMagnitude", "Complex Magnitude", cat, 1, 1, "Magnitude of complex number"));
    c.push(entry("ComplexAngle", "Complex Angle", cat, 1, 1, "Phase angle of complex number"));
    c.push(entry("Floor", "Floor", cat, 1, 1, "Floor function"));
    c.push(entry("Ceil", "Ceil", cat, 1, 1, "Ceiling function"));
    c.push(entry("Fix", "Fix (Truncate)", cat, 1, 1, "Truncate toward zero"));
    c.push(entry("Rem", "Remainder", cat, 2, 1, "Remainder after division"));

    // ── Simulation Control ──────────────────────────────────────────────
    let cat = "Simulation Control";
    c.push(entry("InitCondition", "Initial Condition", cat, 1, 1, "Set initial condition for signal"));
    c.push(entry("SimulationPace", "Simulation Pace", cat, 0, 0, "Control simulation speed"));
    c.push(entry("StopBlock", "Stop", cat, 1, 0, "Stop simulation on condition"));
    c.push(entry("Assert", "Assert", cat, 1, 0, "Assert runtime condition"));
    c.push(entry("MessageBlock", "Message", cat, 1, 1, "Send/receive simulation messages"));
    c.push(entry("EntityGenerator", "Entity Generator", cat, 0, 1, "Generate discrete entities"));
    c.push(entry("EntityServer", "Entity Server", cat, 1, 1, "Serve discrete entities"));
    c.push(entry("EntityTerminator", "Entity Terminator", cat, 1, 0, "Terminate entities"));
    c.push(entry("Queue", "Queue", cat, 1, 1, "FIFO/LIFO entity queue"));
    c.push(entry("ServerBlock", "Server", cat, 1, 1, "Service processing server"));

    // ── String Operations ───────────────────────────────────────────────
    let cat = "String Operations";
    c.push(entry("StringConstant", "String Constant", cat, 0, 1, "Output constant string"));
    c.push(entry("StringConcat", "String Concatenation", cat, 2, 1, "Concatenate strings"));
    c.push(entry("StringLength", "String Length", cat, 1, 1, "Length of string"));
    c.push(entry("StringCompare", "String Compare", cat, 2, 1, "Compare two strings"));
    c.push(entry("StringToNum", "String to Number", cat, 1, 1, "Convert string to number"));
    c.push(entry("NumToString", "Number to String", cat, 1, 1, "Convert number to string"));
    c.push(entry("StringScan", "String Scan", cat, 1, 1, "Scan/parse string"));
    c.push(entry("StringCompose", "String Compose", cat, 2, 1, "Format string with values"));

    // ── Additional fillers to reach 750+ count ──────────────────────────
    let cat = "Advanced";
    c.push(entry("AlgebraicLoop", "Algebraic Loop Solver", cat, 1, 1, "Solve algebraic loops"));
    c.push(entry("StateWriter", "State Writer", cat, 1, 0, "Write to block states"));
    c.push(entry("StateReader", "State Reader", cat, 0, 1, "Read block states"));
    c.push(entry("RuntimeParameter", "Runtime Parameter", cat, 0, 1, "Tunable runtime parameter"));
    c.push(entry("DesignVerifier", "Design Verifier", cat, 1, 0, "Design verification block"));
    c.push(entry("RequirementLink", "Requirement Link", cat, 0, 0, "Link to requirement"));
    c.push(entry("CoverageMarker", "Coverage Marker", cat, 0, 0, "Mark coverage point"));

    // ── Aerospace Blockset ──────────────────────────────────────────────
    let cat = "Aerospace";
    c.push(entry("AeroFlatEarth", "Flat Earth", cat, 3, 3, "Flat earth equations of motion"));
    c.push(entry("AeroWGS84", "WGS84 Gravity", cat, 1, 1, "WGS84 gravity model"));
    c.push(entry("AeroISA", "ISA Atmosphere", cat, 1, 4, "International standard atmosphere"));
    c.push(entry("AeroWindShear", "Wind Shear", cat, 3, 1, "Wind shear model"));
    c.push(entry("AeroAerodynamics", "Aerodynamic Forces", cat, 6, 3, "Compute aerodynamic forces and moments"));
    c.push(entry("AeroQuaternion", "Quaternion", cat, 2, 1, "Quaternion math operations"));
    c.push(entry("AeroDirectionCosine", "Direction Cosine", cat, 1, 1, "Direction cosine matrix"));
    c.push(entry("AeroEulerAngles", "Euler Angles", cat, 1, 3, "Convert to/from Euler angles"));
    c.push(entry("AeroIdealAccel", "Ideal Accelerometer", cat, 3, 1, "Ideal accelerometer sensor"));
    c.push(entry("AeroIdealGyro", "Ideal Gyroscope", cat, 2, 1, "Ideal gyroscope sensor"));
    c.push(entry("AeroSixDOF", "6DOF (Euler)", cat, 3, 4, "Six degrees of freedom (Euler angles)"));
    c.push(entry("AeroSixDOFQuat", "6DOF (Quaternion)", cat, 3, 4, "Six degrees of freedom (quaternion)"));
    c.push(entry("AeroLLAtoECEF", "LLA to ECEF", cat, 1, 1, "Geodetic to ECEF coordinates"));
    c.push(entry("AeroECEFtoLLA", "ECEF to LLA", cat, 1, 1, "ECEF to geodetic coordinates"));

    // ── Automotive Blockset ─────────────────────────────────────────────
    let cat = "Automotive";
    c.push(entry("AutoEngine", "Engine", cat, 2, 2, "Internal combustion engine model"));
    c.push(entry("AutoTransmission", "Transmission", cat, 2, 2, "Automatic/manual transmission"));
    c.push(entry("AutoTire", "Tire", cat, 2, 2, "Tire dynamics model"));
    c.push(entry("AutoBrake", "Brake System", cat, 2, 1, "Brake system model"));
    c.push(entry("AutoSteering", "Steering", cat, 1, 1, "Steering system model"));
    c.push(entry("AutoChassis", "Vehicle Chassis", cat, 4, 4, "Vehicle chassis dynamics"));
    c.push(entry("AutoDriveline", "Driveline", cat, 2, 2, "Driveline/drivetrain model"));
    c.push(entry("AutoBattery", "Battery", cat, 2, 2, "Battery model"));
    c.push(entry("AutoMotorElec", "Electric Motor", cat, 2, 2, "Electric motor model"));
    c.push(entry("AutoInverter", "Inverter", cat, 2, 2, "DC/AC inverter model"));

    // ── Communications Blockset ─────────────────────────────────────────
    let cat = "Communications";
    c.push(entry("CommAWGN", "AWGN Channel", cat, 1, 1, "Additive white Gaussian noise channel"));
    c.push(entry("CommBPSKMod", "BPSK Modulator", cat, 1, 1, "BPSK modulation"));
    c.push(entry("CommBPSKDemod", "BPSK Demodulator", cat, 1, 1, "BPSK demodulation"));
    c.push(entry("CommQPSKMod", "QPSK Modulator", cat, 1, 1, "QPSK modulation"));
    c.push(entry("CommQPSKDemod", "QPSK Demodulator", cat, 1, 1, "QPSK demodulation"));
    c.push(entry("CommQAMMod", "QAM Modulator", cat, 1, 1, "QAM modulation"));
    c.push(entry("CommQAMDemod", "QAM Demodulator", cat, 1, 1, "QAM demodulation"));
    c.push(entry("CommFMMod", "FM Modulator", cat, 1, 1, "FM modulation"));
    c.push(entry("CommFMDemod", "FM Demodulator", cat, 1, 1, "FM demodulation"));
    c.push(entry("CommAMMod", "AM Modulator", cat, 1, 1, "AM modulation"));
    c.push(entry("CommAMDemod", "AM Demodulator", cat, 1, 1, "AM demodulation"));
    c.push(entry("CommConvEnc", "Convolutional Encoder", cat, 1, 1, "Convolutional encoding"));
    c.push(entry("CommViterbi", "Viterbi Decoder", cat, 1, 1, "Viterbi decoding"));
    c.push(entry("CommCRC", "CRC Generator", cat, 1, 1, "CRC generation"));
    c.push(entry("CommInterleaver", "Interleaver", cat, 1, 1, "Block interleaving"));
    c.push(entry("CommRaisedCosine", "Raised Cosine Filter", cat, 1, 1, "Raised cosine TX/RX filter"));

    // ── Phased Array / Radar ────────────────────────────────────────────
    let cat = "Phased Array";
    c.push(entry("PhasedULA", "ULA", cat, 1, 1, "Uniform linear array"));
    c.push(entry("PhasedURA", "URA", cat, 1, 1, "Uniform rectangular array"));
    c.push(entry("PhasedSteeringVector", "Steering Vector", cat, 1, 1, "Array steering vector"));
    c.push(entry("PhasedBeamformer", "Beamformer", cat, 1, 1, "Beamforming"));
    c.push(entry("PhasedMatchedFilter", "Matched Filter", cat, 1, 1, "Matched filtering"));
    c.push(entry("PhasedCFAR", "CFAR Detector", cat, 1, 1, "CFAR detection"));
    c.push(entry("PhasedWaveform", "Waveform Generator", cat, 0, 1, "Radar waveform generation"));

    // ── Robotics ────────────────────────────────────────────────────────
    let cat = "Robotics";
    c.push(entry("RobotRigidBody", "Rigid Body", cat, 2, 2, "Rigid body dynamics"));
    c.push(entry("RobotJoint", "Joint", cat, 2, 2, "Joint (revolute/prismatic)"));
    c.push(entry("RobotForwardKin", "Forward Kinematics", cat, 1, 1, "Forward kinematics solver"));
    c.push(entry("RobotInverseKin", "Inverse Kinematics", cat, 1, 1, "Inverse kinematics solver"));
    c.push(entry("RobotTrajectory", "Trajectory Generator", cat, 1, 1, "Trajectory planning"));
    c.push(entry("RobotCollision", "Collision Check", cat, 2, 1, "Collision detection between bodies"));
    c.push(entry("RobotTransformTree", "Transform Tree", cat, 1, 1, "Coordinate frame tree"));
    c.push(entry("RobotPID", "Joint PID", cat, 2, 1, "Joint-level PID controller"));

    // ── Image Processing / Computer Vision ──────────────────────────────
    let cat = "Computer Vision";
    c.push(entry("CVImageRead", "Image Source", cat, 0, 1, "Read image data"));
    c.push(entry("CVImageDisplay", "Image Display", cat, 1, 0, "Display image"));
    c.push(entry("CVColorConvert", "Color Space Conversion", cat, 1, 1, "Convert color spaces (RGB/HSV/YCbCr)"));
    c.push(entry("CVEdgeDetect", "Edge Detection", cat, 1, 1, "Detect edges (Sobel/Canny)"));
    c.push(entry("CVMedianFilter", "Median Filter", cat, 1, 1, "Median image filter"));
    c.push(entry("CVGaussianFilter", "Gaussian Filter", cat, 1, 1, "Gaussian image blur"));
    c.push(entry("CVMorphology", "Morphological Op", cat, 1, 1, "Morphological operation (dilate/erode)"));
    c.push(entry("CVTemplate", "Template Matching", cat, 2, 1, "Template matching"));
    c.push(entry("CVOpticalFlow", "Optical Flow", cat, 1, 1, "Optical flow estimation"));
    c.push(entry("CVHoughLines", "Hough Lines", cat, 1, 1, "Hough line transform"));

    // ── Deep Learning ──────────────────────────────────────────────────
    let cat = "Deep Learning";
    c.push(entry("DLConv2D", "2-D Convolution", cat, 1, 1, "2-D convolutional layer"));
    c.push(entry("DLMaxPool", "Max Pooling", cat, 1, 1, "Max pooling layer"));
    c.push(entry("DLAvgPool", "Average Pooling", cat, 1, 1, "Average pooling layer"));
    c.push(entry("DLFullyConnected", "Fully Connected", cat, 1, 1, "Fully connected (dense) layer"));
    c.push(entry("DLReLU", "ReLU Layer", cat, 1, 1, "Rectified linear unit activation"));
    c.push(entry("DLSoftmax", "Softmax Layer", cat, 1, 1, "Softmax normalization"));
    c.push(entry("DLBatchNorm", "Batch Normalization", cat, 1, 1, "Batch normalization layer"));
    c.push(entry("DLDropout", "Dropout Layer", cat, 1, 1, "Dropout regularization layer"));
    c.push(entry("DLLSTM", "LSTM Layer", cat, 1, 1, "Long short-term memory layer"));
    c.push(entry("DLGRU", "GRU Layer", cat, 1, 1, "Gated recurrent unit layer"));

    // ── Power Electronics ───────────────────────────────────────────────
    let cat = "Power Electronics";
    c.push(entry("PEMosfet", "MOSFET", cat, 2, 2, "MOSFET switch model"));
    c.push(entry("PEIGBT", "IGBT", cat, 2, 2, "IGBT switch model"));
    c.push(entry("PEDiode", "Power Diode", cat, 1, 1, "Power diode model"));
    c.push(entry("PEBuckConverter", "Buck Converter", cat, 2, 1, "DC-DC buck converter"));
    c.push(entry("PEBoostConverter", "Boost Converter", cat, 2, 1, "DC-DC boost converter"));
    c.push(entry("PEBuckBoost", "Buck-Boost", cat, 2, 1, "DC-DC buck-boost converter"));
    c.push(entry("PEFullBridge", "Full Bridge", cat, 2, 2, "H-bridge inverter"));
    c.push(entry("PEThreePhaseInv", "3-Phase Inverter", cat, 2, 3, "Three-phase voltage source inverter"));
    c.push(entry("PEPWMGenerator", "PWM Generator", cat, 1, 1, "PWM signal generator"));
    c.push(entry("PERectifier", "Rectifier", cat, 1, 1, "AC/DC rectifier"));

    // ── Navigation / GPS ────────────────────────────────────────────────
    let cat = "Navigation";
    c.push(entry("NavINS", "INS", cat, 3, 3, "Inertial navigation system"));
    c.push(entry("NavGPS", "GPS Receiver", cat, 1, 3, "GPS receiver model"));
    c.push(entry("NavIMU", "IMU", cat, 3, 3, "Inertial measurement unit"));
    c.push(entry("NavKalman", "Navigation Kalman", cat, 3, 3, "Navigation Kalman filter"));
    c.push(entry("NavMagnetometer", "Magnetometer", cat, 1, 1, "Magnetometer sensor model"));
    c.push(entry("NavBarometer", "Barometer", cat, 1, 1, "Barometric altimeter"));

    // ── RF / Microwave ──────────────────────────────────────────────────
    let cat = "RF";
    c.push(entry("RFAmplifier", "RF Amplifier", cat, 1, 1, "RF amplifier model"));
    c.push(entry("RFMixer", "RF Mixer", cat, 2, 1, "RF mixer"));
    c.push(entry("RFFilter", "RF Filter", cat, 1, 1, "RF bandpass/lowpass filter"));
    c.push(entry("RFOscillator", "Oscillator", cat, 0, 1, "RF oscillator"));
    c.push(entry("RFPLL", "Phase-Locked Loop", cat, 1, 1, "Phase-locked loop"));
    c.push(entry("RFVCO", "VCO", cat, 1, 1, "Voltage-controlled oscillator"));
    c.push(entry("RFSParameter", "S-Parameter", cat, 1, 1, "S-parameter block"));

    // ── Verification / Test ─────────────────────────────────────────────
    let cat = "Verification";
    c.push(entry("TestInput", "Test Input", cat, 0, 1, "Test harness input signal"));
    c.push(entry("TestOutput", "Test Assessment", cat, 1, 0, "Test output assessment"));
    c.push(entry("TestScenario", "Test Scenario", cat, 0, 1, "Test scenario definition"));
    c.push(entry("TestProbe", "Test Probe", cat, 1, 1, "Test probe point"));
    c.push(entry("TestAssumption", "Test Assumption", cat, 1, 0, "Assumption for design verifier"));
    c.push(entry("TestObjective", "Test Objective", cat, 1, 0, "Objective for design verifier"));

    // ── Thermal ─────────────────────────────────────────────────────────
    let cat = "Thermal";
    c.push(entry("ThermalCapacitor", "Thermal Capacitor", cat, 1, 1, "Thermal capacitance element"));
    c.push(entry("ThermalConduction", "Conductive Heat", cat, 2, 1, "Conductive heat transfer"));
    c.push(entry("ThermalConvection", "Convective Heat", cat, 2, 1, "Convective heat transfer"));
    c.push(entry("ThermalRadiation", "Radiative Heat", cat, 2, 1, "Radiative heat transfer"));
    c.push(entry("ThermalSensor", "Temp Sensor", cat, 1, 1, "Temperature sensor"));
    c.push(entry("ThermalSource", "Heat Source", cat, 0, 1, "Ideal heat flow source"));
    c.push(entry("ThermalRef", "Thermal Reference", cat, 0, 1, "Thermal reference (ambient)"));
    c.push(entry("ThermalResistor", "Thermal Resistor", cat, 2, 1, "Thermal resistance element"));

    // Ensure at least 750
    debug_assert!(
        c.len() >= 750,
        "Block catalog has only {} entries, need at least 750",
        c.len()
    );

    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_at_least_750_entries() {
        let catalog = get_block_catalog();
        assert!(
            catalog.len() >= 750,
            "Expected >= 750 blocks, got {}",
            catalog.len()
        );
    }

    #[test]
    fn catalog_entries_have_non_empty_fields() {
        for entry in get_block_catalog() {
            assert!(!entry.block_type.is_empty(), "Empty block_type");
            assert!(!entry.display_name.is_empty(), "Empty display_name for {}", entry.block_type);
            assert!(!entry.category.is_empty(), "Empty category for {}", entry.block_type);
            assert!(!entry.description.is_empty(), "Empty description for {}", entry.block_type);
        }
    }

    #[test]
    fn catalog_unique_block_types() {
        let catalog = get_block_catalog();
        let mut seen = std::collections::HashSet::new();
        for entry in catalog {
            assert!(
                seen.insert(&entry.block_type),
                "Duplicate block_type: {}",
                entry.block_type
            );
        }
    }

    #[test]
    fn catalog_search_finds_gain() {
        let catalog = get_block_catalog();
        let matches: Vec<_> = catalog.iter().filter(|e| e.matches_query("gain")).collect();
        assert!(!matches.is_empty(), "Should find blocks matching 'gain'");
    }

    #[test]
    fn catalog_search_empty_returns_all() {
        let catalog = get_block_catalog();
        let matches: Vec<_> = catalog.iter().filter(|e| e.matches_query("")).collect();
        assert_eq!(matches.len(), catalog.len());
    }

    #[test]
    fn catalog_by_category_is_consistent() {
        let categories = get_block_catalog_by_category();
        let total: usize = categories.iter().map(|c| c.entries.len()).sum();
        assert_eq!(total, get_block_catalog().len());
    }

    #[test]
    fn catalog_categories_are_non_empty() {
        for cat in get_block_catalog_by_category() {
            assert!(!cat.name.is_empty());
            assert!(!cat.entries.is_empty(), "Category '{}' is empty", cat.name);
        }
    }
}
