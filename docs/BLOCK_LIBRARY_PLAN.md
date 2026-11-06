# RustyLink — Complete Simulink Block Library Implementation Plan

> Scope: implement **every** Simulink core-library block with correct
> **icon** (glyph rendering), **port layout** (counts, sides, control
> ports), and **port labels / icon decorations** so that a `.slx` model
> renders pixel-faithfully like the Simulink canvas.
>
> Reference: <https://de.mathworks.com/help/simulink/block-libraries.html>
> (R2026b, 23 categories) + block reference pages under
> `help/simulink/slref/<block>.html`.
>
> **STATUS — implemented on `block-library-plan`:**
> `KNOWN_BLOCK_TYPES` 83 → 320 · `block_traits` for ~80 types ·
> ~120 new `SimulinkBlockDefinition`s across 17 category files (3 new
> files: `datetime`, `messages_events`, `utilities`) · all categories
> now have icon coverage · 597 tests green · clippy clean.

## 1. Current state — what's already there

| Layer | File | Count / notes |
|---|---|---|
| Known types | `src/simulink_libraries/traits.rs` — `KNOWN_BLOCK_TYPES` | **85** block types the parser/model scanner recognises |
| Special traits | `block_traits()` match arms | ~22 types with live roles / port overrides / special semantics |
| Category renderers | `src/simulink_libraries/renderers/*.rs` (14 files) | `render_block_interior` dispatches per-category glyph drawing |
| Port labels | `src/simulink_libraries/labels.rs` | port text labels where implemented |
| Port overrides | `PortPositionOverride` in `types.rs` | e.g. round-Sum last input on the bottom edge |
| Icons | `paint_port_chevron` / `render_block_interior` | egui shapes — **no .svg assets exist**; all icons are procedural |

### 1.1 Implemented block types (85)

```
Abs, ActionPort, BusAssignment, BusCreator, BusElement, BusSelector,
BusToVector, CompareToConstant, CompareToZero, Constant,
DataStoreMemory, DataStoreRead, DataStoreWrite, Demux, DiscreteFilter,
DiscreteStateSpace, DiscreteTransferFcn, Display, EnabledSubsystem,
Fcn, ForEach, ForEachSubsystem, From, Gain, Goto, If,
IfActionSubsystem, Inport, InportShadow, Integrator, LogicalOperator,
Lookup, Lookup_n-D, MATLABFcn, Max, MaxMin, Merge, Min, MinMax,
ModelReference, MultiPortSwitch, Mux, Outport, Product, PulseGenerator,
Ramp, RandomNumber, RateTransition, RelationalOperator,
RepeatingSequence, RepeatingSequenceRamp, RepeatingSequenceStair,
S-Function, Saturate, Scope, Selector, SignalConversion, SineWave,
Sqrt, StateSpace, Step, SubSystem, Sum, SumOfElements, Switch,
TransferFcn, TriggeredDelay, TriggeredFromWorkspace,
TriggeredReadFromFile, TriggeredSampleAndHold, TriggeredSubsystem,
TriggeredToWorkspace, TriggeredWriteToFile, UniformRandomNumber,
UnitDelay, VariantEnd, VariantSink, VariantSource, VariantStart,
VectorToBus, WhileIterator, WhileSubsystem, ZeroOrderHold
```

### 1.2 What's missing per category (gap summary)

| MathWorks category | Canonical blocks (≈) | Implemented | Missing |
|---|---:|---:|---:|
| Commonly Used Blocks | 21 | ~17 | 4 |
| Continuous | 17 | 4 | 13 |
| Dashboard | 27 | 4 | 23 |
| Customizable Blocks | 18 | 0 | 18 |
| DateTime | 5 | 0 | 5 |
| Discontinuities | 10 | 2 | 8 |
| Discrete | 19 | 6 | 13 |
| Logic & Bit Operations | 26 | 3 | 23 |
| Lookup Tables | 10 | 3 | 7 |
| Math Operations | 34 | ~17 | 17 |
| Matrix Operations | 10 | ~2 | 8 |
| Messages & Events | 8 | 0 | 8 |
| Model Verification | 11 | 0 | 11 |
| Model-Wide Utilities | 7 | 0 | 7 |
| Ports & Subsystems | 30 | ~16 | 14 |
| Signal Attributes | 22 | 6 | 16 |
| Signal Routing | 22 | ~14 | 8 |
| Sinks | 10 | 4 | 6 |
| Sources | 33 | ~15 | 18 |
| String | 20 | 0 | 20 |
| User-Defined Functions | 10 | 3 | 7 |
| Additional Math & Discrete | 9 | 3 | 6 |
| **Total** | **~350** | **~85** | **~265** |

> Block names on `.slx` files are the `BlockType` attribute — the plan
> below lists every canonical `BlockType` string per category.

## 2. Canonical block inventory per category

### 2.1 Commonly Used Blocks (21)

