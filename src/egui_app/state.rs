#![cfg(feature = "egui")]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use camino::Utf8PathBuf;
use eframe::egui::{self, Vec2};

use crate::editor::operations::EditorHistory;
use crate::model::{Annotation, Block, Chart, Line, System};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct LayoutSnapshot {
    version: u32,
    root: System,
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
    pub block: Block,
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

/// Snapshot of all entities within the currently displayed subsystem.
#[derive(Clone)]
pub struct SubsystemEntities {
    pub blocks: Vec<Block>,
    pub lines: Vec<Line>,
    pub annotations: Vec<Annotation>,
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
    /// The subsystem path for which this cache was computed.
    cached_path: Vec<String>,
    /// Model generation at which the cache was computed.
    cached_gen: u64,
}

impl Default for ComputedViewCache {
    fn default() -> Self {
        Self {
            // Start at 1 so the initial cached_gen=0 never matches: cache always starts invalid.
            generation: 1,
            line_colors: Vec::new(),
            port_counts: std::collections::HashMap::new(),
            connected_ports: std::collections::HashSet::new(),
            cached_path: Vec::new(),
            cached_gen: 0,
        }
    }
}

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
}

/// Interactive Egui application that displays and navigates a Simulink subsystem tree.
#[derive(Clone)]
pub struct SubsystemApp {
    pub root: System,
    /// Snapshot of the root system at construction / last load, used for "Restore layout".
    pub original_root: System,
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
    subsystem_change_listeners: Vec<Arc<dyn Fn(&[String], &SubsystemEntities) + Send + Sync>>, // private to encourage using the API
    /// Optional click handler to override default action when clicking a block.
    /// Return true from the handler to indicate the click was handled and suppress the default behavior.
    pub block_click_handler: Option<Arc<dyn Fn(&mut SubsystemApp, &Block) -> bool + Send + Sync>>,

    /// Global default for showing block names.
    ///
    /// Per-block override: `Block::show_name = Some(true/false)`.
    pub show_block_names_default: bool,

    /// Block-name font size as a factor of the port chevron height.
    ///
    /// A value of ~1.0 makes the text approximately the same height as the chevrons.
    pub block_name_font_factor: f32,

    /// Maximum block-name font size factor relative to block width.
    ///
    /// The actual font size will be bounded so that a typical character is at most
    /// `block_width * block_name_max_char_width_factor` pixels wide.
    pub block_name_max_char_width_factor: f32,

    /// Minimum block-name font size factor relative to port chevron height.
    ///
    /// Used when avoiding collisions with other elements.
    pub block_name_min_font_factor: f32,

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
    pub live_values: HashMap<String, f64>,

    /// Default path used to save/load viewer layout overrides.
    pub layout_file_path: Option<Utf8PathBuf>,

    /// Whether the in-memory layout differs from the last loaded/saved layout.
    pub layout_dirty: bool,

