#![cfg(feature = "egui")]

use std::collections::BTreeMap;
use std::sync::Arc;

use camino::Utf8PathBuf;
use eframe::egui::{self, Vec2};

use crate::model::{Annotation, Block, Chart, Line, System};

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

/// Interactive Egui application that displays and navigates a Simulink subsystem tree.
#[derive(Clone)]
pub struct SubsystemApp {
    pub root: System,
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
        Self {
            root,
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

    /// Navigate one level up, if possible.
    pub fn go_up(&mut self) {
        if !self.path.is_empty() {
            self.path.pop();
            self.reset_view = true;
            self.notify_subsystem_changed();
        }
    }

    /// Navigate to the given path, if it resolves.
    pub fn navigate_to_path(&mut self, p: Vec<String>) {
        if resolve_subsystem_by_vec(&self.root, &p).is_some() {
            self.path = p;
            self.reset_view = true;
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