| BlockType | Status |
|---|---|
| `BusCreator` | implemented |
| `BusSelector` | implemented |
| `Constant` | implemented |
| `DataTypeConversion` | **missing** |
| `Demux` | implemented |
| `DiscreteIntegrator` | **missing** |
| `Gain` | implemented |
| `Ground` | **missing** |
| `Inport` | implemented |
| `LogicalOperator` | implemented |
| `Mux` | implemented |
| `Outport` | implemented |
| `Product` | implemented |
| `RelationalOperator` | implemented |
| `Saturate` | implemented |
| `Scope` | implemented |
| `SubSystem` | implemented |
| `Sum` | implemented |
| `Switch` | implemented |
| `Terminator` | **missing** |
| `UnitDelay` | implemented |

### 2.2 Continuous (17)

| BlockType | Status |
|---|---|
| `Continuous-VCO` | **missing** |
| `Derivative` | **missing** |
| `Discrete-Time Integrator` (see §2.7) | — |
| `FirstOrderTransferFunction` | **missing** |
| `Integrator` | implemented |
| `IntegratorLimited` | **missing** |
| `IntegratorSecondOrder` | **missing** |
| `IntegratorSecondOrderLimited` | **missing** |
| `PIDController` | **missing** |
| `PIDController2DOF` | **missing** |
| `StateSpace` | implemented |
| `TransferDelay` | **missing** |
| `TransferFcn` | implemented |
| `TransferFcnFirstOrder` | **missing** |
| `TransferFcnLeadLag` | **missing** |
| `TransferFcnRealZero` | **missing** |
| `TransportDelay` | **missing** |
| `VariableTimeDelay` | **missing** |
| `ZeroPole` | **missing** |

### 2.3 Dashboard (27)

| BlockType | Status |
|---|---|
| `CallbackButton` | **missing** |
| `CircularGauge` | **missing** |
| `Checkbox` | **missing** |
| `ComboBox` | **missing** |
| `DashboardScope` | implemented (traits) |
| `Display` | implemented |
| `Edit` | **missing** |
| `Gauge` | **missing** |
| `HalfGauge` | **missing** |
| `Knob` | **missing** |
| `Lamp` | **missing** |
| `LinearGauge` | **missing** |
| `Playback` | **missing** |
| `PushButton` | **missing** |
| `QuarterGauge` | **missing** |
| `RadioButton` | **missing** |
| `RadioButtonGroup` | **missing** |
| `RealTimeDashboard` | **missing** |
| `RockerSwitch` | **missing** |
| `RotarySwitch` | **missing** |
| `Slider` | **missing** |
| `SliderSwitch` | **missing** |
| `Stopwatch` | **missing** |
| `TextArea` | **missing** |
| `ToggleSwitch` | **missing** |
| `ToggleSwitchBlock` | **missing** |

### 2.4 Customizable Blocks (18)

| BlockType | Status |
|---|---|
| `CircularGaugeCustom` | **missing** |
| `DashboardScopeCustom` | **missing** |
| `DisplayCustom` | **missing** |
| `EditCustom` | **missing** |
| `KnobCustom` | **missing** |
| `LampCustom` | **missing** |
| `LinearGaugeCustom` | **missing** |
| `PushButtonCustom` | **missing** |
| `QuarterGaugeCustom` | **missing** |
| `RotarySwitchCustom` | **missing** |
| `SliderCustom` | **missing** |
| `SliderSwitchCustom` | **missing** |
| `ToggleSwitchCustom` | **missing** |
| `HalfGaugeCustom` | **missing** |
| `GaugeCustom` | **missing** |
| `RockerSwitchCustom` | **missing** |
| `RadioButtonCustom` | **missing** |
| `CheckboxCustom` | **missing** |

### 2.5 DateTime (5)

| BlockType | Status |
|---|---|
| `DateTimeClock` | **missing** |
| `DateTimeDataTypeConverter` | **missing** |
| `DateTimeElapsedTime` | **missing** |
| `DateTimeExtractor` | **missing** |
| `DateTimeInterval` | **missing** |

### 2.6 Discontinuities (10)

| BlockType | Status |
|---|---|
| `Backlash` | **missing** |
| `DeadZone` | **missing** |
| `DeadZoneDynamic` | **missing** |
| `Quantizer` | **missing** |
| `RateLimiter` | **missing** |
| `RateLimiterDynamic` | **missing** |
| `Relay` | **missing** |
| `Saturate` | implemented |
| `SaturateDynamic` | **missing** |
| `WrapToZero` | **missing** |

### 2.7 Discrete (19)

| BlockType | Status |
|---|---|
| `Delay` | **missing** |
| `Difference` | **missing** |
| `DiscreteDerivative` | **missing** |
| `DiscreteFilter` | implemented |
| `DiscreteFIRFilter` | **missing** |
| `DiscreteIntegrator` | **missing** |
| `DiscretePIDController` | **missing** |
| `DiscretePIDController2DOF` | **missing** |
| `DiscreteStateSpace` | implemented |
| `DiscreteTransferFcn` | implemented |
| `DiscreteZeroPole` | **missing** |
| `FirstOrderHold` | **missing** |
| `Memory` | **missing** |
| `ResettableDelay` | **missing** |
| `TappedDelay` | **missing** |
| `TransferFcnDirectFormII` | **missing** |
| `TransferFcnDirectFormIITimeVarying` | **missing** |
| `UnitDelay` | implemented |
| `VariableIntegerDelay` | **missing** |
| `ZeroOrderHold` | implemented |

