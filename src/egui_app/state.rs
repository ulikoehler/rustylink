#![cfg(feature = "egui")]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::BufReader;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use eframe::egui::{self, Vec2};

use crate::editor::operations::EditorHistory;
use crate::model::{Annotation, Block, Chart, Line, SlxArchive, System};
use crate::parser::{FsSource, SimulinkParser, ZipSource};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct LayoutSnapshot {
    version: u32,
    root: System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveTooltipKind {
    Signal,
    Parameter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveTooltipEntry {
    pub datafield_name: String,
    pub kind: LiveTooltipKind,
    pub formatted_value: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct NavigationViewState {
    pub zoom: f32,
    pub pan: [f32; 2],
    pub view_bounds: Option<[f32; 4]>,
}

impl NavigationViewState {
    fn from_runtime(zoom: f32, pan: Vec2, view_bounds: Option<egui::Rect>) -> Self {
        Self {
            zoom,
            pan: [pan.x, pan.y],
            view_bounds: view_bounds.map(|r| [r.min.x, r.min.y, r.max.x, r.max.y]),
        }
    }

    fn to_runtime(&self) -> (f32, Vec2, Option<egui::Rect>) {
        let view_bounds = self
            .view_bounds
            .map(|r| egui::Rect::from_min_max(egui::pos2(r[0], r[1]), egui::pos2(r[2], r[3])));
        (self.zoom, Vec2::new(self.pan[0], self.pan[1]), view_bounds)
    }
}

fn authored_navigation_view_states(root: &System) -> BTreeMap<String, NavigationViewState> {
    fn walk(
        system: &System,
        path: &mut Vec<String>,
        out: &mut BTreeMap<String, NavigationViewState>,
    ) {
        if let Some(state) = authored_navigation_view_state(system) {
            out.insert(SubsystemApp::path_key(path), state);
        }

        for block in &system.blocks {
            if !matches!(block.block_type.as_str(), "SubSystem" | "Reference") {
                continue;
            }
            let Some(subsystem) = block.subsystem.as_ref() else {
                continue;
            };
            if subsystem.chart.is_some() {
                continue;
            }

            path.push(block.name.clone());
            walk(subsystem, path, out);
            let _ = path.pop();
        }
    }

    let mut defaults = BTreeMap::new();
    walk(root, &mut Vec::new(), &mut defaults);
    defaults
}

fn authored_navigation_view_state(system: &System) -> Option<NavigationViewState> {
    let zoom = system
        .properties
        .get("ZoomFactor")
        .and_then(|value| parse_authored_zoom_factor(value));
    let view_bounds = system
        .properties
        .get("Location")
        .and_then(|value| parse_authored_view_bounds(value));

    if zoom.is_none() && view_bounds.is_none() {
        return None;
    }

    Some(NavigationViewState {
        zoom: zoom.unwrap_or(1.0),
        pan: [0.0, 0.0],
        view_bounds,
    })
}

fn parse_authored_zoom_factor(raw: &str) -> Option<f32> {
    let zoom = raw.trim().parse::<f32>().ok()?;
    (zoom > 0.0).then_some(zoom / 100.0)
}

fn parse_authored_view_bounds(raw: &str) -> Option<[f32; 4]> {
    let values = parse_authored_number_list(raw)?;
    let [left, top, right, bottom] = values.as_slice() else {
        return None;
    };

    let width = (right - left).abs();
    let height = (bottom - top).abs();
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    Some([0.0, 0.0, width, height])
}

fn parse_authored_number_list(raw: &str) -> Option<Vec<f32>> {
    let trimmed = raw.trim().trim_start_matches('[').trim_end_matches(']');
    let values = trimmed
        .split(',')
        .map(|part| part.trim().parse::<f32>().ok())
        .collect::<Option<Vec<_>>>()?;
    (!values.is_empty()).then_some(values)
}

#[allow(clippy::type_complexity)]
fn load_source_model(
    source_path: &Utf8Path,
) -> anyhow::Result<(System, BTreeMap<u32, Chart>, BTreeMap<String, u32>)> {
    if source_path.extension() == Some("slx") {
        let archive = SlxArchive::from_file(source_path)?;
        let system = archive.assembled_root_system()?;

        let file =
            std::fs::File::open(source_path).with_context(|| format!("Open {}", source_path))?;
        let reader = BufReader::new(file);
        let mut parser = SimulinkParser::new("", ZipSource::new(reader)?);
        let root = Utf8PathBuf::from("simulink/systems/system_root.xml");
        let _ = parser.parse_system_file(&root)?;

        let charts = parser.get_charts().clone();
        let mut chart_map: BTreeMap<String, u32> = parser
            .get_sid_to_chart_map()
            .iter()
            .map(|(sid, cid)| (sid.to_string(), *cid))
            .collect();
        for (name, cid) in parser.get_system_to_chart_map().iter() {
            chart_map.entry(name.clone()).or_insert(*cid);
        }
        let mut system = system;
        crate::parser::annotate_matlab_function_names(&mut system, &charts, &chart_map);
        return Ok((system, charts, chart_map));
    }

    let root_dir = source_path.parent().unwrap_or(Utf8Path::new("."));
    let mut parser = SimulinkParser::new(root_dir, FsSource);
    let system = parser
        .parse_system_file(source_path)
        .with_context(|| format!("Failed to parse {}", source_path))?;

    let charts = parser.get_charts().clone();
    let mut chart_map: BTreeMap<String, u32> = parser
        .get_sid_to_chart_map()
        .iter()
        .map(|(sid, cid)| (sid.to_string(), *cid))
        .collect();
    for (name, cid) in parser.get_system_to_chart_map().iter() {
        chart_map.entry(name.clone()).or_insert(*cid);
    }
    let mut system = system;
    crate::parser::annotate_matlab_function_names(&mut system, &charts, &chart_map);
    Ok((system, charts, chart_map))
}

// use super::geometry::parse_block_rect;
use super::navigation::{collect_subsystems_paths, resolve_subsystem_by_vec};
// use super::render::get_block_type_cfg;
// use super::text::highlight_query_job;
// use crate::label_place::{self};

/// Data needed to open a chart popup.
#[derive(Clone)]
pub struct ChartView {
    pub title: String,
    pub script: String,
    pub open: bool,
}

/// Data for a selected signal information dialog.
#[derive(Clone)]
pub struct SignalDialog {
    pub title: String,
    pub line_idx: usize,
    pub open: bool,
}

/// Data for a selected block information dialog.
#[derive(Clone)]
pub struct BlockDialog {
    pub title: String,
    pub block: Arc<Block>,
    pub open: bool,
}

/// Button specification for customizing the Signal dialog.
#[derive(Clone)]
pub struct SignalDialogButton {
    pub label: String,
    pub filter: Arc<dyn Fn(&crate::model::Line) -> bool + Send + Sync>,
    pub on_click: Arc<dyn Fn(&crate::model::Line) + Send + Sync>,
}

/// Button specification for customizing the Block dialog.
#[derive(Clone)]
pub struct BlockDialogButton {
    pub label: String,
    pub filter: Arc<dyn Fn(&Block) -> bool + Send + Sync>,
    pub on_click: Arc<dyn Fn(&Block) + Send + Sync>,
}

/// Context menu item specification for signals.
#[derive(Clone)]
pub struct SignalContextMenuItem {
    pub label: String,
    pub filter: Arc<dyn Fn(&crate::model::Line) -> bool + Send + Sync>,
    pub on_click: Arc<dyn Fn(&crate::model::Line) + Send + Sync>,
}

/// Context menu item specification for blocks.
#[derive(Clone)]
pub struct BlockContextMenuItem {
    pub label: String,
    pub filter: Arc<dyn Fn(&Block) -> bool + Send + Sync>,
    pub on_click: Arc<dyn Fn(&Block) + Send + Sync>,
}

/// Borrowed view of all entities within the currently displayed subsystem.
///
/// This holds references into the model tree, avoiding expensive per-frame
/// clones of all blocks, lines, and annotations.
pub struct SubsystemEntities<'a> {
    pub blocks: &'a [Block],
    pub lines: &'a [Line],
    pub annotations: Vec<&'a Annotation>,
}

/// State for a scope popout window.
#[cfg(feature = "dashboard")]
#[derive(Clone)]
pub struct ScopePopout {
    /// Window title (e.g. "Scope: MyScope").
    pub title: String,
    /// Key into `scope_instances` for the liveplot data.
    pub scope_key: String,
    /// Whether the window is still open.
    pub open: bool,
}

/// A live dashboard control update emitted by the embedded viewer.
#[cfg(feature = "dashboard")]
#[derive(Clone, Debug)]
pub enum DashboardControlValue {
    Scalar(f64),
    Bool(bool),
    PulseHigh,
    PulseLow,
}

/// A queued dashboard control interaction awaiting consumption by the host.
#[cfg(feature = "dashboard")]
#[derive(Clone, Debug)]
pub struct DashboardControlEvent {
    pub block: Block,
    pub value: DashboardControlValue,
}

/// Active drag interaction inside the viewer move mode.
#[derive(Clone, Default)]
pub enum ViewerDragState {
    #[default]
    None,
    Blocks {
        current_dx: i32,
        current_dy: i32,
    },
    Resize {
        sid: String,
        handle: u8,
        original_l: i32,
        original_t: i32,
        original_r: i32,
        original_b: i32,
        current_dx: i32,
        current_dy: i32,
    },
    LinePointDrag {
        line_idx: usize,
        point_idx: usize,
        acc_dx: i32,
        acc_dy: i32,
    },
    BranchPointDrag {
        line_idx: usize,
        branch_path: Vec<usize>,
        point_idx: usize,
        acc_dx: i32,
        acc_dy: i32,
    },
    SignalLabelDrag {
        line_idx: usize,
        acc_dx: i32,
        acc_dy: i32,
    },
}

/// Result of checking whether the connection-target resolver is ready.
#[derive(Debug, Clone)]
pub enum ResolverStatus {
    /// Resolver is ready to use.
    Ready(Arc<crate::connection_targets::ConnectionTargetResolver>),
    /// Resolver is being built in a background thread.
    Building {
        /// 0.0..=1.0 (for the progress bar fill).
        progress: f32,
        /// Subsystems visited so far.
        current: usize,
        /// Estimated total work.
        total: usize,
    },
}

/// Shared state for the background resolver build, protected by a `Mutex`.
#[derive(Debug, Default)]
enum ResolverBuildInner {
    #[default]
    Idle,
    Building {
        /// 0..=1000 (bar fill × 1000).
        progress: Arc<AtomicU32>,
        /// Total `resolve_system` calls so far.
        visited: Arc<AtomicUsize>,
        /// Estimated total work for the progress display.
        total: usize,
        /// Topology signature this build was started for.
        sig: u64,
    },
    Ready {
        resolver: Arc<crate::connection_targets::ConnectionTargetResolver>,
        sig: u64,
    },
}

/// Cached per-frame computations that only need to be recalculated when the
/// model changes (e.g. after a drag-commit, navigation, or layout load/save).
///
/// Stored in [`SubsystemApp`] and invalidated by bumping `generation`.
#[derive(Clone)]
pub struct ComputedViewCache {
    /// Monotonically increasing counter; cached values are valid when their
    /// stored generation matches.
    pub generation: u64,
    /// Pre-computed line colors (one per line in the current subsystem).
    pub line_colors: Vec<egui::Color32>,
    /// Port-count map: (SID, port_type_byte) → count.
    pub port_counts: std::collections::HashMap<(String, u8), u32>,
    /// Set of (SID, port_index, is_input) triples that have a connected signal.
    pub connected_ports: std::collections::HashSet<(String, u32, bool)>,
    /// Cached connection target graph reused across paint passes until invalidated.
    pub connection_target_resolver:
        Option<Arc<crate::connection_targets::ConnectionTargetResolver>>,
    /// Cached shallow-cloned blocks for the current subsystem view.
    /// Avoids re-cloning all blocks every frame when the path/model hasn't changed.
    pub cached_owned_blocks: Vec<crate::model::Block>,
    /// Cached cloned lines for the current subsystem view.
    pub cached_sys_lines: Vec<crate::model::Line>,
    /// Cached annotations (system + block-level) for the current subsystem view.
    pub cached_sys_annotations: Vec<crate::model::Annotation>,
    /// Cached subsystem block lookup map (SID → full block with subsystem).
    pub cached_subsystem_block_lookup: HashMap<String, crate::model::Block>,
    /// Topology signature the cached resolver was built from.  The resolver
    /// depends only on model topology (not geometry) and spans the whole tree,
    /// so it is reused across subsystem navigation and layout-only edits and is
    /// rebuilt only when this signature changes.
    cached_resolver_sig: Option<u64>,
    /// Generation at which the topology signature was last computed.
    /// Avoids re-hashing the entire tree every frame when the model is unchanged.
    cached_sig_gen: u64,
    /// The subsystem path for which this cache was computed.
    pub cached_path: Vec<String>,
    /// Model generation at which the cache was computed.
    pub cached_gen: u64,
    /// Shared state for background resolver construction.
    resolver_build: Arc<Mutex<ResolverBuildInner>>,
}

impl Default for ComputedViewCache {
    fn default() -> Self {
        Self {
            // Start at 1 so the initial cached_gen=0 never matches: cache always starts invalid.
            generation: 1,
            line_colors: Vec::new(),
            port_counts: std::collections::HashMap::new(),
            connected_ports: std::collections::HashSet::new(),
            connection_target_resolver: None,
            cached_resolver_sig: None,
            cached_sig_gen: 0,
            cached_owned_blocks: Vec::new(),
            cached_sys_lines: Vec::new(),
            cached_sys_annotations: Vec::new(),
            cached_subsystem_block_lookup: HashMap::new(),
            cached_path: Vec::new(),
            cached_gen: 0,
            resolver_build: Arc::new(Mutex::new(ResolverBuildInner::Idle)),
        }
    }
}

static NEXT_VIEWER_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

impl ComputedViewCache {
    /// Returns `true` if the cache is valid for the given path and generation.
    pub fn is_valid(&self, path: &[String], generation: u64) -> bool {
        self.cached_gen == generation && self.cached_path == path
    }

    /// Mark the cache as valid for the given path and generation.
    pub fn mark_valid(&mut self, path: &[String], generation: u64) {
        self.cached_path = path.to_vec();
        self.cached_gen = generation;
    }

    /// Bump the generation counter, invalidating the cache.
    pub fn invalidate(&mut self) {
        self.generation += 1;
    }

    /// Ensure the cached connection-target resolver is up to date for `root`,
    /// rebuilding it only when the model topology signature has changed.
    ///
    /// This is independent of the path/generation validity used for the
    /// geometry-sensitive caches: navigating subsystems or moving blocks does
    /// not rebuild the (whole-tree, topology-only) resolver.
    pub fn ensure_resolver(
        &mut self,
        root: &System,
    ) -> Arc<crate::connection_targets::ConnectionTargetResolver> {
        // Only re-hash the topology when the model generation has changed.
        // This avoids walking the entire system tree every frame.
        if self.connection_target_resolver.is_none() || self.cached_sig_gen != self.generation {
            let sig = crate::connection_targets::model_topology_signature(root);
            self.cached_sig_gen = self.generation;
            if self.connection_target_resolver.is_none() || self.cached_resolver_sig != Some(sig) {
                self.connection_target_resolver = Some(Arc::new(
                    crate::connection_targets::ConnectionTargetResolver::new(root),
                ));
                self.cached_resolver_sig = Some(sig);
            }
        }
        self.connection_target_resolver
            .clone()
            .expect("resolver just populated")
    }

    /// Check whether the connection-target resolver is ready, starting a
    /// background build if needed.
    ///
    /// Unlike [`ensure_resolver`](Self::ensure_resolver) (which blocks), this
    /// spawns a background thread to build the resolver and returns
    /// [`ResolverStatus::Building`] while it works.  When the build finishes,
    /// the next call picks up the result and returns
    /// [`ResolverStatus::Ready`].
    pub fn resolver_status(&mut self, root: &System) -> ResolverStatus {
        // Check if we need to recompute the topology signature.
        let need_sig_check =
            self.connection_target_resolver.is_none() || self.cached_sig_gen != self.generation;

        if need_sig_check {
            let sig = crate::connection_targets::model_topology_signature(root);
            self.cached_sig_gen = self.generation;

            // If cached resolver matches the new sig, it's still valid.
            if self.cached_resolver_sig == Some(sig)
                && let Some(r) = &self.connection_target_resolver
            {
                return ResolverStatus::Ready(r.clone());
            }
            // Clear stale cache so connection_target_resolver() returns None.
            self.connection_target_resolver = None;
            return self.start_or_check_build(root, sig);
        }

        // Generation unchanged and resolver exists.
        if let Some(r) = &self.connection_target_resolver {
            return ResolverStatus::Ready(r.clone());
        }

        // No resolver — start build.
        let sig = crate::connection_targets::model_topology_signature(root);
        self.start_or_check_build(root, sig)
    }

    fn start_or_check_build(&mut self, root: &System, sig: u64) -> ResolverStatus {
        let mut state = self.resolver_build.lock().unwrap();

        match &*state {
            // Completed build with matching sig → pick it up.
            ResolverBuildInner::Ready {
                resolver,
                sig: build_sig,
            } if *build_sig == sig => {
                let r = resolver.clone();
                *state = ResolverBuildInner::Idle;
                drop(state);
                self.connection_target_resolver = Some(r.clone());
                self.cached_resolver_sig = Some(sig);
                ResolverStatus::Ready(r)
            }
            // In-progress build with matching sig → report progress.
            ResolverBuildInner::Building {
                progress,
                visited,
                total,
                sig: build_sig,
            } if *build_sig == sig => {
                let p = progress.load(Ordering::Relaxed) as f32 / 1000.0;
                let c = visited.load(Ordering::Relaxed);
                ResolverStatus::Building {
                    progress: p,
                    current: c,
                    total: *total,
                }
            }
            // Idle, or sig mismatch → start new build.
            _ => {
                let progress = Arc::new(AtomicU32::new(0));
                let visited = Arc::new(AtomicUsize::new(0));
                // Compute total work for progress display.
                let total_subsystems = crate::connection_targets::count_subsystems(root).max(1);
                let max_depth = crate::connection_targets::max_subsystem_depth(root);
                let estimated_passes = crate::connection_targets::MAX_GLOBAL_RESOLVE_PASSES
                    .min(max_depth + 2)
                    .max(1);
                let total = estimated_passes * total_subsystems;

                *state = ResolverBuildInner::Building {
                    progress: progress.clone(),
                    visited: visited.clone(),
                    total,
                    sig,
                };
                drop(state);

                let root_clone = root.clone();
                let build_state = self.resolver_build.clone();
                std::thread::spawn(move || {
                    let resolver =
                        crate::connection_targets::ConnectionTargetResolver::new_with_progress(
                            &root_clone,
                            progress,
                            visited,
                        );
                    let mut state = build_state.lock().unwrap();
                    // Only store if we're still the active build (sig matches).
                    if matches!(&*state,
                        ResolverBuildInner::Building { sig: s, .. }
                        if *s == sig)
                    {
                        *state = ResolverBuildInner::Ready {
                            resolver: Arc::new(resolver),
                            sig,
                        };
                    }
                    // Otherwise: stale result, discard.
                });
                ResolverStatus::Building {
                    progress: 0.0,
                    current: 0,
                    total,
                }
            }
        }
    }
}

/// Interactive Egui application that displays and navigates a Simulink subsystem tree.
#[derive(Clone)]
pub struct SubsystemApp {
    pub instance_id: u64,
    pub root: System,
    /// Snapshot of the root system at construction / last load, used for "Restore layout".
    pub original_root: System,
    /// Original source-model path when the viewer was loaded from disk.
    pub source_model_path: Option<Utf8PathBuf>,
    pub path: Vec<String>,
    pub all_subsystems: Vec<Vec<String>>,
    pub search_query: String,
    pub search_matches: Vec<Vec<String>>,
    pub zoom: f32,
    pub pan: Vec2,
    pub reset_view: bool,
    pub chart_view: Option<ChartView>,
    pub charts: BTreeMap<u32, Chart>,
    pub chart_map: BTreeMap<String, u32>,
    pub signal_view: Option<SignalDialog>,
    pub block_view: Option<BlockDialog>,
    /// Custom buttons to render inside the signal dialog.
    pub signal_buttons: Vec<SignalDialogButton>,
    /// Custom buttons to render inside the block dialog.
    pub block_buttons: Vec<BlockDialogButton>,
    /// Custom context menu items for signals.
    pub signal_menu_items: Vec<SignalContextMenuItem>,
    /// Custom context menu items for blocks.
    pub block_menu_items: Vec<BlockContextMenuItem>,
    /// Transient in-GUI notification shown for a short time.
    pub transient_notification: Option<(String, std::time::Instant)>,
    /// The library search paths that were used when the root system was parsed.
    /// Empty if no library lookup was performed.
    pub library_search_paths: Vec<Utf8PathBuf>,
    /// Registered listeners to be notified whenever the displayed subsystem changes.
    #[allow(clippy::type_complexity)]
    subsystem_change_listeners:
        Vec<Arc<dyn for<'a> Fn(&'a [String], &'a SubsystemEntities<'a>) + Send + Sync>>, // private to encourage using the API
    /// Optional click handler to override default action when clicking a block.
    /// Return true from the handler to indicate the click was handled and suppress the default behavior.
    #[allow(clippy::type_complexity)]
    pub block_click_handler: Option<Arc<dyn Fn(&mut SubsystemApp, &Block) -> bool + Send + Sync>>,

    /// Global default for showing block names.
    ///
    /// Per-block override: `Block::show_name = Some(true/false)`.
    pub show_block_names_default: bool,

    /// "Less colorful" rendering mode.
    ///
    /// When enabled, every block body is drawn with a neutral light-gray fill
    /// (bordered so it stays visible in light themes) and signal lines are drawn
    /// in neutral gray, regardless of block-type/signal coloring.  Area
    /// annotations keep their model-defined colors.
    pub monochrome: bool,

    /// Block-name font size as a factor of the port chevron height.
    ///
    /// A value of ~1.0 makes the text approximately the same height as the chevrons.
    pub block_name_font_factor: f32,

    /// Multiplier for the horizontal width available to wrapped block names.
    pub block_name_extend_factor: f32,

    /// Value-text font size factor for Constant/Display block content.
    pub block_value_font_factor: f32,

    /// Selected block SIDs in the current view (supports multi-selection).
    pub selected_block_sids: BTreeSet<String>,

    /// Selected line indices in the current subsystem view.
    pub selected_line_indices: BTreeSet<usize>,

    /// Whether interactive move/resize mode is enabled.
    pub move_mode_enabled: bool,

    /// Whether "assign UI elements" mode is enabled.
    ///
    /// When `true`, a primary click on a block or signal triggers the
    /// host application's element-assignment action instead of opening
    /// the default info dialog.  Rustylink renders an "Assign: On/Off"
    /// toggle button next to the "Edit: On/Off" button in the toolbar;
    /// the host can also toggle this programmatically.
    pub add_mode_enabled: bool,

    /// When `true`, dashboard blocks render live values from `live_values` instead of static icons.
    pub live_mode_enabled: bool,

    /// Live values for dashboard blocks, keyed by `DashboardBinding::uuid()`.
    pub live_values: HashMap<String, crate::live_values::LiveValueEntry>,

    /// Live values for visible blocks, keyed by block SID or fallback key.
    pub live_block_values: HashMap<String, crate::live_values::LiveValueEntry>,

    /// Live tooltips for visible blocks, keyed by block SID or fallback key.
    pub live_block_tooltips: HashMap<String, Vec<LiveTooltipEntry>>,

    /// Live tooltips for visible lines, keyed by line index in the current subsystem.
    pub live_line_tooltips: HashMap<usize, Vec<LiveTooltipEntry>>,

    /// Default live-value display options used when no per-value override is provided.
    pub live_display_defaults: crate::live_values::LiveValueDisplayOptions,

    /// Default path used to save/load viewer layout overrides.
    pub layout_file_path: Option<Utf8PathBuf>,

    /// Whether the in-memory layout differs from the last loaded/saved layout.
    pub layout_dirty: bool,

    /// Persistent model-space bounds used for viewer auto-fit.
    ///
    /// This avoids recomputing the fit from edited block positions every frame,
    /// which would otherwise make moved/resized blocks appear to snap back.
    pub view_bounds: Option<egui::Rect>,

    /// Per-subsystem persisted navigation state.
    pub navigation_view_states: BTreeMap<String, NavigationViewState>,

    /// Authored default navigation state loaded from the source model.
    pub default_navigation_view_states: BTreeMap<String, NavigationViewState>,

    /// Optional default zoom factors per subsystem path (for initial view only).
    pub default_zoom_by_path: BTreeMap<String, f32>,

    /// Active move/resize gesture in viewer move mode.
    pub viewer_drag_state: ViewerDragState,

    /// Cached per-frame computations (line colors, port info) that are
    /// recomputed only when the model changes.
    pub view_cache: ComputedViewCache,

    /// Undo/redo history for viewer layout editing operations.
    pub viewer_history: EditorHistory,

    /// Per-block `MiniScope` instances for interactive liveplot rendering.
    ///
    /// Keyed by a stable block identifier (SID or name). Scope instances are
    /// lazily created the first time a Scope/DashboardScope block is rendered.
    #[cfg(feature = "dashboard")]
    pub scope_instances:
        Arc<std::sync::Mutex<std::collections::HashMap<String, super::scope_widget::MiniScope>>>,

    /// Scope popup window state.  When set, an `egui::Window` is opened
    /// showing a full-size liveplot for the given scope block.
    #[cfg(feature = "dashboard")]
    pub scope_popout: Option<ScopePopout>,

    /// Per-block editable value overrides for Constant blocks.
    ///
    /// Keyed by block SID. When the user edits a Constant block's value in
    /// the viewer, the edited text is stored here. If a block's SID is not
    /// present, the original `block.value` is used.
    #[cfg(feature = "dashboard")]
    pub constant_edits: std::collections::HashMap<String, String>,

    /// Per-block edit buffers for live dashboard text entry widgets.
    #[cfg(feature = "dashboard")]
    pub dashboard_edit_buffers: std::collections::HashMap<String, String>,

    /// Buttons currently held down for pulse-style controls.
    #[cfg(feature = "dashboard")]
    pub dashboard_active_pulses: BTreeSet<String>,

    /// Pending live dashboard control update for the host application.
    #[cfg(feature = "dashboard")]
    pub pending_dashboard_control: Option<DashboardControlEvent>,
}

impl SubsystemApp {
    /// Create a new app showing the provided `root` system.
    pub fn new(
        root: System,
        initial_path: Vec<String>,
        charts: BTreeMap<u32, Chart>,
        chart_map: BTreeMap<String, u32>,
    ) -> Self {
        let all = collect_subsystems_paths(&root);
        let original_root = root.clone();
        let default_navigation_view_states = authored_navigation_view_states(&root);
        let mut app = Self {
            instance_id: NEXT_VIEWER_INSTANCE_ID.fetch_add(1, Ordering::Relaxed),
            root,
            original_root,
            source_model_path: None,
            path: initial_path,
            all_subsystems: all,
            search_query: String::new(),
            search_matches: Vec::new(),
            zoom: 1.0,
            pan: Vec2::ZERO,
            reset_view: true,
            chart_view: None,
            charts,
            chart_map,
            signal_view: None,
            block_view: None,
            signal_buttons: Vec::new(),
            block_buttons: Vec::new(),
            signal_menu_items: Vec::new(),
            block_menu_items: Vec::new(),
            transient_notification: None,
            library_search_paths: Vec::new(),
            subsystem_change_listeners: Vec::new(),
            block_click_handler: None,
            show_block_names_default: true,
            monochrome: false,
            block_name_font_factor: 0.4,
            block_name_extend_factor: 3.0,
            block_value_font_factor: 0.8,
            selected_block_sids: BTreeSet::new(),
            selected_line_indices: BTreeSet::new(),
            move_mode_enabled: false,
            add_mode_enabled: false,
            live_mode_enabled: false,
            live_values: HashMap::new(),
            live_block_values: HashMap::new(),
            live_block_tooltips: HashMap::new(),
            live_line_tooltips: HashMap::new(),
            live_display_defaults: crate::live_values::LiveValueDisplayOptions {
                float_decimals: crate::live_values::DEFAULT_LIVE_FLOAT_DECIMALS,
                scientific_lower_bound: crate::live_values::LIVE_SCIENTIFIC_LOWER_BOUND,
                scientific_upper_bound: crate::live_values::LIVE_SCIENTIFIC_UPPER_BOUND,
                always_scientific: false,
            },
            layout_file_path: None,
            layout_dirty: false,
            view_bounds: None,
            navigation_view_states: BTreeMap::new(),
            default_navigation_view_states,
            default_zoom_by_path: BTreeMap::new(),
            viewer_drag_state: ViewerDragState::None,
            view_cache: ComputedViewCache::default(),
            viewer_history: EditorHistory::new(200),
            #[cfg(feature = "dashboard")]
            scope_instances: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            #[cfg(feature = "dashboard")]
            scope_popout: None,
            #[cfg(feature = "dashboard")]
            constant_edits: std::collections::HashMap::new(),
            #[cfg(feature = "dashboard")]
            dashboard_edit_buffers: std::collections::HashMap::new(),
            #[cfg(feature = "dashboard")]
            dashboard_active_pulses: BTreeSet::new(),
            #[cfg(feature = "dashboard")]
            pending_dashboard_control: None,
        };
        if app
            .default_navigation_view_states
            .contains_key(&app.current_path_key())
        {
            app.apply_navigation_view_state_for_current_path();
        }
        app
    }

    /// Return a borrowed view of entities (blocks, lines, annotations) in the current subsystem.
    ///
    /// This returns references into the model tree, avoiding expensive per-frame
    /// clones of all blocks, lines, and annotations.
    pub fn current_entities(&self) -> Option<SubsystemEntities<'_>> {
        let sys = self.current_system()?;
        let annotations = sys
            .annotations
            .iter()
            .chain(sys.blocks.iter().flat_map(|b| b.annotations.iter()))
            .collect();
        Some(SubsystemEntities {
            blocks: &sys.blocks,
            lines: &sys.lines,
            annotations,
        })
    }

    /// Register a listener to be called whenever the displayed subsystem changes.
    /// The callback receives the new path (relative to root) and an entity snapshot.
    pub fn add_subsystem_change_listener<F>(&mut self, f: F)
    where
        F: for<'a> Fn(&'a [String], &'a SubsystemEntities<'a>) + Send + Sync + 'static,
    {
        self.subsystem_change_listeners.push(Arc::new(f));
    }

    /// Manually emit a subsystem-changed event for the current selection.
    /// Useful right after registering listeners to get an initial snapshot.
    pub fn emit_subsystem_changed(&self) {
        if let Some(entities) = self.current_entities() {
            for cb in &self.subsystem_change_listeners {
                cb(&self.path, &entities);
            }
        }
    }

    /// Show a short-lived in-GUI notification message (milliseconds).
    pub fn show_notification(&mut self, msg: impl Into<String>, duration_ms: u64) {
        let expiry = std::time::Instant::now() + std::time::Duration::from_millis(duration_ms);
        self.transient_notification = Some((msg.into(), expiry));
    }

    /// Clear the transient notification immediately.
    pub fn clear_notification(&mut self) {
        self.transient_notification = None;
    }

    /// Queue a live dashboard control event for the host application.
    #[cfg(feature = "dashboard")]
    pub fn queue_dashboard_control(&mut self, block: Block, value: DashboardControlValue) {
        if let Some(binding) = block.dashboard_binding.as_ref() {
            let preview_value = match value {
                DashboardControlValue::Scalar(value) => Some(value),
                DashboardControlValue::Bool(value) => Some(if value { 1.0 } else { 0.0 }),
                DashboardControlValue::PulseHigh => Some(1.0),
                DashboardControlValue::PulseLow => Some(0.0),
            };
            if let Some(preview_value) = preview_value {
                self.live_values.insert(
                    binding.uuid().to_string(),
                    crate::live_values::LiveValueEntry::new(crate::live_values::LiveValue::new(
                        vec![1],
                        crate::live_values::LiveValueList::Float64(vec![preview_value]),
                    ))
                    .with_display(self.live_display_defaults.clone()),
                );
            }
        }
        self.pending_dashboard_control = Some(DashboardControlEvent { block, value });
    }

    /// Take the latest queued dashboard control event, if any.
    #[cfg(feature = "dashboard")]
    pub fn take_dashboard_control(&mut self) -> Option<DashboardControlEvent> {
        self.pending_dashboard_control.take()
    }

    fn notify_subsystem_changed(&self) {
        self.emit_subsystem_changed();
    }

    /// Override the default block click action. If set, the handler is called on each
    /// block click; return true to consume the event and skip the default action.
    pub fn set_block_click_handler<F>(&mut self, f: F)
    where
        F: Fn(&mut SubsystemApp, &Block) -> bool + Send + Sync + 'static,
    {
        self.block_click_handler = Some(Arc::new(f));
    }

    /// Restore the default block click behavior.
    pub fn clear_block_click_handler(&mut self) {
        self.block_click_handler = None;
    }

    pub fn egui_id(&self, key: impl std::hash::Hash + std::fmt::Debug) -> egui::Id {
        egui::Id::new(("rustylink_viewer", self.instance_id, key))
    }

    #[cfg(feature = "dashboard")]
    pub fn embedded_scope_storage_key(&self, scope_key: &str) -> String {
        format!("embedded::{scope_key}")
    }

    #[cfg(feature = "dashboard")]
    pub fn popout_scope_storage_key(&self, scope_key: &str) -> String {
        format!("popout::{scope_key}")
    }

    #[cfg(feature = "dashboard")]
    pub fn scope_key_for_block(&self, block: &Block) -> String {
        block
            .sid
            .clone()
            .unwrap_or_else(|| format!("__scope_{}", block.name))
    }

    pub fn live_value_key_for_block(&self, block: &Block) -> String {
        block.sid.clone().unwrap_or_else(|| {
            if self.path.is_empty() {
                format!("__block_{}", block.name)
            } else {
                format!("__block_{}/{}", self.path.join("/"), block.name)
            }
        })
    }

    /// Register a custom button in the signal dialog.
    pub fn add_signal_dialog_button<F, G>(
        &mut self,
        label: impl Into<String>,
        filter: F,
        on_click: G,
    ) where
        F: Fn(&crate::model::Line) -> bool + Send + Sync + 'static,
        G: Fn(&crate::model::Line) + Send + Sync + 'static,
    {
        self.signal_buttons.push(SignalDialogButton {
            label: label.into(),
            filter: Arc::new(filter),
            on_click: Arc::new(on_click),
        });
    }

    /// Register a custom button in the block dialog.
    pub fn add_block_dialog_button<F, G>(
        &mut self,
        label: impl Into<String>,
        filter: F,
        on_click: G,
    ) where
        F: Fn(&Block) -> bool + Send + Sync + 'static,
        G: Fn(&Block) + Send + Sync + 'static,
    {
        self.block_buttons.push(BlockDialogButton {
            label: label.into(),
            filter: Arc::new(filter),
            on_click: Arc::new(on_click),
        });
    }

    /// Register a custom context menu item for signals.
    pub fn add_signal_context_menu_item<F, G>(
        &mut self,
        label: impl Into<String>,
        filter: F,
        on_click: G,
    ) where
        F: Fn(&crate::model::Line) -> bool + Send + Sync + 'static,
        G: Fn(&crate::model::Line) + Send + Sync + 'static,
    {
        self.signal_menu_items.push(SignalContextMenuItem {
            label: label.into(),
            filter: Arc::new(filter),
            on_click: Arc::new(on_click),
        });
    }

    /// Register a custom context menu item for blocks.
    pub fn add_block_context_menu_item<F, G>(
        &mut self,
        label: impl Into<String>,
        filter: F,
        on_click: G,
    ) where
        F: Fn(&Block) -> bool + Send + Sync + 'static,
        G: Fn(&Block) + Send + Sync + 'static,
    {
        self.block_menu_items.push(BlockContextMenuItem {
            label: label.into(),
            filter: Arc::new(filter),
            on_click: Arc::new(on_click),
        });
    }

    /// Get the current subsystem based on `self.path`.
    pub fn current_system(&self) -> Option<&System> {
        resolve_subsystem_by_vec(&self.root, &self.path)
    }

    /// Get the current subsystem mutably based on `self.path`.
    pub fn current_system_mut(&mut self) -> Option<&mut System> {
        resolve_subsystem_by_vec_mut(&mut self.root, &self.path)
    }

    /// Returns the cached connection-target resolver, or `None` while it is
    /// being built in the background.
    ///
    /// Callers should handle `None` gracefully (skip target resolution for
    /// that frame).  The UI layer uses [`ComputedViewCache::resolver_status`]
    /// to show a progress bar while the build is in progress.
    pub fn connection_target_resolver(
        &self,
    ) -> Option<Arc<crate::connection_targets::ConnectionTargetResolver>> {
        self.view_cache.connection_target_resolver.clone()
    }

    /// Configure the default layout file path from the original model path.
    pub fn set_layout_source_path(&mut self, source_path: impl Into<Utf8PathBuf>) {
        let source_path = source_path.into();
        self.source_model_path = Some(source_path.clone());
        self.layout_file_path = Some(Utf8PathBuf::from(format!(
            "{}.rustylink-layout.json",
            source_path
        )));
    }

    fn replace_root_state(
        &mut self,
        root: System,
        charts: BTreeMap<u32, Chart>,
        chart_map: BTreeMap<String, u32>,
    ) {
        self.root = root;
        self.default_navigation_view_states = authored_navigation_view_states(&self.root);
        self.charts = charts;
        self.chart_map = chart_map;
        self.all_subsystems = collect_subsystems_paths(&self.root);
        if resolve_subsystem_by_vec(&self.root, &self.path).is_none() {
            self.path.clear();
        }
        self.navigation_view_states.clear();
        self.reset_navigation_view_state();
        if self
            .default_navigation_view_states
            .contains_key(&self.current_path_key())
            || self
                .default_zoom_by_path
                .contains_key(&self.current_path_key())
        {
            self.apply_navigation_view_state_for_current_path();
        }
        self.layout_dirty = false;
        self.view_cache.invalidate();
    }

    fn reset_navigation_view_state(&mut self) {
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
        self.reset_view = true;
        self.view_bounds = None;
        self.selected_block_sids.clear();
        self.selected_line_indices.clear();
        self.live_line_tooltips.clear();
        self.viewer_drag_state = ViewerDragState::None;
        self.viewer_history.clear();
    }

    fn finish_navigation_change(&mut self) {
        self.selected_block_sids.clear();
        self.selected_line_indices.clear();
        self.live_line_tooltips.clear();
        self.viewer_drag_state = ViewerDragState::None;
        self.viewer_history.clear();
        self.apply_navigation_view_state_for_current_path();
        self.notify_subsystem_changed();
    }

    fn path_key(path: &[String]) -> String {
        if path.is_empty() {
            "<root>".to_string()
        } else {
            path.join("/")
        }
    }

    pub fn apply_authored_navigation_view_state_for_current_path(&mut self) -> bool {
        let key = self.current_path_key();

        if let Some(state) = self.default_navigation_view_states.get(&key) {
            let (zoom, pan, view_bounds) = state.to_runtime();
            self.zoom = zoom;
            self.pan = pan;
            self.view_bounds = view_bounds;
            self.reset_view = false;
            return true;
        }

        if let Some(zoom) = self.default_zoom_by_path.get(&key).copied() {
            self.zoom = zoom;
            self.pan = Vec2::ZERO;
            self.view_bounds = None;
            self.reset_view = false;
            return true;
        }

        false
    }

    fn current_path_key(&self) -> String {
        Self::path_key(&self.path)
    }

    fn apply_navigation_view_state_for_current_path(&mut self) {
        let key = self.current_path_key();
        if let Some(state) = self.navigation_view_states.get(&key) {
            let (zoom, pan, view_bounds) = state.to_runtime();
            self.zoom = zoom;
            self.pan = pan;
            self.view_bounds = view_bounds;
            self.reset_view = false;
            return;
        }

        // First visit: trigger fit-to-view reset.  Authored states
        // (ZoomFactor/Location from the model) are not applied automatically;
        // use `apply_authored_navigation_view_state_for_current_path` for that.
        self.pan = Vec2::ZERO;
        self.view_bounds = None;
        self.zoom = self.default_zoom_by_path.get(&key).copied().unwrap_or(1.0);
        self.reset_view = true;
    }

    pub fn remember_current_navigation_view_state(&mut self) {
        let key = self.current_path_key();
        self.navigation_view_states.insert(
            key,
            NavigationViewState::from_runtime(self.zoom, self.pan, self.view_bounds),
        );
    }

    pub fn export_navigation_view_states(&self) -> BTreeMap<String, NavigationViewState> {
        self.navigation_view_states.clone()
    }

    pub fn import_navigation_view_states(&mut self, states: BTreeMap<String, NavigationViewState>) {
        self.navigation_view_states = states;
        self.apply_navigation_view_state_for_current_path();
    }

    pub fn set_default_zoom_by_path(&mut self, defaults: BTreeMap<String, f32>) {
        self.default_zoom_by_path = defaults;
    }

    pub fn set_default_navigation_view_states(
        &mut self,
        defaults: BTreeMap<String, NavigationViewState>,
    ) {
        self.default_navigation_view_states = defaults;
        if !self
            .navigation_view_states
            .contains_key(&self.current_path_key())
        {
            self.apply_navigation_view_state_for_current_path();
        }
    }

    /// Save the current viewer layout to an explicit path and remember it as
    /// the default layout file for later save/load operations.
    pub fn save_layout_to_path(&mut self, path: impl Into<Utf8PathBuf>) -> anyhow::Result<()> {
        let path = path.into();
        let snapshot = LayoutSnapshot {
            version: 1,
            root: self.root.clone(),
        };
        let text = serde_json::to_string_pretty(&snapshot)?;
        std::fs::write(path.as_str(), text)?;
        self.layout_file_path = Some(path);
        self.layout_dirty = false;
        Ok(())
    }

    /// Save the current viewer layout to the configured layout file.
    pub fn save_layout_to_default_path(&mut self) -> anyhow::Result<()> {
        let Some(path) = self.layout_file_path.clone() else {
            anyhow::bail!("No layout file path configured");
        };
        self.save_layout_to_path(path)
    }

    /// Load the viewer layout from the configured layout file.
    pub fn load_layout_from_default_path(&mut self) -> anyhow::Result<()> {
        let Some(path) = self.layout_file_path.clone() else {
            anyhow::bail!("No layout file path configured");
        };
        self.load_layout_from_path(path)
    }

    /// Load the viewer layout from an explicit layout file and remember it.
    pub fn load_layout_from_path(&mut self, path: impl Into<Utf8PathBuf>) -> anyhow::Result<()> {
        let path = path.into();
        let text = std::fs::read_to_string(path.as_str())?;
        let snapshot: LayoutSnapshot = serde_json::from_str(&text)?;
        if snapshot.version != 1 {
            anyhow::bail!("Unsupported layout version {}", snapshot.version);
        }
        self.original_root = snapshot.root.clone();
        self.replace_root_state(snapshot.root, self.charts.clone(), self.chart_map.clone());
        self.layout_file_path = Some(path);
        Ok(())
    }

    /// Restore the root system to its original state (at construction or last load).
    pub fn restore_original_layout(&mut self) -> anyhow::Result<()> {
        if let Some(source_path) = self.source_model_path.clone() {
            let (root, charts, chart_map) = load_source_model(&source_path)?;
            self.original_root = root.clone();
            self.replace_root_state(root, charts, chart_map);
            return Ok(());
        }

        self.replace_root_state(
            self.original_root.clone(),
            self.charts.clone(),
            self.chart_map.clone(),
        );
        Ok(())
    }

    /// Navigate one level up, if possible.
    pub fn go_up(&mut self) {
        if !self.path.is_empty() {
            self.remember_current_navigation_view_state();
            self.path.pop();
            self.finish_navigation_change();
        }
    }

    /// Navigate to the given path, if it resolves.
    pub fn navigate_to_path(&mut self, p: Vec<String>) {
        if resolve_subsystem_by_vec(&self.root, &p).is_some() {
            self.remember_current_navigation_view_state();
            self.path = p;
            self.finish_navigation_change();
        }
    }

    /// If the block is a non-chart subsystem, open it and return true.
    pub fn open_block_if_subsystem(&mut self, b: &Block) -> bool {
        if (b.block_type == "SubSystem" || b.block_type == "Reference")
            && !b.is_matlab_function
            && let Some(sub) = &b.subsystem
            && sub.chart.is_none()
        {
            self.remember_current_navigation_view_state();
            self.path.push(b.name.clone());
            self.finish_navigation_change();
            return true;
        }
        false
    }

    /// Update `search_matches` based on `search_query`.
    pub fn update_search_matches(&mut self) {
        let q = self.search_query.trim();
        if q.is_empty() {
            self.search_matches.clear();
            return;
        }
        let ql = q.to_lowercase(); // Convert search query to lowercase
        let mut m: Vec<Vec<String>> = self
            .all_subsystems
            .iter()
            .filter(|p| {
                p.last()
                    .map(|n| n.to_lowercase().contains(&ql))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        m.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
        m.truncate(30);
        self.search_matches = m;
    }
}

impl eframe::App for SubsystemApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            super::ui::update_with_info(self, ui);
        });
    }
}

/// Resolve a mutable reference to a subsystem by path.
pub(crate) fn resolve_subsystem_by_vec_mut<'a>(
    root: &'a mut System,
    path: &[String],
) -> Option<&'a mut System> {
    if path.is_empty() {
        return Some(root);
    }

    let mut current = root;
    for name in path {
        let block = current
            .blocks
            .iter_mut()
            .find(|b| b.name == *name && b.subsystem.is_some())?;
        current = block.subsystem.as_mut()?;
    }
    Some(current)
}