    /// Persistent model-space bounds used for viewer auto-fit.
    ///
    /// This avoids recomputing the fit from edited block positions every frame,
    /// which would otherwise make moved/resized blocks appear to snap back.
    pub view_bounds: Option<egui::Rect>,

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
        Self {
            root,
            original_root,
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
            block_name_font_factor: 0.85,
            block_name_max_char_width_factor: 1.0 / 8.0,
            block_name_min_font_factor: 0.5,
            selected_block_sids: BTreeSet::new(),
            selected_line_indices: BTreeSet::new(),
            move_mode_enabled: false,
            add_mode_enabled: false,
            live_mode_enabled: false,
            live_values: HashMap::new(),
            layout_file_path: None,
            layout_dirty: false,
            view_bounds: None,
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
            pending_dashboard_control: None,
        }
    }

    /// Return a snapshot of entities (blocks, lines, annotations) in the current subsystem.
    pub fn current_entities(&self) -> Option<SubsystemEntities> {
        self.current_system().map(|sys| SubsystemEntities {
            blocks: sys.blocks.clone(),
            lines: sys.lines.clone(),
            annotations: {
                // Combine system-level and block-attached annotations into a single list
                let mut anns = sys.annotations.clone();
                for b in &sys.blocks {
                    anns.extend(b.annotations.clone());
                }
                anns
            },
        })
    }

    /// Register a listener to be called whenever the displayed subsystem changes.
    /// The callback receives the new path (relative to root) and an entity snapshot.
    pub fn add_subsystem_change_listener<F>(&mut self, f: F)
    where
        F: Fn(&[String], &SubsystemEntities) + Send + Sync + 'static,
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

    /// Configure the default layout file path from the original model path.
    pub fn set_layout_source_path(&mut self, source_path: impl Into<Utf8PathBuf>) {
        let source_path = source_path.into();
        self.layout_file_path = Some(Utf8PathBuf::from(format!(
            "{}.rustylink-layout.json",
            source_path
        )));
    }

    /// Save the current viewer layout to the configured layout file.
    pub fn save_layout_to_default_path(&mut self) -> anyhow::Result<()> {
        let Some(path) = self.layout_file_path.clone() else {
            anyhow::bail!("No layout file path configured");
        };
        let snapshot = LayoutSnapshot {
            version: 1,
            root: self.root.clone(),
        };
        let text = serde_json::to_string_pretty(&snapshot)?;
        std::fs::write(path.as_str(), text)?;
        self.layout_dirty = false;
        Ok(())
    }

    /// Load the viewer layout from the configured layout file.
    pub fn load_layout_from_default_path(&mut self) -> anyhow::Result<()> {
        let Some(path) = self.layout_file_path.clone() else {
            anyhow::bail!("No layout file path configured");
        };
        let text = std::fs::read_to_string(path.as_str())?;
        let snapshot: LayoutSnapshot = serde_json::from_str(&text)?;
        if snapshot.version != 1 {
            anyhow::bail!("Unsupported layout version {}", snapshot.version);
        }
        self.root = snapshot.root;
        self.original_root = self.root.clone();
        self.all_subsystems = collect_subsystems_paths(&self.root);
        if resolve_subsystem_by_vec(&self.root, &self.path).is_none() {
            self.path.clear();
        }
        self.reset_view = true;
        self.view_bounds = None;
        self.selected_block_sids.clear();
        self.selected_line_indices.clear();
        self.viewer_drag_state = ViewerDragState::None;
        self.layout_dirty = false;
        self.viewer_history.clear();
        self.notify_subsystem_changed();
        Ok(())
    }

    /// Restore the root system to its original state (at construction or last load).
    pub fn restore_original_layout(&mut self) {
        self.root = self.original_root.clone();
        self.all_subsystems = collect_subsystems_paths(&self.root);
        if resolve_subsystem_by_vec(&self.root, &self.path).is_none() {
            self.path.clear();
        }
        self.reset_view = true;
        self.view_bounds = None;
        self.selected_block_sids.clear();
        self.selected_line_indices.clear();
        self.viewer_drag_state = ViewerDragState::None;
        self.layout_dirty = false;
        self.view_cache.invalidate();
        self.viewer_history.clear();
        self.notify_subsystem_changed();
    }

    /// Navigate one level up, if possible.
    pub fn go_up(&mut self) {
        if !self.path.is_empty() {
            self.path.pop();
            self.reset_view = true;
            self.view_bounds = None;
            self.selected_block_sids.clear();
            self.selected_line_indices.clear();
            self.viewer_drag_state = ViewerDragState::None;
            self.viewer_history.clear();
            self.notify_subsystem_changed();
        }
    }

    /// Navigate to the given path, if it resolves.
    pub fn navigate_to_path(&mut self, p: Vec<String>) {
        if resolve_subsystem_by_vec(&self.root, &p).is_some() {
            self.path = p;
            self.reset_view = true;
            self.view_bounds = None;
            self.selected_block_sids.clear();
            self.selected_line_indices.clear();
            self.viewer_drag_state = ViewerDragState::None;
            self.viewer_history.clear();
            self.notify_subsystem_changed();
        }
    }

    /// If the block is a non-chart subsystem, open it and return true.
    pub fn open_block_if_subsystem(&mut self, b: &Block) -> bool {
        if b.block_type == "SubSystem" || b.block_type == "Reference" {
            if let Some(sub) = &b.subsystem {
                if sub.chart.is_none() {
                    self.path.push(b.name.clone());
                    self.reset_view = true;
                    self.view_bounds = None;
                    self.selected_block_sids.clear();
                    self.selected_line_indices.clear();
                    self.viewer_drag_state = ViewerDragState::None;
                    self.viewer_history.clear();
                    self.notify_subsystem_changed();
                    return true;
                }
            }
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_starts_invalid() {
        let cache = ComputedViewCache::default();
        // Cache should always start invalid (generation=1, cached_gen=0).
        assert!(!cache.is_valid(&[], cache.generation));
        assert!(!cache.is_valid(&["Root".to_string()], cache.generation));
    }

    #[test]
    fn cache_valid_after_mark() {
        let mut cache = ComputedViewCache::default();
        let path = vec!["Root".to_string()];
        cache.mark_valid(&path, cache.generation);
        assert!(cache.is_valid(&path, cache.generation));
    }

    #[test]
    fn cache_invalid_after_invalidate() {
        let mut cache = ComputedViewCache::default();
        let path = vec!["Root".to_string()];
        cache.mark_valid(&path, cache.generation);
        assert!(cache.is_valid(&path, cache.generation));
        cache.invalidate();
        // Generation bumped, so old gen no longer matches
        assert!(!cache.is_valid(&path, cache.generation));
    }

    #[test]
    fn cache_invalid_on_path_change() {
        let mut cache = ComputedViewCache::default();
        let path1 = vec!["Root".to_string()];
        let path2 = vec!["Root".to_string(), "Sub".to_string()];
        cache.mark_valid(&path1, cache.generation);
        assert!(cache.is_valid(&path1, cache.generation));
        assert!(!cache.is_valid(&path2, cache.generation));
    }

    #[test]
    fn cache_revalidates_after_invalidate() {
        let mut cache = ComputedViewCache::default();
        let path = vec!["Root".to_string()];
        cache.mark_valid(&path, cache.generation);
        cache.invalidate();
        assert!(!cache.is_valid(&path, cache.generation));
        // Mark valid again at new generation
        cache.mark_valid(&path, cache.generation);
        assert!(cache.is_valid(&path, cache.generation));
    }
}