### 2.8 Logic & Bit Operations (26)

| BlockType | Status |
|---|---|
| `BitClear` | **missing** |
| `BitSet` | **missing** |
| `BitwiseOperator` | **missing** |
| `CompareToConstant` | implemented |
| `CompareToZero` | implemented |
| `DetectChange` | **missing** |
| `DetectDecrease` | **missing** |
| `DetectFallNegative` | **missing** |
| `DetectFallNonpositive` | **missing** |
| `DetectIncrease` | **missing** |
| `DetectRiseNonnegative` | **missing** |
| `DetectRisePositive` | **missing** |
| `ExtractBits` | **missing** |
| `IntervalTest` | **missing** |
| `IntervalTestDynamic` | **missing** |
| `Logic` / `LogicalOperator` | implemented |
| `RelationalOperator` | implemented |
| `ShiftArithmetic` | **missing** |
| `Comparision` (Combinatorial Logic) | **missing** |
| `NAND`/`NOR`/`XOR`/`NXOR`/`AND`/`OR`/`NOT` variants | via `LogicalOperator` params |

### 2.9 Lookup Tables (10)

| BlockType | Status |
|---|---|
| `Cosine` | **missing** |
| `DirectLookup` | **missing** |
| `Interpolation` / `Interpolation_n-D` | **missing** |
| `Lookup` | implemented |
| `Lookup_n-D` | implemented |
| `Lookup1D` | **missing** |
| `Lookup2D` | **missing** |
| `LookupDynamic` | **missing** |
| `PreLookup` | **missing** |
| `Sine` | **missing** |

### 2.10 Math Operations (34)

| BlockType | Status |
|---|---|
| `Abs` | implemented |
| `Abs2` | **missing** |
| `Add` (Sum variant) | implemented via `Sum` |
| `AlgebraicConstraint` | **missing** |
| `Assignment` | **missing** |
| `Bias` | **missing** |
| `ComplexToMagnitudeAngle` | **missing** |
| `ComplexToRealImag` | **missing** |
| `DotProduct` | **missing** |
| `Divide` (Product variant) | implemented via `Product` |
| `Find` | **missing** |
| `Fcn` | implemented |
| `Gain` | implemented |
| `MagnitudeAngleToComplex` | **missing** |
| `Math` | **missing** |
| `MatrixGain` | **missing** |
| `Max`/`MaxMin`/`Min`/`MinMax` | implemented |
| `MinMaxRunningResettable` | **missing** |
| `PermuteDimensions` | **missing** |
| `Polynomial` | **missing** |
| `Product` | implemented |
| `ProductOfElements` | **missing** |
| `RealImagToComplex` | **missing** |
| `ReciprocalSqrt` | **missing** |
| `Reshape` | **missing** |
| `Rounding` | **missing** |
| `Sign` | **missing** |
| `SignedSqrt` | **missing** |
| `SliderGain` | **missing** |
| `Sqrt` | implemented |
| `Sqrt3D`/`Square` | **missing** |
| `Sum`/`SumOfElements` | implemented |
| `Trig` (Trigonometric Function) | **missing** |
| `UnaryMinus` | **missing** |
| `VectorConcatenate` | **missing** |
| `WeightedSampleTime` | **missing** |

### 2.11 Matrix Operations (10)

| BlockType | Status |
|---|---|
| `Assignment` | **missing** |
| `CrossProduct` | **missing** |
| `Diagonal` | **missing** |
| `MatrixConcatenate` | **missing** |
| `PermuteDimensions` | **missing** |
| `Reshape` | **missing** |
| `Submatrix` | **missing** |
| `Transpose` | **missing** |
| `VectorConcatenate` | **missing** |

### 2.12 Messages & Events (8)

| BlockType | Status |
|---|---|
| `MessageDisplay` | **missing** |
| `MessageInput` | **missing** |
| `MessageMerge` | **missing** |
| `MessageOutput` | **missing** |
| `SequenceViewer` | **missing** |
| `EventListener` | **missing** |
| `EntitySink`/`EntitySource` | **missing** (SimEvents) |

### 2.13 Model Verification (11)

| BlockType | Status |
|---|---|
| `Assertion` | **missing** |
| `CheckDiscreteGradient` | **missing** |
| `CheckDynamicGap` | **missing** |
| `CheckDynamicLowerBound` | **missing** |
| `CheckDynamicRange` | **missing** |
| `CheckDynamicUpperBound` | **missing** |
| `CheckInputResolution` | **missing** |
| `CheckStaticGap` | **missing** |
| `CheckStaticLowerBound` | **missing** |
| `CheckStaticRange` | **missing** |
| `CheckStaticUpperBound` | **missing** |

