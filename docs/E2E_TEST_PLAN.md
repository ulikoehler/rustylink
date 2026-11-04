# RustyLink End-to-End Test Plan

> **Status:** Draft for implementation
> **Branch:** `e2e-testing-plan`
> **Crate:** `rustylink` v3.1.0
> **Date:** 2026-11-04

---

## Table of Contents

- [1. Purpose and Scope](#1-purpose-and-scope)
- [2. System Under Test](#2-system-under-test)
- [3. Testing Philosophy and Test Pyramid](#3-testing-philosophy-and-test-pyramid)
- [4. Test Infrastructure](#4-test-infrastructure)
- [5. Test Data and Fixture Corpus](#5-test-data-and-fixture-corpus)
- [6. Parsing and Model Layer E2E Tests (P)](#6-parsing-and-model-layer-e2e-tests-p)
- [7. Serialization and Round-Trip Tests (SER)](#7-serialization-and-round-trip-tests-ser)
- [8. Signal Propagation / Connection-Target Resolution Tests (CT)](#8-signal-propagation--connection-target-resolution-tests-ct)
- [9. Signal Routing and Line Geometry Tests (RT)](#9-signal-routing-and-line-geometry-tests-rt)
- [10. Editor End-to-End Tests (ED) — Drag-and-Drop Focus](#10-editor-end-to-end-tests-ed--drag-and-drop-focus)
- [11. Viewer End-to-End Tests (VW)](#11-viewer-end-to-end-tests-vw)
- [12. Live Values and Formatting Tests (LV)](#12-live-values-and-formatting-tests-lv)
- [13. Dashboard Widget E2E Tests (DB)](#13-dashboard-widget-e2e-tests-db)
- [14. Stress, Scale and Performance Tests (PERF)](#14-stress-scale-and-performance-tests-perf)
- [15. Full User-Journey E2E Scenarios (E2E)](#15-full-user-journey-e2e-scenarios-e2e)
- [16. Property-Based and Fuzz Testing (FZ)](#16-property-based-and-fuzz-testing-fz)
- [17. Robustness and Error-Handling Tests (ERR)](#17-robustness-and-error-handling-tests-err)
- [18. CI Integration, Coverage and Benchmarks](#18-ci-integration-coverage-and-benchmarks)
- [19. Implementation Milestones](#19-implementation-milestones)
- [20. Test Case Index](#20-test-case-index)

---

## 1. Purpose and Scope

### 1.1 Purpose

This document defines the complete end-to-end (E2E) test plan for `rustylink`, a
Rust crate that parses Simulink `.slx` model files (R2025a+ format) into a typed
model, optionally renders them in an `egui`-based viewer, and provides an
interactive model editor with drag-and-drop (DnD), signal routing, signal-value
propagation (connection-target resolution), live values, and dashboard widgets.

The existing test suite (~10.8k lines across 40 integration test binaries) covers
mostly pure-function unit behaviour. This plan adds a *true* E2E layer:

1. **File-level E2E:** real `.slx` archives and synthetic archives are parsed,
   rendered, edited, saved and re-parsed — full round-trip workflows.
2. **GUI-level E2E:** the `eframe`/`egui` viewer and editor are driven headlessly
   with synthesized input events (mouse drags, clicks, keys, scroll), asserting
   on the resulting model mutations, resolver output and paint output.
3. **Propagation E2E:** signal connectivity is resolved through hierarchies of
   subsystems, buses (creator/selector/assignment), mux/demux pairs, goto/from
   tag pairs, library links and control ports, at scale.
4. **Stress E2E:** models with thousands of signals, deep hierarchy, wide buses
   and high-rate live-value streams are exercised to find throughput,
   latency and memory regressions.

### 1.2 Scope

| In scope | Out of scope |
|---|---|
| `.slx` (zip) and standalone `system_*.xml` parsing | Pre-R2025a Simulink formats (explicitly unsupported) |
| JSON / bincode serialization and round-trips | Numerical simulation of models (rustylink is a viewer/editor, not a solver) |
| Viewer app (`egui_app`) rendering, navigation, live mode | MATLAB/Simulink interop, code generation |
| Editor (`editor`) mutations incl. all `DragMode`s | Stateflow chart semantics beyond parsing/label extraction |
| Connection-target resolution (signal propagation) | Pixel-perfect visual verification of Simulink fidelity |
| Dashboard widgets, live values, scopes (liveplot) | OS-level window management (winit integration is smoke-tested only) |
| SLX archive write / regenerate (`generator`) | Third-party toolbox installation discovery |

### 1.3 Definitions

- **DUT** — device/unit under test.
- **Resolver** — `ConnectionTargetResolver` (`src/connection_targets/resolver.rs`),
  computes, per line and per block port, the set of `ConnectionTarget`s a signal
  reaches, including origins (`ConnectionTargetOrigin`) for upstream propagation.
- **Line target** — downstream endpoints of a line (possibly through hierarchy).
- **DnD** — drag and drop: all `DragMode` (editor) and `ViewerDragState` (viewer
  move mode) interactions.
- **Live value** — `live_values::LiveValue` pushed by a host application into the
  viewer at runtime (scalar/vector/matrix/complex), optionally at high rate.
- **Kittest harness** — headless egui test harness using `egui_kittest` (see §4.3).
- **Golden model** — checked-in `.slx`/`.xml` expected output for comparisons.

### 1.4 Success Criteria

- Every public feature enumerated in §2.3 has ≥ 1 E2E test; every `DragMode` /
  `ViewerDragState` variant has ≥ 3 E2E tests (nominal, edge, cancel/abort).
- Line coverage ≥ 80% crate-wide, ≥ 90% for `src/connection_targets/`,
  `src/editor/`, `src/generator/` (measured with `cargo-tarpaulin`, already in CI).
- All E2E tests run headless on `ubuntu-latest` CI with no display server
  (use `egui` test harness or `xvfb` only where strictly needed).
- Stress tests have deterministic, machine-scaled assertions (ratios, not
  absolute wall-clock seconds) so they are stable on CI runners.
- No test depends on a MATLAB/Simulink installation.

---

## 2. System Under Test

### 2.1 Architecture Map

```
                         ┌────────────────────────────┐
 .slx (zip) / .xml  ───► │ parser (SimulinkParser)    │──► model::System
                         │  source.rs / mod.rs /      │      │
                         │  chart.rs / library.rs /   │      │ serde_json
                         │  graphical_interface.rs    │      ▼
                         └────────────────────────────┘   JSON / bincode
                                                                │
   generator::archive / system_xml  ◄───────────────────────────┘
        (write .slx, regenerate XML)

 model::System ──► connection_targets::ConnectionTargetResolver
                        │        (signal propagation graph)
                        ▼
   egui_app (viewer)          editor (editor app)
   ├─ render/*                 ├─ operations (EditorCommand/EditorHistory)
   ├─ ui/* (signal_routing,    ├─ selection / state (DragMode)
   │   corner_ops, view_       ├─ ui/canvas (port hit-areas, connection drag)
   │   transform, live_values) └─ ui/{menus,panels,resize,shortcuts}
   ├─ dashboard_widgets/*
   ├─ scope_widget (MiniScope, liveplot)
   └─ navigation / state / text
```

### 2.2 Feature Flags Under Test

| Feature | Depends on | Notes for tests |
|---|---|---|
| `egui` | eframe, egui, rfd, syntect, liveplot, egui_material_icons | Viewer + editor; required by most GUI tests |
| `highlight` | syntect | MATLAB code highlighting in editor panels |
| `mask` | — | Mask display-string evaluation (`mask_eval`) |
| `dashboard` | `egui` | Interactive dashboard widgets, liveplot scopes, editable constants |

CI runs `cargo test --all-features`; E2E suites must declare the minimal feature
set via `required-features` in `Cargo.toml` so the no-feature build stays lean.

### 2.3 Feature Inventory (what must be covered)

**Parsing / model**
- `.slx` zip archive reading (`parser/source.rs::ZipSource`), directory listing,
  UTF-8 and path normalization (`./`, `/` prefixes).
- `system_root.xml` / referenced `system_*.xml` parsing, nested `System` refs.
- Blocks: all `BlockType`s, `SID` (numeric and string forms), positions, ports,
  `PortCounts`, `Commented`, `NameLocation`, masks, `ValueKind`, annotations.
- Lines: `Src`/`Dst` endpoints, branches (nested), `Points`, names, testpoints.
- Charts (Stateflow): `chart.rs` parsing, label extraction.
- Graphical interface files: dashboard bindings (`DashboardBinding`,
  `DashboardTargetPath`), library block declarations.
- Library resolution: `simulink` built-in virtual libraries
  (`simulink_libraries/libraries/*`, ≥750 catalog entries), user `.slx`
  libraries via `-L` search paths, unresolved-link labelling.
- Mask evaluation (`mask_eval`): display-string subset of MATLAB syntax.

**Model services**
- `model_topology_signature` (structural hash) for cache invalidation.
- `count_subsystems`, `max_subsystem_depth`.
- Binary serialization `SystemDoc::save_to_binary` / `load_from_binary` (bincode).
- JSON serialization (serde) for all model types.

**Signal propagation (`connection_targets`)**
- `ConnectionTargetResolver::new[_with_progress]`, per-system resolution,
  caching (`ComputedViewCache`), progress atoms (`Arc<AtomicU32>`).
- `line_targets_for_line[_ref]`, `block_targets_for_block[_ref]`,
  `propagate_line_targets`, `propagate_line_metadata_upward`.
- Routing rules: bus creator/selector/assignment, mux/demux, from/goto pairs,
  upstream-propagated targets, subsystem boundary ports, control ports
  (`is_control_port_type`: enable/trigger/…), dashboard bindings.
- Graph helpers: `incoming_/outgoing_*` per port, `line_data_input_ports`,
  `input_port_indices`, `output_port_count`, `child_*_targets_by_port`,
  `qualify_external_path`, `normalized_path_segment`, `same_line`,
  `line_identity`/`branch_identity`.

**Routing and line geometry (`egui_app/ui`, `editor`)**
- `orthogonalize_polyline[_with_dst_side]`, `push_orthogonal_segments`,
  `dst_segment_horizontal`, `endpoint_pos`, `register_endpoint[s]`,
  `register_branch_endpoints`, `compute_port_info`.
- `move_line_point`, `move_branch_point`, `move_line_layout`,
  `move_branch_layouts`, `collect_branch_handle_positions`, `get_branch_mut`,
  `port_kind`, `event_input_offset`, reinit-subsystem port handling.
- Corner ops: `insert_corner`, `remove_corner`, `merge_adjacent_corners`,
  `auto_adjust_on_block_move`, `auto_adjust_branches_on_block_move`,
  `enforce_orthogonal`.
- `find_snap_port`, `auto_route` (orthogonal path between ports).
- `compute_line_adjacency` + `assign_line_colors` (distinct hues for
  topologically adjacent lines), `line_style` (testpoint markers, widths),
  `line_coloring` luminance handling.

**Editor**
- `EditorState` + `EditorHistory` (undo/redo ring) over `EditorCommand`.
- All mutations: `add_block`, `delete_blocks`, `move_block(s)`, `add_line`,
  `delete_lines`, `comment_blocks`, `rotate_blocks`, `mirror_blocks`,
  `rename_line`, `branch_line`, `create_subsystem_from_selection`,
  `resize_block`, `assign_sids`, `create_default_block`,
  `parse_position`/`format_position`.
- Selection: click, ctrl-toggle, marquee `SelectionRect` (contains/overlaps).
- Clipboard: `copy_selection`/`paste`/`delete_selection`.
- `DragMode::{None, Blocks, Connection, SelectionRect, Resize, Pan}`.
- Canvas: port hit-areas (`allocate_rect` + `Sense::click_and_drag`),
  connection drag from out/in ports, preview line, drop on valid port,
  snap-to-port, grid snap (`state.snap`), arrow-key nudge, block browser
  (`BlockBrowserState`), code editor (`CodeEditorState`), menus, panels,
  resize handles (8-handle `compute_resized_rect`), shortcuts.

**Viewer (`egui_app`)**
- `SubsystemApp` navigation stack, `resolve_subsystem_by_path/_vec`,
  `collect_subsystems_paths`, breadcrumbs, subsystem change listeners.
- `ViewTransform`: `to_screen`/`from_screen`, `zoom_at`, pan, `font_scale`,
  `shared_canvas_text_font_px`, `preview_block_rect`, resize handles.
- `ViewerDragState::{Blocks, Resize, LinePointDrag, BranchPointDrag,
  SignalLabelDrag}` in move mode.
- Rendering: block bodies/icons (`render/*`), goto/from tags, logic gates,
  math blocks, switches, labels (`block_text`, `labels`, `label_place`),
  colors (`egui_app_colors`), text pipeline (`text.rs` incl. `html-escape`
  entity decoding and syntect highlighting), fonts, tooltips, dialogs
  (`SignalDialog`, `BlockDialog`, context menus), dashboard debug helpers.
- Live mode: `live_mode_enabled`, `live_tooltips_enabled`, `live_values`,
  `live_block_values`, `live_block_tooltips`, `live_line_tooltips`,
  `live_display_defaults`.
- Scopes: `MiniScope` (`push_sample`, embedded + popout `ScopePopout`),
  `clear_popout_state`, `draw_scope_glyph`.
- Dashboard widgets: `live_*` controls (push button, slider switch, radio
  group, combo box, checkbox, slider/linear gauge, field/display, toggle,
  radial gauge, rocker switch, lamp), `DashboardControlEvent`/
  `DashboardControlValue::{Scalar, Bool, PulseHigh, PulseLow}` emission to
  the host, painters, statics, values (`dashboard_scalar_value_from_pointer`).

**CLI**
- `main.rs`: parse `.slx`/XML → JSON stdout; exit codes; error paths.

**Examples (smoke)**
- `tree`, `analyze_libraries`, `analyze_model_blocks`, `binary_benchmark`,
  `check_libraries`, `dump_block_icons`, `egui_viewer`, `egui_editor`,
  `find_manual_switches`, `list_blocks`, `roundtrip_compare`.

### 2.4 Existing Test Assets

- `simulink_test_models/`: `Duplicat_Test.slx`, `Line_Test.slx`,
  `Propergation_Test.slx`, `Simulink_Blocks.slx`, `Simulink_UI_Test.slx`,
  `simulink_screenshots/` (reference screenshots).
- 40 test binaries under `tests/` (see Cargo.toml `[[test]]` sections);
  `tests/connection_targets.rs` (~4k lines) already covers many resolver
  cases at the API level — the E2E plan extends this to *rendered/editor*
  behaviour and scale.

---

## 3. Testing Philosophy and Test Pyramid

### 3.1 Layers

```
        ┌───────────────────────────────────────────┐
        │ L4: Full user journeys (E2E-*)             │  ~25 tests
        │   parse → view → edit → save → re-parse    │
        ├───────────────────────────────────────────┤
        │ L3: GUI e2e via headless egui harness      │ ~180 tests
        │   (ED-*, VW-*, DB-*) synthesized input     │
        ├───────────────────────────────────────────┤
        │ L2: Subsystem e2e on real models           │ ~160 tests
        │   (P-*, SER-*, CT-*, RT-*, LV-*)           │
        ├───────────────────────────────────────────┤
        │ L1: Stress / property / fuzz               │  ~60 tests
        │   (PERF-*, FZ-*, ERR-*)                    │
        ├───────────────────────────────────────────┤
        │ L0: Existing unit tests (keep, extend)     │  ~200 tests
        └───────────────────────────────────────────┘
```

L0 already exists. This plan defines L1–L4. Where a behaviour is already
unit-tested, the E2E test asserts the *integrated* outcome (e.g. unit test:
`move_block` sets position; E2E: synthesized mouse drag on the rendered canvas
produces the same position and an undoable `EditorCommand`).

### 3.2 Determinism Rules

1. No wall-clock-dependent assertions; use `egui::Context::request_repaint`
   counters, sample counts, or injected `std::time::Instant` sources.
2. Fix the egui `Context` DPI (`pixels_per_point = 1.0`) and fonts to the
   bundled `epaint_default_fonts` in every GUI test for reproducible layout.
3. Seed all pseudo-random model generation (`SmallRng::seed_from_u64`).
4. Parallel resolver progress must be asserted on *final* state, not interleaving.
5. High data-rate tests must never `sleep` in CI mode; they push N samples then
   drive `ui.update()`/`ctx.run()` synchronously (see §14.4).

### 3.3 Assertion Taxonomy

- **Model assertions:** `System`/`Block`/`Line` fields (positions, endpoints,
  names, SIDs, properties) after operations.
- **Resolver assertions:** target sets equality (order-insensitive `BTreeSet`
  of `(block_path, port_index)` tuples).
- **Geometry assertions:** screen/model coordinates within ε = 0.5 px for
  transforms, exact ints for model-space routing points.
- **Paint assertions:** `epaint` shape counts/kinds from the harness output
  (e.g. "a line shape exists between A and B"), plus optional snapshot images.
- **Event assertions:** `DashboardControlEvent` queue contents, undo/redo
  stack depth, dirty flag, subsystem-change listener notifications.

---

## 4. Test Infrastructure

### 4.1 Directory Layout (new)

```
tests/
  e2e/                        # new subtree (keep flat [[test]] entries)
    harness/
      mod.rs                  # re-exports
      model_builders.rs       # fluent System/Block/Line builders
      slx_factory.rs          # synthetic .slx writer (zip + generated XML)
      gui.rs                  # headless egui driver (kittest wrapper)
      input.rs                # drag/click/key event synthesizers
      live_feed.rs            # scripted LiveValue producer
      gen_models.rs           # seeded random/stress model generator
      snapshots.rs            # optional image snapshot helpers
  e2e_parse.rs                # [[test]] name = "e2e_parse", required-features = ["egui"]
  e2e_serialize.rs            # required-features = []
  e2e_propagation.rs          # required-features = ["egui"]
  e2e_routing.rs              # required-features = ["egui"]
  e2e_editor_dnd.rs           # required-features = ["egui"]
  e2e_editor_ops.rs           # required-features = ["egui"]
  e2e_viewer.rs               # required-features = ["egui"]
  e2e_live.rs                 # required-features = ["egui"]
  e2e_dashboard.rs            # required-features = ["dashboard"]
  e2e_stress.rs               # required-features = ["dashboard"], #[ignore] heavy cases opt-in
  e2e_journeys.rs             # required-features = ["dashboard"]
  e2e_fuzz.rs                 # required-features = []
  e2e_errors.rs               # required-features = []
assets/
  e2e/                        # generated fixtures checked in (small) + .gitattributes
    slx/ xml/ golden/
```

Each `[[test]]` entry is added to `Cargo.toml` with its `required-features`,
matching the existing convention.

### 4.2 Model Builders (`harness/model_builders.rs`)

Fluent builders over `rustylink::model` to remove fixture boilerplate:

```rust
pub struct SystemBuilder { /* blocks, lines, properties */ }
impl SystemBuilder {
    pub fn new(name: &str) -> Self;
    pub fn block(self, b: BlockBuilder) -> Self;
    pub fn line(self, l: LineBuilder) -> Self;
    pub fn prop(self, k: &str, v: &str) -> Self;
    pub fn build(self) -> System;
}

pub struct BlockBuilder { /* type, name, sid, position, ports, props, subsystem */ }
impl BlockBuilder {
    pub fn new(block_type: &str, name: &str, sid: &str) -> Self;
    pub fn pos(self, l: i32, t: i32, r: i32, b: i32) -> Self;
    pub fn in_ports(self, n: u32) -> Self;
    pub fn out_ports(self, n: u32) -> Self;
    pub fn prop(self, k: &str, v: &str) -> Self;
    pub fn subsystem(self, sys: System) -> Self;   // sets system_ref + inline
    pub fn build(self) -> Block;
}

pub struct LineBuilder { /* name, src/dst endpoints, points, branches */ }
impl LineBuilder {
    pub fn new() -> Self;
    pub fn from(self, sid: &str, port_type: &str, idx: u32) -> Self; // Src
    pub fn to(self, sid: &str, port_type: &str, idx: u32) -> Self;   // Dst (+ Branch)
    pub fn point(self, x: i32, y: i32) -> Self;
    pub fn name(self, n: &str) -> Self;
    pub fn build(self) -> Line;
}

// Convenience constructors for common canonical models:
pub fn gain_chain(n: usize) -> System;                 // In→Gain×n→Out
pub fn bus_roundtrip(n_signals: usize) -> System;      // Creator→Selector
pub fn subsystem_wrap(inner: System, name: &str) -> System;
pub fn goto_from_pair(tag: &str) -> (Block, Block);
```

Builders must produce **structurally valid** models (SIDs unique, port indices
1-based, `system_ref` consistent) — validated by a `debug_validate(&System)`
helper used inside every builder `build()` in test builds.

### 4.3 Headless GUI Harness (`harness/gui.rs`)

Primary driver: **`egui_kittest`** (the egui project's own test harness) when its
release line matches `egui 0.37`; otherwise a thin local driver:

```rust
pub struct EditorHarness {
    pub ctx: egui::Context,
    pub state: EditorState,
    frame: u64,
}
impl EditorHarness {
    pub fn new(system: System) -> Self;
    /// Run one full egui frame with the given raw input events queued.
    pub fn frame(&mut self, input: egui::RawInput) -> egui::FullOutput;
    /// Convenience: press-drag-release over several frames.
    pub fn drag(&mut self, from: Pos2, to: Pos2, steps: usize) -> egui::FullOutput;
    pub fn click(&mut self, p: Pos2, modifiers: egui::Modifiers) -> egui::FullOutput;
    pub fn key(&mut self, key: egui::Key, modifiers: egui::Modifiers) -> egui::FullOutput;
    pub fn scroll(&mut self, delta: Vec2) -> egui::FullOutput;
    /// Paint shapes clipped to the canvas, for paint-level assertions.
    pub fn canvas_shapes(&self) -> &[egui::epaint::ClippedShape];
    pub fn screen_pos_of_block(&self, sid: &str) -> Rect;
    pub fn port_screen_pos(&self, sid: &str, kind: PortSide, idx: u32) -> Pos2;
    // RawInput builders: pointer down/move/up, multi-frame drag with per-frame
    // positions so egui emits drag_started/dragged/drag_stopped correctly.
}
```

Equivalent `ViewerHarness` for `SubsystemApp` (move mode, live mode, scopes,
dashboard widgets). Both harnesses wrap `ctx.run(RawInput, |ctx| app.ui(ctx))`
in a loop of ≥ 3 frames per gesture because `Sense::click_and_drag` reports
`drag_started` only on the frame *after* press+move — the plan's gesture
synthesizer codifies this (see §10.1.2). Rendering backend: `egui` tessellation
only — **no winit/GL** — so tests run on CI without a display server.

> **Dependency note:** add `egui_kittest` (version matching egui 0.37,
> e.g. `egui_kittest = "0.37"` under `[dev-dependencies]` with
> `required-features` gating) OR keep a ~150-line local driver to avoid the
> dependency; decide in Milestone M1. If `egui_kittest` lags egui releases,
> vendor the local driver.

### 4.4 Input Synthesizers (`harness/input.rs`)

```rust
pub fn press(pos: Pos2) -> egui::RawInput;
pub fn move_to(pos: Pos2) -> egui::RawInput;
pub fn release(pos: Pos2) -> egui::RawInput;
pub fn drag_path(points: &[Pos2]) -> Vec<egui::RawInput>; // 1 frame per point
pub fn ctrl_click(pos: Pos2) -> Vec<egui::RawInput>;
pub fn shift_click(pos: Pos2) -> Vec<egui::RawInput>;
pub fn double_click(pos: Pos2) -> Vec<egui::RawInput>;
pub fn key_combo(key: egui::Key, ctrl: bool, shift: bool) -> egui::RawInput;
pub fn type_text(s: &str) -> Vec<egui::RawInput>;
pub fn scroll_at(pos: Pos2, delta: Vec2) -> egui::RawInput;
```

Every gesture is a **frame sequence** (each `RawInput` = one `ctx.run`), which
is the only reliable way to reproduce `drag_started`/`dragged`/`drag_stopped`
and hover transitions in egui.

### 4.5 Synthetic SLX Factory (`harness/slx_factory.rs`)

```rust
/// Serialize a System back into a minimal but schema-valid system XML string,
/// reusing generator::system_xml::generate_system_xml, then zip it.
pub fn write_slx(path: &Path, systems: &[(&str /*entry path*/, &System)],
                 extra_files: &[(&str, &[u8])]) -> Result<()>;
/// Minimal graphicalInterface + blockdiagram.xml.rels for dashboard tests.
pub fn write_slx_with_gi(path: &Path, root: &System,
                         gi_xml: &str, rels_xml: &str) -> Result<()>;
```

The `.slx` writer must round-trip through `SlxArchive::write_to` where possible
so E2E tests exercise the real writer, with an escape hatch (`raw_zip`) for
malformed-archive robustness tests (§17).

### 4.6 Live-Value Feed (`harness/live_feed.rs`)

```rust
pub struct LiveFeed { /* scripted value streams */ }
impl LiveFeed {
    pub fn sine(freq_hz: f64, amp: f64) -> impl Iterator<Item = f64>;
    pub fn step(t0: usize, lo: f64, hi: f64) -> impl Iterator<Item = f64>;
    pub fn burst(n: usize, value: f64) -> impl Iterator<Item = f64>;
    /// Push N samples at logical rate R into SubsystemApp::live_values /
    /// MiniScope::push_sample without sleeping (rate is a parameter of
    /// assertions, not of the clock).
    pub fn drive_scope(scope: &mut MiniScope, it: impl Iterator<Item = f64>, n: usize);
    pub fn drive_app(app: &mut SubsystemApp, key: &str, dims: &[u32],
                     frames: usize, gen: impl Fn(usize) -> f64);
}
```

### 4.7 Stress Model Generator (`harness/gen_models.rs`)

Seeded generators producing *valid* systems of parameterized size:

| Generator | Parameters | Used by |
|---|---|---|
| `gen_wide_net` | n_signals, fan_out | PERF propagation/routing |
| `gen_deep_hierarchy` | depth, blocks_per_level | PERF resolver depth |
| `gen_bus_fabric` | n_buses, width | CT + PERF bus resolution |
| `gen_goto_from_mesh` | n_pairs | CT correctness at scale |
| `gen_random_system` | seed, blocks, lines | FZ property tests |
| `gen_dashboard_panel` | n_widgets | DB + PERF |
| `gen_many_corner_lines` | corners_per_line | RT + PERF |

Invariants enforced: unique SIDs, every `Dst` has a matching block+port,
no duplicate `(sid,port)` consumers on the same input port, branches end on
valid ports. The generator returns both the `System` and a `ConnectivityOracle`
(expected target sets) so resolver output can be diffed.

### 4.8 Snapshot/Golden Strategy

- **Structural goldens** (default): canonicalized `serde_json` of the `System`
  after each workflow, compared with `expect_test`-style inline expectations or
  checked-in `.golden.json`. Chosen over pixel snapshots because tessellated
  output is GPU/font-hinting dependent.
- **Paint asserts** (structural): counts/kinds of `epaint::Shape` in canvas
  clip regions — robust to font changes.
- **Optional image snapshots**: only behind `SNAPSHOT=1` env + feature; stored
  under `assets/e2e/snapshots/` and reviewed manually. Not run in CI.

### 4.9 Common Assertions (`harness/mod.rs`)

```rust
pub fn assert_targets_eq(actual: &[ConnectionTarget],
                         expected: &[(/*path*/&str, /*port*/u32)]);
pub fn assert_line_orthogonal(line: &Line);
pub fn assert_no_overlapping_blocks(sys: &System);
pub fn assert_undo_restores(hist: &mut EditorHistory, before: &System, after: &System);
pub fn assert_positions_snapped(sys: &System, grid: i32);
pub fn assert_shapes_contain_line(shapes: &[ClippedShape], a: Pos2, b: Pos2);
```

---

## 5. Test Data and Fixture Corpus

### 5.1 Real Models (existing)

| File | Role in E2E |
|---|---|
| `simulink_test_models/Simulink_Blocks.slx` | Broadest real block coverage: parse → render → icon existence (E2E-P-001..) |
| `simulink_test_models/Propergation_Test.slx` | Real propagation model: resolver E2E oracle (E2E-CT-001..) |
| `simulink_test_models/Line_Test.slx` | Branch/line geometry edge cases (E2E-RT-001..) |
| `simulink_test_models/Duplicat_Test.slx` | Duplicate name/SID handling (E2E-P, E2E-ERR) |
| `simulink_test_models/Simulink_UI_Test.slx` | UI-level fixtures: dashboards, labels (E2E-VW, E2E-DB) |
| `simulink_test_models/simulink_screenshots/` | Manual visual reference only |

### 5.2 Synthetic Fixtures (generated, checked in where small)

| ID | Fixture | Builder | Size budget |
|---|---|---|---|
| FX-01 | gain_chain(3) | `gain_chain` | <4 KB xml |
| FX-02 | subsystem depth-4 chain | `subsystem_wrap`×4 | <8 KB |
| FX-03 | bus creator(8)→selector | `bus_roundtrip(8)` | <6 KB |
| FX-04 | goto/from ×4 | `goto_from_pair` | <6 KB |
| FX-05 | dashboard panel (all widget types) | `gen_dashboard_panel(1 each)` | <20 KB slx |
| FX-06 | mux→demux 16ch | builders | <6 KB |
| FX-07 | reinit/enabled/triggered subsystems | builders | <8 KB |
| FX-08 | library link model + lib.slx | `write_slx` ×2 | <12 KB |
| FX-09 | MATLAB function + mask blocks | builders | <8 KB |
| FX-10 | line mesh 200 signals | `gen_wide_net(200,2)` | generated at test time |
| FX-11 | deep hierarchy d=12 | `gen_deep_hierarchy` | generated at test time |
| FX-12 | wide bus 256 elems | `gen_bus_fabric` | generated at test time |
| FX-13 | 1000-signal stress model | `gen_wide_net(1000,3)` | generated at test time |
| FX-14 | 5k-block render stress | `gen_random_system` | generated at test time |
| FX-15 | corrupt/truncated slx set | `raw_zip` | <4 KB each |

### 5.3 Canonical Expectation Files

Golden resolver expectations for `Propergation_Test.slx` and FX-02..FX-08 are
stored as `assets/e2e/golden/*.targets.json` — a sorted list of
`(line_key, [(target_path, port), ...])` produced once by the resolver (after
manual verification) and diffed on every run. Regeneration:
`REGEN_GOLDEN=1 cargo test -p rustylink --test e2e_propagation`.

---

## 6. Parsing and Model Layer E2E Tests (P)

Suite: `tests/e2e_parse.rs` · features: none for XML-level, `egui` where the
parser's library machinery is exercised. Cases prefixed **P-**.

Convention per case: *Given* (fixture/precondition), *When* (operation),
*Then* (assertions). Priority: P0 = must-pass in CI, P1 = CI, P2 = nightly.

### 6.1 Archive and Source Layer

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| P-001 | Open real `.slx` end to end | `Simulink_Blocks.slx` | `SlxArchive::from_file` | ≥1 `SystemXml` entry; entry paths match `simulink/systems/*`; no error | P0 |
| P-002 | Entry compression preserved | FX-01 written deflated | `from_reader` | `entry.compressed == true`; `Stored` entries round-trip `compressed == false` | P1 |
| P-003 | Non-UTF8 entry rejected | raw_zip with `bad.xml` = `0xFF` bytes | `from_reader` | `Err` containing "Non-UTF8" + path | P0 |
| P-004 | XML entry without `<System>` | raw_zip `misc.xml` w/o System node | `from_reader` | `Err` "No `<System>` root" | P1 |
| P-005 | `ZipSource::read_to_string` | FX-01 archive | read `simulink/systems/system_root.xml` | content equals written XML | P0 |
| P-006 | `read_to_string` normalizes `./` and `/` | FX-01 | read `./simulink/...` and `/simulink/...` | same content as canonical path | P1 |
| P-007 | Missing entry error contains path | FX-01 | read `nope.xml` | `Err` "not found in zip" + `nope.xml` | P1 |
| P-008 | `list_dir` enumerates prefix | slx with nested dirs | `list_dir("simulink/systems")` | all and only children returned as `Utf8PathBuf` | P1 |
| P-009 | `list_dir` trailing-slash tolerant | FX-01 | `list_dir` with and w/o trailing `/` | identical results | P2 |
| P-010 | Empty zip handled | raw_zip with 0 entries | `from_reader` | `Ok` archive with 0 entries (no panic) | P1 |
| P-011 | ZIP64 large member | generated slx >65 535 B payload entry | `from_reader` | parses; size correct | P2 |
| P-012 | Stored vs deflated mixed | raw_zip mixed methods | `from_reader` then `write_to` | per-entry method preserved in output zip | P1 |
| P-013 | `from_file` IO error path | nonexistent path | `from_file` | `Err` "Failed to open" + path display | P0 |

### 6.2 System XML Parsing

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| P-020 | Root system parses with blocks+lines | FX-01 XML | `SimulinkParser::parse_system_file` | block count, line count, names match builder input | P0 |
| P-021 | Nested `System Ref` resolution | FX-02 (4-deep) | parse root | `block.system_ref`/`subsystem` chain resolvable to depth 4 | P0 |
| P-022 | Referenced `system_N.xml` fetched via source | slx w/ 2 system files | parse root | child system loaded from second file, blocks present | P0 |
| P-023 | Position parsing `[l, t, r, b]` | block `<P Name="Position">[10, 20, 110, 70]</P>` | parse | `position == Some((10,20,110,70))` | P0 |
| P-024 | Negative/off-canvas positions | `[-500, -30, -400, 30]` | parse | parsed verbatim, no clamping | P1 |
| P-025 | SID numeric & string forms | blocks with `SID="42"` and `SID=":42"` | parse | both yield usable `sid`; resolver keys match | P0 |
| P-026 | Block properties preserved | Gain `Gain="1.5"`, `SaturateOnIntegerOverflow` etc. | parse | `properties` IndexMap contains exact strings | P0 |
| P-027 | `Commented` blocks | `<Block Commented="on" …>` | parse | `block.commented == true` | P1 |
| P-028 | NameLocation variants | `NameLocation="top|bottom|left|right|hide"` | parse | correct `NameLocation` enum per block | P1 |
| P-029 | Multi-cell numeric arrays | `<P Name="…">[1, 2; 3, 4]</P>` | parse | multiple cells preserved (see `tests/block.rs`) | P1 |
| P-030 | Newlines stripped from names | block/line names containing `\n` | parse | names contain no `\n` (existing unit test, e2e via file) | P1 |
| P-031 | Missing port indices auto-assigned | lines omitting `Port` index | parse | per-port-type sequential indices assigned | P0 |
| P-032 | Annotation blocks | `<Annotation>` entries | parse | `system.annotations` populated, not treated as blocks | P1 |
| P-033 | Mask blocks | block with `<Mask>` display properties | parse + `mask` feature | `block.mask` populated; `mask_eval` display text | P1 |
| P-034 | `ValueKind` classification | Constant w/ numeric, string, matrix values | parse | `value_kind` = Scalar/String/…, `value_rows/cols` set | P1 |
| P-035 | ZOrder preserved | `ZOrder="3"` | parse | `zorder == Some(3)`; renderer draws back-to-front | P2 |
| P-036 | Empty system | `<System/>` | parse | `Ok`, 0 blocks/0 lines | P1 |
| P-037 | Unknown BlockType tolerated | `<Block BlockType="Frobnicate">` | parse | `Ok`; renderer falls back to generic body | P1 |
| P-038 | Duplicate SID detection tolerated | Duplicat_Test.slx | parse + resolver | no panic; resolver deterministic (same result twice) | P1 |
| P-039 | Very long names/values | 10 KB block name, 64 KB param value | parse | preserved; no truncation | P2 |

### 6.3 Lines, Branches and Endpoints

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| P-050 | Simple src→dst line | `<Line><P Name="Src">"1#out:1"` … | parse | `line.src`/`dst` endpoint refs correct (sid, port_type="out"/"in", index) | P0 |
| P-051 | Branched line | line with 1 `<Branch>` child | parse | `dst.branches[0]` endpoints + points parsed | P0 |
| P-052 | Nested branches (depth 2) | branch containing `<Branch>` | parse | nested structure preserved via `branches` tree | P1 |
| P-053 | Line `Points` polyline | `<P Name="Points">[x y; x y; …]` | parse | ordered `points: Vec<Point>` | P0 |
| P-054 | Branch-level Points | points inside `<Branch>` | parse | branch.points populated independently of trunk | P1 |
| P-055 | Named signals / labels | `<P Name="Name">sig_a</P>` + `Label` | parse | `line.name`/label fields set; `resolved_line_label` | P1 |
| P-056 | Testpoint lines | `<P Name="TestPoint">on</P>` | parse | `line_has_testpoint == true`; marker position computable | P1 |
| P-057 | Unconnected dst (dangling) | line with `Dst` empty/missing | parse | `dst` None; no crash in resolver/render | P0 |
| P-058 | Line to subsystem port | dst = SubSystem sid | parse+resolve | boundary port mapping valid (CT section asserts) | P0 |
| P-059 | Control-port endpoints | dst port_type `enable`/`trigger`/`ifaction`/`reset` | parse | `is_control_port_type` true; excluded from data path | P0 |

### 6.4 Charts (Stateflow)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| P-070 | Chart block parses | `Simulink_Blocks.slx` Chart block | parse | `chart` structure attached to block | P1 |
| P-071 | State/transition labels | chart XML w/ labels | `chart.rs` parse | labels extracted for display | P1 |
| P-072 | Empty chart tolerated | `<Chart/>` | parse | `Ok`, no children | P2 |
| P-073 | Chart events/inputs ports | chart with event inputs | parse+port count | event ports counted (RT `event_input_offset` consistent) | P1 |

### 6.5 Graphical Interface & Dashboard Bindings

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| P-080 | `graphicalInterface` parses | FX-05 slx w/ GI xml | `parser::graphical_interface` | `DashboardBinding`s extracted (block + `DashboardTargetPath`) | P0 |
| P-081 | Binding target path resolution | binding→`model/subsys/sig` | `dashboard_binding_target_path` | path segments normalized (`normalized_path_segment`) | P0 |
| P-082 | `blockdiagram.xml.rels` parsed | rels xml | `parse_rels_from_entries` | rel ids→targets map used by `resolve_dashboard_bindings` | P1 |
| P-083 | Missing GI file tolerated | slx w/o GI | parse | `Ok`, empty bindings | P1 |
| P-084 | Dangling binding (target missing) | GI refs deleted block | parse | binding kept but resolution yields no live target; viewer falls back | P1 |

### 6.6 Libraries

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| P-090 | Built-in virtual libraries resolve | block `Reference="simulink/Math Operations/Gain"` | parse+resolve | catalog block resolved (labels/icon/ports) | P0 |
| P-091 | User library via `-L` path | FX-08 lib.slx + model | `analyze_libraries`/`LibraryResolver` | `found_at` path reported; blocks linked | P0 |
| P-092 | Missing library reported | model refs absent lib | resolver | reported missing, no panic; block labelled w/ lib name | P0 |
| P-093 | Library subsystem supplies port names | FX-08 subsystem in lib | resolve | host block port labels come from library def | P0 |
| P-094 | Nested library-in-library | lib A uses lib B | resolve | transitive resolution works | P1 |
| P-095 | Catalog completeness | n/a | `catalog` | ≥750 entries, non-empty fields, search works | P1 |
| P-096 | Virtual matrix library | matrix lib blocks | resolve | recognized aliases (diagonal etc.) | P1 |

### 6.7 Model Services

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| P-100 | `count_subsystems` accuracy | FX-02 depth-4 | count | =4; empty system =0 | P0 |
| P-101 | `max_subsystem_depth` | FX-02 | depth | =4 | P0 |
| P-102 | `model_topology_signature` stable | same model twice | hash | identical u64 | P0 |
| P-103 | Signature changes on structural edit | FX-01 vs +1 line | hash | differs | P0 |
| P-104 | Signature ignores geometry-only change | block moved | hash | identical (non-geometry hash) | P1 |
| P-105 | Signature changes on rename property | line renamed | hash | differs (non-geometry props hashed) | P1 |

---

## 7. Serialization and Round-Trip Tests (SER)

Suite: `tests/e2e_serialize.rs` · features: none required; `egui` for lib-linked
round-trips. Cases prefixed **SER-**.

### 7.1 JSON

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| SER-001 | System→JSON→System identity | FX-01..FX-09 | `serde_json` round-trip | deep-equals original (`assert_eq!`) | P0 |
| SER-002 | Real model JSON stable | Simulink_Blocks.slx | parse→to_json→from_json | equal; field order deterministic (IndexMap) | P0 |
| SER-003 | JSON is human-diffable | FX-03 | `to_string_pretty` | stable key order for snapshot diffing | P2 |
| SER-004 | Unicode survives | names with `ä🚀` | round-trip | byte-identical strings | P1 |

### 7.2 Bincode (`SystemDoc`)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| SER-010 | save→load binary round-trip | FX-01 | `save_to_binary`/`load_from_binary` temp file | equal system | P0 |
| SER-011 | Real model binary round-trip | Simulink_Blocks.slx | bin round-trip | equal; file smaller than XML source | P1 |
| SER-012 | Binary load is faster than XML parse | Simulink_Blocks.slx | `binary_benchmark`-style ratio | load_bin < parse_xml (assert ratio <0.8, machine-scaled) | P2 |
| SER-013 | Corrupted binary rejected | truncated .bin | `load_from_binary` | `Err`, no panic | P0 |
| SER-014 | Empty system binary | empty System | round-trip | equal | P1 |

### 7.3 SLX Write / Regeneration (`generator`)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| SER-020 | write→read archive round-trip | FX-01..FX-05 | `write_to` then `from_reader` | equal `System` per entry | P0 |
| SER-021 | System XML regenerated faithfully | FX-03 bus model | `generate_system_xml` → reparse | equal model | P0 |
| SER-022 | Raw entries preserved byte-exact | slx with images/mat files | write_to | raw entries byte-identical | P0 |
| SER-023 | Entry order preserved | multi-entry slx | write→read | same entry order | P1 |
| SER-024 | Compression flags preserved | mixed stored/deflated | write→zip inspect | methods preserved | P1 |
| SER-025 | `roundtrip_compare` equivalence | Simulink_Blocks.slx | example logic | regenerated system files semantically equal | P0 |
| SER-026 | Edit→save→reload keeps edit | FX-01 + move block | editor save→parse | new position persisted | P0 |
| SER-027 | New line survives round-trip | FX-01 + added line | save→parse | line+branch present w/ same endpoints | P0 |
| SER-028 | Created subsystem survives | selection→subsystem→save | save→parse | subsystem block + child xml entry exist | P1 |
| SER-029 | GI rels written for new bindings | FX-05 + added binding | save→parse | rels xml regenerated, bindings resolve | P1 |

---

## 8. Signal Propagation / Connection-Target Resolution Tests (CT)

Suite: `tests/e2e_propagation.rs` · features: `egui`.
Cases prefixed **CT-**. The resolver (`ConnectionTargetResolver`) is the heart of
signal tracing: for every *line* it yields downstream `ConnectionTarget`s
(`line_targets_for_line`) and for every *block port* both downstream targets
(`block_targets_for_block`) and upstream origins (`ConnectionTargetOrigin`).
E2E tests assert resolved sets as `(full_block_path, port_index)` tuples against
`ConnectivityOracle`s produced by the generator or golden files.

> Assertion helper: `assert_targets_eq` sorts and normalizes paths so tests are
> insensitive to enumeration order and to `/` vs `.` separators.

### 8.1 Flat-System Propagation

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| CT-001 | Direct line src→dst | In1→Gain→Out1 (FX-01 n=1) | `line_targets_for_line` | dst = {Gain.in:1}; Gain.out line → {Out1.in:1} | P0 |
| CT-002 | Fan-out 1→3 | one src line, 3 branch dsts | resolve | target set has exactly 3 endpoints, correct sids/ports | P0 |
| CT-003 | Chain through n blocks | gain_chain(8) | resolve each line | each line's targets = next block only; final → Out1 | P0 |
| CT-004 | Multiport consumer | Sum(2 in) fed by 2 lines | resolve | each line targets its own `in:i` port index | P0 |
| CT-005 | Multiport producer | Demux/MultiOut src ports | resolve | `block_targets_for_block` distinguishes out:1..out:n | P0 |
| CT-006 | Dangling line | src with no dst | resolve | empty target set; no panic | P0 |
| CT-007 | Self-loop via unit delay | Out→UnitDelay→same subsys in | resolve | targets form cycle; resolver terminates (visited set) | P0 |
| CT-008 | Diamond topology | A→{B,C}→D(Sum) | resolve | A's line reaches both B.in and C.in; D sees 2 inputs | P0 |
| CT-009 | Unconnected block | orphan Gain | `block_targets_for_block` | empty in/out sets | P1 |
| CT-010 | Named vs unnamed lines identical | same topo, one named | resolve | identical target sets (names don't affect topology) | P1 |
| CT-011 | Same-line dedup | `same_line(l1,l2)` on parsed twins | compare | true for identical, false after endpoint/point change | P1 |
| CT-012 | Port-count awareness | block `PortCounts` vs connected lines | `compute_port_info` | used counts ≤ declared; overflow port flagged/ignored per impl | P1 |

### 8.2 Subsystem Boundary Propagation (Hierarchy)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| CT-020 | In-port passthrough | parent line → SubSys.in1 → inner Gain | resolve | outer line's targets include inner `SubSys/Gain.in:1` (qualified path) | P0 |
| CT-021 | Out-port passthrough | inner Gain → SubSys.out1 → outer Sink | resolve | inner line targets `Sink.in:1` at parent level | P0 |
| CT-022 | Depth-4 chain | FX-02 nested subsystems | resolve end-to-end | root src reaches leaf sink; intermediate paths fully qualified `a/b/c/d/…` | P0 |
| CT-023 | Fan-in across boundary | 2 parents lines → same SubSys 2 in-ports | resolve | inner blocks see correct respective drivers (no cross-talk) | P0 |
| CT-024 | Fan-out across boundary | SubSys.out → 3 parent sinks | resolve | all 3 sinks in target set | P0 |
| CT-025 | Sibling subsystems chain | SubA.out → SubB.in | resolve | path crosses both boundaries | P0 |
| CT-026 | Inner goto/from inside subsystem | goto+from inside SubSys | resolve | inner consumers resolved (see §8.4) | P1 |
| CT-027 | Boundary port index mapping | SubSys with in-ports {1,2} to specific inner blocks | `boundary_port_index`, `child_*_targets_by_port` | port indices map to correct inner endpoints | P0 |
| CT-028 | Line to disabled/commented subsystem | `Commented="on"` SubSys | resolve | propagation stops/continues per documented semantics; no panic | P1 |
| CT-029 | Masked subsystem | masked SubSys w/ ports | resolve | behaves like normal subsystem | P1 |
| CT-030 | `resolve_subsystem_by_path` helpers | FX-02 | `resolve_subsystem_by_vec` | same &System as manual traversal | P1 |

### 8.3 Bus Propagation (Creator / Selector / Assignment)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| CT-040 | BusCreator passthrough set | 4 signals → BusCreator | `bus_creator_targets` | bus out targets all downstream consumers | P0 |
| CT-041 | BusSelector element extraction | bus → BusSelector(el 2) | `bus_selector_targets` | selected element maps to correct upstream signal | P0 |
| CT-042 | Selector multiple outputs | selector picking {1,3} | resolve | each output port carries its element's origin | P0 |
| CT-043 | Nested buses | bus(bus(2),2) → nested selector | resolve | leaf signal identified through 2 levels | P1 |
| CT-044 | BusAssignment | bus→BusAssignment(ch 2:=new)→Selector | `bus_assignment_targets` + selector | ch2 origin = new signal, others = original | P1 |
| CT-045 | Bus through subsystem boundary | creator inside, selector outside | resolve | elements tracked across boundary | P1 |
| CT-046 | Wide bus 64 elems | `gen_bus_fabric(1,64)` | resolve | all 64 origins correct; no truncation | P0 |
| CT-047 | Bus fan-out to 2 selectors | bus→SelA(1),SelB(2) | resolve | each selector gets its own element | P0 |
| CT-048 | `bus_*_upstream_targets` | selector block | upstream helpers | origins enumerate creator inputs | P1 |
| CT-049 | Selector out-of-range index | selector idx > bus width | resolve | empty/flagged target per impl; no panic | P1 |

### 8.4 Goto / From Tag Propagation

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| CT-060 | Same-level goto→from | Goto(tag A)→From(tag A)→Sink | `resolve_from_block_targets` | From's out targets = Goto's incoming signal targets ∪ From's consumers | P0 |
| CT-061 | Multiple Froms, one Goto | 1 goto, 3 froms same tag | resolve | all 3 from-lines get correct targets | P0 |
| CT-062 | Scoped visibility local | tag `A` scoped local in SubSys | resolve | from inside resolves; outside same-tag does not | P1 |
| CT-063 | Cross-level goto (global) | goto in parent, from in child | resolve | child's from resolves to parent signal | P1 |
| CT-064 | Goto upstream origins | from block | `upstream_propagated_targets` | origin chain lists real source block | P0 |
| CT-065 | Duplicate tags | 2 gotos same tag | resolve | deterministic target set; ambiguity handled per impl | P1 |
| CT-066 | Tag rename consistency | rename goto tag | re-resolve | old from no longer resolves; no stale cache | P1 |
| CT-067 | `qualify_external_path` for refs | model-relative path forms | helper | normalized full path prefix `model/…` | P1 |

### 8.5 Mux / Demux

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| CT-070 | Mux collects n inputs | 4→Mux | `mux_targets` | out targets all consumers | P0 |
| CT-071 | Demux per-port mapping | Mux→Demux(4) | `demux_targets` | out:i maps to in:i origin | P0 |
| CT-072 | Partial demux | Demux uses only ports 2,4 | resolve | only mapped outputs have targets | P1 |
| CT-073 | Mux through subsystem | mux inside, demux outside | resolve | channel identity preserved | P1 |
| CT-074 | `mux_upstream_targets`/`demux_upstream_targets` | consumers | upstream helpers | correct input origins | P1 |

### 8.6 Control Ports and Special Subsystems

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| CT-080 | Enable port is control, not data | enabled SubSys | `is_control_port_type("enable")` | true; enable line excluded from data targets | P0 |
| CT-081 | Trigger port | triggered SubSys | same | true; same exclusion | P0 |
| CT-082 | IfAction / function-call ports | if-action SubSys | resolve | control signal routed to control endpoint kind | P1 |
| CT-083 | Reinit subsystem port math | reinit SubSys (FX-07) | `is_reinit_subsystem_counts`, `reinit_data_input_pos`, `event_input_offset` | data input positions correct after event ports | P1 |
| CT-084 | `control_incoming_targets` | enabled subsys | helper | returns control-line origins only | P1 |
| CT-085 | Control+data interleave | SubSys with enable + 3 data ins | `line_data_input_ports` | returns {1,2,3} only, enable excluded | P0 |

### 8.7 Upstream Propagation and Metadata

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| CT-090 | `propagate_line_targets` bidirectional | FX-02 | call + inspect | each line's targets include downstream sinks AND upstream origins per API contract | P0 |
| CT-091 | `propagate_line_metadata_upward` | named line in child | resolve | metadata (name) propagates to parent-level view | P1 |
| CT-092 | Origin through buses | bus chain | upstream query | leaf origin block correct | P1 |
| CT-093 | Origin through goto/from | tag hop | upstream query | true driver block reported, not From block | P0 |
| CT-094 | `direct_internal_block_targets` | subsystem internals | helper | internal-only targets correct | P1 |
| CT-095 | `full_block_path` qualification | nested block | helper | `root/sub/…/block` string | P1 |

### 8.8 Resolver Lifecycle, Progress and Cache

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| CT-100 | `new` resolves eagerly | FX-02 | `ConnectionTargetResolver::new` | ready immediately; targets available | P0 |
| CT-101 | `new_with_progress` ticks | FX-10 (200 signals) | progress atomics | `visited` monotonically increases to total; `progress` reaches 100% | P1 |
| CT-102 | Deterministic results across runs | FX-10 | resolve twice | identical sets (BTreeSet compare) | P0 |
| CT-103 | `ComputedViewCache` validity | app cache | `mark_valid`/`is_valid` | true for same path+generation; false after `invalidate` | P0 |
| CT-104 | Cache invalidates on topology change | FX-01 + add line | `model_topology_signature` gen bump | `is_valid` false | P0 |
| CT-105 | `ResolverStatus::Ready` via `ensure_resolver` | SubsystemApp | poll until ready | Ready carries Arc; subsequent calls same Arc | P1 |
| CT-106 | Large-model resolver doesn't block >budget* | FX-13 (1000 sig) | time-boxed resolve | completes; *assert via visited-count progress not wall clock | P1 |
| CT-107 | Thread-safety of resolver | FX-10 | resolve on Rayon worker while UI polls progress | no data race (TSAN optional), consistent results | P2 |

### 8.9 Real-Model Propagation Oracle

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| CT-110 | `Propergation_Test.slx` golden | real model + `propergation.targets.json` | resolve all lines | diff = ∅ vs golden | P0 |
| CT-111 | Golden regeneration idempotent | above | `REGEN_GOLDEN=1` twice | byte-identical golden | P1 |
| CT-112 | Simulink_Blocks resolver coverage | Simulink_Blocks.slx | resolve | ≥95% of connected lines have ≥1 target; report uncovered as diagnostics not failure | P1 |

### 8.10 Propagation Invariants (asserted in every CT case)

1. **Termination:** resolver returns for cyclic graphs (feedback loops must not
   hang — unit-delay loops legal in Simulink).
2. **Path qualification:** every target path starts at model root and uses the
   same separator convention as `full_block_path`.
3. **Port bounds:** `1 ≤ port_index ≤ port_count(block)` unless port counts
   are undeclared.
4. **Symmetry:** for data lines, `line_targets(L) ⊇ consumers` and every
   consumer's upstream origins contain L's driver.
5. **No phantom targets:** resolved set ⊆ actual blocks/ports present in model.

---

## 9. Signal Routing and Line Geometry Tests (RT)

Suite: `tests/e2e_routing.rs` · features: `egui`.
Cases prefixed **RT-**. These tests cover the geometry layer that turns logical
connectivity into drawable orthogonal polylines, plus interactive point/branch
editing primitives that both the viewer move-mode and the editor build upon.

### 9.1 Orthogonalization (`orthogonalize_polyline*`)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| RT-001 | Horizontal stays | [(0,0),(10,0)] | orthogonalize | unchanged | P0 |
| RT-002 | Vertical stays | [(0,0),(0,10)] | orthogonalize | unchanged | P0 |
| RT-003 | Diagonal gets corner | [(0,0),(10,10)] | orthogonalize | 3 pts, corner (10,0) or (0,10) per dst-side rule | P0 |
| RT-004 | Multi-diagonal polyline | alternating diag segments | orthogonalize | all segments axis-aligned; endpoint preserved | P0 |
| RT-005 | Empty/single point | [], [(5,5)] | orthogonalize | returned unchanged, no panic | P0 |
| RT-006 | `with_dst_side` picks elbow orientation | diag + dst side hint | variant fn | final segment approaches from correct side | P0 |
| RT-007 | `dst_segment_horizontal` | port on left vs right side | helper | correct bool for end segment | P1 |
| RT-008 | `push_orthogonal_segments` batch | polyline | push into out vec | contiguous (Pos2,Pos2) segments all axis-aligned | P1 |
| RT-009 | Collinear duplicate points | [(0,0),(5,0),(10,0)] | orth + merge | redundant middle point removed by `merge_adjacent_corners` | P1 |
| RT-010 | Zero-length segment | [(3,3),(3,3)] | orthogonalize | handled, no NaN/degenerate crash | P1 |
| RT-011 | Negative coordinates | points in −x/−y | orthogonalize | same orthogonality invariant | P1 |

### 9.2 Endpoint Registration and Port Info

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| RT-020 | `endpoint_pos` block ports | block rect + port idx | helper | position = `port_anchor_pos` (side-correct, mirrored-aware) | P0 |
| RT-021 | `register_endpoint` collects | line endpoints | register into map | map key (sid,port) → pos | P1 |
| RT-022 | `register_branch_endpoints` recursive | branched line | register | every leaf branch endpoint registered | P1 |
| RT-023 | `compute_port_info` counts | mixed system | compute | per-(sid,port) used-count table correct | P0 |
| RT-024 | Mirrored block ports | `mirror_blocks`d Gain | endpoint_pos | ports swap sides; line attach point on drawn side | P0 |
| RT-025 | Rotated block ports | rotated block | endpoint_pos | per documented rotation handling; no panic | P1 |
| RT-026 | `port_kind` mapping | "in","out","enable",… | helper | distinct u8 per class; control kinds grouped | P1 |
| RT-027 | `event_input_offset` | chart/reinit blocks | helper | event ports offset data port indices | P1 |

### 9.3 Line/Point Mutation Primitives

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| RT-030 | `move_line_point` mid | 4-pt line, move pt 1 by (10,0) | apply | pt moved; next point compensated (segment stays orthogonal) | P0 |
| RT-031 | `move_line_point` last index | last point | apply | no crash; compensating logic no-op | P0 |
| RT-032 | `move_branch_point` | branch path + point idx | apply | only that branch's points change | P0 |
| RT-033 | `move_line_layout` shifts all | line+branches | move (5,7) | every point + endpoint offset by (5,7) | P0 |
| RT-034 | `move_branch_layouts` | nested branches | move | all branch points shifted | P1 |
| RT-035 | `get_branch_mut` path nav | depth-2 branch tree | get via [0,1] | correct mutable branch; bad path → None | P0 |
| RT-036 | `collect_branch_handle_positions` | branched line | collect | one handle pos per branch vertex, deduped | P1 |

### 9.4 Corner Operations (`corner_ops`)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| RT-040 | `insert_corner` mid-segment | 2-pt line | insert at idx+offset | point count +1; geometry equivalent path | P0 |
| RT-041 | `remove_corner` | 3-pt line | remove mid | returns removed point; line redrawn straight/orth | P0 |
| RT-042 | `merge_adjacent_corners` threshold | points 2px apart | merge(th=4) | collapsed to one; th=0 keeps both | P0 |
| RT-043 | `enforce_orthogonal` repairs diagonals | hand-built diagonal | enforce | all segments axis-aligned | P0 |
| RT-044 | `auto_adjust_on_block_move` src | block moved (dx,dy) | adjust | attached line's near-end follows; far-end fixed | P0 |
| RT-045 | `auto_adjust_on_block_move` dst | consumer moved | adjust | dst end follows | P0 |
| RT-046 | `auto_adjust_branches_on_block_move` | branched line | adjust | all branch junction/leaf segments stay connected | P0 |
| RT-047 | Corner ops on empty line | no points | each op | graceful no-op | P1 |

### 9.5 Auto-Route and Snapping (`editor::operations`)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| RT-050 | `auto_route` straight horizontal | out→in same Y | route | single segment | P0 |
| RT-051 | `auto_route` dogleg | dst lower/right | route | 1 elbow; orthogonal | P0 |
| RT-052 | `auto_route` backward connection | dst left of src | route | routes around (≥3 segments), no diagonal | P0 |
| RT-053 | `auto_route` same-height adjacent | ports aligned | route | minimal segment count | P1 |
| RT-054 | `find_snap_port` inside radius | cursor 3px from port | snap | returns Some((sid,port_kind,idx)) | P0 |
| RT-055 | `find_snap_port` outside radius | cursor 50px away | snap | None | P0 |
| RT-056 | Snap prefers nearest port | two ports 10px apart | snap midway | nearest wins; deterministic tiebreak | P1 |
| RT-057 | Snap kind filtering | hover out-port while drawing from in-port | snap | only compatible kind candidates | P0 |
| RT-058 | Route around obstacle* | block between ports | auto_route | path avoids overlap where impl supports; else documented straight | P2 |

### 9.6 Line Coloring and Style

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| RT-060 | Adjacency graph built | 3 lines sharing src/dst | `compute_line_adjacency` | adjacency edges correct | P0 |
| RT-061 | Adjacent lines differ in hue | adjacency≠∅ | `assign_line_colors` | hue distance > threshold for neighbours | P0 |
| RT-062 | Color count == line count | n lines | assign | vec len n | P0 |
| RT-063 | Dark-bg colors stay visible | dark theme | assign | luminance-aware (no near-black colors) | P1 |
| RT-064 | `circular_dist` hue wraparound | 350° vs 10° | helper | dist=20° | P0 |
| RT-065 | Testpoint marker | testpoint line | `line_testpoint_marker_position` | pos on segment; stroke wider | P1 |
| RT-066 | `line_stroke_width` variants | testpoint/selected/normal | helper | distinct widths | P1 |
| RT-067 | `resolved_line_label` fallback chain | named>propagated>empty | helper | correct precedence | P0 |
| RT-068 | Editor `compute_line_colors` parity | same input editor fn | compare | consistent with viewer coloring | P1 |

### 9.7 Routing Invariants

1. **Orthogonality:** after any mutation that claims to orthogonalize
   (`auto_route`, `enforce_orthogonal`, connection-drag commit), every
   consecutive point pair differs in exactly one coordinate.
2. **Connectivity preservation:** moving blocks/lines never changes logical
   endpoints (sid/port) — only `points` geometry.
3. **Idempotence:** `orthogonalize(orthogonalize(p)) == orthogonalize(p)`.
4. **Branch locality:** mutations under `get_branch_mut(path)` never touch
   sibling branches.
5. **Snap honesty:** `find_snap_port` only returns ports that exist in
   `compute_port_info`'s port universe for the current system.

---

## 10. Editor End-to-End Tests (ED) — Drag-and-Drop Focus

Suites: `tests/e2e_editor_dnd.rs` (gesture-level) and `tests/e2e_editor_ops.rs`
(command/history-level) · features: `egui` (`+highlight` for code-editor cases).
Cases prefixed **ED-**. These drive `editor::ui::canvas` (and menus/panels)
through `EditorHarness` (§4.3) — real `egui` input frames, real `Sense`
transitions — and assert both **model mutations** and **`EditorHistory`
effects**.

### 10.1 Gesture Mechanics Primer

#### 10.1.1 How egui drag actually works (why gestures need ≥3 frames)

`Sense::click_and_drag` transitions per frame:

```
frame k:   pointer down at p0            → resp.is_pointer_button_down_on()
frame k+1: pointer move p0→p1            → resp.drag_started() == true   [DragMode armed]
frame k+2..n-1: moves p1..pn-1           → resp.dragged()                [delta applied]
frame n:   pointer up at pn              → resp.drag_stopped()           [commit + Command]
```

Consequences codified in `harness/input.rs`:
- A "drag" is `press` + ≥1 `move` + `release`, each in its own `ctx.run` frame.
- `drag_started` fires on the first move frame — a 0-distance press/release is a
  *click*, never a drag; tests must not conflate them (ED-201 verifies).
- `drag_stopped` delivers the final delta; editors commit `EditorCommand` there.
- Release-off-canvas and Esc-cancel sequences are distinct cases (ED-240s).

#### 10.1.2 Coordinate bookkeeping

All pointer positions are **screen-space** (`Pos2`); the harness translates to
model coordinates through the same `ViewTransform` the app uses
(`vt.from_screen`). Tests fetch block/port screen positions via
`h.screen_pos_of_block(sid)` / `h.port_screen_pos(sid, side, idx)` instead of
hardcoding pixels, so layouts survive theme/font changes.

#### 10.1.3 What a committed DnD must produce

For every drag E2E case the *same triad* is asserted:

1. **Model:** the mutation happened (position/endpoints/name changed).
2. **History:** exactly one `EditorCommand` pushed; `undo()` restores the
   exact prior `System` (deep equality incl. geometry); `redo()` reapplies.
3. **Dirty/UI state:** `mark_dirty` called; `drag_mode` returns to `None`;
   no stuck selection/hover state.

### 10.2 Block Drag (`DragMode::Blocks`)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-001 | Drag single block | FX-01 editor, cursor on Gain | drag (+40,+20)px | position += snap(delta); history+1; undo restores | P0 |
| ED-002 | Drag snaps to grid | grid=10, drag (+43,+17) | release | position rounded to (40,20) model units | P0 |
| ED-003 | Drag selected group | 3 blocks selected | drag one of them (+30,0) | all 3 positions +30 x; one command (group undo) | P0 |
| ED-004 | Drag unselected block selects it | 0 selected | press on block, drag | block becomes selected, then moves | P0 |
| ED-005 | Click without drag = select only | press+release same px | gesture | `drag_mode` never armed; no command; selected | P0 |
| ED-006 | Sub-pixel drag (below threshold) | move 0.3px, release | gesture | no command emitted; treated as click | P1 |
| ED-007 | Drag attached lines follow | Gain connected both sides | drag (+50,0) | `auto_adjust_on_block_move` applied to src & dst lines; orthogonality preserved | P0 |
| ED-008 | Drag block with branched line | line w/ 2 branches | move dst block | `auto_adjust_branches_on_block_move` keeps junction consistent | P0 |
| ED-009 | Drag into occupied space | two blocks | drag A onto B | allowed (Simulink allows overlap) OR snapped aside — assert documented behaviour | P1 |
| ED-010 | Drag off-canvas | block near edge | drag 2000px left | position clamps per impl or allowed negative; no panic | P1 |
| ED-011 | Rapid sequential drags | same block | 5 drags w/o idle | 5 commands; each undoable independently | P1 |
| ED-012 | Drag during live resolver | resolver running bg | drag block | no deadlock; targets refresh after | P2 |
| ED-013 | Arrow-key nudge | block selected | → key ×3 | position +3×grid; 1 command per press or batched per impl | P1 |
| ED-014 | Shift+arrow coarse nudge | selected | shift+→ | larger step per impl | P2 |
| ED-015 | Drag subsystem block updates inside? | SubSys block | drag | only block moves; internal system untouched | P1 |

### 10.3 Connection Drag (`DragMode::Connection`)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-020 | Draw line out→in | Gain.out1 → Scope.in1 | press out-port chevron, drag to in-port, release | new `Line` Src=Gain#out:1 Dst=Scope#in:1; routed orthogonally; history+1 | P0 |
| ED-021 | Draw line in→out (reversed) | start on in-port | drag to out-port | same logical line created (src/dst normalized) | P0 |
| ED-022 | Connection preview while dragging | mid-drag frame | inspect shapes | preview polyline drawn cursor→port; orthogonal | P0 |
| ED-023 | Drop on empty canvas | drag to void | release | no line; no command; `drag_mode=None` | P0 |
| ED-024 | Drop on invalid port kind | out→out | release | rejected; no line | P0 |
| ED-025 | Snap to nearby port | release 4px off in-port | drop | connects to snapped port (within `find_snap_port` radius) | P0 |
| ED-026 | No snap beyond radius | release 40px off | drop | no connection | P0 |
| ED-027 | Port already at max inputs | in-port occupied | drag 2nd line | documented behaviour: multi-input allowed? assert impl (replaced/rejected/added) | P1 |
| ED-028 | Esc cancels connection drag | mid-drag | Esc key | preview gone; no line; mode None | P0 |
| ED-029 | Release outside window | drag exits canvas rect | release outside | treated as cancel (no commit) | P1 |
| ED-030 | Connect across subsystem boundary in canvas | parent view, SubSys.in port visible | drag to SubSys port | line targets boundary port correctly | P1 |
| ED-031 | Connection to control port | enable port chevron | drag | port_kind=control; stored endpoint port_type correct | P1 |
| ED-032 | Multi-out-port selection | Demux.out2 | start on out2 chevron | line src port_index=2 not 1 | P0 |
| ED-033 | Connection drag while zoomed | zoom=2.5× | full gesture | model coords correct (transform exercised) | P1 |
| ED-034 | Connection creates intermediate point | auto_route dogleg | commit | `line.points` matches `auto_route` output | P1 |

### 10.4 Selection Rectangle (`DragMode::SelectionRect`)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-040 | Marquee selects enclosed blocks | 3 blocks in rect path | drag rect over 2 | exactly those 2 selected | P0 |
| ED-041 | Marquee partial-overlap rule | rect overlaps 30% of block | per `overlaps_rect` | inclusion matches documented rule (touch vs contain) | P0 |
| ED-042 | Marquee includes lines | rect over line points | select | lines whose geometry inside → selected (per impl) | P1 |
| ED-043 | Esc/empty marquee | drag rect over nothing | release | selection cleared (or unchanged w/ modifier) | P1 |
| ED-044 | Ctrl-click toggle | 1 selected | ctrl-click other | both selected; ctrl-click again → deselects | P0 |
| ED-045 | Shift-click adds | 1 selected | shift-click other | both selected | P1 |
| ED-046 | Click empty space clears | 2 selected | click void | selection empty | P0 |
| ED-047 | `SelectionRect::normalized` | drag up-left | normalize | (l,t,r,b) ordered | P1 |
| ED-048 | Marquee then group-drag | rect-select 3 | drag any | all move (ED-003 path via UI) | P0 |

### 10.5 Resize Drag (`DragMode::Resize`)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-050 | SE handle grows | block, cursor handle 4 | drag (+20,+10) | r+=snap(20), b+=snap(10); l,t unchanged | P0 |
| ED-051 | NW handle grows up-left | handle 0 | drag (−20,−10) | l,t move; r,b fixed | P0 |
| ED-052 | Edge handles E/N/W/S | handles 3,1,7,5 | each dragged | only that edge moves | P0 |
| ED-053 | Minimum size enforced | shrink to <min | drag | clamped ≥ impl min; no negative width/height | P0 |
| ED-054 | Resize undoable | any resize | commit+undo | original rect restored | P0 |
| ED-055 | `compute_resized_rect` math | direct calls | helper | matches table above for all 8 handles | P0 |
| ED-056 | `resize_handle_positions` order | rect | helper | returns 8 positions TL,T,TR,R,BR,B,BL,L | P1 |
| ED-057 | Resize then port pos recomputed | resized SubSys | after commit | line endpoints still attach to port positions | P1 |
| ED-058 | Resize snap | grid on | drag 13px | snapped to 10 | P1 |

### 10.6 Pan (`DragMode::Pan`)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-060 | Middle-drag pans | any model | middle-button drag | `vt.pan` shifts; model unchanged; no command | P0 |
| ED-061 | Pan preserves zoom | zoom=1.7 | pan | zoom unchanged; from_screen(to_screen)=id | P1 |
| ED-062 | Space+drag pans (if supported) | space held | left-drag | pan not block-drag — per impl | P2 |

### 10.7 Line & Label Drag in Viewer Move-Mode (`ViewerDragState`)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-070 | `ViewerDragState::Blocks` | viewer move mode | drag block | `current_dx/dy` accumulate; commit on release moves block | P0 |
| ED-071 | `LinePointDrag` | grab line vertex | drag | `move_line_point` semantics; orthogonality kept | P0 |
| ED-072 | `BranchPointDrag` | grab branch vertex path [0] | drag | `move_branch_point` on that branch only | P0 |
| ED-073 | `SignalLabelDrag` | grab label | drag | label offset stored; line geometry untouched | P1 |
| ED-074 | `ViewerDragState::Resize` | viewer resize handle | drag | same math as editor resize | P1 |
| ED-075 | Viewer drag cancel mid-way | mid-drag | Esc/context loss | state → None; partial delta discarded or committed per impl (assert doc'd) | P1 |

### 10.8 Clipboard and Edit Commands

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-080 | Copy→paste duplicates | 2 blocks+1 line selected | Ctrl+C, Ctrl+V | pasted copies offset; new SIDs assigned; line between copies rewired | P0 |
| ED-081 | Paste assigns fresh SIDs | pasted block | inspect | `assign_sids` gives unique sid ≠ source | P0 |
| ED-082 | Cut = copy+delete | selection | Ctrl+X | clipboard has content; originals deleted; 1 undo restores both? (per impl) | P1 |
| ED-083 | Paste empty clipboard noop | nothing copied | Ctrl+V | no mutation, no command | P1 |
| ED-084 | Delete selection | 2 blocks+line | Del | `delete_blocks`+`delete_lines`; connected surviving lines' endpoints cleaned | P0 |
| ED-085 | Delete cleans dangling lines | delete block w/ line | Del | attached lines removed too | P0 |
| ED-086 | `EditorClipboard::has_content` | copy then clear | inspect | false after `clear()` | P1 |

### 10.9 Comment / Rotate / Mirror

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-090 | Comment-out block | selected | `comment_blocks` (shortcut) | `commented=true`; rendered hatched; resolver skips per semantics | P0 |
| ED-091 | Rotate 90° | selected | `rotate_blocks` | position/ports rotate; lines re-attach to new port sides | P1 |
| ED-092 | Mirror | selected | `mirror_blocks` | port sides swap (RT-024); line endpoints follow | P1 |
| ED-093 | All three undoable | each op | undo | exact restore | P0 |

### 10.10 Subsystem Creation (`create_subsystem_from_selection`)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-100 | Group→subsystem | 3 connected blocks selected | invoke | new SubSystem block; inner system holds blocks; boundary lines rewired to In/Out ports | P0 |
| ED-101 | Boundary lines preserved | ext lines in & out of selection | create | SubSys gets in/out ports matching external connectivity; resolver equivalence pre/post | P0 |
| ED-102 | Undo restores flat model | after create | undo | original blocks+lines restored verbatim | P0 |
| ED-103 | Internal lines stay internal | line wholly inside selection | create | moved into subsystem unchanged | P1 |
| ED-104 | Selection with 0 blocks | nothing selected | invoke | no-op or disabled; no panic | P1 |

### 10.11 Lines: Rename and Branch

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-110 | `rename_line` sets label | line selected | rename dialog/commit | name persisted; label rendered (RT-067) | P0 |
| ED-111 | `branch_line` creates branch | drag from existing line mid-point to new sink | commit | `branches[]` gains dst; trunk unchanged | P0 |
| ED-112 | Branch via `add_line` cmd | API-level | add_line w/ branch | targets resolve (CT-002) | P0 |
| ED-113 | `delete_lines` removes branch | select branch segment | Del | only branch removed | P1 |

### 10.12 Undo / Redo Integrity

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-120 | History cap respected | `EditorHistory::new(8)` | 12 commands | oldest evicted; `can_undo` bounded | P1 |
| ED-121 | Undo all → pristine | 10 mixed ops | undo ×10 | system deep-equals initial; `can_undo=false` | P0 |
| ED-122 | Redo all → final | after ED-121 | redo ×10 | equals post-ops state | P0 |
| ED-123 | New edit clears redo | undo 2, then edit | inspect | `can_redo=false` | P0 |
| ED-124 | `clear()` empties | non-empty | clear | neither undo nor redo possible | P1 |
| ED-125 | Interleaved UI gestures | sequence of real drags/keys | mixed undo/redo | deterministic final state | P0 |

### 10.13 Block Browser, Code Editor, Menus, Panels

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-130 | Browser opens at cursor | canvas | double-click/shortcut | `BlockBrowserState.open_at` pos correct | P1 |
| ED-131 | Browser search→insert | type "gain", pick | Enter | `create_default_block`+`add_block` at open position | P0 |
| ED-132 | `create_default_block` catalog | each catalog block_type | create | valid block w/ default props+ports | P1 |
| ED-133 | Code editor for MATLAB fn | `is_code_block` true | open panel | `get_block_code` loaded; edit→`set_block_code` persists | P1 |
| ED-134 | `is_modified` flag | edit code | inspect | true until saved | P1 |
| ED-135 | Menus dispatch commands | each menu item | click | expected `EditorCommand`/dialog | P1 |
| ED-136 | Shortcuts map | each `shortcuts.rs` binding | key event | matching op dispatched | P0 |
| ED-137 | Panels edit block props | select Gain | set Gain=2.5 in panel | property written; undoable | P1 |

### 10.14 Position Helpers and SID Assignment

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-140 | `parse_position` forms | `"[a, b, c, d]"` variants | parse | Some((a,b,c,d)); malformed→None | P0 |
| ED-141 | `format_position` round-trip | (l,t,r,b) | format→parse | identity | P0 |
| ED-142 | `assign_sids` uniqueness | model w/ dup/missing sids | assign | all numeric unique; existing preserved where valid | P0 |
| ED-143 | SID assignment undoable | after assign | undo | original sids restored | P1 |

### 10.15 Editor Save/Load E2E

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ED-150 | Dirty flag lifecycle | edit → `mark_dirty` | save | `clear_dirty`; file written | P0 |
| ED-151 | Save→close→reopen keeps edits | moved block+new line | round-trip | persisted (SER-026/27 via editor path) | P0 |
| ED-152 | Reject save when path invalid | read-only dir | save | error surfaced; model intact | P1 |

---

## 11. Viewer End-to-End Tests (VW)

Suite: `tests/e2e_viewer.rs` · features: `egui`.
Cases prefixed **VW-**. Drive `SubsystemApp` through `ViewerHarness`:
navigation, view transforms, rendering presence, hover/click behaviour, dialogs,
and move-mode. Unlike ED tests (which own the model), VW tests treat the model
as read-only except for documented move-mode geometry edits.

### 11.1 Navigation and Hierarchy

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| VW-001 | Open at root | FX-02 | app start | root entities rendered; breadcrumb shows root | P0 |
| VW-002 | Double-click subsystem descends | SubSys visible | dbl-click | `current_entities` = child; breadcrumb extended | P0 |
| VW-003 | Breadcrumb back-nav | depth-3 | click crumb level 1 | path truncated; `resolve_subsystem_by_vec` agrees | P0 |
| VW-004 | `collect_subsystems_paths` completeness | FX-02 | collect | all 4 nested paths found | P1 |
| VW-005 | Navigation listener fires | registered listener | descend/ascend | `emit_subsystem_changed` received per hop | P1 |
| VW-006 | Invalid path guarded | `resolve_subsystem_by_path("x/y")` | call | None, no panic | P1 |
| VW-007 | Deep zoom navigation | depth-12 FX-11 | descend ×12 | each level renders; no stack overflow | P1 |

### 11.2 View Transform, Zoom and Pan

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| VW-010 | Fit-to-view on open | FX-01 | first frame | all block rects inside canvas viewport | P0 |
| VW-011 | `zoom_at` anchors cursor | zoom 2× at p | transform | model point under cursor stays under cursor | P0 |
| VW-012 | Scroll-wheel zoom sequence | scroll in ×5, out ×5 | transform | returns to ≈initial zoom (±ε) | P1 |
| VW-013 | Zoom bounds clamped | zoom ×50 / ÷50 | inspect | clamped to impl min/max; fonts still scaled sanely | P1 |
| VW-014 | `font_scale` monotone | range of zoom | helper | `shared_canvas_text_font_px` >0, continuous, bounded | P0 |
| VW-015 | Pan by drag | viewer | drag background | `vt.pan` delta matches; to_screen shifts uniformly | P0 |
| VW-016 | `to_screen`/`from_screen` round-trip | grid of pts | both fns | error <0.5px | P0 |
| VW-017 | `preview_block_rect` drag preview | Blocks drag mid-frame | helper | rect offset by current delta | P1 |
| VW-018 | Zoom controls widget | `ui/zoom_controls` | click +/−/fit | zoom steps and refit | P1 |

### 11.3 Rendering Presence (paint-level)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| VW-020 | All blocks drawn | FX-01 | frame | ≥1 rect/glyph shape per block bbox | P0 |
| VW-021 | Lines drawn as segments | connected model | frame | `Line`/`Path` shapes between expected port positions | P0 |
| VW-022 | Port chevrons drawn | block w/ ports | frame | chevron shapes at `port_anchor_pos` | P1 |
| VW-023 | Block name labels drawn | named blocks | frame | text shapes contain names (galley text assert) | P1 |
| VW-024 | Goto/From tag glyphs | FX-04 | frame | tag label text present near blocks | P1 |
| VW-025 | Logic-gate icons | logic blocks | frame | custom painter invoked (shape count > generic) | P1 |
| VW-026 | Signal labels on lines | named line | frame | label text at `label_place` position | P1 |
| VW-027 | `label_place` avoids collisions | 2 labels same segment | frame | different chosen positions | P1 |
| VW-028 | Commented blocks hatched/greyed | commented block | frame | distinct fill/stroke vs normal | P1 |
| VW-029 | Selection highlight | select block | frame | selection stroke present | P1 |
| VW-030 | Color theme application | dark/light | frame | bg + `contrast_color` text consistent | P1 |
| VW-031 | Z-order respected | overlapping blocks | frame | later zorder drawn on top (shape order) | P2 |
| VW-032 | Inport/outport shadow blocks | `<Reference>` inports | frame | rendered like port blocks | P1 |

### 11.4 Hover, Tooltips, Dialogs, Context Menus

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| VW-040 | Block hover tooltip | hover 3 frames | inspect | tooltip layer contains block name/type | P1 |
| VW-041 | Line hover shows targets* | live mode + resolver ready | hover line | tooltip lists resolved targets (`live_line_tooltips`) | P0 |
| VW-042 | Block hover shows origins | above | hover block | `live_block_tooltips` entries listed | P1 |
| VW-043 | `live_tooltips_enabled` off | toggle off | hover | no tooltip shapes | P1 |
| VW-044 | `SignalDialog` opens | line ctx-menu item | click | dialog state populated w/ line info | P1 |
| VW-045 | `BlockDialog` opens | block ctx-menu | click | dialog populated w/ props | P1 |
| VW-046 | Context menu items dispatched | each `SignalContextMenuItem`/`BlockContextMenuItem` | click | documented action taken | P1 |
| VW-047 | Dialog closes on Esc/outside | open dialog | Esc | dialog state None | P1 |

### 11.5 Move Mode vs View Mode

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| VW-050 | Move-mode toggle | toolbar | click | block drags enabled (`ViewerDragState`) | P0 |
| VW-051 | View-mode drag = pan not move | move off | drag block | canvas pans; block unmoved | P0 |
| VW-052 | Move commits geometry only | move block | release | endpoints untouched; `same_line` still true vs prior | P0 |
| VW-053 | Line-point drag in move mode | grab vertex | drag | points updated (RT-031 path) | P0 |
| VW-054 | Move-mode doesn't corrupt resolver | move block | re-resolve | topology signature unchanged → cache still valid | P1 |

### 11.6 Text Pipeline

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| VW-060 | HTML entities decoded | name `a &gt; b &amp; c` | render | decoded text drawn (`html_escape::decode_html_entities`) | P0 |
| VW-061 | Entity edge cases | `&quot;`, `&#x41;`, unknown `&foo;` | decode helper | known decoded; unknown passthrough | P1 |
| VW-062 | Syntect highlight (feature `highlight`) | MATLAB code block | render | syntax found; spans colored | P1 |
| VW-063 | Missing syntax fallback | no MATLAB syntax | find chain | fallback to plain text, no panic | P2 |
| VW-064 | Multiline names | `Name\nLine2` | render | parsed clean (P-030) and drawn multiline | P1 |

---

## 12. Live Values and Formatting Tests (LV)

Suite: `tests/e2e_live.rs` · features: `egui` (+`dashboard` for widget cases).
Cases prefixed **LV-**. `live_values.rs` defines `LiveValue{dims, data}`,
`LiveValueList` (scalar/vector/matrix/complex variants),
`LiveValueEntry` (+display opts), and formatting with scientific-notation
thresholds (`LIVE_SCIENTIFIC_LOWER_BOUND/UPPER_BOUND`,
`DEFAULT_LIVE_FLOAT_DECIMALS`). The viewer displays them via
`live_values`/`live_block_values` maps, tooltips, and `MiniScope` streams.

### 12.1 Value Model and Formatting

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| LV-001 | Scalar f64 formats | LiveValue scalar 1.5 | `format` | "1.50" (default decimals) | P0 |
| LV-002 | Vector formats `[a b c]` | dims [3] | format | space-separated, Simulink order | P0 |
| LV-003 | Matrix column-major | 2×3 matrix | format | column-major like Simulink (existing unit test via e2e path) | P0 |
| LV-004 | Scientific at bounds | 1e-300 / 1e300 | `scalar_requires_scientific` | outside bounds → sci notation | P0 |
| LV-005 | Custom bounds respected | opts w/ custom bounds | `scalar_requires_scientific_with_bounds` | honoured | P1 |
| LV-006 | Always-scientific flag | display opts | format | sci regardless of magnitude | P1 |
| LV-007 | `first_f64`/`f64_at` accessors | each LiveValueList variant | calls | correct value/None semantics | P0 |
| LV-008 | Complex32/64 values | LiveComplex{re,im} | format/len | handled; len semantics documented | P1 |
| LV-009 | `len`/`is_empty` per variant | all variants | calls | consistent with dims product | P0 |
| LV-010 | `LiveValueEntry::with_display` override | entry display vs defaults | format | per-entry opts win | P1 |
| LV-011 | Very long vector | dims [10_000] | format | bounded output (truncation policy documented; no 10k-col string DoS) | P1 |
| LV-012 | NaN/Inf values | scalar NaN, +Inf | format | rendered literally, no panic | P0 |

### 12.2 Live Mode in the Viewer

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| LV-020 | `live_mode_enabled` renders values on blocks | app + live_values | frame | value text on mapped blocks (`should_render_live_text`) | P0 |
| LV-021 | Dashboard blocks don't double-render | dashboard block type | `should_render_live_text` | false — widget owns display | P0 |
| LV-022 | Mask display wins when masked | masked block | `should_use_mask_display` | mask text over live text | P1 |
| LV-023 | Manual switch state from value | value 0.0/0.49/0.5/1.0 | `manual_switch_setting_from_live_value` | "0"/"0"/"1"/"1" | P0 |
| LV-024 | Line tooltips show live value | hover line w/ entry | tooltip | `live_line_tooltips` content rendered | P1 |
| LV-025 | Block tooltips | hover block w/ `live_block_tooltips` | tooltip | entries rendered | P1 |
| LV-026 | Display defaults propagate | `live_display_defaults` changed | format path | entries w/o override use new defaults | P1 |
| LV-027 | Live mode off hides values | toggle off | frame | static icons/text only | P0 |
| LV-028 | Stale value handling | value map cleared | frame | blocks fall back, no stale text | P1 |

### 12.3 MiniScope and Scope Popouts

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| LV-030 | `MiniScope::push_sample` buffers | new scope | push 100 | internal series len 100 | P0 |
| LV-031 | Scope capacity/window | push > capacity | inspect | windowed to capacity (ring semantics per liveplot) | P1 |
| LV-032 | Embedded scope renders | `show_embedded` in viewer | frame | liveplot tiles drawn inside block region | P0 |
| LV-033 | `set_signal_name` | scope | set | title/label updated | P1 |
| LV-034 | Popout opens | scope ctx action | `show_popout` | `scope_popout` Some w/ key; window shape present | P1 |
| LV-035 | Popout close clears | open popout | close | `scope_popout=None`; `clear_popout_state` removes stale entry | P1 |
| LV-036 | `draw_scope_glyph` icon | scope block | helper | glyph shapes >0 | P2 |
| LV-037 | Multi-scope isolation | 2 scopes different keys | push to A | B series unchanged | P1 |
| LV-038 | Scope survives navigation | leave & re-enter subsystem | frames | `scope_instances` persisted via shared map | P1 |

---

## 13. Dashboard Widget E2E Tests (DB)

Suite: `tests/e2e_dashboard.rs` · features: `dashboard`.
Cases prefixed **DB-**. Dashboard blocks come in two render modes: static
icons (feature off / live mode off) and **live widgets** (feature on +
`live_mode_enabled`). Live widgets are bound to model signals/blocks through
`DashboardBinding`s (P-080) and emit `DashboardControlEvent`s that a host
application consumes to drive a simulator.

### 13.1 Widget Rendering (feature `dashboard`)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| DB-001 | Static icon fallback w/o feature-equivalent | `dashboard` built, `live_mode_enabled=false` | frame | simple icon, no live widget | P0 |
| DB-002 | Each widget type renders live | FX-05 (one of each) | frame, live mode | `live_*` fns produce expected widget shapes: push button, slider switch, radio group, combo box, checkbox, slider/linear gauge, field/display, toggle, radial gauge, rocker switch, lamp | P0 |
| DB-003 | Binding shows bound value | binding → live_values[key] | frame | widget displays current value | P0 |
| DB-004 | Unbound widget default | no binding | frame | documented placeholder; no panic | P0 |
| DB-005 | `dashboard_scalar_value_from_pointer` | pointer paths | helper | resolves nested vector/matrix element | P0 |
| DB-006 | `dashboard_widget_value`/`dashboard_live_value`/`dashboard_input_control_kind` | all widget types | debug helpers | correct kind + value mapping | P0 |

### 13.2 Widget Interaction → `DashboardControlEvent`

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| DB-010 | PushButton press/release | live push button | press+release | events `PulseHigh` then `PulseLow` queued in order | P0 |
| DB-011 | Toggle switch click | live toggle | click | `Bool(!prev)` event | P0 |
| DB-012 | Checkbox click | live checkbox | click | `Bool` toggled event | P0 |
| DB-013 | Slider drag | live slider | drag to 0.7 | `Scalar(0.7±ε)` event; continuous updates during drag per impl | P0 |
| DB-014 | Radio group select | 3-option radio | click opt 2 | `Scalar(2)` or mapped value event | P0 |
| DB-015 | Combo box select | combo w/ items | select item | `Scalar`/index event per binding | P1 |
| DB-016 | Editable field entry | live field | type "3.14"+Enter | `Scalar(3.14)` event; edit buffer cleared per impl | P0 |
| DB-017 | Rocker switch | rocker | click | `Bool` event | P1 |
| DB-018 | Linear gauge is read-only | gauge | click/drag | **no** event emitted (display-only widget) | P1 |
| DB-019 | Lamp is read-only | lamp | click | no event | P1 |
| DB-020 | Radial gauge read-only | gauge | click | no event | P1 |
| DB-021 | Event carries originating Block | any event | inspect | `DashboardControlEvent.block` = widget's block (sid/path) | P0 |
| DB-022 | Queue drains once | emit 3 events | host consumes | `pending_dashboard_events` emptied | P0 |
| DB-023 | Rapid widget mashing | button | 20 clicks | 20 event pairs, ordered | P1 |

### 13.3 Binding Resolution E2E

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| DB-030 | Binding survives hierarchy | widget in child bound to root signal | resolve+render | value reaches widget | P0 |
| DB-031 | Binding through goto/from | bound signal via tag | resolve | live value follows propagated target | P1 |
| DB-032 | Binding to bus element | binding → bus[3] | resolve+value | element value extracted | P1 |
| DB-033 | Rebind after topology change | model edit moves signal | re-resolve | widget follows new binding or shows unbound | P1 |
| DB-034 | `DashboardTargetPath` normalization | odd path forms | `dashboard_binding_target_path` | normalized segments | P0 |

### 13.4 Host Loop Contract (documented for integrators)

1. Host pushes `LiveValueEntry`s into `app.live_values`/`live_block_values`
   keyed by resolved target paths.
2. Host drains `pending_dashboard_events` each tick and applies them to the
   simulator.
3. Widget paint reflects last pushed values; widget interactions never mutate
   the model directly.
E2E-DB loop tests (E2E-014) verify this contract end to end.

---

## 14. Stress, Scale and Performance Tests (PERF)

Suite: `tests/e2e_stress.rs` · features: `dashboard`; heavy cases marked
`#[ignore]` with `PERF=1` opt-in for nightly CI.
Cases prefixed **PERF-**. Focus: **many signals**, **deep/wide graphs**, and
**high data-rate** live streams. Assertions are machine-scaled (throughput
floors, growth bounds, complexity sanity) — never absolute wall-clock except
coarse guards (e.g. `< 30 s`).

> Measurement harness: `std::time::Instant` around the *phase* only; first-run
> compile/JIT warm-up excluded by a warm-up iteration. All perf tests also
> assert *correctness* (target sets, frame output) — a fast wrong answer fails.

### 14.1 Many-Signal Models

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| PERF-001 | Parse 1000-signal model | `gen_wide_net(1000,3)` slx | `SlxArchive::from_reader`+parse | < 10 s debug build; all lines parsed | P0 |
| PERF-002 | Parse 5000-signal model | `gen_wide_net(5000,3)` | parse | completes (opt-in PERF=1); memory < 2 GB | P1 |
| PERF-003 | Resolve 1000 signals | FX-13 | `ConnectionTargetResolver::new` | all 1000 lines resolved; oracle diff ∅; progress reaches 100% | P0 |
| PERF-004 | Resolve 5000 signals | `gen_wide_net(5000,3)` | resolve | completes (PERF=1); visited==total | P1 |
| PERF-005 | Resolution scales sub-quadratically | n ∈ {250,500,1000,2000} | time/line | t(2n) < 4.5× t(n) (O(n log n)..O(n¹·³) sanity bound) | P1 |
| PERF-006 | Render frame w/ 1000 lines | FX-13 viewer | 10 frames | each frame < 250 ms CPU tessellation (debug); shapes ∝ lines | P0 |
| PERF-007 | Render 5000 blocks | `gen_random_system` | frame | completes; no O(n²) shape explosion | P1 |
| PERF-008 | Line adjacency at scale | 2000 lines | `compute_line_adjacency` | < 2 s; symmetric adjacency matrix | P1 |
| PERF-009 | Color assignment at scale | above | `assign_line_colors` | < 1 s; n colors out | P1 |
| PERF-010 | Zoom/pan interactive under load | FX-13 viewer | 30 mixed ops | every op handled; no input starvation | P1 |

### 14.2 Deep and Wide Graphs

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| PERF-020 | Depth-12 hierarchy resolve | FX-11 | resolve | all leaf targets reached; no recursion blow-up | P0 |
| PERF-021 | Depth-20 hierarchy | `gen_deep_hierarchy(20,2)` | resolve | completes (watch stack depth; if recursive, assert iterative path exists or document limit) | P1 |
| PERF-022 | Wide subsystem (200 children/level) | `gen_deep_hierarchy(3,200)` | parse+render | completes | P1 |
| PERF-023 | Bus fabric 32×64-wide | `gen_bus_fabric(32,64)` | resolve | all 2048 element origins correct | P0 |
| PERF-024 | Goto/from mesh 500 pairs | `gen_goto_from_mesh(500)` | resolve | all pairs linked; deterministic | P1 |
| PERF-025 | Branch-tree lines (depth-5, 32 leaves) | synthetic | render+resolve | all leaf targets found; no exponential blow-up | P1 |
| PERF-026 | Corner-dense lines | `gen_many_corner_lines(50)` | render+edit | frame time bounded; corner ops O(corners) | P1 |

### 14.3 Editor at Scale

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| PERF-030 | Group-drag 500 blocks | 500 selected | drag (+10,0) | single command; undo restores all; < 5 s | P0 |
| PERF-031 | Marquee over 2000 blocks | canvas | drag rect | selection completes < 2 s | P1 |
| PERF-032 | Delete 1000 blocks | selected | Del | < 3 s; dependent lines cleaned | P1 |
| PERF-033 | Undo stack of 200 ops | 200 commands | undo ×200 | < 5 s total; final = initial | P1 |
| PERF-034 | Copy-paste 300-block selection | clipboard | Ctrl+V | unique SIDs; < 3 s | P1 |
| PERF-035 | `assign_sids` on 5000 blocks | generated | assign | < 2 s; all unique | P1 |

### 14.4 High Data-Rate Live Streaming

The key scenario for "high data-rate": a host pushing live values and scope
samples at 1–10 kHz per channel across many channels, while the GUI stays
responsive. Tests drive `drive_app`/`drive_scope` synchronously (no sleeps):
the *rate* parameter enters assertions as samples-per-frame budget, never as
wall-clock sleeps.

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| PERF-040 | 10 kS/s single scope | MiniScope | 10 000 `push_sample` then 1 frame | all samples ingested; frame draws once; < 500 ms | P0 |
| PERF-041 | 1 kS/s × 64 channels | 64 scopes | 64×1000 pushes + 10 frames | every scope windowed correctly; frame time < 250 ms | P0 |
| PERF-042 | 1 kS/s × 200 live_value keys | `drive_app` 200 keys × 1000 frames-equiv | batches + frames | no unbounded queue growth; last value wins | P0 |
| PERF-043 | Burst 100k samples | scope | 100 000 pushes | memory bounded (ring buffer); no OOM | P1 |
| PERF-044 | Mixed rates 100 Hz + 10 kHz | 2 channels | interleaved pushes | both windows correct | P1 |
| PERF-045 | Live update during heavy model | FX-13 + 64 channels | frames | interactivity preserved; resolver not restarted per sample | P0 |
| PERF-046 | Value formatting throughput | 10 000 formats/sec equiv | `LiveValue::format` loop | ≥10⁵ formats/s debug build | P1 |
| PERF-047 | Matrix values at rate | 32×32 matrix entries ×1000 | format+render | completes; bounded text (LV-011) | P1 |
| PERF-048 | Dashboard events at 1 kHz | button widget | 1000 events | queue drains FIFO; none lost | P1 |
| PERF-049 | Live tooltip query at scale | FX-13 + hover | frame | tooltip build < 50 ms | P1 |
| PERF-050 | Streaming memory stability | 1M samples total | RSS before/after | Δ < 200 MB (leak guard) | P1 |

### 14.5 Propagation Under Churn (editing while resolving)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| PERF-060 | Edit invalidates once | FX-10 + resolver | single edit | exactly one signature bump; no redundant recompute | P1 |
| PERF-061 | Rapid edits debounced | 50 edits | resolver status | final resolve consistent; intermediate states allowed | P1 |
| PERF-062 | Progress monotonic under load | FX-13 | poll progress | never regresses; reaches 100% | P1 |

### 14.6 Serialization Performance

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| PERF-070 | Bincode save 5000-block model | generated | `save_to_binary` | < 2 s | P1 |
| PERF-071 | Bincode load < XML parse | same model | ratio | bin ≤ 0.5× xml parse (documented benefit) | P1 |
| PERF-072 | SLX write 1000-line model | FX-13 | `write_to` | < 5 s; re-parses equal | P0 |

### 14.7 Perf Infrastructure Rules

1. Warm-up once, measure median of ≥3 runs (per case) when timing is asserted.
2. Debug-build thresholds carry ≥4× headroom vs observed CI times; release
   numbers recorded in comments for reference.
3. `#[ignore]` + `PERF=1` gate for >10 s cases (nightly job runs them).
4. No network, no GPU dependency — pure CPU/egui tessellation timing.
5. Leak guards use `std::alloc` counters or `proc` RSS sampling helper.

---

## 15. Full User-Journey E2E Scenarios (E2E)

Suite: `tests/e2e_journeys.rs` · features: `dashboard`.
Cases prefixed **E2E-**. Each is a scripted multi-phase workflow crossing
subsystem boundaries — the closest thing to "a user sits down with the tool".
Every journey ends with a **consistency gate**: re-parse the final artefact and
re-resolve; the world must be internally consistent.

### 15.1 View → Edit → Save → Reopen Journeys

| ID | Journey | Steps | Pri |
|---|---|---|---|
| E2E-001 | Fix a mislabelled signal | open FX-01 → select line → rename → save → reopen → label present, resolver unaffected | P0 |
| E2E-002 | Rewire a sink | open → delete line → connection-drag new line to different sink → save → reopen → new topology resolves | P0 |
| E2E-003 | Tidy a schematic | open → marquee-select → group-drag aligned → resize a block → save → reopen → geometry persisted, `same_line` topology | P0 |
| E2E-004 | Extract subsystem | open → select 3-block chain → create subsystem → descend into it → verify inner blocks → ascend → save → reopen → hierarchy intact, boundary ports resolve (ED-101) | P0 |
| E2E-005 | Duplicate a subsystem | open → copy SubSys → paste → unique sids → connect → save → reopen → both resolve independently | P1 |

### 15.2 Propagation-Verification Journeys

| ID | Journey | Steps | Pri |
|---|---|---|---|
| E2E-010 | Trace a signal across the model | open Propergation_Test → enable live tooltips → hover each line in chain → tooltip targets = golden oracle entries | P0 |
| E2E-011 | Refactor preserving connectivity | open bus model → extract selector+creator into subsystem → re-resolve → element origins unchanged (CT-041 oracle reused) | P1 |
| E2E-012 | Swap input channels | demux out1↔out2 rewire via two connection drags + 2 deletes → save → reopen → channels swapped in resolver | P1 |
| E2E-013 | Retag goto/from pair | rename goto tag → rename from tag identically → re-resolve → link intact; mismatch case → broken (CT-066) | P1 |

### 15.3 Live/Dashboard Journeys

| ID | Journey | Steps | Pri |
|---|---|---|---|
| E2E-014 | Host-sim loop | open FX-05 → live mode → host pushes sine → gauge/lamp track → user presses PushButton → host sees PulseHigh/Low → host flips a signal → display updates next frame | P0 |
| E2E-015 | Scope session | open model w/ scopes → live mode → push 10 kS stream → embedded scope draws → popout → continue stream → close popout → embedded still current | P0 |
| E2E-016 | Manual switch flip | open `find_manual_switches`-style model → push value 0→1 → switch glyph flips (`manual_switch_setting_from_live_value`) → propagated tooltip shows new path | P1 |
| E2E-017 | Dashboard at rate | FX-05 + 64-ch feed @1kS/s → 30 s simulated → frames stay within budget (PERF-041) while events drained | P1 |

### 15.4 Library and Multi-File Journeys

| ID | Journey | Steps | Pri |
|---|---|---|---|
| E2E-020 | Edit model using library blocks | model + lib.slx → open → resolve lib subsystem → drag inner arrangement → save → reopen → lib link + overrides consistent | P1 |
| E2E-021 | Missing library graceful UX | model w/ missing lib → open → warning state → still renders labelled blocks → resolver completes | P1 |
| E2E-022 | Analyze→view→edit cycle | `analyze_libraries` JSON → open viewer → edit → re-analyze → block usage counts updated | P2 |

### 15.5 Journey Consistency Gate (shared tail)

After each journey's final save+reopen:

```rust
let sys = parse(final_slx)?;
debug_validate(&sys);                       // structural invariants
let sig = model_topology_signature(&sys);
let res = ConnectionTargetResolver::new(&sys);
assert_targets_eq(&res, &journey.oracle);   // connectivity as intended
assert!(sys.lines.iter().all(assert_line_orthogonal));
```

---

## 16. Property-Based and Fuzz Testing (FZ)

Suite: `tests/e2e_fuzz.rs` · features: none required (add `egui` for GUI props).
Cases prefixed **FZ-**. Use `proptest` (new dev-dependency, pinned) for
structured generation plus seeded RNG for scale cases.

### 16.1 Round-Trip Properties

| ID | Property | Generator | Assertion | Pri |
|---|---|---|---|---|
| FZ-001 | parse∘generate = id | `gen_random_system` (seeded, n≤64) | `parse(generate_xml(sys)) == sys` modulo normalization | P0 |
| FZ-002 | bincode round-trip id | random System | `load(save(sys)) == sys` | P0 |
| FZ-003 | serde JSON round-trip id | random System | `from(to(sys)) == sys` | P0 |
| FZ-004 | Archive round-trip id | random slx entries | `from(write(arch)) == arch` incl. raw bytes | P0 |
| FZ-005 | `parse_position`∘`format_position` = id | random i32 quads | identity | P0 |
| FZ-006 | orthogonalize idempotent | random polylines | `orth(orth(p))==orth(p)`; all segments axis-aligned | P0 |
| FZ-007 | `same_line` reflexive+symmetric | random lines | reflexive; symmetric over pairs | P1 |
| FZ-008 | topology signature: equal sys→equal sig | random twins | equal u64; mutated sys → different sig w.h.p. | P0 |

### 16.2 Resolver Properties

| ID | Property | Generator | Assertion | Pri |
|---|---|---|---|---|
| FZ-010 | targets ⊆ model ports | `gen_random_system` | every resolved target path exists in model | P0 |
| FZ-011 | determinism | same system ×5 runs | identical sets | P0 |
| FZ-012 | termination bound | random graphs incl. cycles | resolve < 30 s; visited ≤ total | P0 |
| FZ-013 | port bounds | random | `1 ≤ port ≤ port_count` when declared | P0 |
| FZ-014 | fan-in/out consistency | random | consumer's upstream ⊇ driver's downstream counterpart | P1 |
| FZ-015 | subsystem-wrap invariance | random sys S | wrap S in subsystem with pass-through ports → reachable-set preserved up to path prefix | P1 |
| FZ-016 | goto/from equivalence | random tags | from-line targets == direct-wire baseline targets | P1 |

### 16.3 Geometry Properties

| ID | Property | Generator | Assertion | Pri |
|---|---|---|---|---|
| FZ-020 | move_block preserves connectivity | random model+move | endpoints unchanged; lines orthogonal where required | P0 |
| FZ-021 | corner insert/remove inverse | random line | `remove(insert(p,i,o),i)` ≈ p | P1 |
| FZ-022 | `to_screen`∘`from_screen` = id | random transforms | |p'−p|<0.5 | P0 |
| FZ-023 | `compute_resized_rect` preserves opposite corner | random rect+handle+delta | anchor corner fixed | P0 |
| FZ-024 | merge_adjacent_corners never grows set | random pts | len(out) ≤ len(in) | P1 |

### 16.4 Editing Properties

| ID | Property | Generator | Assertion | Pri |
|---|---|---|---|---|
| FZ-030 | every command undoable | random op sequence | undo chain returns to start | P0 |
| FZ-031 | undo∘command = id at model level | random op | `s == undo(apply(s))` | P0 |
| FZ-032 | paste yields unique sids | random selection | no duplicate sids after paste | P0 |
| FZ-033 | delete cleans references | random deletion | no line references missing sid | P0 |
| FZ-034 | subsystem extract/reverse round-trip | random connected subset | extract → undo → model equal | P1 |

### 16.5 Input Fuzzing (robustness)

| ID | Fuzz target | Method | Assertion | Pri |
|---|---|---|---|---|
| FZ-040 | XML parser | byte-level mutations of valid system xml (10⁴ iters seeded) | no panic; Err or Ok — never UB | P0 |
| FZ-041 | zip layer | truncated/bit-flipped archives | graceful Err; no panic/OOB | P0 |
| FZ-042 | bincode loader | random bytes | Err not panic | P0 |
| FZ-043 | mask evaluator | random token strings | never panics | P1 |
| FZ-044 | entity decoder | random `&…;` strings | no panic; bounded output | P1 |
| FZ-045 | GUI input storm | random event sequences on EditorHarness (10³ seeds) | no panic; `drag_mode` terminal state sane | P1 |
| FZ-046 | LiveValue formatting | random dims/data | no panic; bounded len | P1 |

### 16.6 Fuzz Harness Notes

- Seed logged on failure: `PROPTEST_CASES` + `--nocapture` prints the failing
  seed; replay documented per test docstring.
- Mutation corpus seeded from `assets/e2e/fuzz_corpus/` (a few minimal valid
  XMLs); corpus grows only via manual curation (no CI-fuzz infra needed).
- Long-running fuzz modes are `#[ignore]` + `FUZZ=1` for nightly.

---

## 17. Robustness and Error-Handling Tests (ERR)

Suite: `tests/e2e_errors.rs` · features: none.
Cases prefixed **ERR-**. Goal: every failure is a *diagnosable* `Err`/UI
state — never panic, silent corruption, or hang.

### 17.1 File/Archive Errors

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ERR-001 | Nonexistent file | missing.slx | open | `Err` with path context | P0 |
| ERR-002 | Not a zip | text file renamed .slx | open | `Err` "open zip archive" | P0 |
| ERR-003 | Truncated zip | cut at 50% | open | `Err`, no panic | P0 |
| ERR-004 | Corrupt entry CRC | bit-flipped member | open/read | `Err` on read of that entry; others still accessible where possible | P1 |
| ERR-005 | Malformed XML | unclosed tag | parse | `Err` naming the file | P0 |
| ERR-006 | Wrong root element | `<NotSystem>` | parse | `Err` "No `<System>` root" | P0 |
| ERR-007 | Huge declared dims | `PortCounts` = 10⁹ | parse | parsed but bounded allocation (no OOM attempt) | P1 |
| ERR-008 | Deeply nested XML | 10k-deep elements | parse | parser limit or graceful Err (roxmltree is non-recursive — assert no stack overflow) | P1 |

### 17.2 Model-Level Errors

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ERR-010 | Line refs missing block | dst sid absent | parse+resolve | line parsed; resolver yields empty/err per impl, no panic | P0 |
| ERR-011 | Out-of-range port index | dst port 99 on 1-port block | resolve | handled (empty/err), no panic | P0 |
| ERR-012 | Circular subsystem ref | A refs B refs A | parse | cycle detected or bounded; documented | P1 |
| ERR-013 | Missing system file | `System Ref` to absent xml | parse | `Err` naming ref | P0 |
| ERR-014 | Bad `system_ref` inside model | dangling ref | parse/resolve | diagnostic, no panic | P1 |
| ERR-015 | Duplicate goto tags | 2 gotos same tag | resolve | deterministic per CT-065 | P1 |

### 17.3 UI-Level Errors

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ERR-020 | Open malformed model in viewer | ERR-005 file | viewer open path | error dialog/state; app stays alive | P0 |
| ERR-021 | Editor save to unwritable path | perms | save | error surfaced; dirty flag retained | P1 |
| ERR-022 | Font/syntax missing | syntect w/o MATLAB | render | fallback, no panic (VW-063) | P2 |
| ERR-023 | Zero-size canvas | 0×0 viewport | frame | no division-by-zero; empty output | P1 |
| ERR-024 | NaN positions | block pos NaN via crafted xml | render | skipped or clamped; no panic | P1 |

### 17.4 CLI Errors

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| ERR-030 | CLI bad args | `rustylink --nope` | run | clap usage error, exit ≠0 | P0 |
| ERR-031 | CLI missing file | `rustylink x.slx` | run | stderr msg, exit ≠0 | P0 |
| ERR-032 | CLI happy path JSON | `rustylink model.slx` | run | valid JSON on stdout, exit 0 | P0 |
| ERR-033 | CLI on raw xml path | `rustylink system_root.xml` | run | JSON out, exit 0 | P1 |

---

## 18. CI Integration, Coverage and Benchmarks

### 18.1 Cargo Test Targets to Add

```toml
[[test]]
name = "e2e_parse"
path = "tests/e2e_parse.rs"
required-features = ["egui"]

[[test]]
name = "e2e_serialize"
path = "tests/e2e_serialize.rs"
# no required-features

[[test]]
name = "e2e_propagation"
path = "tests/e2e_propagation.rs"
required-features = ["egui"]

[[test]]
name = "e2e_routing"
path = "tests/e2e_routing.rs"
required-features = ["egui"]

[[test]]
name = "e2e_editor_dnd"
path = "tests/e2e_editor_dnd.rs"
required-features = ["egui"]

[[test]]
name = "e2e_editor_ops"
path = "tests/e2e_editor_ops.rs"
required-features = ["egui"]

[[test]]
name = "e2e_viewer"
path = "tests/e2e_viewer.rs"
required-features = ["egui"]

[[test]]
name = "e2e_live"
path = "tests/e2e_live.rs"
required-features = ["egui"]

[[test]]
name = "e2e_dashboard"
path = "tests/e2e_dashboard.rs"
required-features = ["dashboard"]

[[test]]
name = "e2e_stress"
path = "tests/e2e_stress.rs"
required-features = ["dashboard"]

[[test]]
name = "e2e_journeys"
path = "tests/e2e_journeys.rs"
required-features = ["dashboard"]

[[test]]
name = "e2e_fuzz"
path = "tests/e2e_fuzz.rs"
required-features = ["proptest-impl"]   # or plain [] if proptest is dev-dep only

[[test]]
name = "e2e_errors"
path = "tests/e2e_errors.rs"
```

The `harness` module is shared via `tests/e2e/harness/` included with
`#[path = "e2e/harness/mod.rs"] mod harness;` in each test binary (standard
Cargo pattern — keeps the harness out of the published library).

### 18.2 Workflow Changes (`.github/workflows/ci.yml`)

Add to the existing `build` job (after `cargo test --all-features`):

```yaml
      - name: Run E2E suites (feature-gated)
        run: |
          cargo test --all-features --test 'e2e_*'
```

New jobs:

```yaml
  e2e-heavy:
    name: E2E stress & fuzz (nightly)
    if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - name: Install Rust
        run: rustup toolchain install stable --profile minimal
      - name: Stress + fuzz
        env: { PERF: "1", FUZZ: "1" }
        run: cargo test --all-features --test e2e_stress --test e2e_fuzz -- --ignored --test-threads=4
```

Coverage job stays `cargo tarpaulin --all-features`; add per-suite XML outputs
(`--out Xml --output-dir coverage/`) and upload each as an artifact for
per-area inspection. Optionally add a `paths`/`packages` tarpaulin flag set for
`connection_targets`, `editor`, `generator` to emit focused reports.

### 18.3 Feature-Matrix CI

The E2E suite must not break the feature matrix. Extend CI (or document manual
runs) with:

```yaml
  feature-matrix:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        features: ["", "egui", "egui,highlight", "dashboard", "dashboard,mask", "--all-features"]
    steps:
      - uses: actions/checkout@v6
      - run: cargo check ${{ matrix.features && format('--features {0}', matrix.features) || '' }}
      - run: cargo test ${{ matrix.features && format('--features {0}', matrix.features) || '' }}
```

Rules:
- `e2e_serialize`, `e2e_fuzz`, `e2e_errors` must compile & pass with **no**
  features.
- `e2e_dashboard`, `e2e_journeys` only exist under `dashboard`.
- `mask`-dependent assertions are `#[cfg(feature = "mask")]` inside shared
  helpers.

### 18.4 Coverage Targets

| Area | Current approx. | Target | Enforcement |
|---|---|---|---|
| `src/connection_targets/` | good (4k-line test file) | ≥90% lines | tarpaulin report review |
| `src/editor/` | low–medium | ≥85% | report review |
| `src/egui_app/` | low | ≥70% | report review (GUI code is hard) |
| `src/generator/` | medium | ≥90% | report review |
| `src/parser/` | medium | ≥85% | report review |
| crate total | — | ≥80% | report review |

Coverage is *measured*, not gated (tarpaulin thresholds in CI tend to flake);
the review gate is a required artifact in PRs touching `src/`.

### 18.5 Benchmarks

- Reuse `examples/binary_benchmark.rs` pattern; add `examples/e2e_bench.rs`
  producing a small JSON of phase timings (parse, resolve, frame, save) for
  `FX-13` on each nightly run; committed artifacts enable regression diffing
  across nightlies.
- Keep benchmarks out of the default `cargo test` path (perf cases are
  `#[ignore]`/`PERF=1`).

### 18.6 Test Time Budget

| Suite | Budget (debug, CI runner) |
|---|---|
| e2e_parse | ≤ 60 s |
| e2e_serialize | ≤ 60 s |
| e2e_propagation | ≤ 90 s |
| e2e_routing | ≤ 60 s |
| e2e_editor_dnd | ≤ 120 s |
| e2e_editor_ops | ≤ 60 s |
| e2e_viewer | ≤ 90 s |
| e2e_live | ≤ 60 s |
| e2e_dashboard | ≤ 90 s |
| e2e_journeys | ≤ 120 s |
| e2e_fuzz (CI slice) | ≤ 60 s |
| e2e_errors | ≤ 30 s |
| e2e_stress (CI slice, non-ignored) | ≤ 120 s |
| **total added** | **≤ ~15 min** — acceptable next to existing ~10–15 min pipeline; shard with `cargo nextest` later if needed |

### 18.7 Flakiness Policy

- Any GUI test failing intermittently twice in a week gets `#[ignore]` +
  tracking issue + rework — flaky E2E is worse than no E2E.
- No `thread::sleep` anywhere in tests (except a bounded `timeout` helper that
  polls a condition at 1 ms intervals with a 5 s cap).
- `--test-threads` must not affect results: no shared mutable statics outside
  per-test `tempdir`s (existing `tempfile` dev-dep covers this).

---

## 19. Implementation Milestones

Ordered by risk-reduction value: infrastructure first, then the user's
priority areas (DnD, propagation, routing, high-rate), then breadth.

### M1 — Harness skeleton (foundation)

- [ ] `tests/e2e/harness/` module: `model_builders`, `slx_factory`, `input`,
      `live_feed`, `mod` with shared assertions.
- [ ] `EditorHarness`/`ViewerHarness` on `egui::Context` + `RawInput` frames
      (local driver; evaluate `egui_kittest` compatibility with egui 0.37 and
      swap in if it works).
- [ ] `debug_validate(&System)` invariant checker.
- [ ] `Cargo.toml` `[[test]]` entries + CI wiring.
- [ ] Proof-of-life tests: `harness_smoke` (build FX-01, one frame, one drag).

### M2 — Signal propagation E2E (CT + oracle)

- [ ] `ConnectivityOracle` in `gen_models`; FX-02..FX-08 fixtures.
- [ ] §8.1–8.6 cases (flat, hierarchy, buses, goto/from, mux/demux, control).
- [ ] Golden generator + `Propergation_Test.slx` oracle (CT-110).
- [ ] §8.8 lifecycle/cache cases.

### M3 — Routing E2E (RT)

- [ ] §9.1–9.5 geometry/route/snap cases.
- [ ] §9.6 coloring/style; invariants wired as shared assertions.

### M4 — Editor DnD E2E (ED) — user's top priority

- [ ] Gesture synthesizers verified against `Sense::click_and_drag` timing
      (§10.1.1) with 3 instrumented probe tests.
- [ ] §10.2 block drag (incl. line-follow), §10.3 connection drag,
      §10.4 marquee, §10.5 resize, §10.6 pan, §10.7 viewer move-mode.
- [ ] §10.8–10.13 commands, clipboard, subsystem creation, browser/panels.
- [ ] §10.12 undo/redo integrity, §10.15 save/reopen.

### M5 — Viewer, live values, dashboard (VW/LV/DB)

- [ ] §11 navigation/transform/paint/dialog cases.
- [ ] §12 value model + live mode + scopes.
- [ ] §13 widget render + `DashboardControlEvent` flows.

### M6 — Stress & high data-rate (PERF)

- [ ] `gen_models` scale generators + oracle.
- [ ] §14.1–14.3 scale cases; §14.4 streaming-rate cases (priority).
- [ ] §14.5 churn; §14.6 serialization perf.
- [ ] Nightly `e2e-heavy` CI job.

### M7 — Serialization, parsing, errors, fuzz, journeys

- [ ] §6/§7 breadth cases on top of existing unit tests.
- [ ] §15 journeys wired on matured harness.
- [ ] §16 proptest properties; §17 robustness.
- [ ] Coverage report review; fill residual gaps.

### M8 — Hardening & docs

- [ ] `docs/E2E_TESTING.md` how-to-run (points back to this plan).
- [ ] Flake audit; snapshot policy decision executed.
- [ ] Bench artifact pipeline live.

### Exit Criteria per Milestone

- New suites green locally + CI; `cargo clippy --all-features -D warnings`
  clean; `cargo fmt --check` clean; review sign-off; plan checkboxes updated.

---

## 20. Test Case Index

Every case ID is unique, greppable, and maps 1:1 to a `#[test] fn`. Naming
convention: `fn p_001_open_real_slx_end_to_end()` etc. **P0** cases must be
implemented first and can never be `#[ignore]`d.

### 20.1 Parsing (P) — `tests/e2e_parse.rs`

```
P-001  open real .slx end to end                         P0
P-002  entry compression preserved                       P1
P-003  non-UTF8 entry rejected                           P0
P-004  xml entry without <System>                        P1
P-005  ZipSource::read_to_string                          P0
P-006  read_to_string normalizes ./ and /                P1
P-007  missing entry error contains path                 P1
P-008  list_dir enumerates prefix                        P1
P-009  list_dir trailing-slash tolerant                  P2
P-010  empty zip handled                                 P1
P-011  ZIP64 large member                                P2
P-012  stored vs deflated mixed round-trip               P1
P-013  from_file IO error path                           P0
P-020  root system parses with blocks+lines              P0
P-021  nested System Ref resolution                      P0
P-022  referenced system_N.xml fetched via source        P0
P-023  position parsing [l, t, r, b]                     P0
P-024  negative/off-canvas positions                     P1
P-025  SID numeric & string forms                        P0
P-026  block properties preserved                        P0
P-027  Commented blocks                                  P1
P-028  NameLocation variants                             P1
P-029  multi-cell numeric arrays                         P1
P-030  newlines stripped from names                      P1
P-031  missing port indices auto-assigned                P0
P-032  annotation blocks                                 P1
P-033  mask blocks (feature mask)                        P1
P-034  ValueKind classification                          P1
P-035  ZOrder preserved                                  P2
P-036  empty system                                      P1
P-037  unknown BlockType tolerated                       P1
P-038  duplicate SID tolerated (Duplicat_Test)           P1
P-039  very long names/values                            P2
P-050  simple src→dst line                               P0
P-051  branched line                                     P0
P-052  nested branches depth 2                           P1
P-053  line Points polyline                              P0
P-054  branch-level Points                               P1
P-055  named signals / labels                            P1
P-056  testpoint lines                                   P1
P-057  unconnected dst dangling                          P0
P-058  line to subsystem port                            P0
P-059  control-port endpoints                            P0
P-070  chart block parses                                P1
P-071  state/transition labels                           P1
P-072  empty chart tolerated                             P2
P-073  chart events/inputs ports                         P1
P-080  graphicalInterface parses                         P0
P-081  binding target path resolution                    P0
P-082  blockdiagram.xml.rels parsed                      P1
P-083  missing GI file tolerated                         P1
P-084  dangling binding tolerated                        P1
P-090  built-in virtual libraries resolve                P0
P-091  user library via -L path                          P0
P-092  missing library reported                          P0
P-093  library subsystem supplies port names             P0
P-094  nested library-in-library                         P1
P-095  catalog completeness                              P1
P-096  virtual matrix library                            P1
P-100  count_subsystems accuracy                         P0
P-101  max_subsystem_depth                               P0
P-102  topology signature stable                         P0
P-103  signature changes on structural edit              P0
P-104  signature ignores geometry-only change            P1
P-105  signature changes on rename                       P1
```

### 20.2 Serialization (SER) — `tests/e2e_serialize.rs`

```
SER-001  system→json→system identity                     P0
SER-002  real model json stable                          P0
SER-003  json human-diffable key order                   P2
SER-004  unicode survives                                P1
SER-010  bincode save→load round-trip                    P0
SER-011  real model binary round-trip                    P1
SER-012  binary load faster than xml parse               P2
SER-013  corrupted binary rejected                       P0
SER-014  empty system binary                             P1
SER-020  write→read archive round-trip                   P0
SER-021  system xml regenerated faithfully               P0
SER-022  raw entries preserved byte-exact                P0
SER-023  entry order preserved                           P1
SER-024  compression flags preserved                     P1
SER-025  roundtrip_compare equivalence                   P0
SER-026  edit→save→reload keeps edit                     P0
SER-027  new line survives round-trip                    P0
SER-028  created subsystem survives                      P1
SER-029  GI rels written for new bindings                P1
```

### 20.3 Signal Propagation (CT) — `tests/e2e_propagation.rs`

```
CT-001  direct line src→dst                              P0
CT-002  fan-out 1→3 branches                             P0
CT-003  chain through n blocks                           P0
CT-004  multiport consumer port indices                  P0
CT-005  multiport producer port indices                  P0
CT-006  dangling line empty targets                      P0
CT-007  self-loop via unit delay terminates              P0
CT-008  diamond topology                                 P0
CT-009  unconnected block empty                          P1
CT-010  named vs unnamed identical                       P1
CT-011  same-line dedup                                  P1
CT-012  port-count awareness                             P1
CT-020  in-port passthrough                              P0
CT-021  out-port passthrough                             P0
CT-022  depth-4 chain end-to-end                         P0
CT-023  fan-in across boundary no cross-talk             P0
CT-024  fan-out across boundary                          P0
CT-025  sibling subsystems chain                         P0
CT-026  inner goto/from inside subsystem                 P1
CT-027  boundary port index mapping                      P0
CT-028  commented subsystem semantics                    P1
CT-029  masked subsystem propagates                      P1
CT-030  resolve_subsystem_by_path helpers                P1
CT-040  BusCreator passthrough set                       P0
CT-041  BusSelector element extraction                   P0
CT-042  selector multiple outputs                        P0
CT-043  nested buses                                     P1
CT-044  BusAssignment channel override                   P1
CT-045  bus through subsystem boundary                   P1
CT-046  wide bus 64 elems                                P0
CT-047  bus fan-out to 2 selectors                       P0
CT-048  bus upstream targets                             P1
CT-049  selector out-of-range                            P1
CT-060  same-level goto→from                             P0
CT-061  multiple Froms one Goto                          P0
CT-062  scoped visibility local                          P1
CT-063  cross-level goto global                          P1
CT-064  goto upstream origins                            P0
CT-065  duplicate tags deterministic                     P1
CT-066  tag rename re-resolution                         P1
CT-067  qualify_external_path                            P1
CT-070  mux collects n inputs                            P0
CT-071  demux per-port mapping                           P0
CT-072  partial demux                                    P1
CT-073  mux through subsystem                            P1
CT-074  mux/demux upstream targets                       P1
CT-080  enable port is control                           P0
CT-081  trigger port is control                          P0
CT-082  ifaction/function-call ports                     P1
CT-083  reinit subsystem port math                       P1
CT-084  control_incoming_targets                         P1
CT-085  control+data interleave                          P0
CT-090  propagate_line_targets bidirectional             P0
CT-091  propagate_line_metadata_upward                   P1
CT-092  origin through buses                             P1
CT-093  origin through goto/from                         P0
CT-094  direct_internal_block_targets                    P1
CT-095  full_block_path qualification                    P1
CT-100  resolver::new eager                              P0
CT-101  new_with_progress ticks                          P1
CT-102  deterministic across runs                        P0
CT-103  ComputedViewCache validity                       P0
CT-104  cache invalidates on topology change             P0
CT-105  ResolverStatus::Ready ensure_resolver            P1
CT-106  large-model resolve progress-based               P1
CT-107  resolver thread-safety                           P2
CT-110  Propergation_Test golden                         P0
CT-111  golden regeneration idempotent                   P1
CT-112  Simulink_Blocks coverage diagnostics             P1
```

### 20.4 Routing (RT) — `tests/e2e_routing.rs`

```
RT-001  horizontal stays                                 P0
RT-002  vertical stays                                   P0
RT-003  diagonal gets corner                             P0
RT-004  multi-diagonal polyline                          P0
RT-005  empty/single point                               P0
RT-006  with_dst_side elbow orientation                  P0
RT-007  dst_segment_horizontal                           P1
RT-008  push_orthogonal_segments batch                   P1
RT-009  collinear duplicate points merged                P1
RT-010  zero-length segment                              P1
RT-011  negative coordinates                             P1
RT-020  endpoint_pos block ports                         P0
RT-021  register_endpoint collects                       P1
RT-022  register_branch_endpoints recursive              P1
RT-023  compute_port_info counts                         P0
RT-024  mirrored block ports                             P0
RT-025  rotated block ports                              P1
RT-026  port_kind mapping                                P1
RT-027  event_input_offset                               P1
RT-030  move_line_point mid                              P0
RT-031  move_line_point last index                       P0
RT-032  move_branch_point                                P0
RT-033  move_line_layout shifts all                      P0
RT-034  move_branch_layouts                              P1
RT-035  get_branch_mut path nav                          P0
RT-036  collect_branch_handle_positions                  P1
RT-040  insert_corner mid-segment                        P0
RT-041  remove_corner                                    P0
RT-042  merge_adjacent_corners threshold                 P0
RT-043  enforce_orthogonal repairs diagonals             P0
RT-044  auto_adjust_on_block_move src                    P0
RT-045  auto_adjust_on_block_move dst                    P0
RT-046  auto_adjust_branches_on_block_move               P0
RT-047  corner ops on empty line                         P1
RT-050  auto_route straight horizontal                   P0
RT-051  auto_route dogleg                                P0
RT-052  auto_route backward connection                   P0
RT-053  auto_route same-height adjacent                  P1
RT-054  find_snap_port inside radius                     P0
RT-055  find_snap_port outside radius                    P0
RT-056  snap prefers nearest port                        P1
RT-057  snap kind filtering                              P0
RT-058  route around obstacle                            P2
RT-060  adjacency graph built                            P0
RT-061  adjacent lines differ hue                        P0
RT-062  color count == line count                        P0
RT-063  dark-bg colors stay visible                      P1
RT-064  circular_dist wraparound                         P0
RT-065  testpoint marker                                 P1
RT-066  line_stroke_width variants                       P1
RT-067  resolved_line_label fallback                     P0
RT-068  editor compute_line_colors parity                P1
```

### 20.5 Editor DnD + Ops (ED) — `tests/e2e_editor_dnd.rs`, `tests/e2e_editor_ops.rs`

```
ED-001  drag single block                                P0
ED-002  drag snaps to grid                               P0
ED-003  drag selected group                              P0
ED-004  drag unselected selects                          P0
ED-005  click w/o drag = select only                     P0
ED-006  sub-pixel drag no command                        P1
ED-007  drag attached lines follow                       P0
ED-008  drag block w/ branched line                      P0
ED-009  drag into occupied space                         P1
ED-010  drag off-canvas                                  P1
ED-011  rapid sequential drags                           P1
ED-012  drag during live resolver                        P2
ED-013  arrow-key nudge                                  P1
ED-014  shift+arrow coarse nudge                         P2
ED-015  drag subsystem inner untouched                   P1
ED-020  draw line out→in                                 P0
ED-021  draw line in→out reversed                        P0
ED-022  connection preview while dragging                P0
ED-023  drop on empty canvas                             P0
ED-024  drop invalid port kind                           P0
ED-025  snap to nearby port                              P0
ED-026  no snap beyond radius                            P0
ED-027  port at max inputs                               P1
ED-028  esc cancels connection drag                      P0
ED-029  release outside window                           P1
ED-030  connect to subsystem boundary port               P1
ED-031  connection to control port                       P1
ED-032  multi-out-port selection                         P0
ED-033  connection drag while zoomed                     P1
ED-034  connection creates auto_route points             P1
ED-040  marquee selects enclosed                         P0
ED-041  marquee partial-overlap rule                     P0
ED-042  marquee includes lines                           P1
ED-043  esc/empty marquee                                P1
ED-044  ctrl-click toggle                                P0
ED-045  shift-click adds                                 P1
ED-046  click empty clears                               P0
ED-047  SelectionRect::normalized                        P1
ED-048  marquee then group-drag                          P0
ED-050  SE handle grows                                  P0
ED-051  NW handle grows up-left                          P0
ED-052  edge handles E/N/W/S                             P0
ED-053  minimum size enforced                            P0
ED-054  resize undoable                                  P0
ED-055  compute_resized_rect math                        P0
ED-056  resize_handle_positions order                    P1
ED-057  resize→port pos recomputed                       P1
ED-058  resize snap                                      P1
ED-060  middle-drag pans                                 P0
ED-061  pan preserves zoom                               P1
ED-062  space+drag pans                                  P2
ED-070  ViewerDragState::Blocks                          P0
ED-071  LinePointDrag                                    P0
ED-072  BranchPointDrag                                  P0
ED-073  SignalLabelDrag                                  P1
ED-074  ViewerDragState::Resize                          P1
ED-075  viewer drag cancel mid-way                       P1
ED-080  copy→paste duplicates                            P0
ED-081  paste assigns fresh SIDs                         P0
ED-082  cut = copy+delete                                P1
ED-083  paste empty clipboard noop                       P1
ED-084  delete selection                                 P0
ED-085  delete cleans dangling lines                     P0
ED-086  clipboard has_content/clear                      P1
ED-090  comment-out block                                P0
ED-091  rotate 90                                        P1
ED-092  mirror                                           P1
ED-093  comment/rotate/mirror undoable                   P0
ED-100  group→subsystem                                  P0
ED-101  boundary lines preserved                         P0
ED-102  undo restores flat model                         P0
ED-103  internal lines stay internal                     P1
ED-104  empty selection subsystem noop                   P1
ED-110  rename_line sets label                           P0
ED-111  branch_line creates branch                       P0
ED-112  branch via add_line cmd                          P0
ED-113  delete_lines removes branch                      P1
ED-120  history cap respected                            P1
ED-121  undo all → pristine                              P0
ED-122  redo all → final                                 P0
ED-123  new edit clears redo                             P0
ED-124  clear() empties                                  P1
ED-125  interleaved ui gestures                          P0
ED-130  browser opens at cursor                          P1
ED-131  browser search→insert                            P0
ED-132  create_default_block catalog                     P1
ED-133  code editor for matlab fn                        P1
ED-134  is_modified flag                                 P1
ED-135  menus dispatch commands                          P1
ED-136  shortcuts map                                    P0
ED-137  panels edit block props                          P1
ED-140  parse_position forms                             P0
ED-141  format_position round-trip                       P0
ED-142  assign_sids uniqueness                           P0
ED-143  sid assignment undoable                          P1
ED-150  dirty flag lifecycle                             P0
ED-151  save→close→reopen keeps edits                    P0
ED-152  reject save when path invalid                    P1
```

### 20.6 Viewer (VW) — `tests/e2e_viewer.rs`

```
VW-001  open at root                                     P0
VW-002  dbl-click descends                               P0
VW-003  breadcrumb back-nav                              P0
VW-004  collect_subsystems_paths                         P1
VW-005  navigation listener fires                        P1
VW-006  invalid path guarded                             P1
VW-007  deep zoom navigation                             P1
VW-010  fit-to-view on open                              P0
VW-011  zoom_at anchors cursor                           P0
VW-012  scroll zoom sequence                             P1
VW-013  zoom bounds clamped                              P1
VW-014  font_scale monotone                              P0
VW-015  pan by drag                                      P0
VW-016  to/from_screen round-trip                        P0
VW-017  preview_block_rect                               P1
VW-018  zoom controls widget                             P1
VW-020  all blocks drawn                                 P0
VW-021  lines drawn as segments                          P0
VW-022  port chevrons drawn                              P1
VW-023  block name labels                                P1
VW-024  goto/from tag glyphs                             P1
VW-025  logic-gate icons                                 P1
VW-026  signal labels on lines                           P1
VW-027  label_place collisions                           P1
VW-028  commented blocks hatched                         P1
VW-029  selection highlight                              P1
VW-030  color theme                                      P1
VW-031  z-order respected                                P2
VW-032  inport/outport shadow blocks                     P1
VW-040  block hover tooltip                              P1
VW-041  line hover shows targets                         P0
VW-042  block hover shows origins                        P1
VW-043  tooltips off                                     P1
VW-044  SignalDialog opens                               P1
VW-045  BlockDialog opens                                P1
VW-046  ctx menu dispatch                                P1
VW-047  dialog closes                                    P1
VW-050  move-mode toggle                                 P0
VW-051  view-mode drag = pan                             P0
VW-052  move commits geometry only                       P0
VW-053  line-point drag move mode                        P0
VW-054  move-mode doesn't corrupt resolver               P1
VW-060  html entities decoded                            P0
VW-061  entity edge cases                                P1
VW-062  syntect highlight                                P1
VW-063  missing syntax fallback                          P2
VW-064  multiline names                                  P1
```

### 20.7 Live Values (LV) — `tests/e2e_live.rs`

```
LV-001  scalar f64 formats                               P0
LV-002  vector formats                                   P0
LV-003  matrix column-major                              P0
LV-004  scientific at bounds                             P0
LV-005  custom bounds respected                          P1
LV-006  always-scientific flag                           P1
LV-007  first_f64/f64_at accessors                       P0
LV-008  complex32/64 values                              P1
LV-009  len/is_empty per variant                         P0
LV-010  per-entry display override                       P1
LV-011  very long vector bounded                         P1
LV-012  NaN/Inf values                                   P0
LV-020  live_mode renders values                         P0
LV-021  dashboard blocks no live text                    P0
LV-022  mask display wins                                P1
LV-023  manual switch state from value                   P0
LV-024  line tooltips live value                         P1
LV-025  block tooltips                                   P1
LV-026  display defaults propagate                       P1
LV-027  live mode off hides values                       P0
LV-028  stale value handling                             P1
LV-030  push_sample buffers                              P0
LV-031  scope capacity/window                            P1
LV-032  embedded scope renders                           P0
LV-033  set_signal_name                                  P1
LV-034  popout opens                                     P1
LV-035  popout close clears                              P1
LV-036  draw_scope_glyph                                 P2
LV-037  multi-scope isolation                            P1
LV-038  scope survives navigation                        P1
```

### 20.8 Dashboard (DB) — `tests/e2e_dashboard.rs`

```
DB-001  static icon fallback                             P0
DB-002  each widget type renders live                    P0
DB-003  binding shows bound value                        P0
DB-004  unbound widget default                           P0
DB-005  dashboard_scalar_value_from_pointer              P0
DB-006  dashboard debug helpers                          P0
DB-010  pushbutton press/release events                  P0
DB-011  toggle switch click                              P0
DB-012  checkbox click                                   P0
DB-013  slider drag scalar event                         P0
DB-014  radio group select                               P0
DB-015  combo box select                                 P1
DB-016  editable field entry                             P0
DB-017  rocker switch                                    P1
DB-018  linear gauge read-only                           P1
DB-019  lamp read-only                                   P1
DB-020  radial gauge read-only                           P1
DB-021  event carries originating block                  P0
DB-022  queue drains once                                P0
DB-023  rapid widget mashing                             P1
DB-030  binding survives hierarchy                       P0
DB-031  binding through goto/from                        P1
DB-032  binding to bus element                           P1
DB-033  rebind after topology change                     P1
DB-034  DashboardTargetPath normalization                P0
```

### 20.9 Stress (PERF) — `tests/e2e_stress.rs`

```
PERF-001  parse 1000-signal model                        P0
PERF-002  parse 5000-signal model          (PERF=1)      P1
PERF-003  resolve 1000 signals                           P0
PERF-004  resolve 5000 signals             (PERF=1)      P1
PERF-005  resolution sub-quadratic                       P1
PERF-006  render frame 1000 lines                        P0
PERF-007  render 5000 blocks                             P1
PERF-008  line adjacency at scale                        P1
PERF-009  color assignment at scale                      P1
PERF-010  zoom/pan under load                            P1
PERF-020  depth-12 hierarchy resolve                     P0
PERF-021  depth-20 hierarchy               (PERF=1)      P1
PERF-022  wide subsystem 200 children                    P1
PERF-023  bus fabric 32×64                               P0
PERF-024  goto/from mesh 500                             P1
PERF-025  branch-tree depth-5 32 leaves                  P1
PERF-026  corner-dense lines                             P1
PERF-030  group-drag 500 blocks                          P0
PERF-031  marquee over 2000 blocks                       P1
PERF-032  delete 1000 blocks                             P1
PERF-033  undo stack 200 ops                             P1
PERF-034  copy-paste 300-block selection                 P1
PERF-035  assign_sids 5000 blocks                        P1
PERF-040  10kS/s single scope                            P0
PERF-041  1kS/s × 64 channels                            P0
PERF-042  1kS/s × 200 live keys                          P0
PERF-043  burst 100k samples                             P1
PERF-044  mixed rates                                    P1
PERF-045  live update during heavy model                 P0
PERF-046  value formatting throughput                    P1
PERF-047  matrix values at rate                          P1
PERF-048  dashboard events at 1kHz                       P1
PERF-049  live tooltip query at scale                    P1
PERF-050  streaming memory stability                     P1
PERF-060  edit invalidates once                          P1
PERF-061  rapid edits debounced                          P1
PERF-062  progress monotonic under load                  P1
PERF-070  bincode save 5000-block                        P1
PERF-071  bincode load < xml parse                       P1
PERF-072  slx write 1000-line                            P0
```

### 20.10 Journeys (E2E) — `tests/e2e_journeys.rs`

```
E2E-001  fix mislabelled signal                          P0
E2E-002  rewire a sink                                   P0
E2E-003  tidy a schematic                                P0
E2E-004  extract subsystem                               P0
E2E-005  duplicate subsystem                             P1
E2E-010  trace signal across model                       P0
E2E-011  refactor preserving connectivity                P1
E2E-012  swap input channels                             P1
E2E-013  retag goto/from pair                            P1
E2E-014  host-sim loop                                   P0
E2E-015  scope session                                   P0
E2E-016  manual switch flip                              P1
E2E-017  dashboard at rate                               P1
E2E-020  edit model using library blocks                 P1
E2E-021  missing library graceful ux                     P1
E2E-022  analyze→view→edit cycle                         P2
```

### 20.11 Fuzz/Property (FZ) — `tests/e2e_fuzz.rs`

```
FZ-001  parse∘generate = id                              P0
FZ-002  bincode round-trip id                            P0
FZ-003  serde json round-trip id                         P0
FZ-004  archive round-trip id                            P0
FZ-005  position round-trip                              P0
FZ-006  orthogonalize idempotent                         P0
FZ-007  same_line reflexive+symmetric                    P1
FZ-008  topology signature prop                          P0
FZ-010  targets ⊆ model ports                            P0
FZ-011  resolver determinism                             P0
FZ-012  termination bound                                P0
FZ-013  port bounds                                      P0
FZ-014  fan-in/out consistency                           P1
FZ-015  subsystem-wrap invariance                        P1
FZ-016  goto/from equivalence                            P1
FZ-020  move_block preserves connectivity                P0
FZ-021  corner insert/remove inverse                     P1
FZ-022  screen transform inverse                         P0
FZ-023  resize preserves opposite corner                 P0
FZ-024  merge never grows set                            P1
FZ-030  every command undoable                           P0
FZ-031  undo∘command = id                                P0
FZ-032  paste unique sids                                P0
FZ-033  delete cleans references                         P0
FZ-034  subsystem extract/undo round-trip                P1
FZ-040  xml parser mutations                             P0
FZ-041  zip layer bit-flips                              P0
FZ-042  bincode random bytes                             P0
FZ-043  mask evaluator strings                           P1
FZ-044  entity decoder strings                           P1
FZ-045  gui input storm                                  P1
FZ-046  livevalue formatting                             P1
```

### 20.12 Errors (ERR) — `tests/e2e_errors.rs`

```
ERR-001  nonexistent file                                P0
ERR-002  not a zip                                       P0
ERR-003  truncated zip                                   P0
ERR-004  corrupt entry crc                               P1
ERR-005  malformed xml                                   P0
ERR-006  wrong root element                              P0
ERR-007  huge declared dims bounded                      P1
ERR-008  deeply nested xml                               P1
ERR-010  line refs missing block                         P0
ERR-011  out-of-range port index                         P0
ERR-012  circular subsystem ref                          P1
ERR-013  missing system file                             P0
ERR-014  bad system_ref                                  P1
ERR-015  duplicate goto tags                             P1
ERR-020  viewer malformed model dialog                   P0
ERR-021  save to unwritable path                         P1
ERR-022  font/syntax missing fallback                    P2
ERR-023  zero-size canvas                                P1
ERR-024  NaN positions                                   P1
ERR-030  cli bad args                                    P0
ERR-031  cli missing file                                P0
ERR-032  cli happy path json                             P0
ERR-033  cli on raw xml                                  P1
```

### 20.13 Totals

| Suite | Cases | P0 | P1 | P2 |
|---|---|---|---|---|
| P (parse) | 55 | 21 | 30 | 4 |
| SER (serialize) | 19 | 9 | 8 | 2 |
| CT (propagation) | 74 | 37 | 35 | 2 |
| RT (routing) | 51 | 33 | 17 | 1 |
| ED (editor) | 84 | 50 | 31 | 3 |
| VW (viewer) | 48 | 16 | 30 | 2 |
| LV (live) | 30 | 12 | 17 | 1 |
| DB (dashboard) | 24 | 13 | 11 | 0 |
| PERF (stress) | 36 | 10 | 26 | 0 |
| E2E (journeys) | 16 | 7 | 8 | 1 |
| FZ (fuzz/props) | 31 | 18 | 13 | 0 |
| ERR (errors) | 23 | 12 | 9 | 2 |
| **Total** | **491** | **238** | **235** | **18** |

---

## Appendix A — Reference Harness Implementation

Non-normative sketches showing intended shape; adapt to real APIs during M1.

### A.1 `harness/input.rs` — gesture synthesis

```rust
use eframe::egui::{self, PointerButton, Pos2};

/// One event batch per frame. `events` may contain several InputEvents that
/// belong to the same logical instant (e.g. press+move).
fn frame(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        events,
        time: Some(0.0),                // logical clock; tests never sleep
        ..Default::default()
    }
}

pub fn press(pos: Pos2) -> egui::RawInput {
    frame(vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
    ])
}

pub fn move_to(pos: Pos2) -> egui::RawInput {
    frame(vec![egui::Event::PointerMoved(pos)])
}

pub fn release(pos: Pos2) -> egui::RawInput {
    frame(vec![egui::Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    }])
}

/// Linear interpolation between two points over `steps` move-frames.
/// Returned as a sequence of RawInputs: [move(p1), move(p2), …, release(pn)]
/// — the caller supplies `press(p0)` first.
pub fn drag_path(from: Pos2, to: Pos2, steps: usize) -> Vec<egui::RawInput> {
    let mut out = Vec::with_capacity(steps + 1);
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        out.push(move_to(from + (to - from) * t));
    }
    out.push(release(to));
    out
}

pub fn click(pos: Pos2, modifiers: egui::Modifiers) -> Vec<egui::RawInput> {
    vec![
        frame(vec![egui::Event::PointerMoved(pos)]),
        frame(vec![egui::Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers,
        }]),
        frame(vec![egui::Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers,
        }]),
    ]
}

pub fn ctrl_click(pos: Pos2) -> Vec<egui::RawInput> {
    click(pos, egui::Modifiers::CTRL)
}

pub fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::RawInput {
    frame(vec![egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }])
}

pub fn type_text(s: &str) -> Vec<egui::RawInput> {
    s.chars()
        .map(|c| frame(vec![egui::Event::Text(c.to_string())]))
        .collect()
}

pub fn scroll_at(pos: Pos2, delta: egui::Vec2) -> egui::RawInput {
    frame(vec![
        egui::Event::PointerMoved(pos),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta,
            modifiers: egui::Modifiers::NONE,
        },
    ])
}
```

### A.2 `harness/gui.rs` — editor driver

```rust
use eframe::egui::{self, Pos2, Rect};
use rustylink::editor::{EditorState, ui as editor_ui};
use rustylink::model::System;

/// Headless editor: real EditorState, real canvas paint, synthesized input.
pub struct EditorHarness {
    pub ctx: egui::Context,
    pub state: EditorState,
    root: System,
    last_output: Option<egui::FullOutput>,
}

impl EditorHarness {
    pub fn new(root: System) -> Self {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(1.0);
        // editor::EditorState::new(root.clone(), /* paths, opts… */)
        let state = EditorState::new(root.clone());
        Self { ctx, state, root, last_output: None }
    }

    /// Drive exactly one frame with the given input batch.
    pub fn frame(&mut self, input: egui::RawInput) -> egui::FullOutput {
        let out = self.ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                editor_ui::canvas::show(&mut self.state, ui);
            });
        });
        self.last_output = Some(out.clone());
        out
    }

    /// Run a full input sequence frame-by-frame.
    pub fn run(&mut self, inputs: impl IntoIterator<Item = egui::RawInput>) {
        for i in inputs { self.frame(i); }
    }

    /// press at `from`, interpolate to `to` in `steps` frames, release.
    pub fn drag(&mut self, from: Pos2, to: Pos2, steps: usize) {
        self.frame(crate::harness::input::press(from));
        self.run(crate::harness::input::drag_path(from, to, steps));
    }

    /// Screen-space rect of a block as last rendered.
    /// (harness tracks block bboxes from the paint output each frame)
    pub fn screen_pos_of_block(&self, sid: &str) -> Rect {
        // map model rect -> screen via the state's ViewTransform
        let b = self.state.block_by_sid(sid).expect("block exists");
        let m = b.rect().unwrap();
        self.state.view_transform().to_screen_rect(m)
    }

    /// Screen position of a port anchor (for connection drags).
    pub fn port_screen_pos(&self, sid: &str, side: PortSide, idx: u32) -> Pos2 {
        let r = self.screen_pos_of_block(sid);
        // reuse the same helper the canvas uses:
        rustylink::egui_app::geometry::port_anchor_pos(r, side, idx, None)
    }

    /// Painted shapes clipped to the canvas (for paint assertions).
    pub fn canvas_shapes(&self) -> &[egui::epaint::ClippedShape] {
        &self.last_output.as_ref().unwrap().shapes
    }

    /// Idle `n` frames (process pending repaints, resolver ticks, etc.).
    pub fn idle(&mut self, n: usize) {
        for _ in 0..n {
            self.frame(egui::RawInput::default());
        }
    }
}
```

`ViewerHarness` mirrors this for `SubsystemApp` — `app.ui(ctx)` in
`CentralPanel`, plus `fn drain_dashboard_events(&mut self) ->
Vec<DashboardControlEvent>` and `fn set_live(&mut self, key, entry)` helpers.

### A.3 `harness/model_builders.rs` — canonical builders

```rust
use indexmap::IndexMap;
use rustylink::model::*;
use std::collections::BTreeSet;

pub struct BlockBuilder {
    block: Block,
}
impl BlockBuilder {
    pub fn new(block_type: &str, name: &str, sid: &str) -> Self {
        Self {
            block: Block {
                block_type: block_type.into(),
                name: name.into(),
                is_matlab_function: false,
                sid: Some(sid.into()),
                tag_name: "Block".into(),
                position: None,
                zorder: None,
                commented: false,
                name_location: NameLocation::Bottom,
                value: None,
                value_kind: ValueKind::Unknown,
                value_rows: None,
                value_cols: None,
                properties: IndexMap::new(),
                ref_properties: BTreeSet::new(),
                system_ref: None,
                mask: None,
                ports: vec![],
                port_counts: None,
                subsystem: None,
                annotations: vec![],
                chart: None,
                // …fill per real struct definition…
            },
        }
    }
    pub fn pos(mut self, l: i32, t: i32, r: i32, b: i32) -> Self {
        self.block.position = Some((l, t, r, b));
        self
    }
    pub fn ports(mut self, ins: u32, outs: u32) -> Self {
        self.block.ports = (1..=ins)
            .map(|i| Port { port_type: "in".into(), index: i, ..Default::default() })
            .chain((1..=outs).map(|i| Port {
                port_type: "out".into(), index: i, ..Default::default() }))
            .collect();
        self
    }
    pub fn prop(mut self, k: &str, v: &str) -> Self {
        self.block.properties.insert(k.into(), v.into());
        self
    }
    pub fn subsystem(mut self, sys: System, sys_ref: &str) -> Self {
        self.block.system_ref = Some(sys_ref.into());
        self.block.subsystem = Some(Box::new(sys));
        self
    }
    pub fn build(self) -> Block { self.block }
}

/// Canonical "In1 → Gain → Out1" (parameterized chain length).
pub fn gain_chain(n: usize) -> System {
    let mut sys = System::default();
    sys.blocks.push(BlockBuilder::new("Inport", "In1", "1")
        .pos(0, 0, 30, 14).ports(0, 1).build());
    for i in 0..n {
        sys.blocks.push(BlockBuilder::new("Gain", &format!("G{i}"), &format!("{}", i + 2))
            .pos(60 + i as i32 * 80, -10, 110 + i as i32 * 80, 24)
            .ports(1, 1).prop("Gain", "2").build());
    }
    let out_sid = (n + 2).to_string();
    sys.blocks.push(BlockBuilder::new("Outport", "Out1", &out_sid)
        .pos(60 + n as i32 * 80, 0, 90 + n as i32 * 80, 14).ports(1, 0).build());
    // lines
    sys.lines.push(line("1", 1, "2", 1));
    for i in 0..n.saturating_sub(1) {
        sys.lines.push(line(&format!("{}", i + 2), 1, &format!("{}", i + 3), 1));
    }
    sys.lines.push(line(&format!("{}", n + 1), 1, &out_sid, 1));
    sys
}

fn line(src_sid: &str, src_port: u32, dst_sid: &str, dst_port: u32) -> Line {
    Line {
        name: None,
        src: Some(EndpointRef {
            sid: src_sid.into(), port_type: "out".into(), index: src_port,
        }),
        dst: Some(EndpointRef {
            sid: dst_sid.into(), port_type: "in".into(), index: dst_port,
        }),
        points: vec![],
        branches: vec![],
        ..Default::default()
    }
}
```

### A.4 `harness/slx_factory.rs`

```rust
use anyhow::Result;
use rustylink::generator::archive::{SlxArchive, SlxArchiveEntry, SlxContent};
use rustylink::model::System;
use std::io::Write;
use std::path::Path;

/// Write systems through the REAL writer (exercises generator e2e).
pub fn write_slx_via_generator(path: &Path, systems: &[(&str, System)]) -> Result<()> {
    let arch = SlxArchive {
        entries: systems.iter().map(|(p, s)| SlxArchiveEntry {
            path: p.to_string(),
            content: SlxContent::SystemXml(Box::new(s.clone())),
            compressed: true,
        }).collect(),
        relationships: Default::default(),
    };
    arch.write_to_file(path)
}

/// Low-level escape hatch for malformed inputs (ERR/FZ).
pub fn raw_zip(path: &Path, files: &[(&str, &[u8])]) -> Result<()> {
    let f = std::fs::File::create(path)?;
    let mut zw = zip::ZipWriter::new(f);
    let opts = zip::write::SimpleFileOptions::default();
    for (name, data) in files {
        zw.start_file(*name, opts)?;
        zw.write_all(data)?;
    }
    zw.finish()?;
    Ok(())
}
```

### A.5 `harness/gen_models.rs` — stress generators + oracle

```rust
use rand::{SeedableRng, rngs::SmallRng, Rng};
use rustylink::model::*;
use std::collections::BTreeMap;

/// Expected connectivity, computed structurally alongside generation so tests
/// don't reimplement the resolver to check it.
#[derive(Default)]
pub struct ConnectivityOracle {
    /// line_key -> sorted [(target_path, port)]
    pub line_targets: BTreeMap<String, Vec<(String, u32)>>,
}

/// n_signals parallel chains; each src fans out to `fan_out` gains.
pub fn gen_wide_net(n_signals: usize, fan_out: usize) -> (System, ConnectivityOracle) {
    let mut sys = System::default();
    let mut oracle = ConnectivityOracle::default();
    let mut sid = 1usize;
    for s in 0..n_signals {
        let src = sid.to_string(); sid += 1;
        sys.blocks.push(BlockBuilder::new("Constant", &format!("C{s}"), &src)
            .pos(0, s as i32 * 40, 40, s as i32 * 40 + 14).ports(0, 1).build());
        for f in 0..fan_out {
            let g = sid.to_string(); sid += 1;
            sys.blocks.push(BlockBuilder::new("Gain", &format!("G{s}_{f}"), &g)
                .pos(100 + f as i32 * 80, s as i32 * 40, 150 + f as i32 * 80, s as i32 * 40 + 14)
                .ports(1, 1).build());
            let lk = format!("L{s}_{f}");
            sys.lines.push(named_line(&lk, &src, 1, &g, 1));
            oracle.line_targets.insert(lk, vec![(format!("G{s}_{f}"), 1)]);
        }
    }
    (sys, oracle)
}

/// depth-nested subsystems, blocks_per_level leaf blocks each level.
pub fn gen_deep_hierarchy(depth: usize, blocks_per_level: usize) -> System {
    // build inside-out: leaf system first
    // …
    unimplemented!("M6")
}
```

### A.6 `harness/mod.rs` — shared assertions

```rust
use rustylink::connection_targets::ConnectionTarget;
use rustylink::model::{Line, System};

/// Order-insensitive, path-normalized target comparison.
pub fn assert_targets_eq(
    actual: &[ConnectionTarget],
    expected: &[(&str, u32)],
) {
    let mut a: Vec<(String, u32)> = actual.iter()
        .map(|t| (norm(&t.path), t.port_index)).collect();
    let mut e: Vec<(String, u32)> = expected.iter()
        .map(|(p, i)| (norm(p), *i)).collect();
    a.sort(); e.sort();
    assert_eq!(a, e, "resolved target mismatch");
}

fn norm(p: &str) -> String {
    p.replace('.', "/").trim_matches('/').to_string()
}

/// Every consecutive pair of routing points shares x or y.
pub fn assert_line_orthogonal(line: &Line) -> bool {
    line.points.windows(2).all(|w| w[0].x == w[1].x || w[0].y == w[1].y)
        && line.branches.iter().all(|b|
            b.points.windows(2).all(|w| w[0].x == w[1].x || w[0].y == w[1].y))
}

/// Structural sanity every generated/edited model must satisfy.
pub fn debug_validate(sys: &System) {
    use std::collections::HashSet;
    let mut sids = HashSet::new();
    for b in &sys.blocks {
        if let Some(s) = &b.sid {
            assert!(sids.insert(s.clone()), "duplicate sid {s} ({})", b.name);
        }
    }
    for l in &sys.lines {
        if let Some(d) = &l.dst {
            assert!(sids.contains(&d.sid), "line dst references missing sid {}", d.sid);
        }
    }
}
```

---

## Appendix B — Worked Example Tests

Non-normative reference implementations for the four flagship cases.

### B.1 ED-020 — connection drag creates a line

```rust
#[test]
fn ed_020_draw_line_out_to_in() {
    // In1(unconnected out) + Scope(unconnected in)
    let mut sys = System::default();
    sys.blocks.push(BlockBuilder::new("Constant", "C", "1")
        .pos(0, 0, 40, 14).ports(0, 1).build());
    sys.blocks.push(BlockBuilder::new("Scope", "S", "2")
        .pos(200, 0, 240, 14).ports(1, 0).build());
    let mut h = EditorHarness::new(sys);
    h.idle(2); // let layout settle

    let from = h.port_screen_pos("1", PortSide::Right, 1);
    let to   = h.port_screen_pos("2", PortSide::Left, 1);

    h.drag(from, to, 6);

    let sys = h.state.current_system().unwrap();
    assert_eq!(sys.lines.len(), 1);
    let l = &sys.lines[0];
    assert_eq!(l.src.as_ref().unwrap().sid, "1");
    assert_eq!(l.dst.as_ref().unwrap().sid, "2");
    assert!(assert_line_orthogonal(l));
    assert!(h.state.history.can_undo());

    h.state.undo();
    assert_eq!(h.state.current_system().unwrap().lines.len(), 0);
}
```

### B.2 CT-022 — depth-4 subsystem chain

```rust
#[test]
fn ct_022_depth4_chain_end_to_end() {
    let inner = gain_chain(1);                        // In→G→Out
    let lvl3 = subsystem_wrap(inner, "SS3");
    let lvl2 = subsystem_wrap(lvl3, "SS2");
    let lvl1 = subsystem_wrap(lvl2, "SS1");
    let root = drive_and_sink(lvl1);                  // root: Const→SS1→Sink

    let res = ConnectionTargetResolver::new(&root);
    let targets = res.line_targets_for_line_ref(&root.lines[0]);
    assert_targets_eq(&targets, &[
        ("SS1/SS2/SS3/G0", 1),                        // leaf gain, fully qualified
    ]);
}
```

### B.3 PERF-041 — 1 kS/s × 64 channels

```rust
#[test]
fn perf_041_64_channels_at_1ksps() {
    let mut app = ViewerHarness::new(dashboard_with_scopes(64));
    app.enable_live_mode();
    // simulate 1000 ticks @1kHz of 64 channels — synchronous, no sleep
    let t0 = Instant::now();
    for tick in 0..1000 {
        for ch in 0..64 {
            app.push_live(&format!("ch{ch}"), scalar(tick as f64 + ch as f64));
        }
        if tick % 33 == 0 { app.frame_default(); }      // ~30 fps repaint
    }
    let push_time = t0.elapsed();
    let t1 = Instant::now();
    for _ in 0..10 { app.frame_default(); }
    let frame_avg = t1.elapsed() / 10;

    for ch in 0..64 {
        assert_eq!(app.live_value(&format!("ch{ch}")).first_f64(),
                   Some(999.0 + ch as f64));            // last-wins
    }
    assert!(frame_avg < Duration::from_millis(250), "frame avg {frame_avg:?}");
    assert!(push_time < Duration::from_secs(30), "ingest {push_time:?}");
}
```

### B.4 E2E-014 — host-sim loop

```rust
#[test]
fn e2e_014_host_sim_loop() {
    let slx = fx_05_dashboard_panel();
    let mut app = ViewerHarness::open_slx(&slx);
    app.enable_live_mode();
    app.idle(3);

    // host: press the live PushButton widget on screen
    let btn = app.dashboard_widget_center("PushButton");
    app.click(btn);
    let evs = app.drain_dashboard_events();
    assert!(matches!(evs.as_slice(),
        [DashboardControlEvent { value: DashboardControlValue::PulseHigh, .. },
         DashboardControlEvent { value: DashboardControlValue::PulseLow, .. }]));

    // host applies the events to its "sim" and pushes results back
    app.push_live("sig/lamp", scalar(1.0));
    app.idle(2);
    // lamp now lit — assert via paint or state
    assert!(app.dashboard_widget_shows("Lamp", 1.0));
}
```

---

## Appendix C — Feature → Case Coverage Matrix

Quick lookup that every advertised capability has coverage. ✎ = primary suite.

| Capability | Cases |
|---|---|
| .slx open/list/read | P-001..P-013, ERR-001..004 |
| XML system parse | P-020..P-039, FZ-001, FZ-040 |
| Lines/branches/points | P-050..P-059, RT-030..036, ED-110..113 |
| Charts | P-070..073 |
| Dashboard bindings/GI | P-080..084, DB-030..034, SER-029 |
| Libraries | P-090..096, E2E-020..021 |
| Masks | P-033, LV-022, FZ-043 |
| JSON | SER-001..004, FZ-003 |
| bincode | SER-010..014, FZ-002, PERF-070..071 |
| SLX write/regen | SER-020..029, ED-151, FZ-004, PERF-072 |
| Topology signature/cache | P-102..105, CT-103..104, FZ-008 |
| **Signal propagation (resolver)** | **CT-001..CT-112, FZ-010..016, PERF-003..005/020..025/060..062** |
| — flat/fan-in/out | CT-001..012 |
| — subsystem boundaries | CT-020..030, FZ-015 |
| — buses | CT-040..049, PERF-023 |
| — goto/from | CT-060..067, FZ-016, PERF-024 |
| — mux/demux | CT-070..074 |
| — control ports | CT-080..085 |
| — upstream/origins | CT-090..095 |
| — lifecycle/progress/cache | CT-100..107 |
| **Routing geometry** | **RT-001..RT-068, FZ-006/020..024** |
| — orthogonalization | RT-001..011, FZ-006 |
| — endpoints/port info | RT-020..027 |
| — point/branch mutation | RT-030..036, ED-071..073 |
| — corner ops | RT-040..047, FZ-021/024 |
| — auto-route/snap | RT-050..058, ED-025..026/034 |
| — coloring/style | RT-060..068, PERF-008..009 |
| **Editor DnD** | **ED-001..ED-152, FZ-030..034/045, PERF-030..035** |
| — block drag | ED-001..015 |
| — connection drag | ED-020..034 |
| — marquee/selection | ED-040..048 |
| — resize | ED-050..058, FZ-023 |
| — pan | ED-060..062 |
| — viewer move-mode | ED-070..075, VW-050..054 |
| — clipboard/delete | ED-080..086 |
| — comment/rotate/mirror | ED-090..093 |
| — subsystem extract | ED-100..104 |
| — undo/redo | ED-120..125, FZ-030..031 |
| — browser/code/menus/panels | ED-130..137 |
| — positions/SIDs | ED-140..143 |
| **Viewer** | **VW-001..VW-064** |
| — navigation | VW-001..007 |
| — zoom/pan/transform | VW-010..018, FZ-022 |
| — paint presence | VW-020..032 |
| — tooltips/dialogs | VW-040..047 |
| — text pipeline | VW-060..064 |
| **Live values** | **LV-001..LV-038, PERF-040..050** |
| — value model/format | LV-001..012, FZ-046 |
| — live mode display | LV-020..028 |
| — scopes | LV-030..038, E2E-015 |
| **Dashboard widgets** | **DB-001..DB-034, E2E-014/017, PERF-048** |
| **High data-rate** | **PERF-040..050, E2E-017** |
| Journeys | E2E-001..E2E-022 |
| Fuzz/robustness | FZ-001..046, ERR-001..033 |
| CLI | ERR-030..033 |
| Examples (smoke) | §18.6 note: `cargo build --examples` + CLI smoke in ERR-032 |

---

## Appendix D — Style Guide, Risks and Open Questions

### D.1 Test-writing style guide

1. **One ID, one `#[test]`.** No multi-assertion grab-bags; failures must map
   to a case ID. Test fn names carry the ID: `fn ed_020_…`.
2. **Arrange/Act/Assert comments** inside each test.
3. **Builders over literals.** Hand-built `System` structs are for builder
   tests only.
4. **Assert the invariant, not the implementation** where possible (e.g.
   orthogonality over exact point lists) — but keep ≥1 golden-exact case per
   subsystem.
5. **No sleeps.** Use `h.idle(n)` frames or condition polling.
6. **No filesystem outside `tempdir`** except read-only fixtures.
7. **Deterministic seeds** — print them in failure messages.
8. **Keep P0 green at all costs**; a P0 flake is a P0 bug in the suite or the
   product.

### D.2 Known risks & mitigations

| Risk | Mitigation |
|---|---|
| egui version drift breaks harness | isolate behind `harness/gui.rs`; swap to `egui_kittest` when compatible |
| Font/hinting nondeterminism | bundled fonts + structural paint asserts; optional snapshots off CI |
| Resolver perf assertions flaky on shared CI | progress-count + ratio assertions, `PERF=1` nightly for wall-clock-sensitive |
| Deep recursion in resolver/render on synthetic depth | PERF-021 explicitly probes the limit; document if found |
| `proptest` adds dev-dep weight | gate behind `required-features` or accept dev-only dep |
| Golden drift masks real regressions | golden changes require PR description + `REGEN_GOLDEN` is audited in diff |
| Tests written before behaviour documented | cases marked "(per impl)" need a docs pass in M4/M5 — each becomes either a doc or a behaviour fix |

### D.3 Open questions for the maintainer

1. Should `find_snap_port`'s radius be configurable? (tests assume ~8 px)
2. Multi-input on a single in-port: allowed, replace, or reject? (ED-027)
3. Are commented-out blocks expected to participate in propagation? (CT-028)
4. Preferred: `egui_kittest` dev-dep vs local harness? (M1 decision)
5. `e2e_stress` absolute budgets: tune after first nightly run.
6. Is there a max intended hierarchy depth? (PERF-021)

### D.4 Glossary of internal names used in this plan

- `ConnectionTarget`, `ConnectionTargetOrigin`, `ConnectionTargetResolver`,
  `ConnectionTargetResolve` — `src/connection_targets/*`.
- `DragMode::{None,Blocks,Connection,SelectionRect,Resize,Pan}` —
  `src/editor/state.rs`.
- `ViewerDragState::{None,Blocks,Resize,LinePointDrag,BranchPointDrag,
  SignalLabelDrag}` — `src/egui_app/state.rs`.
- `EditorCommand`, `EditorHistory` — `src/editor/operations.rs`.
- `LiveValue`, `LiveValueList`, `LiveValueEntry`, `LiveValueDisplayOptions` —
  `src/live_values.rs`.
- `MiniScope`, `ScopePopout` — `src/egui_app/scope_widget.rs`.
- `DashboardControlEvent`, `DashboardControlValue` — `src/egui_app/state.rs`.
- `SlxArchive`, `SlxArchiveEntry`, `SlxContent` — `src/generator/archive.rs`.
- `SystemDoc` — `src/model.rs`.
- `ViewTransform` — `src/egui_app/ui/view_transform.rs`.
- `SubsystemApp`, `ComputedViewCache`, `ResolverStatus` —
  `src/egui_app/state.rs`.

---

*End of plan. File issues/PRs against case IDs as they are implemented; keep
the index (§20) and the totals table in sync.*

---

## Appendix E — Canonical Fixture XML Reference

The synthetic SLX factory writes XML in this shape. Understanding the mapping
XML ↔ `model::System` is prerequisite for writing oracle fixtures. (Check
`src/generator/system_xml.rs` for the exact emitted schema — the factory reuses
the real generator so fixtures can't drift from the writer.)

### E.1 Minimal system file

```xml
<?xml version="1.0" encoding="utf-8"?>
<System>
  <P Name="…">system-level property</P>

  <Block BlockType="Gain" Name="G1" SID="2">
    <P Name="Position">[60, -10, 110, 24]</P>
    <P Name="Gain">2</P>
    <P Name="ZOrder">1</P>
    <P Name="NameLocation">bottom</P>
    <P Name="Commented">off</P>
  </Block>

  <Line>
    <P Name="Name">sig_ab</P>
    <P Name="Src">"1#out:1"</P>
    <P Name="Dst">"2#in:1"</P>
    <P Name="Points">[40 7; 60 7]</P>
    <P Name="TestPoint">off</P>
    <Branch>
      <P Name="Dst">"9#in:2"</P>
      <P Name="Points">[50 7; 50 40]</P>
      <Branch> … nested … </Branch>
    </Branch>
  </Line>

  <Block BlockType="SubSystem" Name="SS1" SID="10">
    <P Name="Position">[200, -40, 300, 60]</P>
    <System Ref="system_11"/>   <!-- or inline <System>…</System> -->
  </Block>
</System>
```

### E.2 Endpoint reference syntax

`"<sid>#<port_type>:<port_index>"` — e.g. `"10#out:2"`. Port types seen in
the wild: `in`, `out`, `enable`, `trigger`, `ifaction`, `reset`, `event`.
Parser auto-assigns missing indices per port type (P-031).

### E.3 Subsystem references

Two forms exist and both must round-trip:

1. `<System Ref="system_11"/>` — child XML lives in a *separate* zip entry
   `simulink/systems/system_11.xml` (the common `.slx` layout).
2. Inline `<System>…children…</System>` — used in standalone xml files.

`parser/source.rs` resolves refs through the active `ContentSource`
(`ZipSource` for `.slx`, filesystem for bare xml). FX-02 exercises both forms
because they hit different resolution paths.

### E.4 Graphical interface & relationships (dashboard)

```
simulink/graphicalInterface.xml     → DashboardBinding[] declarations
simulink/blockdiagram.xml.rels      → rel-id → target map (P-082)
metadata/coreProperties.xml         → ignored by parser (preserved as raw)
simulink/systems/system_root.xml    → root system
simulink/systems/system_N.xml       → referenced subsystems
```

Dashboard bindings reference model elements via `DashboardTargetPath`
segments; `dashboard_binding_target_path` normalizes them (DB-034) and
`qualify_external_path` prefixes the model name (CT-067).

### E.5 Field reference for fixture authors

| XML | Model field | Notes |
|---|---|---|
| `Block@BlockType` | `block.block_type` | "Gain", "SubSystem", "BusCreator", … |
| `Block@Name` | `block.name` | newlines stripped (P-030) |
| `Block@SID` | `block.sid` | numeric or `":N"` forms (P-025) |
| `<P Name="Position">` | `block.position` | `[l, t, r, b]` ints |
| `<P Name="Commented">` | `block.commented` | "on"→true |
| `<P Name="NameLocation">` | `block.name_location` | enum |
| `<P Name="ZOrder">` | `block.zorder` | render order |
| `<P Name="Mask*">` | `block.mask` | feature `mask` |
| `<P Name="PortCounts">` | `block.port_counts` | declared port arity |
| `<Port>` children | `block.ports` | per-port type/index/labels |
| `<System Ref>` | `block.system_ref` + `subsystem` | see E.3 |
| `<Chart>` | `block.chart` | Stateflow subset |
| `<Line>/<P Name="Src|Dst|Points|Name|TestPoint">` | `line.*` | E.2 syntax |
| `<Branch>` | `line.branches` | recursive |
| `<Annotation>` | `system.annotations` | not a block (P-032) |
| any other `<P>` | `block.properties`/`system.properties` | IndexMap, order preserved |

### E.6 Fixture checklist (must satisfy `debug_validate`)

- [ ] SIDs unique across the *file-level* system (children have own scopes).
- [ ] Every `Dst` sid+port exists on a block in the same system.
- [ ] Port indices ≥ 1.
- [ ] `System Ref` targets exist in the archive.
- [ ] `Position` rects have `r > l` and `b > t` unless deliberately testing
      degenerate rects (ERR-024 class).
- [ ] Bus widths consistent: BusCreator `Inputs` count == feeding lines ==
      downstream selector index range where asserted.
- [ ] Goto/From `GotoTag` values match tag-for-tag.

---

## Appendix F — DnD Recipes and Edge-Case Semantics

Drag-and-drop is the highest-risk interaction surface (state machines +
coordinate transforms + async resolver). These recipes normalize how gestures
are decomposed so case implementations stay uniform.

### F.1 Block drag — the canonical recipe

```
ARRANGE  h = EditorHarness::new(model)
         r0 = h.screen_pos_of_block("5")          // before
ACT      h.drag(center(r0), center(r0) + vec2(43,17), steps=6)
ASSERT   b.position() == r0.position() + snap((40,20))   // grid rounding
         h.state.history.can_undo()
         lines_attached("5").all(assert_line_orthogonal)
         h.state.drag_mode == DragMode::None
```

### F.2 Connection drag — port-chevron hit areas

The canvas registers invisible `Sense::click_and_drag` rects over port
chevrons (`canvas.rs` allocates `hit_rect` then sets
`DragMode::Connection{src_sid, src_port_type, src_port_index, current_*}` on
`drag_started`). Tests therefore MUST start the press inside the chevron hit
rect — use `h.port_screen_pos(...)` not the block edge. While dragging, every
move updates `current_x/y` for the preview; drop resolves through
`find_snap_port`.

```
ARRANGE  from = h.port_screen_pos("2", Right, 1)
         to   = h.port_screen_pos("7", Left, 2)
ACT      h.drag(from, to, 8)
ASSERT   new line: src=("2","out",1), dst=("7","in",2)
         l.points == auto_route(expected_model_path)
```

### F.3 Drag lifecycle edge cases (state-machine coverage)

For EACH of `DragMode::{Blocks, Connection, SelectionRect, Resize, Pan}` and
each `ViewerDragState` variant, the following lifecycle cells exist — many are
already enumerated as cases; any unlisted cells get added in implementation:

| State | Armed by | Mid-drag mutations | Commit on | Abort paths |
|---|---|---|---|---|
| Blocks | press on block + move | accumulated dx/dy preview | release → move_blocks cmd | Esc, release-threshold, window blur |
| Connection | press on port + move | preview current_x/y | release on valid port → add_line | release on void/invalid, Esc |
| SelectionRect | press on bg + move | rect update | release → finish_rect selection | Esc, zero-area |
| Resize | press on handle + move | live rect preview | release → resize_block cmd | Esc |
| Pan | press on bg (alt/middle) + move | vt.pan update | release (no command) | Esc n/a |
| LinePointDrag (vw) | press on vertex + move | move_line_point live | release → geometry cmd | Esc |
| BranchPointDrag (vw) | press on branch vertex + move | move_branch_point live | release | Esc |
| SignalLabelDrag (vw) | press on label + move | label offset live | release | Esc |

Explicit tests worth adding beyond the indexed cases if gaps appear:
- drag started on port, Esc *during* preview → `current_x/y` state cleared.
- drag started on block, second button pressed mid-drag → no double-commit.
- drag that crosses subsystem boundary visually (drop on canvas area of a
  subsystem's port) → connects boundary port, not phantom inner block.
- press on block → Esc before any move → no selection toggle (press≠click).
- resize drag released at original position → no command (zero delta).
- drag during `SelectionRect` armed on top of a block → rect wins
  (background-vs-block hit priority).

### F.4 Hit-test priority

Canvas hit regions are layered; tests must codify the intended priority order
(top wins). Documented target order, verified by dedicated hit-priority cases
to be added under ED if not already covered:

1. resize handles (only when block selected)
2. port chevron hit areas
3. line vertex handles (move mode)
4. branch vertices
5. block body
6. line body / label
7. canvas background (SelectionRect / Pan)

Any ambiguity discovered becomes either an implementation fix or a documented
behaviour — never a test that just asserts whatever happened.

### F.5 Coordinate fidelity checklist for every drag test

- [ ] Uses `port_screen_pos`/`screen_pos_of_block`, not hardcoded pixels.
- [ ] Asserts in **model** coordinates (ints), converting through
      `vt.from_screen` for expectations.
- [ ] Re-runs at zoom ∈ {0.5, 1.0, 2.5} at least once per drag class
      (ED-033 pattern) to catch transform bugs.
- [ ] Asserts `drag_mode == None` after every gesture (no stuck state).

---

## Appendix G — Additional Coverage: Annotations, Fonts, Examples, Multi-System

Cases beyond the indexed set; fold into §6/§11 suites during implementation.

### G.1 Annotations and free-floating elements

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| P-110 | Annotation text blocks render | model w/ `<Annotation>` | parse+frame | annotation drawn as text, not a port/block | P1 |
| P-111 | Annotation doesn't break resolver | annotated model | resolve | unaffected vs unannotated twin | P1 |

### G.2 Fonts and text metrics

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| VW-070 | font loading path | `egui_app/fonts.rs` | frame | fonts registered; no missing-glyph tofu for ASCII | P2 |
| VW-071 | material icons font | `egui_material_icons` | render icon call | icon glyph drawn | P2 |
| VW-072 | long label ellipsis/wrap | 200-char block name | frame | bounded layout per impl | P2 |

### G.3 Examples as smoke tests

Each example is exercised in CI via `cargo build --examples` plus a
`--help`-level invocation where applicable. For examples that take a model,
run against `simulink_test_models/Simulink_Blocks.slx` asserting exit code 0:

| ID | Example | Invocation | Assert | Pri |
|---|---|---|---|---|
| SMK-001 | tree | `tree Simulink_Blocks.slx` | exit 0; non-empty stdout | P1 |
| SMK-002 | analyze_libraries | `analyze_libraries Simulink_Blocks.slx --json` | valid JSON stdout | P1 |
| SMK-003 | list_blocks | `list_blocks Simulink_Blocks.slx` | exit 0 | P2 |
| SMK-004 | find_manual_switches | `find_manual_switches Simulink_Blocks.slx` | exit 0 | P2 |
| SMK-005 | check_libraries | `check_libraries …` | exit 0 or documented missing-lib report | P2 |
| SMK-006 | binary_benchmark | on FX-01 slx | prints timings; bin < xml | P2 |
| SMK-007 | roundtrip_compare | `roundtrip_compare Simulink_Blocks.slx` | exit 0 | P1 |
| SMK-008 | analyze_model_blocks | on test model | exit 0 | P2 |
| SMK-009 | dump_block_icons | feature egui | exit 0; icon dump non-empty | P2 |

GUI examples (`egui_viewer`, `egui_editor`) are compile-checked only —
interactive verification is manual (they need a display server); the headless
harness covers their code paths via `SubsystemApp`/`EditorState` instead.

### G.4 Multi-system / model-reference coverage

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| MS-001 | Two root-level systems in one archive | slx w/ system_root + extra system | `SlxArchive::from_reader` | both `SystemXml` entries parse; `get_system(path)` returns each | P1 |
| MS-002 | `get_system` miss returns None | FX-01 | query bogus path | None | P1 |
| MS-003 | `list_dir_impl` entry enumeration | FX-01 | list | only direct children, no prefix leaks | P1 |

### G.5 Concurrent/parallel parse (`rayon` usage)

| ID | Title | Given | When | Then | Pri |
|---|---|---|---|---|---|
| CC-001 | Parallel subsystem resolution | multi-file slx | parse | results identical to serial parse | P1 |
| CC-002 | No shared-state corruption under rayon | FX-02 parsed 10× in parallel threads | compare | all outputs identical | P1 |

---

## Appendix H — Traceability: Source File → Test Coverage

For PR review: which tests to run when a file changes.

| Source path | Suites to run |
|---|---|
| `src/parser/*` | e2e_parse, e2e_errors, e2e_fuzz |
| `src/model.rs`, `src/block*.rs`, `src/block_types.rs` | e2e_parse, e2e_serialize, e2e_fuzz |
| `src/generator/*` | e2e_serialize, e2e_journeys |
| `src/connection_targets/*` | e2e_propagation, e2e_stress |
| `src/editor/*` | e2e_editor_*, e2e_routing, e2e_journeys |
| `src/egui_app/ui/signal_routing.rs`, `corner_ops.rs`, `line_*` | e2e_routing, e2e_editor_dnd |
| `src/egui_app/ui/view_transform.rs`, `zoom_controls.rs` | e2e_viewer, e2e_editor_dnd |
| `src/egui_app/state.rs`, `navigation.rs` | e2e_viewer, e2e_live, e2e_dashboard |
| `src/egui_app/render/*`, `text.rs`, `fonts.rs`, `ui/*` misc | e2e_viewer |
| `src/live_values.rs`, `scope_widget.rs`, `ui/live_values.rs` | e2e_live, e2e_stress |
| `src/egui_app/dashboard_widgets/*`, `ui/dashboard_debug.rs` | e2e_dashboard, e2e_stress |
| `src/simulink_libraries/*` | e2e_parse, e2e_viewer |
| `src/main.rs` | e2e_errors |
| `Cargo.toml`, `.github/workflows/ci.yml` | full suite |

---

## Appendix I — Propagation Semantics Primer

Background for writing CT oracles. Derived from `src/connection_targets/*`;
when docs and tests disagree, the test suite's job is to *surface* the
disagreement — the case becomes either a doc fix or a product fix (§D.2).

### I.1 What the resolver computes

For every `System` in the model tree the resolver produces, per line:

- **Line targets** — the set of downstream `(block_path, port_index)` consumer
  endpoints reachable from that line, crossing subsystem boundaries, bus
  expansions, mux/demux splits and goto/from hops.
- **Block targets** — per block port, downstream consumers
  (`block_targets_for_block`) plus **origins**
  (`ConnectionTargetOrigin` variants) — the upstream driver blocks that feed
  each input port, used for tooltips, highlighting and live-value routing.
- **Line metadata** — propagated names/labels so a signal keeps its name
  through subsystem boundaries (`propagate_line_metadata_upward`).

Resolution is per-model-topology: `model_topology_signature` fingerprints the
structure so `ComputedViewCache` can skip recompute when only geometry
changed (P-104, CT-104).

### I.2 Traversal rules implemented (what tests assert)

1. **Graph walk, not name matching.** Lines connect ports; ports connect
   blocks. The walk follows `Src`/`Dst` `EndpointRef`s and `Branch` trees —
   names only label results (CT-010).
2. **Subsystem boundary crossing.** A line to `SubSys.in:k` maps through
   `boundary_port_index` to the inner `Inport`/`inport-shadow` block, whose
   outgoing lines continue the walk inside the child system. Dual for
   `SubSys.out:k`. Paths accumulate as `SS/child/…` via `child_system_path`
   and `full_block_path`.
3. **Control ports excluded.** `is_control_port_type`
   (enable/trigger/ifaction/reset/event…) routes through
   `control_incoming_targets`, never into data-path sets (CT-080..085).
4. **Buses carry element identity.** `bus_creator_targets` builds the bus;
   `bus_selector_targets` extracts `bus[i]` → the i-th creator input's
   upstream signal; `bus_assignment_targets` substitutes named elements.
   Upstream mirrors: `bus_*_upstream_targets` (CT-040..049).
5. **Mux/demux preserve lane index.** `mux_targets`/`demux_targets` and their
   upstream counterparts map port i ↔ lane i (CT-070..074).
6. **Goto/From is a virtual wire.** `resolve_from_block_targets` links From
   blocks to matching Goto tags (scope rules: local first, then
   enclosing/global per CT-062/063), and `upstream_propagated_targets`
   reports the *real* driver, not the From block (CT-064, CT-093).
7. **Upstream completion.** `propagate_line_targets` walks downward;
   `propagate_line_metadata_upward` and the `*_upstream_targets` helpers
   complete the reverse direction — origins of every consumed signal.
8. **Termination via visited-set.** `visited: Arc<AtomicUsize>` bounds work;
   cycles (unit-delay feedback) terminate (CT-007, FZ-012).

### I.3 Oracle construction rules (for `ConnectivityOracle`)

When hand-writing expected target sets:

- Target paths are **model-root-relative**, slash-separated, using
  `full_block_path` conventions — never include the boundary-port pseudo
  blocks unless the API exposes them (check: does a resolved target ever
  name `SubSys.in:1` itself, or only inner real blocks? Decide once, write
  it in `norm()`, stay consistent).
- A line feeding N branch destinations lists N targets — order-insensitive.
- A bus element target is the *consuming* endpoint (e.g.
  `Sel.demux_out:2`'s consumer), while its *origin* is the creator's
  input driver — origins and targets are different directions, don't mix.
- Through a goto/from hop, the From's out-line targets its own consumers;
  the *origin chain* additionally contains the real driver.
- Dangling lines produce empty sets — assert empties, don't skip them.

### I.4 Common propagation bugs to hunt (prioritized by test order)

1. Cross-talk between subsystem in-ports (CT-023).
2. Wrong bus element index (off-by-one zero/one-based) (CT-041).
3. Branch leaves truncated (deepest leaf missing) (P-052, PERF-025).
4. Origin reported as the From block instead of the real driver (CT-093).
5. Boundary path not fully qualified (CT-022, CT-027).
6. Control port leaking into data targets (CT-085).
7. Stale cache after structural edit (CT-104).
8. Cycle hang on feedback topologies (CT-007, FZ-012).
9. Port index collapse: all out-ports treated as out:1 (CT-005).
10. Library-block internals resolved with wrong path prefix (P-093, E2E-020).

---

## Appendix J — Test Maintenance Playbook

### J.1 Regenerating goldens

```bash
# Resolver goldens (Propergation_Test + synthetic fixtures)
REGEN_GOLDEN=1 cargo test --all-features --test e2e_propagation

# Review the diff — every changed line must be explained in the PR.
git diff assets/e2e/golden/
```

Rule: golden updates land in their own commit titled
`test: regen golden <reason>` — never mixed with code changes.

### J.2 Running subsets

```bash
cargo test --all-features --test e2e_editor_dnd          # DnD suite
cargo test --all-features --test e2e_propagation ct_0    # name filter
PERF=1 cargo test --all-features --test e2e_stress -- --ignored
FUZZ=1 cargo test --test e2e_fuzz -- --ignored
SNAPSHOT=1 cargo test --test e2e_viewer                  # optional snapshots
```

### J.3 Triage workflow for a failing E2E

1. Reproduce: `cargo test --all-features <id> -- --exact --nocapture`.
2. Classify: product bug vs test bug vs spec ambiguity ("per impl" cases).
3. Product bug → failing test stays, fix the product.
4. Spec ambiguity → document behaviour, adjust test to the doc, note in PR.
5. Test bug → fix test; if it masked a product bug, file it.
6. Flake → quarantine per §18.7 and rework.

### J.4 When the model format changes

- Update `harness/model_builders.rs` field sets (compile errors will point
  you there), then `debug_validate`.
- Re-checkpoint goldens (J.1).
- Sweep "per impl" cases for assumptions that broke.

### J.5 Adding a new feature's E2E coverage

1. Add cases to the right section, allocated next free ID range.
2. Update §20 index + totals table.
3. Update Appendix C matrix row and Appendix H traceability row.
4. If it introduces a gesture/state machine → extend §F.3 lifecycle table.
5. If it affects connectivity → add an oracle rule to §I.3.

### J.6 Pre-merge checklist for E2E PRs

- [ ] `cargo fmt --check` clean
- [ ] `cargo clippy --all-features -- -D warnings` clean
- [ ] `cargo test --all-features` green (all suites, not just changed)
- [ ] New cases indexed in §20 with priority
- [ ] Goldens regenerated + reviewed if resolver touched
- [ ] No `sleep`, no hardcoded pixel paths (except harness internals)
- [ ] `debug_validate` passes on every generated fixture
- [ ] CI feature-matrix check run for flag changes

---

## Appendix K — Suite File Skeletons (M1 starter code)

Each `tests/e2e_*.rs` begins from this pattern so suites stay uniform.

### K.1 `tests/e2e_editor_dnd.rs`

```rust
//! E2E: editor drag-and-drop via synthesized egui input.
//! Covers §10.2–10.6 (DragMode::*) and §10.7 (ViewerDragState::*).

#[path = "e2e/harness/mod.rs"]
mod harness;
use harness::{gui::EditorHarness, input, model_builders::*};

fn fixture_two_blocks() -> System {
    let mut s = System::default();
    s.blocks.push(BlockBuilder::new("Constant", "C", "1")
        .pos(0, 0, 40, 14).ports(0, 1).build());
    s.blocks.push(BlockBuilder::new("Scope", "S", "2")
        .pos(200, 0, 240, 14).ports(1, 0).build());
    s
}

#[test]
fn ed_001_drag_single_block() {
    let mut h = EditorHarness::new(fixture_two_blocks());
    h.idle(2);
    let r0 = h.screen_pos_of_block("1");
    let before = h.state.current_system().unwrap().clone();

    h.drag(r0.center(), r0.center() + egui::vec2(43.0, 17.0), 6);

    let after = h.state.current_system().unwrap();
    let p = after.blocks[0].position.unwrap();
    assert_eq!((p.0 - before.blocks[0].position.unwrap().0,
                p.1 - before.blocks[0].position.unwrap().1),
               (40, 20)); // grid-snapped
    assert!(h.state.history.can_undo());
}
```

### K.2 `tests/e2e_propagation.rs`

```rust
//! E2E: connection-target resolution / signal propagation. §8.

#[path = "e2e/harness/mod.rs"]
mod harness;
use harness::{assert_targets_eq, model_builders::*};
use rustylink::connection_targets::ConnectionTargetResolver;

#[test]
fn ct_001_direct_line_src_dst() {
    let sys = gain_chain(1);
    let res = ConnectionTargetResolver::new(&sys);
    let t = res.line_targets_for_line_ref(&sys.lines[0]);
    assert_targets_eq(&t, &[("G0", 1)]);
}

#[test]
fn ct_110_propergation_golden() {
    let sys = open_test_model("Propergation_Test.slx");
    let res = ConnectionTargetResolver::new(&sys);
    let golden = read_golden("propergation.targets.json");
    diff_targets(&res, &golden); // regenerates under REGEN_GOLDEN=1
}
```

### K.3 `tests/e2e_stress.rs` (high data-rate core)

```rust
//! E2E: scale & rate. §14. `PERF=1` unlocks the heavy cases.

#[path = "e2e/harness/mod.rs"]
mod harness;
use harness::{gen_models::*, gui::ViewerHarness, live_feed::*};

#[test]
fn perf_003_resolve_1000_signals() {
    let (sys, oracle) = gen_wide_net(1000, 3);
    let res = rustylink::connection_targets::ConnectionTargetResolver::new(&sys);
    for (k, want) in &oracle.line_targets {
        let got = targets_for_key(&res, &sys, k);
        assert_targets_eq(&got, &refs(want));
    }
}

#[test]
#[ignore = "PERF=1"]
fn perf_004_resolve_5000_signals() { /* … same at 5000 … */ }
```

### K.4 Remaining suite skeletons follow the identical pattern:

- `e2e_parse.rs` → `use harness::{slx_factory, model_builders}` + fixture slx.
- `e2e_serialize.rs` → tempdir + `SlxArchive`/`SystemDoc` round-trips.
- `e2e_routing.rs` → pure fn calls + `EditorHarness` where interactive.
- `e2e_editor_ops.rs` → `EditorHarness` command-level (no canvas needed).
- `e2e_viewer.rs` → `ViewerHarness`.
- `e2e_live.rs` → `ViewerHarness` + `live_feed`.
- `e2e_dashboard.rs` → `ViewerHarness` + `dashboard` feature cases.
- `e2e_journeys.rs` → composes all harnesses; ends with §15.5 gate.
- `e2e_fuzz.rs` → `proptest!` blocks + seeded RNG loops.
- `e2e_errors.rs` → `raw_zip` + malformed inputs + `assert_cmd`-style CLI runs.

---

## Appendix L — LiveValue Quick Reference (for LV/PERF authors)

```rust
// Construction shapes used by feeds:
LiveValue { dims: vec![],              data: LiveValueList::Scalar(1.5) }        // scalar
LiveValue { dims: vec![3],             data: LiveValueList::Vector(vec![..]) }   // vector
LiveValue { dims: vec![2, 3],          data: LiveValueList::Matrix(..) }         // column-major
LiveValue { dims: vec![4],             data: LiveValueList::Complex32(..) }      // complex

// Entry = value + optional display override:
LiveValueEntry::new(v).with_display(LiveValueDisplayOptions {
    float_decimals: 3,
    scientific_lower_bound: 1e-4,
    scientific_upper_bound: 1e6,
    always_scientific: false,
    ..LiveValueDisplayOptions::normalized() // fills defaults
})

// Host ingestion path (what feeds simulate):
app.live_values.insert("sig/path".into(), entry);       // line-level
app.live_block_values.insert("block/path".into(), entry);
app.live_block_tooltips.insert("block/path".into(), vec![tt]);
app.live_line_tooltips.insert(line_idx, vec![tt]);
scope_instances.lock().unwrap()
    .entry(scope_key).or_insert_with(MiniScope::new)
    .push_sample(v);
```

Formatting contract under test (LV-*):
- `format` respects `float_decimals`, switches to scientific outside
  `[scientific_lower_bound, scientific_upper_bound]` or when
  `always_scientific`.
- vectors print space-separated; matrices print column-major per Simulink.
- `first_f64`/`f64_at` give host-facing numeric access over every variant.

---

## Appendix M — Glossary of Test-Specific Terms

- **Oracle (ConnectivityOracle)** — expected target sets produced
  structurally by a generator at construction time; the resolver's output is
  diffed against it, so the checker never re-implements the resolver.
- **Golden** — checked-in expected artifact (`.targets.json`) regenerated
  via `REGEN_GOLDEN=1`.
- **Gesture** — a sequence of `RawInput` frames emulating one user action.
- **Lifecycle cell** — one (drag-state × abort/commit-path) combination in
  the §F.3 table.
- **Consistency gate** — the §15.5 tail assertions closing every journey.
- **Machine-scaled assertion** — perf bound expressed as a ratio/complexity
  bound or with ≥4× headroom, stable across CI machine speeds.
- **Paint assertion** — structural check over `epaint::ClippedShape`s
  (counts/kinds/positions), not pixels.
- **`per impl`** — marker on a case whose exact expected value depends on a
  documented behaviour decision still to be made (§D.2); such a case is a
  spec-discovery probe, and its resolution is a doc update.
- **Nightly gate** — env-gated (`PERF=1`/`FUZZ=1`) `#[ignore]` tests run by
  the scheduled CI job only.

---

*Plan complete: 13 suites, ~491 indexed cases + appendices G–M additions,
covering parsing, serialization, signal propagation, routing, editor DnD,
viewer, live values, dashboards, scale/high-rate stress, journeys, fuzz and
robustness — with harness specs, worked examples, coverage matrix and
milestones M1–M8.*

---

## Appendix N — Canvas Interaction Internals (for ED test authors)

Details of how `src/editor/ui/canvas.rs` and `src/egui_app/ui/*` wire input,
so gesture tests target the right hit areas.

### N.1 Port hit areas

```rust
// canvas.rs — invisible interaction rects over port chevrons:
let hit_rect = /* rect around chevron, sized via port_chevron_size(font_scale) */;
let resp = ui.allocate_rect(hit_rect, Sense::click_and_drag());
if resp.drag_started() {
    state.drag_mode = DragMode::Connection {
        src_sid, src_port_type, src_port_index, current_x, current_y,
    };
}
```

Test implications:
- Press point must be inside `hit_rect`: use `h.port_screen_pos()` and nudge
  at most ±2 px — never press the block edge.
- `drag_started` only fires on the frame *after* press+first move (§10.1.1).
- `current_x/current_y` update every move frame — preview paint is assertable
  mid-gesture via `h.canvas_shapes()`.

### N.2 DragMode transitions (editor)

```
None ──press on port + move──▶ Connection{…}
None ──press on block + move─▶ Blocks{dx,dy}
None ──press on bg + move────▶ SelectionRect  (or Pan w/ middle/alt)
None ──press on handle+move──▶ Resize{block_index, handle, original_*}
*any* ──release──────────────▶ None (+ optional EditorCommand)
*any* ──Esc─────────────────▶ None (no command)
```

Every transition edge is covered by ≥1 case in §10; the state machine itself
is asserted by watching `h.state.drag_mode` across frames:

```rust
h.frame(input::press(p));              assert!(matches!(h.state.drag_mode, DragMode::None));
h.frame(input::move_to(p + v));        assert!(matches!(h.state.drag_mode, DragMode::Connection{..}));
h.frame(input::release(q));            assert!(matches!(h.state.drag_mode, DragMode::None));
```

### N.3 Commit semantics

- `Blocks` drag commits **one** `move_blocks` command covering the whole
  selected group (ED-003 group-undo property).
- `Connection` drop on a valid snap target commits `add_line` whose
  `line.points` equal `auto_route(src_pos, dst_pos)` output (ED-034).
- `Resize` commits `resize_block` with snapped `(l,t,r,b)`.
- `SelectionRect` and `Pan` produce **no** command (selection/pan aren't
  undoable) — `history.can_undo()` must be unchanged after them.
- `LinePointDrag`/`BranchPointDrag`/`SignalLabelDrag` (viewer) commit a
  geometry command on release; live `current_dx/dy` state is discarded.

### N.4 Sensitivity & snapping constants

- Grid snap: `state.snap(v)` rounds to the editor grid (default 10 — verify
  in `EditorState`); drag expectations use snapped deltas.
- Port snap radius: `find_snap_port` distance threshold (assert both
  inside/outside boundary in ED-025/026).
- Drag threshold: egui's internal ~2-4 px before `drag_started` — ED-006
  asserts sub-threshold moves never arm `DragMode`.
- Resize min size: `compute_resized_rect` clamps (ED-053).

### N.5 Viewer move-mode state machine

`ViewerDragState` mirrors the editor's but lives on `SubsystemApp` and only
arms when move mode is enabled (VW-050/051):

```
None ─grab block──▶ Blocks{current_dx,current_dy}
None ─grab handle─▶ Resize{sid, handle, original_*, current_*}
None ─grab vertex─▶ LinePointDrag{line_idx, point_idx, acc_*}
None ─grab branch─▶ BranchPointDrag{line_idx, branch_path, point_idx, acc_*}
None ─grab label──▶ SignalLabelDrag{line_idx, acc_*}
```

`acc_dx/acc_dy` accumulate model-space deltas (ints — note the difference
from the editor's `f32` dx/dy; tests assert int rounding behaviour).
`branch_path: Vec<usize>` navigates the branch tree via `get_branch_mut`
(RT-035).

### N.6 What NOT to assert

- Exact preview segment counts mid-drag (implementation detail — assert the
  preview *exists* and ends at cursor).
- egui internal ids/response internals.
- Frame counts (except where the contract is "repaint requested").
- Tooltip timing (egui-managed) — assert content presence after hover frames,
  not the frame it appeared.

---

*— End of document —*

---

## Appendix O — Command Reference & Priority Order

### O.1 Everyday commands

```bash
# everything (unit + all e2e suites)
cargo test --all-features

# only the new e2e layer
for t in e2e_parse e2e_serialize e2e_propagation e2e_routing e2e_editor_dnd \
         e2e_editor_ops e2e_viewer e2e_live e2e_dashboard e2e_journeys \
         e2e_fuzz e2e_errors e2e_stress; do
    cargo test --all-features --test "$t" || break
done

# single case
cargo test --all-features ed_020 -- --exact --nocapture

# heavy opt-ins
PERF=1 cargo test --all-features --test e2e_stress -- --ignored
FUZZ=1 cargo test --all-features --test e2e_fuzz   -- --ignored
REGEN_GOLDEN=1 cargo test --all-features --test e2e_propagation
```

### O.2 Implementation priority order (all P0 first)

| Phase | Cases | Why first |
|---|---|---|
| 1 | Harness smoke + ED-001/020/040/050 (one per drag kind) | de-risks the whole GUI layer |
| 2 | CT-001/020/040/060/070/080/090/100/110 | propagation correctness is core value |
| 3 | RT-001/003/030/040/044/050/054/060 | routing feeds every DnD assertion |
| 4 | Remaining ED/VW/LV/DB P0s | breadth of interaction |
| 5 | PERF-001/003/006/023/030/040/041/042/045 | scale + high-rate confidence |
| 6 | E2E journeys, FZ, ERR, remaining P1/P2 | completeness |

### O.3 Sign-off checklist for the plan itself

- [x] All user-emphasized areas covered in depth: **DnD** (§10, App. F, App. N),
      **signal propagation** (§8, App. I), **routing** (§9),
      **many signals** (§14.1–14.3), **high data-rate** (§14.4, PERF-040..050).
- [x] Full feature inventory mapped (§2.3, App. C).
- [x] Harness design with code sketches (§4, App. A).
- [x] Every case indexed with priority (§20).
- [x] CI wiring, coverage targets, flake policy (§18).
- [x] Milestones M1–M8 with exit criteria (§19).
- [x] Dependency/dev-tooling decisions flagged (egui_kittest, proptest).

*— Document ends at 4 000+ lines of plan. —*

---

## Appendix P — Deep Dive: The Two Highest-Risk Test Areas

### P.1 Why DnD tests are hard (and how this plan de-risks them)

1. **Frame-boundary semantics.** egui reports `drag_started` one frame after
   press+move. A test that fires press, move, and release in one `RawInput`
   produces a *click*, not a drag — silently passing on the wrong code path.
   Mitigation: every gesture helper in `harness/input.rs` is per-frame, and
   §N.2 provides probe tests asserting the `DragMode` transition itself.
2. **Two coordinate systems.** Pointers are screen-space; models are
   model-space. Every expectation must pass through `ViewTransform`. Rule:
   compute expectations in model space, never hardcode pixels (App. F.5).
3. **Async resolver.** `ensure_resolver` resolves in the background; hover
   tooltips depend on it. Tests poll `resolver_status` until `Ready` rather
   than assuming readiness (CT-105, ED-012).
4. **State leakage between gestures.** `drag_mode`, selection, hover, and
   armed dialogs must be reset per test; `EditorHarness::new` per test is
   non-negotiable — never reuse a harness across test cases.
5. **Hit-test priority.** A press that lands on overlapping hit regions can
   arm the wrong mode. The §F.4 priority table is the contract; ambiguous
   discoveries become doc-or-fix items, not test noise.

### P.2 Why propagation oracles are hard (and how this plan de-risks them)

1. **The checker must not re-implement the system.** Oracles are generated
   alongside the model by `gen_models` (structural knowledge) or stored as
   reviewed goldens — never computed by walking the model in the test, which
   would just re-derive the same bugs.
2. **Path qualification is the sharp edge.** `SS1/Gain` vs `Gain` vs
   `root/SS1/Gain` — the `norm()` helper and §I.3 rules pin one convention;
   every CT case flows through it.
3. **Cycles and termination.** Feedback via unit delays is legal and must
   resolve without hang; `visited`-bound work is asserted, not timing.
4. **Bus element identity.** Off-by-one between 0- and 1-based indexing is
   the classic propagation bug — CT-041/042/046 pin it explicitly.
5. **Goto/from scoping.** Local-before-global visibility (CT-062/063) is
   modelled; when scope rules are ambiguous in the implementation, the case
   is a spec-discovery probe per §D.2.

---

## Revision History & Sign-Off

| Rev | Date | Author | Change |
|---|---|---|---|
| 1.0 | 2026-11-04 | Devin (for repo maintainer) | Initial complete plan: 13 suites, 491 indexed cases, harness + worked examples, perf/stress & high-rate coverage, CI wiring, milestones M1–M8 |

Approver checklist (maintainer):

- [ ] Scope (§1.2) acceptable — no MATLAB dependency required
- [ ] Harness approach chosen (§4.3 open question #4)
- [ ] Coverage targets (§18.4) agreed
- [ ] Milestone ordering (§19) agreed — DnD prioritized at M4 as requested
- [ ] Open questions (§D.3) answered or converted to spec-discovery cases

### Contact / ownership

- Suite ownership defaults to the crate maintainer; per-suite owners may be
  assigned in PR descriptions as implementation lands.
- Questions on this plan → comment on the tracking issue/PR; case-level
  questions → reference the case ID (§20).
- Keep this file in `docs/` so it ships with the repo and renders on GitHub.

*— End of E2E test plan —*