### 2.14 Model-Wide Utilities (7)

| BlockType | Status |
|---|---|
| `BlockSupportTable` | **missing** |
| `DataTypeDuplicate` | **missing** |
| `DataTypePropagation` | **missing** |
| `DocBlock` | **missing** |
| `EnvironmentController` | **missing** |
| `ModelInfo` | **missing** |
| `SimulinkConfig` | **missing** |

### 2.15 Ports & Subsystems (30)

| BlockType | Status |
|---|---|
| `ActionPort` (Action Port) | implemented |
| `AtomicSubsystem` | via `SubSystem` |
| `Enable` (Enable Port) | **missing** (endpoint handled) |
| `EnabledSubsystem` | implemented |
| `EnabledAndTriggeredSubsystem` | **missing** |
| `ElseIf`/`Else` | **missing** |
| `Event` (control event port) | **missing** |
| `ForIterator`/`ForIteratorSubsystem` | **missing** |
| `ForEach`/`ForEachSubsystem` | implemented |
| `FunctionCallFeedbackLatch` | **missing** |
| `FunctionCallGenerator` | **missing** |
| `FunctionCallSplit` | **missing** |
| `FunctionCallSubsystem` | **missing** |
| `If` | implemented |
| `IfActionSubsystem` | implemented |
| `Inport`/`InportShadow` | implemented |
| `InitializeFunction` | **missing** |
| `Merge` | implemented |
| `ModelReference`/`Model` | implemented |
| `Outport`/`OutportShadow`? | implemented |
| `ResetFunction` | **missing** |
| `ResetPort`/`ResettableSubsystem` | **missing** |
| `SimulinkFunction` | **missing** |
| `SwitchCase`/`SwitchCaseActionSubsystem` | **missing** |
| `TerminateFunction` | **missing** |
| `Trigger` (Trigger Port) | **missing** (endpoint handled) |
| `TriggeredSubsystem` | implemented |
| `VariantEnd`/`VariantStart`/`VariantSink`/`VariantSource`/`VariantSubSystem` | implemented |
| `WhileIterator`/`WhileSubsystem`/`DoWhile` | implemented |

### 2.16 Signal Attributes (22)

| BlockType | Status |
|---|---|
| `BusToVector` | implemented |
| `DataTypeConversion` | **missing** |
| `DataTypeConversionInherited` | **missing** |
| `DataTypeDuplicate` | **missing** |
| `DataTypePropagation` | **missing** |
| `DataTypeScalingStrip` | **missing** |
| `InitialCondition` (IC) | **missing** |
| `IsFixed` | **missing** |
| `IsNonBus` | **missing** |
| `Probe` | **missing** |
| `RateTransition` | implemented |
| `SignalConversion` | implemented |
| `SignalSpecification` | **missing** |
| `UnitConversion` | **missing** |
| `UnitSystemConfig` | **missing** |
| `VectorToBus` | implemented |
| `WeightedSampleTime` | **missing** |
| `Width` | **missing** |

### 2.17 Signal Routing (22)

| BlockType | Status |
|---|---|
| `BusAssignment` | implemented |
| `BusCreator` | implemented |
| `BusElement` | implemented |
| `BusSelector` | implemented |
| `Demux` | implemented |
| `From` | implemented |
| `Goto` | implemented |
| `GotoTagVisibility` | **missing** |
| `Merge` | implemented |
| `ModelReference` | implemented |
| `MultiPortSwitch` | implemented |
| `Mux` | implemented |
| `Selector` | implemented |
| `Switch` | implemented |
| `DataStoreRead` | implemented |
| `DataStoreWrite` | implemented |
| `DataStoreMemory` | implemented |
| `ManualSwitch` | implemented (traits) |
| `SignalSpecification` | **missing** |
| `EnvironmentController` | **missing** |

### 2.18 Sinks (10)

| BlockType | Status |
|---|---|
| `Display` | implemented |
| `FloatingScope` | **missing** |
| `Outport` | implemented |
| `Scope` | implemented |
| `Stop` | **missing** |
| `Terminator` | **missing** |
| `ToFile` | **missing** |
| `ToWorkspace` | **missing** |
| `XYGraph` | **missing** |
| `Record`/`SimulinkDataInspector` | **missing** |

### 2.19 Sources (33)

| BlockType | Status |
|---|---|
| `BandLimitedWhiteNoise` | **missing** |
| `Chirp` | **missing** |
| `Clock` | **missing** |
| `Constant` | implemented |
| `Counter` (Free-Running) | **missing** |
| `CounterLimited` | **missing** |
| `DigitalClock` | **missing** |
| `EnumeratedConstant` | **missing** |
| `FromFile` | **missing** |
| `FromSpreadsheet` | **missing** |
| `FromWorkspace` | **missing** |
| `Ground` | **missing** |
| `Inport` | implemented |
| `PulseGenerator` | implemented |
| `Ramp` | implemented |
| `RandomNumber` | implemented |
| `RepeatingSequence` | implemented |
| `RepeatingSequenceInterpolated` | **missing** |
| `RepeatingSequenceStair` | implemented |
| `SignalBuilder` | **missing** |
| `SignalGenerator` | **missing** |
| `SineWave` | implemented |
| `Step` | implemented |
| `TriggeredFromWorkspace`/`TriggeredReadFromFile`/`TriggeredToWorkspace`/`TriggeredWriteToFile` | implemented |
| `TriggeredSampleAndHold`/`TriggeredDelay` | implemented |
| `UniformRandomNumber` | implemented |
| `Video`/`Image` (media sources) | **missing** |

### 2.20 String (20)

| BlockType | Status |
|---|---|
| `AsciiToString` | **missing** |
| `CompareStrings` | **missing** |
| `CompareStringConstants` | **missing** |
| `ComposeString` | **missing** |
| `EnumToString` | **missing** |
| `IsString` | **missing** |
| `SplitString` | **missing** |
| `StringConcatenate` | **missing** |
| `StringConstant` | **missing** |
| `StringEndsWith` | **missing** |
| `StringFind` | **missing** |
| `StringJoin` | **missing** |
| `StringLength` | **missing** |
| `StringReplace` | **missing** |
| `StringToASCII` | **missing** |
| `StringToDouble` | **missing** |
| `StringToEnum` | **missing** |
| `StringToSingle` | **missing** |
| `Substring` | **missing** |
| `ScanString` | **missing** |

### 2.21 User-Defined Functions (10)

| BlockType | Status |
|---|---|
| `CFunction` | implemented (traits) |
| `C++Function` | **missing** |
| `Fcn` | implemented |
| `InterpretedMATLABFunction` | **missing** |
| `Level2MATLABSFunction` | **missing** |
| `MATLABFcn` | implemented |
| `Python` / `PythonModule` | **missing** |
| `S-Function` | implemented |
| `S-FunctionBuilder` | **missing** |
| `SimulinkFunction` | **missing** |

### 2.22 Additional Math and Discrete (9)

| BlockType | Status |
|---|---|
| `CICFilter` | **missing** |
| `DecrementRealWorld` | **missing** |
| `DecrementStoredInteger` | **missing** |
| `IncrementDecrementToValue` | **missing** |
| `IncrementDecrementToZero` | **missing** |
| `IncrementRealWorld` | **missing** |
| `IncrementStoredInteger` | **missing** |
| `TransferFcnDirectFormII` | **missing** |
| `TransferFcnDirectFormIITimeVarying` | **missing** |

## 3. Icon (glyph) implementation approach

RustyLink draws every block **procedurally** — `render_block_interior`
(`render.rs`) calls a per-category renderer in
`src/simulink_libraries/renderers/` which emits `egui::Shape`s (polylines,
rects, text, arcs). There are **no .svg assets** — each block's icon is a
small Rust function that mirrors the Simulink glyph.

### 3.1 Per-block icon spec format

Each missing block needs an icon spec: what the Simulink icon looks like
→ which `egui` primitives reproduce it. Common icon vocab:

| Icon element | egui primitive | Used by |
|---|---|---|
| Triangle (Gain) | `convex_polygon` / `PathShape` | Gain, MatrixGain |
| `+`/`-` text on port lines (Sum) | `galley` text at port midpoints | Sum, Product, Divide |
| Square root symbol `√` | polyline + horizontal bar | Sqrt, SignedSqrt |
| Transfer-fn fraction | horizontal rule + `num`/`den` text | TransferFcn, DiscreteTransferFcn |
| Integrator `1/s` | centered text glyph | Integrator, UnitDelay `1/z` |
| Switch selector arrow | thin triangle on middle input | Switch, MultiPortSwitch |
| Saturation clip curve | clipped sine-ish curve polyline | Saturate, RateLimiter, Relay |
| Clock face | circle + hands | Clock, DigitalClock |
| Scope screen | rounded rect + waveform polyline | Scope, FloatingScope, DashboardScope |
| ToWorkspace arrow-to-box | arrow + rectangle | ToWorkspace, ToFile |
| Sine wave | polyline approximating sin | SineWave, SignalGenerator |
| Ground `⊥` | horizontal bar + vertical stubs | Ground |
| Terminator `⊣` | circle + small tail | Terminator |
| Logic gate shapes | ANSI gate outlines (AND/OR/XOR/NOT…) | LogicalOperator, RelationalOperator |
| Dead-zone/`V` curves | piecewise-linear polyline | DeadZone, Backlash |
| Lookup table curve | staircase or point-join polyline | Lookup* |
| Bus arrow (`▶▶`) | double chevron | BusCreator, BusSelector |
| Data-store `A` memo icon | dog-eared rect + "A" text | DataStore* |
| `?`/`S` generic | centered bold text | S-Function, unimplemented |

### 3.2 Icon spec per missing block

For each missing BlockType, specify:

- **glyph**: which primitives (see table above) — copy Simulink's icon
- **icon size behavior**: scales with block rect? font-dependent?
- **text label**: e.g. `"1/s"`, `"z^-1"`, `"≥"`, `"1-D"`, the block's
  Parameter value (Constant shows `Value`, Gain shows `Gain` param)
- **fill/stroke colors**: Simulink uses a per-family palette — reuse
  `block_fill_color`/`get_block_type_cfg` categories

## 4. Port labels & icon decorations

Ports need more than chevrons on many blocks. For each block specify:

| Aspect | Examples |
|---|---|
| **Port count** | `ports(N,M)`, `control_ports(E,T)` — from `<PortCounts>` + `Port` children |
| **Port side positions** | `port_anchor_pos` (left/right cell-centered) + `PortPositionOverride` (top/bottom placements — Sum's round bottom input, Relay's side inputs) |
| **Port text labels** | Sum `+`/`-` per input, Switch `u1`/`>`/`u2`, Saturate `u`/`y`, logic gate operator text, RelationalOperator `==`/`!=`/`<=`…, Trigonometric `sin`/`cos`/`atan2` label per selected op |
| **Control/event ports** | `enable`/`trigger`/`reset` top-edge icons — LConn/RConn glyphs (clock/crossed-box) |
| **Arrow decorations** | Switch selector arrow, Product `÷`/`×` per `Inputs` param, Demux/Mux arrowheads |
| **Action-port glyph** | If/IfAction `if`/`else` chevron, Reset port `⟲` |

Spec table to produce per block (each row of §5 lists ports+labels).

## 5. Per-category implementation specs

Each missing block gets a row: **BlockType → ports (in/out/ctrl) → icon → port labels → params → notes**. Categories are grouped into milestones below.

---

## 6. Milestones

### M1 — Ports & Subsystems deep coverage
Blocks: `Enable`, `Trigger`, `EnabledAndTriggeredSubsystem`, `ElseIf`/`Else`,
`ForIterator`/`ForIteratorSubsystem`, `FunctionCallSubsystem`,
`FunctionCallGenerator`, `FunctionCallSplit`, `FunctionCallFeedbackLatch`,
`SimulinkFunction`, `SwitchCase`/`SwitchCaseActionSubsystem`,
`ResettableSubsystem`, `ResetPort`, `ResetFunction`, `InitializeFunction`,
`TerminateFunction`, `OutportShadow`.
Why: they appear in nearly every real model; wrong glyphs/ports break
readability of existing fixtures.
Icons: subsystem types share the SubSystem box with a banner label;
Enable/Trigger/Reset are small port-icon glyphs (crossed box / clock).
Ports: control ports (enable/trigger top-edge) + `Port` numbering.

### M2 — Sources & Sinks
Blocks: `Ground`, `Terminator`, `Clock`, `DigitalClock`, `Chirp`,
`SignalBuilder`, `SignalGenerator`, `BandLimitedWhiteNoise`,
`FromFile`, `FromWorkspace`, `FromSpreadsheet`, `EnumeratedConstant`,
`Counter`, `CounterLimited`, `RepeatingSequenceInterpolated`,
`Video/Image` (media), `ToWorkspace`, `ToFile`, `XYGraph`, `FloatingScope`,
`Stop`, `Record`.
Icons: ground `⊥`, terminator `⊣`, clock face, sine/chirp waves, scope
screen, arrow-into-box. Ports: mostly `ports(0,1)` or `ports(1,0)`.

### M3 — Math & Matrix
Blocks: `Trig`, `Math`, `Bias`, `DotProduct`, `Sign`, `Rounding`,
`UnaryMinus`, `SignedSqrt`, `Sqrt3D`/`Square`, `Abs2`, `MinMaxRunningResettable`,
`ProductOfElements`, `AlgebraicConstraint`, `Find`, `Reshape`,
`PermuteDimensions`, `VectorConcatenate`, `MatrixConcatenate`,
`Submatrix`, `Transpose`, `Diagonal`, `CrossProduct`, `ComplexTo*`,
`RealImagToComplex`, `MagnitudeAngleToComplex`, `ReciprocalSqrt`,
`Polynomial`, `SliderGain`, `MatrixGain`, `WeightedSampleTime`,
`Assignment`, `Fcn` variants.
Icons: mostly text-glyph blocks (`sin`, `cos`, `atan2`, `min`, `|x|`…);
Sum/Product already implement port `+`/`×`/`÷` labels — reuse that code.
Ports: mostly `ports(1,1)`; relational/multi-input use `ports(2,1)`.

### M4 — Discrete & Continuous
Blocks: `Delay`, `Difference`, `DiscreteDerivative`, `DiscreteIntegrator`,
`DiscreteFIRFilter`, `DiscretePIDController(2DOF)`, `DiscreteZeroPole`,
`FirstOrderHold`, `Memory`, `ResettableDelay`, `TappedDelay`,
`VariableIntegerDelay`, `Derivative`, `IntegratorLimited`,
`IntegratorSecondOrder(Limited)`, `PIDController(2DOF)`,
`TransferFcnFirstOrder`, `TransferFcnLeadLag`, `TransferFcnRealZero`,
`TransferDelay`, `TransportDelay`, `VariableTimeDelay`, `ZeroPole`,
`Continuous-VCO`, `FirstOrderTransferFunction`, `TransferFcnDirectFormII(TimeVarying)`,
`CICFilter`, `IncrementDecrementToZero`, `IncrementDecrementToValue`,
`Increment(Real|Stored)`, `Decrement(Real|Stored)`.
Icons: `1/z`, `z^-1`, `1/s`, `K`, pole-zero text, small `Σ`/integrator
symbol, `Ts` sample-time badge. Ports: `ports(1,1)`; PIDs `ports(1,1)`;
delays `ports(1,1)`.

### M5 — Signal Routing & Signal Attributes
Blocks: `GotoTagVisibility`, `SignalSpecification`, `EnvironmentController`,
`DataTypeConversion(Inherited)`, `DataTypeDuplicate`, `DataTypePropagation`,
`DataTypeScalingStrip`, `InitialCondition`, `Probe`, `Width`, `UnitConversion`,
`UnitSystemConfig`, `IsFixed`, `IsNonBus`, `WeightedSampleTime`.
Icons: tag badge, `Conv`, `IC`, `Probe` label text, `RT` rate badge.
Ports: mostly `ports(1,1)`.

### M6 — Discontinuities & Logic & Lookup
Blocks: `Backlash`, `DeadZone(Dynamic)`, `Quantizer`, `RateLimiter(Dynamic)`,
`Relay`, `SaturateDynamic`, `WrapToZero`,
`BitClear`, `BitSet`, `BitwiseOperator`, `ExtractBits`, `ShiftArithmetic`,
`CombinatorialLogic`, `DetectChange`, `DetectDecrease`, `DetectFallNegative`,
`DetectFallNonpositive`, `DetectIncrease`, `DetectRiseNonnegative`,
`DetectRisePositive`, `IntervalTest(Dynamic)`,
`Cosine`, `Sine`, `DirectLookup`, `Interpolation`, `Lookup1D`, `Lookup2D`,
`LookupDynamic`, `PreLookup`.
Icons: dead-zone/relay/backlash curves, ANSI logic gates (param-selected
glyph), staircase lookup curves. Ports: mostly `ports(1,1)`; detect/
interval `ports(2,1)`.

### M7 — Dashboard & Customizable
Blocks: all §2.3/§2.4 — `CircularGauge`, `LinearGauge`, `HalfGauge`,
`QuarterGauge`, `Knob`, `Lamp`, `PushButton`, `ToggleSwitch(Block)`,
`RockerSwitch`, `RotarySwitch`, `Slider(Switch)`, `Checkbox`, `RadioButton(Group)`,
`ComboBox`, `Edit`, `TextArea`, `Stopwatch`, `Playback`, `CallbackButton`,
`RealTimeDashboard`, plus their `*Custom` variants.
Icons: gauge arc + needle, lamp circle, switch/knob shapes, button box.
Ports: `dashboard_ports` already models binding endpoints — reuse;
widgets mostly `ports(1,0)` or `ports(0,0)` + binding.
This milestone needs the dashboard feature flag — keep all work behind
`#[cfg(feature = "dashboard")]`.

### M8 — Verification, Utilities, String, Messages, UDF
Blocks: all §2.12–§2.14, §2.20–§2.21 — `Assertion`, `Check*`,
`DocBlock`, `ModelInfo`, `BlockSupportTable`, `SimulinkConfig`,
`Message*`, `SequenceViewer`, `EventListener`, `EntitySink/Source`,
all `String*` blocks, `C++Function`, `InterpretedMATLABFunction`,
`Level2MATLABSFunction`, `Python`, `S-FunctionBuilder`, `SimulinkFunction`.
Icons: checkmark/`!` badges, doc-page icon, string-quote icons, `fn` text.
Ports: `ports(1,1)`/message ports.

## 7. Data-model requirements

1. `KNOWN_BLOCK_TYPES` — add every missing BlockType (keep sorted for
   `binary_search`). ~265 new entries.
2. `block_traits()` — new match arms per block: `live_role`,
   `dashboard_ports`, `reads_signal_line`, `path_prefix_matched`,
   special port overrides.
3. `Block` fields — some blocks need new typed fields (e.g. `Icon`,
   `PortLabels`, `Mask` sub-props); otherwise properties map suffices.
4. `PortCounts` — most new blocks use standard ins/outs; control/event
   ports (`enable`/`trigger`/`reset`/`ifaction`/`elseaction`/`event`)
   need `Port` child parsing where not already handled.
5. `PortPositionOverride` entries for blocks with non-standard port
   placement: round-Sum bottom input, Relay side-inputs, Merge/Switch
   middle input, TransferFcn side-by-side, PID ref input, etc.
6. `simulink_libraries/renderers/<category>.rs` — one new file per
   milestone category where missing (e.g. `datetime.rs`,
   `messages_events.rs`, `verification.rs`, `utilities.rs`,
   `strings.rs`, `customizable.rs`), matching the existing pattern:
   `fn render_<category>_block(ui, block, r_screen, font_scale) -> bool`.

## 8. Icon-spec format (proposed)

To keep ~265 icons maintainable, add a small declarative spec — a
`BlockIconSpec` per block type:

```rust
pub struct BlockIconSpec {
    /// Centered text glyph (e.g. "1/z", "min", "≥", "IC").
    pub text_glyph: Option<(&'static str, GlyphSize)>,
    /// Param-driven text (e.g. Constant shows `Value`).
    pub param_glyph: Option<&'static str>, // property key
    /// Polyline path in normalized block-rect coords [0,1]².
    pub path: Option<&'static [(f32, f32)]>,
    /// Filled polygon in normalized coords (e.g. Gain triangle).
    pub polygon: Option<&'static [(f32, f32)]>,
    /// Arc/pie slice (gauges, clocks).
    pub arc: Option<ArcSpec>,
    /// Port text labels keyed by side+index (e.g. Sum "+"/"-").
    pub port_labels: &'static [PortLabelSpec],
    /// Whether the icon scales with the block rect.
    pub scales: bool,
}
```

`render_block_interior` looks up the spec and emits the primitives;
custom renderers stay for complex shapes (subsystems, gauges, chart
icons) — spec covers the ~80% that are "text + path" icons.

## 9. Port-label spec format

```rust
pub struct PortLabelSpec {
    pub side: PortSide,          // In/Out/Top
    pub index: u32,              // 1-based port index
    pub text: &'static str,      // "+", "-", "u1", "ctrl", "if", "≥" …
    pub placement: PortLabelPlacement, // inside-edge / outside / above / below
}
```

- Sum: `[(In,1,"+"),(In,2,"+")]` or `-` per `Inputs` param (`+-`/`++`/`|` handling already parses the sign string — extend to render them).
- Switch/MultiPortSwitch: `[(In,1,"u1"),(In,2,"ctrl"),(In,3,"u2")]`
  (or `>`,`>=`,`==`,`!=`,`<`,`<=` per `Criteria`).
- RelationalOperator: text glyph `==`,`!=`,`<=`,`<`,`>=`,`>`, `isInf`,
  `isNan`, `isFinite` per `Operator` param.
- LogicalOperator: ANSI gate shapes + input labels where Simulink shows them.
- Trigonometric/MinMax/Math/Fcn: centered op name from the `Operator`/`Function`/`Expr` param.
- Bus Selector/Creator/Element: element-name labels on ports where the model specifies them.
- S-Function: `fun(...)` header text.
- TransferFcn/DiscreteTransferFcn: `num(s)`/`den(s)` or `num(z)`/`den(z)` fraction text.
- PID: `P`, `I`, `D` coefficient text or `PID(s)` glyph.
- Enable/Trigger/Reset ports: LConn/RConn glyphs — enable = small "⊏" box, trigger = `⚡` pulse, reset = `⟲` arrow.

## 10. Verification strategy

1. **Golden-image fixtures**: for each new block, author a synthetic
   `.slx` XML snippet, render via `render_block_interior`, rasterise
   to a `Vec<Shape>` snapshot — compare against a checked-in expectation.
   Uses the existing headless-egui harness (`tests/e2e/harness/`).
2. **Real-model spot checks**: `simulink_test_models/*.slx` contain
   many of the target blocks — add per-block screenshots under
   `simulink_test_models/simulink_screenshots/` as the visual oracle.
3. **Trait assertions**: `block_traits` match arms get table-driven
   tests (one test per block: `ports`, `dashboard_ports`, `live_role`).
4. **Coverage gate**: a CI test that iterates `KNOWN_BLOCK_TYPES` and
   asserts `is_known_block_type` + a renderer entry exists (no
   silently-generic icon) for the top ~200 canonical types.
5. **E2E canvas tests** (existing harness): place each new block on a
   canvas, click/drag/resize, assert no panic + ports visible.

## 11. Risks / open questions

- **Icon fidelity vs cost**: ~265 icons is a lot; the spec-table approach
  (§8) covers ~80%; the rest are hand-drawn renderers.
- **Port labels needing params**: Sum's `+-` string, Switch's `Criteria`,
  Trig's `Function`, Fcn's `Expr` — labels are param-driven, not static;
  the spec table must support `{param}` interpolation.
- **Dashboard widgets**: need the `dashboard` feature and live-value
  plumbing — most work lands in `dashboard_ui.rs`, not generic renderers.
- **SimEvents/Messages**: real Simulink "Messages & Events" blocks are
  SimEvents-domain; decide whether to scope them out or stub icons only.
- **`BlockType` name variants**: Simulink writes aliases
  (`"Logic"` vs `"LogicalOperator"`, `"Sum"` vs `"Add"`, `"Math"` vs
  `"MathFunction"`); `KNOWN_BLOCK_TYPES` must accept aliases.

