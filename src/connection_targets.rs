use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use crate::model::{
    Block, Branch, DashboardBinding, DashboardTargetPath, EndpointRef, Line, System,
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum ConnectionTargetOrigin {
    #[default]
    SourceBlock,
    SelfBlock,
    Internal,
    DashboardBinding,
    BusCreator,
    BusSelector,
    Mux,
    Demux,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ConnectionTargetResolve {
    Signal(String),
    Index(u32),
    TargetPath(DashboardTargetPath),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct ConnectionTarget {
    pub path: String,
    pub signal_name: Option<String>,
    /// Every signal name this target carries along the traced signal line,
    /// including names picked up crossing subsystem In/Outport boundaries and
    /// passing through Bus/Mux/Demux blocks. Order-preserving and deduplicated;
    /// `signal_name` (when set) is always included as the primary alias.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signal_names: Vec<String>,
    pub resolve: Option<ConnectionTargetResolve>,
    pub element_index: Option<u32>,
    pub origin: ConnectionTargetOrigin,
    pub signals_only: bool,
    pub testpoint: bool,
    /// The Simulink block type that produced this target (e.g. "Reference",
    /// "SubSystem", etc.). When the block type is "Reference", the matcher
    /// may use prefix matching instead of exact whole-path matching.
    pub block_type: Option<String>,
}

impl ConnectionTarget {
    pub fn new(path: String, origin: ConnectionTargetOrigin) -> Self {
        Self {
            path,
            origin,
            ..Self::default()
        }
    }
}

/// How many times the root-level resolution loop re-resolves the entire
/// tree, waiting for cross-boundary signal targets to settle.  Each pass
/// propagates information one subsystem level, so 32 passes handles models
/// up to ~16 levels deep with multi-hop sibling chains.
pub const MAX_GLOBAL_RESOLVE_PASSES: usize = 32;

/// Tracks progress of `ConnectionTargetResolver` construction for background
/// builds.  `tick()` is called on every `resolve_system` visit; the shared
/// atomics let the UI thread read live progress without locking.
struct ProgressTracker {
    /// Total `resolve_system` calls across all root-level passes.
    counter: AtomicUsize,
    /// Shared with the UI thread: 0..=1000 (0.0%..=100.0% for the bar fill).
    progress: Arc<AtomicU32>,
    /// Shared with the UI thread: total subsystems visited so far.
    visited: Arc<AtomicUsize>,
    /// Estimated total work: `estimated_passes × total_subsystems`.
    total: usize,
}

impl ProgressTracker {
    fn new(progress: Arc<AtomicU32>, visited: Arc<AtomicUsize>, total: usize) -> Self {
        Self {
            counter: AtomicUsize::new(0),
            progress,
            visited,
            total,
        }
    }
    fn tick(&self) {
        let count = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
        self.visited.store(count, Ordering::Relaxed);
        let p = ((count as u32 * 1000) / self.total.max(1) as u32).min(1000);
        self.progress.store(p, Ordering::Relaxed);
    }
}

/// Count the total number of `System` nodes in the tree (root + all nested
/// subsystems).  Used to estimate progress for the background build.
pub fn count_subsystems(system: &System) -> usize {
    1 + system
        .blocks
        .iter()
        .filter_map(|b| b.subsystem.as_ref())
        .map(|sub| count_subsystems(sub))
        .sum::<usize>()
}

/// Maximum nesting depth of subsystems in the tree (root = 1).
pub fn max_subsystem_depth(system: &System) -> usize {
    1 + system
        .blocks
        .iter()
        .filter_map(|b| b.subsystem.as_ref())
        .map(|sub| max_subsystem_depth(sub))
        .max()
        .unwrap_or(0)
}

#[derive(Debug, Clone, Default, PartialEq)]
struct ParentSubsystemContext {
    incoming_by_port: BTreeMap<u32, Vec<ConnectionTarget>>,
    outgoing_by_port: BTreeMap<u32, Vec<ConnectionTarget>>,
}

#[derive(Debug, Clone, Default)]
struct ChildSubsystemSummary {
    incoming_by_port: BTreeMap<u32, Vec<ConnectionTarget>>,
    outgoing_by_port: BTreeMap<u32, Vec<ConnectionTarget>>,
}

#[derive(Debug, Clone, Default)]
pub struct ConnectionTargetResolver {
    block_targets: HashMap<String, Vec<ConnectionTarget>>,
    line_targets: HashMap<String, Vec<ConnectionTarget>>,
    model_name: String,
    /// Child subsystem summaries keyed by block SID, persisted across root-level
    /// resolution passes so that information from a previous pass can seed the
    /// next one without re-resolving every child at every recursion level.
    child_summaries: HashMap<String, ChildSubsystemSummary>,
}

impl ConnectionTargetResolver {
    pub fn new(root: &System) -> Self {
        let mut resolver = Self {
            block_targets: HashMap::new(),
            line_targets: HashMap::new(),
            model_name: root.properties.get("Name").cloned().unwrap_or_default(),
            child_summaries: HashMap::new(),
        };
        let empty_path: Vec<String> = Vec::new();
        // Re-resolve the entire tree until the cached targets stop changing.
        // Each pass propagates cross-boundary signal information one subsystem
        // level, so this converges in O(depth) passes instead of the
        // exponential 8^depth that a per-recursion-level loop would cost.
        for _ in 0..MAX_GLOBAL_RESOLVE_PASSES {
            let prev_block = resolver.block_targets.clone();
            let prev_line = resolver.line_targets.clone();
            resolver.resolve_system(root, &empty_path, None, None);
            if resolver.block_targets == prev_block && resolver.line_targets == prev_line {
                break;
            }
        }
        resolver
    }

    /// Like [`new`](Self::new) but reports build progress through shared atomics.
    ///
    /// `progress` is written as 0..=1000 (0.0%..=100.0%) and `visited` as the
    /// total number of `resolve_system` calls so far.  Both use
    /// `Ordering::Relaxed` so the UI thread can read them without locking.
    pub fn new_with_progress(
        root: &System,
        progress: Arc<AtomicU32>,
        visited: Arc<AtomicUsize>,
    ) -> Self {
        let total_subsystems = count_subsystems(root).max(1);
        let max_depth = max_subsystem_depth(root);
        let estimated_passes = MAX_GLOBAL_RESOLVE_PASSES.min(max_depth + 2).max(1);
        let total_work = estimated_passes * total_subsystems;
        let tracker = ProgressTracker::new(progress, visited, total_work);

        let mut resolver = Self {
            block_targets: HashMap::new(),
            line_targets: HashMap::new(),
            model_name: root.properties.get("Name").cloned().unwrap_or_default(),
            child_summaries: HashMap::new(),
        };
        let empty_path: Vec<String> = Vec::new();
        for _ in 0..MAX_GLOBAL_RESOLVE_PASSES {
            let prev_block = resolver.block_targets.clone();
            let prev_line = resolver.line_targets.clone();
            resolver.resolve_system(root, &empty_path, None, Some(&tracker));
            if resolver.block_targets == prev_block && resolver.line_targets == prev_line {
                break;
            }
        }
        // Ensure progress shows 100% when done.
        tracker.progress.store(1000, Ordering::Relaxed);
        tracker.visited.store(total_work, Ordering::Relaxed);
        resolver
    }

    pub fn block_targets_for_block(
        &self,
        system_path: &[String],
        block: &Block,
    ) -> Vec<ConnectionTarget> {
        let key = block_cache_key(system_path, block);
        self.block_targets.get(&key).cloned().unwrap_or_default()
    }

    /// Like `block_targets_for_block` but returns a reference to avoid cloning.
    pub fn block_targets_for_block_ref(
        &self,
        system_path: &[String],
        block: &Block,
    ) -> &[ConnectionTarget] {
        let key = block_cache_key(system_path, block);
        self.block_targets
            .get(&key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn line_targets_for_line(
        &self,
        system_path: &[String],
        line: &Line,
    ) -> Vec<ConnectionTarget> {
        let key = line_cache_key(system_path, line);
        self.line_targets.get(&key).cloned().unwrap_or_default()
    }

    /// Like `line_targets_for_line` but returns a reference to avoid cloning.
    pub fn line_targets_for_line_ref(
        &self,
        system_path: &[String],
        line: &Line,
    ) -> &[ConnectionTarget] {
        let key = line_cache_key(system_path, line);
        self.line_targets
            .get(&key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn resolve_system(
        &mut self,
        system: &System,
        system_path: &[String],
        parent_ctx: Option<&ParentSubsystemContext>,
        progress: Option<&ProgressTracker>,
    ) -> ChildSubsystemSummary {
        if let Some(p) = progress {
            p.tick();
        }
        let block_lookup = build_block_lookup(system);
        let mut line_targets: Vec<Vec<ConnectionTarget>> = system
            .lines
            .iter()
            .map(|line| self.base_line_targets(system, system_path, &block_lookup, line))
            .collect();

        // Initial propagation with child summaries from the previous root-level
        // pass.  On the first pass this is empty, so lines fall back to their
        // base targets; subsequent passes see the summaries computed below.
        self.propagate_line_targets(
            system,
            system_path,
            &block_lookup,
            parent_ctx,
            &self.child_summaries,
            &mut line_targets,
        );
        self.propagate_line_metadata_upward(
            system,
            system_path,
            &block_lookup,
            parent_ctx,
            &self.child_summaries,
            &mut line_targets,
        );

        // Resolve each child subsystem once.  The root-level loop in `new`
        // re-invokes `resolve_system` until the cached targets converge, which
        // is what lets cross-boundary signal information propagate across
        // sibling subsystems without an exponential per-level loop here.
        for block in &system.blocks {
            if let Some(subsystem) = &block.subsystem {
                let child_ctx = ParentSubsystemContext {
                    incoming_by_port: incoming_targets_by_port(system, block, &line_targets),
                    outgoing_by_port: outgoing_targets_by_port(system, block, &line_targets),
                };
                let child_path = child_system_path(system_path, &block.name);
                let summary =
                    self.resolve_system(subsystem, &child_path, Some(&child_ctx), progress);
                if let Some(sid) = &block.sid {
                    self.child_summaries.insert(sid.clone(), summary);
                }
            }
        }

        // Final propagation with the freshly computed child summaries.
        self.propagate_line_targets(
            system,
            system_path,
            &block_lookup,
            parent_ctx,
            &self.child_summaries,
            &mut line_targets,
        );
        self.propagate_line_metadata_upward(
            system,
            system_path,
            &block_lookup,
            parent_ctx,
            &self.child_summaries,
            &mut line_targets,
        );

        for (line, targets) in system.lines.iter().zip(line_targets.iter()) {
            self.line_targets.insert(
                line_cache_key(system_path, line),
                dedup_targets(targets.clone()),
            );
        }

        for block in &system.blocks {
            let mut targets = Vec::new();
            targets.push(ConnectionTarget::new(
                self.full_block_path(system_path, &block.name),
                ConnectionTargetOrigin::SelfBlock,
            ));

            targets.extend(self.direct_internal_block_targets(system_path, block));

            for incoming in incoming_lines_for_block(system, block) {
                if let Some(index) = system
                    .lines
                    .iter()
                    .position(|candidate| same_line(candidate, incoming))
                {
                    targets.extend(line_targets[index].clone());
                }
            }

            // Tag every target that belongs to this block with its block type so
            // downstream matchers (e.g. ASX Simulink plugin) can decide whether
            // to use exact or prefix path matching.
            let block_type = block.block_type.clone();
            for target in &mut targets {
                target.block_type = Some(block_type.clone());
            }

            if let Some(binding) = &block.dashboard_binding {
                let target_path =
                    qualify_external_path(&self.model_name, dashboard_binding_block_path(binding));
                let mut target =
                    ConnectionTarget::new(target_path, ConnectionTargetOrigin::DashboardBinding);
                target.block_type = Some(block.block_type.clone());
                if let DashboardBinding::SignalSpec { signal_name, .. } = binding {
                    set_signal_name_only(&mut target, Some(signal_name.clone()));
                }
                target.signals_only = matches!(binding, DashboardBinding::SignalSpec { .. });
                if let DashboardBinding::SignalSpec { target_path, .. } = binding {
                    target.element_index = target_path.port_index;
                }
                let binding_target_path = dashboard_binding_target_path(binding);
                if !binding_target_path.is_empty() {
                    target.resolve = Some(ConnectionTargetResolve::TargetPath(
                        binding_target_path.clone(),
                    ));
                }
                targets.push(target);
            }

            let deduped = dedup_targets(targets);
            self.block_targets
                .insert(block_cache_key(system_path, block), deduped);
        }

        ChildSubsystemSummary {
            incoming_by_port: child_incoming_targets_by_port(system, &line_targets),
            outgoing_by_port: child_outgoing_targets_by_port(
                self,
                system,
                system_path,
                &line_targets,
            ),
        }
    }

    fn propagate_line_targets(
        &self,
        system: &System,
        system_path: &[String],
        block_lookup: &HashMap<&str, &Block>,
        parent_ctx: Option<&ParentSubsystemContext>,
        child_summaries: &HashMap<String, ChildSubsystemSummary>,
        line_targets: &mut [Vec<ConnectionTarget>],
    ) {
        for _ in 0..8 {
            let mut changed = false;
            for (index, line) in system.lines.iter().enumerate() {
                let Some(src) = &line.src else {
                    continue;
                };
                let Some(block) = block_lookup.get(src.sid.as_str()).copied() else {
                    continue;
                };

                let mut new_targets = match block.block_type.as_str() {
                    "BusCreator" => {
                        self.bus_creator_targets(system, system_path, block, line, line_targets)
                    }
                    "BusSelector" => self.bus_selector_targets(system, block, line, line_targets),
                    "BusAssignment" => {
                        self.bus_assignment_targets(system, block, line, line_targets)
                    }
                    "Mux" => self.mux_targets(system, block, line_targets),
                    "Demux" => self.demux_targets(system, block, src.port_index, line_targets),
                    "Inport" => parent_ctx
                        .and_then(|ctx| ctx.incoming_by_port.get(&boundary_port_index(block)))
                        .cloned()
                        .unwrap_or_else(|| {
                            self.base_line_targets(system, system_path, block_lookup, line)
                        }),
                    "SubSystem" | "Reference" => child_summaries
                        .get(&src.sid)
                        .and_then(|summary| summary.outgoing_by_port.get(&src.port_index))
                        .map(|targets| {
                            let mut propagated = targets.clone();
                            // When the signal originates from a Reference block,
                            // tag the propagated targets so downstream matchers
                            // know to use prefix path matching.
                            if block.block_type == "Reference" {
                                for t in &mut propagated {
                                    t.block_type = Some("Reference".to_string());
                                }
                            }
                            propagated
                        })
                        .unwrap_or_else(|| {
                            self.base_line_targets(system, system_path, block_lookup, line)
                        }),
                    "From" => self.resolve_from_block_targets(system, block, line_targets),
                    _ => self.base_line_targets(system, system_path, block_lookup, line),
                };

                if matches!(
                    block.block_type.as_str(),
                    "BusCreator"
                        | "BusSelector"
                        | "BusAssignment"
                        | "Mux"
                        | "Demux"
                        | "Inport"
                        | "SubSystem"
                        | "Reference"
                        | "From"
                ) {
                    apply_local_line_metadata(line, &mut new_targets);
                    apply_source_port_testpoint(block, line, &mut new_targets);
                }

                let deduped = dedup_targets(new_targets);
                if deduped != line_targets[index] {
                    line_targets[index] = deduped;
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }
    }

    fn propagate_line_metadata_upward(
        &self,
        system: &System,
        system_path: &[String],
        block_lookup: &HashMap<&str, &Block>,
        parent_ctx: Option<&ParentSubsystemContext>,
        child_summaries: &HashMap<String, ChildSubsystemSummary>,
        line_targets: &mut [Vec<ConnectionTarget>],
    ) {
        for _ in 0..8 {
            let mut changed = false;

            for (index, line) in system.lines.iter().enumerate() {
                // A branched line ends at several blocks at once, and each of
                // them can hand metadata back upstream.
                let mut propagated = Vec::new();
                let mut crosses_boundary = false;
                for dst in line_destination_endpoints(line) {
                    let Some(block) = block_lookup.get(dst.sid.as_str()).copied() else {
                        continue;
                    };
                    propagated.extend(self.upstream_propagated_targets(
                        system,
                        system_path,
                        block,
                        dst,
                        parent_ctx,
                        child_summaries,
                        line_targets,
                    ));
                    crosses_boundary |= matches!(
                        block.block_type.as_str(),
                        "SubSystem" | "Reference" | "Outport"
                    );
                }
                if propagated.is_empty() {
                    continue;
                }

                let merged = merge_upstream_metadata(
                    line,
                    &line_targets[index],
                    &propagated,
                    crosses_boundary,
                );
                if merged != line_targets[index] {
                    line_targets[index] = merged;
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }
    }

    fn base_line_targets(
        &self,
        system: &System,
        system_path: &[String],
        block_lookup: &HashMap<&str, &Block>,
        line: &Line,
    ) -> Vec<ConnectionTarget> {
        let Some(src) = &line.src else {
            return Vec::new();
        };
        let Some(block) = block_lookup.get(src.sid.as_str()).copied() else {
            return Vec::new();
        };

        let signal_name = routing_line_signal_name(system, line);
        let mut target = ConnectionTarget::new(
            self.full_block_path(system_path, &block.name),
            ConnectionTargetOrigin::SourceBlock,
        );
        target.block_type = Some(block.block_type.clone());
        target.signals_only = true;
        target.testpoint =
            port_testpoint(block, src.port_type.as_str(), src.port_index) || line_testpoint(line);
        if src.port_type == "out" && output_port_count(block) > 1 {
            target.element_index = Some(src.port_index);
        }
        set_signal_name_only(&mut target, signal_name);
        apply_line_resolve_hint(line, block_lookup, &mut target);
        vec![target]
    }

    fn bus_creator_targets(
        &self,
        system: &System,
        _system_path: &[String],
        block: &Block,
        _line: &Line,
        line_targets: &[Vec<ConnectionTarget>],
    ) -> Vec<ConnectionTarget> {
        let mut targets = Vec::new();
        for incoming in incoming_lines_for_block(system, block) {
            let Some(line_index) = system
                .lines
                .iter()
                .position(|candidate| same_line(candidate, incoming))
            else {
                continue;
            };
            let signal_name = explicit_line_signal_name(incoming);
            for input_index in input_port_indices(block, incoming) {
                for mut target in line_targets[line_index].clone() {
                    let element_name = signal_name
                        .clone()
                        .or_else(|| target.signal_name.clone())
                        .or_else(|| Some(format!("signal{input_index}")));
                    let next_signal_name = element_name.clone();
                    // Only prepend to the resolve path when the target came
                    // from another bus block (nested bus).  Direct leaf inputs
                    // have their resolve set by `base_line_targets` to the
                    // line name, which is the same as `element_name` —
                    // prepending would double it.
                    let is_from_bus = matches!(
                        target.origin,
                        ConnectionTargetOrigin::BusCreator | ConnectionTargetOrigin::BusSelector
                    );
                    let next_resolve_signal = if is_from_bus {
                        if let Some(existing) =
                            resolve_signal_value(&target.resolve).map(str::to_string)
                        {
                            element_name.map(|en| format!("{en}.{existing}"))
                        } else {
                            element_name
                        }
                    } else {
                        element_name
                    };
                    set_signal_name_only(&mut target, next_signal_name);
                    set_signal_resolve(&mut target, next_resolve_signal);
                    target.origin = ConnectionTargetOrigin::BusCreator;
                    targets.push(target);
                }
            }
        }
        targets
    }

    fn bus_selector_targets(
        &self,
        system: &System,
        block: &Block,
        line: &Line,
        line_targets: &[Vec<ConnectionTarget>],
    ) -> Vec<ConnectionTarget> {
        // Hierarchical path from OutputSignals property (when available).
        let output_path = line
            .src
            .as_ref()
            .and_then(|src| bus_selector_output_path(block, src.port_index));

        // Flat selected name for backward-compat fallback.
        let selected_name = explicit_line_signal_name(line).or_else(|| {
            line.src
                .as_ref()
                .and_then(|src| port_signal_name(block, src.port_type.as_str(), src.port_index))
                .or_else(|| {
                    line.src
                        .as_ref()
                        .map(|src| format!("signal{}", src.port_index))
                })
        });

        let Some(incoming) = incoming_lines_for_block(system, block).into_iter().next() else {
            return Vec::new();
        };
        let Some(line_index) = system
            .lines
            .iter()
            .position(|candidate| same_line(candidate, incoming))
        else {
            return Vec::new();
        };

        line_targets[line_index]
            .iter()
            .filter(|target| {
                if let Some(ref path) = output_path {
                    // Hierarchical matching using OutputSignals.
                    bus_signal_path_matches(target, path)
                } else {
                    // Flat matching (backward compat, no OutputSignals).
                    let name = selected_name.as_deref().unwrap_or("");
                    matches_resolve_signal(target, name)
                        || target
                            .signal_name
                            .as_deref()
                            .is_some_and(|n| signal_keys_match(n, name))
                }
            })
            .cloned()
            .map(|mut target| {
                target.origin = ConnectionTargetOrigin::BusSelector;
                target
            })
            .collect()
    }

    fn bus_assignment_targets(
        &self,
        system: &System,
        block: &Block,
        _line: &Line,
        line_targets: &[Vec<ConnectionTarget>],
    ) -> Vec<ConnectionTarget> {
        // Parse AssignedSignals (comma-separated hierarchical paths).
        let assigned_signals: Vec<String> = block
            .properties
            .get("AssignedSignals")
            .map(|s| {
                s.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        // Targets from the main bus input (in:1).
        let main_targets = self.bus_assignment_input_targets(system, block, 1, line_targets);

        if assigned_signals.is_empty() {
            return main_targets;
        }

        let mut result = Vec::new();

        // Process replacement inputs (in:2, in:3, ...).
        for (i, assigned_path) in assigned_signals.iter().enumerate() {
            let replacement_port = (i + 2) as u32;
            let replacement_targets =
                self.bus_assignment_input_targets(system, block, replacement_port, line_targets);

            let normalized_assigned = normalize_resolve_signal(assigned_path);
            for mut target in replacement_targets {
                if let Some(existing) = resolve_signal_value(&target.resolve).map(str::to_string) {
                    // Sub-bus replacement: prepend assigned path to existing
                    // resolve so leaf identity is preserved.
                    if let Some(ref np) = normalized_assigned {
                        let new_resolve = format!("{np}.{existing}");
                        set_signal_resolve(&mut target, Some(new_resolve));
                    }
                } else {
                    // Leaf replacement: set resolve to the assigned path.
                    set_signal_resolve(&mut target, normalized_assigned.clone());
                }
                result.push(target);
            }
        }

        // Add pass-through targets, excluding those matching assigned signals.
        for target in main_targets {
            let is_assigned = assigned_signals
                .iter()
                .any(|assigned_path| bus_signal_path_matches(&target, assigned_path));
            if !is_assigned {
                result.push(target);
            }
        }

        result
    }

    /// Get the line targets for a specific input port of a block.
    fn bus_assignment_input_targets(
        &self,
        system: &System,
        block: &Block,
        port_index: u32,
        line_targets: &[Vec<ConnectionTarget>],
    ) -> Vec<ConnectionTarget> {
        let Some(block_sid) = block.sid.as_deref() else {
            return Vec::new();
        };
        let mut targets = Vec::new();
        for (line_index, line) in system.lines.iter().enumerate() {
            if line_data_input_ports(line, block_sid).contains(&port_index) {
                targets.extend(line_targets[line_index].clone());
            }
        }
        dedup_targets(targets)
    }

    fn mux_targets(
        &self,
        system: &System,
        block: &Block,
        line_targets: &[Vec<ConnectionTarget>],
    ) -> Vec<ConnectionTarget> {
        let mut targets = Vec::new();
        for incoming in incoming_lines_for_block(system, block) {
            let Some(line_index) = system
                .lines
                .iter()
                .position(|candidate| same_line(candidate, incoming))
            else {
                continue;
            };
            let signal_name = explicit_line_signal_name(incoming);
            for input_index in input_port_indices(block, incoming) {
                for mut target in line_targets[line_index].clone() {
                    target.resolve = Some(ConnectionTargetResolve::Index(input_index));
                    let next_signal_name = signal_name.clone().or(target.signal_name.clone());
                    set_signal_name_only(&mut target, next_signal_name);
                    target.origin = ConnectionTargetOrigin::Mux;
                    targets.push(target);
                }
            }
        }
        targets
    }

    fn demux_targets(
        &self,
        system: &System,
        block: &Block,
        output_index: u32,
        line_targets: &[Vec<ConnectionTarget>],
    ) -> Vec<ConnectionTarget> {
        let Some(incoming) = incoming_lines_for_block(system, block).into_iter().next() else {
            return Vec::new();
        };
        let Some(line_index) = system
            .lines
            .iter()
            .position(|candidate| same_line(candidate, incoming))
        else {
            return Vec::new();
        };

        line_targets[line_index]
            .iter()
            .filter(|target| {
                target.resolve == Some(ConnectionTargetResolve::Index(output_index))
                    || target.element_index == Some(output_index)
            })
            .cloned()
            .map(|mut target| {
                target.resolve = None;
                target.origin = ConnectionTargetOrigin::Demux;
                target
            })
            .collect()
    }

    fn resolve_from_block_targets(
        &self,
        system: &System,
        block: &Block,
        line_targets: &[Vec<ConnectionTarget>],
    ) -> Vec<ConnectionTarget> {
        let tag = block
            .properties
            .get("GotoTag")
            .map(|s| s.trim())
            .unwrap_or("A");

        let goto_blocks: Vec<&Block> = system
            .blocks
            .iter()
            .filter(|b| b.block_type == "Goto")
            .filter(|b| b.properties.get("GotoTag").map(|s| s.trim()).unwrap_or("A") == tag)
            .collect();

        let mut targets = Vec::new();
        for goto in goto_blocks {
            for incoming in incoming_lines_for_block(system, goto) {
                let Some(line_index) = system
                    .lines
                    .iter()
                    .position(|candidate| same_line(candidate, incoming))
                else {
                    continue;
                };
                targets.extend(line_targets[line_index].clone());
            }
        }
        targets
    }

    #[allow(clippy::too_many_arguments)]
    fn upstream_propagated_targets(
        &self,
        system: &System,
        _system_path: &[String],
        block: &Block,
        dst: &EndpointRef,
        parent_ctx: Option<&ParentSubsystemContext>,
        child_summaries: &HashMap<String, ChildSubsystemSummary>,
        line_targets: &[Vec<ConnectionTarget>],
    ) -> Vec<ConnectionTarget> {
        match block.block_type.as_str() {
            "BusCreator" => self.bus_creator_upstream_targets(system, block, line_targets),
            "BusSelector" => self.bus_selector_upstream_targets(system, block, line_targets),
            "BusAssignment" => self.bus_selector_upstream_targets(system, block, line_targets),
            "Mux" => self.mux_upstream_targets(system, block, dst.port_index, line_targets),
            "Demux" => self.demux_upstream_targets(system, block, line_targets),
            "Inport" => outgoing_line_indices_for_block(system, block)
                .into_iter()
                .flat_map(|(line_index, _)| line_targets[line_index].clone())
                .collect(),
            "Outport" => parent_ctx
                .and_then(|ctx| ctx.outgoing_by_port.get(&boundary_port_index(block)))
                .cloned()
                .unwrap_or_default(),
            "SubSystem" | "Reference" => child_summaries
                .get(block.sid.as_deref().unwrap_or_default())
                .filter(|_| !is_control_port_type(&dst.port_type))
                .and_then(|summary| summary.incoming_by_port.get(&dst.port_index))
                .map(|targets| {
                    let mut propagated = targets.clone();
                    if block.block_type == "Reference" {
                        for t in &mut propagated {
                            t.block_type = Some("Reference".to_string());
                        }
                    }
                    propagated
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn bus_creator_upstream_targets(
        &self,
        system: &System,
        block: &Block,
        line_targets: &[Vec<ConnectionTarget>],
    ) -> Vec<ConnectionTarget> {
        outgoing_line_indices_for_block(system, block)
            .into_iter()
            .flat_map(|(line_index, _)| line_targets[line_index].clone())
            .collect()
    }

    fn bus_selector_upstream_targets(
        &self,
        system: &System,
        block: &Block,
        line_targets: &[Vec<ConnectionTarget>],
    ) -> Vec<ConnectionTarget> {
        outgoing_line_indices_for_block(system, block)
            .into_iter()
            .flat_map(|(line_index, _)| {
                line_targets[line_index]
                    .iter()
                    .cloned()
                    .map(|mut target| {
                        // BusSelector output names (e.g. "<bus_a>") are NOT the
                        // bus's signal names. Clear them so merge_upstream_metadata
                        // does not overwrite the bus line's signal_name. Testpoint
                        // is preserved for cross-boundary propagation.
                        target.signal_name = None;
                        target.signal_names.clear();
                        target
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn mux_upstream_targets(
        &self,
        system: &System,
        block: &Block,
        input_index: u32,
        line_targets: &[Vec<ConnectionTarget>],
    ) -> Vec<ConnectionTarget> {
        outgoing_line_indices_for_block(system, block)
            .into_iter()
            .flat_map(|(line_index, _)| {
                line_targets[line_index]
                    .iter()
                    .filter(move |target| {
                        target.resolve == Some(ConnectionTargetResolve::Index(input_index))
                            || target.element_index == Some(input_index)
                    })
                    .cloned()
                    .map(|mut target| {
                        target.resolve = None;
                        target.element_index = None;
                        target
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn demux_upstream_targets(
        &self,
        system: &System,
        block: &Block,
        line_targets: &[Vec<ConnectionTarget>],
    ) -> Vec<ConnectionTarget> {
        outgoing_line_indices_for_block(system, block)
            .into_iter()
            .flat_map(|(line_index, outgoing_line)| {
                let output_index = outgoing_line.src.as_ref().map(|src| src.port_index);
                line_targets[line_index]
                    .iter()
                    .cloned()
                    .map(move |mut target| {
                        target.resolve = output_index.map(ConnectionTargetResolve::Index);
                        target.element_index = None;
                        target
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn full_block_path(&self, system_path: &[String], block_name: &str) -> String {
        let mut parts = Vec::new();
        if let Some(model_name) = normalized_path_segment(&self.model_name) {
            parts.push(model_name);
        }
        parts.extend(
            system_path
                .iter()
                .filter_map(|part| normalized_path_segment(part)),
        );
        if let Some(block_name) = normalized_path_segment(block_name) {
            parts.push(block_name);
        }
        parts.join("/")
    }

    fn direct_internal_block_targets(
        &self,
        system_path: &[String],
        block: &Block,
    ) -> Vec<ConnectionTarget> {
        let Some(subsystem) = &block.subsystem else {
            return Vec::new();
        };

        let child_path = child_system_path(system_path, &block.name);
        subsystem
            .blocks
            .iter()
            .map(|child| {
                let mut target = ConnectionTarget::new(
                    self.full_block_path(&child_path, &child.name),
                    ConnectionTargetOrigin::Internal,
                );
                target.block_type = Some(child.block_type.clone());
                target
            })
            .collect()
    }
}

pub fn debug_print_block_targets(root: &System, system_path: &[String], block: &Block) {
    let resolver = ConnectionTargetResolver::new(root);
    let targets = resolver.block_targets_for_block(system_path, block);
    println!("  [Targets] block '{}'", block.name);
    print_targets(&targets);
}

pub fn debug_print_block_targets_with_resolver(
    resolver: &ConnectionTargetResolver,
    system_path: &[String],
    block: &Block,
) {
    let targets = resolver.block_targets_for_block(system_path, block);
    println!("  [Targets] block '{}'", block.name);
    print_targets(&targets);
}

pub fn debug_print_line_targets(root: &System, system_path: &[String], line: &Line) {
    let resolver = ConnectionTargetResolver::new(root);
    let targets = resolver.line_targets_for_line(system_path, line);
    println!("  [Targets] line {}", line_identity(line));
    print_targets(&targets);
}

pub fn debug_print_line_targets_with_resolver(
    resolver: &ConnectionTargetResolver,
    system_path: &[String],
    line: &Line,
) {
    let targets = resolver.line_targets_for_line(system_path, line);
    println!("  [Targets] line {}", line_identity(line));
    print_targets(&targets);
}

fn print_targets(targets: &[ConnectionTarget]) {
    if targets.is_empty() {
        println!("    (no targets)");
        return;
    }

    for target in targets {
        println!(
            "    - path='{}' origin={:?} signal={:?} signal_names={:?} resolve={:?} index={:?} signals_only={} testpoint={} block_type={:?}",
            target.path,
            target.origin,
            target.signal_name,
            target.signal_names,
            target.resolve,
            target.element_index,
            target.signals_only,
            target.testpoint,
            target.block_type
        );
    }
}

fn build_block_lookup(system: &System) -> HashMap<&str, &Block> {
    system
        .blocks
        .iter()
        .filter_map(|block| block.sid.as_deref().map(|sid| (sid, block)))
        .collect()
}

fn child_system_path(system_path: &[String], block_name: &str) -> Vec<String> {
    let mut path = system_path.to_vec();
    path.push(block_name.to_string());
    path
}

fn boundary_port_index(block: &Block) -> u32 {
    block
        .properties
        .get("Port")
        .or_else(|| block.properties.get("PortNumber"))
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(1)
}

/// The data input ports of `block_sid` that `line` ends at, counting every
/// branch: a branched signal reaches a port through `line.branches`, where the
/// line's own `dst` says nothing about which port that is.  Control endpoints
/// (`enable`, `trigger`, …) are skipped — they belong to the matching control
/// port block, not to the numbered `Inport`s.
fn line_data_input_ports(line: &Line, block_sid: &str) -> BTreeSet<u32> {
    fn collect(dst: Option<&EndpointRef>, block_sid: &str, ports: &mut BTreeSet<u32>) {
        if let Some(dst) = dst
            && dst.sid == block_sid
            && !is_control_port_type(&dst.port_type)
        {
            ports.insert(dst.port_index);
        }
    }

    fn collect_branches(branches: &[Branch], block_sid: &str, ports: &mut BTreeSet<u32>) {
        for branch in branches {
            collect(branch.dst.as_ref(), block_sid, ports);
            collect_branches(&branch.branches, block_sid, ports);
        }
    }

    let mut ports = BTreeSet::new();
    collect(line.dst.as_ref(), block_sid, &mut ports);
    collect_branches(&line.branches, block_sid, &mut ports);
    ports
}

/// The input ports of `block` that `line` ends at, falling back to port 1 when
/// the wiring does not say (a block without a SID).
fn input_port_indices(block: &Block, line: &Line) -> Vec<u32> {
    let ports = block
        .sid
        .as_deref()
        .map(|sid| line_data_input_ports(line, sid))
        .unwrap_or_default();
    if ports.is_empty() {
        vec![1]
    } else {
        ports.into_iter().collect()
    }
}

/// Every endpoint a line ends at: its own `dst` plus the destination of every
/// branch, because a branched line has no `dst` of its own.
fn line_destination_endpoints(line: &Line) -> Vec<&EndpointRef> {
    fn collect<'a>(branches: &'a [Branch], out: &mut Vec<&'a EndpointRef>) {
        for branch in branches {
            out.extend(branch.dst.as_ref());
            collect(&branch.branches, out);
        }
    }

    let mut endpoints: Vec<&EndpointRef> = line.dst.as_ref().into_iter().collect();
    collect(&line.branches, &mut endpoints);
    endpoints
}

fn is_control_port_type(port_type: &str) -> bool {
    matches!(
        port_type.to_ascii_lowercase().as_str(),
        "enable" | "trigger" | "ifaction" | "action" | "reset" | "state" | "event"
    )
}

fn incoming_targets_by_port(
    system: &System,
    block: &Block,
    line_targets: &[Vec<ConnectionTarget>],
) -> BTreeMap<u32, Vec<ConnectionTarget>> {
    let mut by_port = BTreeMap::new();
    let Some(block_sid) = block.sid.as_deref() else {
        return by_port;
    };

    for (line, targets) in system.lines.iter().zip(line_targets.iter()) {
        for port_index in line_data_input_ports(line, block_sid) {
            by_port
                .entry(port_index)
                .or_insert_with(Vec::new)
                .extend(targets.clone());
        }
    }

    for targets in by_port.values_mut() {
        *targets = dedup_targets(std::mem::take(targets));
    }

    by_port
}

fn child_outgoing_targets_by_port(
    resolver: &ConnectionTargetResolver,
    system: &System,
    system_path: &[String],
    line_targets: &[Vec<ConnectionTarget>],
) -> BTreeMap<u32, Vec<ConnectionTarget>> {
    let mut by_port = BTreeMap::new();
    let inport_boundary_paths = subsystem_boundary_paths(resolver, system, system_path, "Inport");
    for block in &system.blocks {
        if block.block_type != "Outport" {
            continue;
        }

        let port_index = boundary_port_index(block);
        let mut targets = Vec::new();
        for incoming in incoming_lines_for_block(system, block) {
            if let Some(line_index) = system
                .lines
                .iter()
                .position(|candidate| same_line(candidate, incoming))
            {
                targets.extend(line_targets[line_index].clone());
            }
        }
        targets.retain(|target| !inport_boundary_paths.contains(&target.path));
        if !targets.is_empty() {
            by_port.insert(port_index, dedup_targets(targets));
        }
    }
    by_port
}

fn child_incoming_targets_by_port(
    system: &System,
    line_targets: &[Vec<ConnectionTarget>],
) -> BTreeMap<u32, Vec<ConnectionTarget>> {
    let mut by_port = BTreeMap::new();
    for block in &system.blocks {
        if block.block_type != "Inport" {
            continue;
        }

        let port_index = boundary_port_index(block);
        let mut targets = Vec::new();
        for (line_index, _) in outgoing_line_indices_for_block(system, block) {
            targets.extend(line_targets[line_index].clone());
        }
        if !targets.is_empty() {
            by_port.insert(port_index, dedup_targets(targets));
        }
    }
    by_port
}

fn apply_local_line_metadata(line: &Line, targets: &mut [ConnectionTarget]) {
    let explicit_name = explicit_line_signal_name(line);
    let explicit_testpoint = line_testpoint(line);
    for target in targets {
        set_signal_name_only(target, explicit_name.clone().or(target.signal_name.clone()));
        if target.resolve.is_none() {
            set_signal_resolve(target, explicit_name.clone());
        }
        target.testpoint = target.testpoint || explicit_testpoint;
    }
}

fn apply_source_port_testpoint(block: &Block, line: &Line, targets: &mut [ConnectionTarget]) {
    let Some(src) = &line.src else {
        return;
    };
    if !port_testpoint(block, src.port_type.as_str(), src.port_index) {
        return;
    }
    for target in targets {
        target.testpoint = true;
    }
}

fn merge_upstream_metadata(
    line: &Line,
    current_targets: &[ConnectionTarget],
    propagated_targets: &[ConnectionTarget],
    allow_cross_path: bool,
) -> Vec<ConnectionTarget> {
    let explicit_name = explicit_line_signal_name(line);
    let explicit_testpoint = line_testpoint(line);
    let path_counts = path_match_counts(current_targets);
    let mut merged_targets = current_targets.to_vec();

    for target in &mut merged_targets {
        // Split propagated targets into same-path and cross-path matches.
        // signal_names/signal_name should only flow along the same signal
        // path — a bus is just pack/unpack of signal lines, they should
        // NOT share their names with each other. testpoint, however, should
        // propagate across subsystem boundaries regardless of path.
        let same_path_count = path_counts.get(target.path.as_str()).copied().unwrap_or(0);
        let mut same_path_propagated: Vec<&ConnectionTarget> = Vec::new();
        let mut all_propagated: Vec<&ConnectionTarget> = Vec::new();

        for candidate in propagated_targets {
            if metadata_paths_match(target, candidate, same_path_count, allow_cross_path) {
                all_propagated.push(candidate);
                // Only treat as same-path for signal_names purposes when
                // the paths actually match (not a cross-path match).
                if target.path == candidate.path {
                    same_path_propagated.push(candidate);
                }
            }
        }

        if all_propagated.is_empty() {
            continue;
        }

        // signal_name and signal_names: only from same-path matches.
        if !same_path_propagated.is_empty() {
            let propagated_name = same_path_propagated
                .iter()
                .find_map(|candidate| candidate.signal_name.clone());
            set_signal_name_only(
                target,
                explicit_name
                    .clone()
                    .or(propagated_name)
                    .or(target.signal_name.clone()),
            );
            for candidate in &same_path_propagated {
                merge_signal_aliases(target, &candidate.signal_names);
            }
        } else if all_propagated.len() == 1 {
            // No same-path match, but exactly one cross-path match — this
            // is a single signal crossing a subsystem boundary, not a bus.
            // Safe to merge signal_names.
            let propagated_name = all_propagated
                .iter()
                .find_map(|candidate| candidate.signal_name.clone());
            set_signal_name_only(
                target,
                explicit_name
                    .clone()
                    .or(propagated_name)
                    .or(target.signal_name.clone()),
            );
            for candidate in &all_propagated {
                merge_signal_aliases(target, &candidate.signal_names);
            }
        } else if let Some(ref explicit) = explicit_name {
            // No same-path match, but the line itself has an explicit name.
            set_signal_name_only(target, Some(explicit.clone()));
        }

        // testpoint: from all matches (same-path + cross-path).
        target.testpoint = explicit_testpoint
            || target.testpoint
            || all_propagated.iter().any(|candidate| candidate.testpoint);
    }

    dedup_targets(merged_targets)
}

fn path_match_counts(targets: &[ConnectionTarget]) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for target in targets {
        *counts.entry(target.path.as_str()).or_insert(0) += 1;
    }
    counts
}

fn metadata_paths_match(
    current: &ConnectionTarget,
    propagated: &ConnectionTarget,
    same_path_count: usize,
    allow_cross_path: bool,
) -> bool {
    match (
        resolve_index_value(&current.resolve),
        resolve_index_value(&propagated.resolve),
    ) {
        (Some(current_index), Some(propagated_index)) => {
            return current_index == propagated_index;
        }
        (Some(_), None) | (None, Some(_)) if !allow_cross_path => {}
        _ => {}
    }

    if current.path != propagated.path {
        if allow_cross_path {
            return true;
        }
        return false;
    }

    match (current.element_index, propagated.element_index) {
        (_, None) => true,
        (Some(current_index), Some(propagated_index)) => current_index == propagated_index,
        (None, Some(_)) => same_path_count <= 1,
    }
}

fn set_signal_name_only(target: &mut ConnectionTarget, signal_name: Option<String>) {
    let normalized = signal_name.and_then(|signal_name| normalized_path_segment(&signal_name));
    if let Some(name) = &normalized {
        push_signal_alias(target, name);
    }
    target.signal_name = normalized;
}

/// Record `name` as one of the signal names this target carries, keeping
/// insertion order and skipping duplicates. `name` is expected to already be a
/// normalized path segment.
fn push_signal_alias(target: &mut ConnectionTarget, name: &str) {
    if !target.signal_names.iter().any(|existing| existing == name) {
        target.signal_names.push(name.to_string());
    }
}

/// Union `aliases` into `target.signal_names`, preserving order and dropping
/// duplicates.
fn merge_signal_aliases(target: &mut ConnectionTarget, aliases: &[String]) {
    for alias in aliases {
        push_signal_alias(target, alias);
    }
}

fn set_signal_resolve(target: &mut ConnectionTarget, signal_name: Option<String>) {
    target.resolve = signal_name
        .and_then(|signal_name| normalize_resolve_signal(&signal_name))
        .map(ConnectionTargetResolve::Signal);
}

fn normalize_resolve_signal(signal_name: &str) -> Option<String> {
    let trimmed = signal_name
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>');
    normalized_path_segment(trimmed)
}

fn resolve_signal_value(resolve: &Option<ConnectionTargetResolve>) -> Option<&str> {
    match resolve {
        Some(ConnectionTargetResolve::Signal(signal_name)) => Some(signal_name.as_str()),
        _ => None,
    }
}

fn resolve_index_value(resolve: &Option<ConnectionTargetResolve>) -> Option<u32> {
    match resolve {
        Some(ConnectionTargetResolve::Index(index)) => Some(*index),
        Some(ConnectionTargetResolve::TargetPath(target_path)) => target_path.port_index,
        _ => None,
    }
}

fn matches_resolve_signal(target: &ConnectionTarget, selected_name: &str) -> bool {
    resolve_signal_value(&target.resolve)
        .is_some_and(|signal_name| signal_keys_match(signal_name, selected_name))
}

fn signal_keys_match(left: &str, right: &str) -> bool {
    let Some(left) = normalize_resolve_signal(left) else {
        return false;
    };
    let Some(right) = normalize_resolve_signal(right) else {
        return false;
    };
    left.eq_ignore_ascii_case(&right)
}

/// Returns the hierarchical signal path for a BusSelector output port,
/// parsed from the block's `OutputSignals` property.  Returns `None` when
/// the property is absent (caller falls back to flat matching).
fn bus_selector_output_path(block: &Block, port_index: u32) -> Option<String> {
    let output_signals = block.properties.get("OutputSignals")?;
    let paths: Vec<&str> = output_signals
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let idx = (port_index as usize).checked_sub(1)?;
    let path = paths.get(idx)?;
    normalize_resolve_signal(path)
}

/// Checks whether a target's resolve path matches the given hierarchical
/// path.  Matches exactly for leaf signals, or as a prefix for sub-bus
/// selection (e.g. path `bus_c.bus_a` matches resolve `bus_c.bus_a.a`).
fn bus_signal_path_matches(target: &ConnectionTarget, path: &str) -> bool {
    let Some(target_path) = resolve_signal_value(&target.resolve) else {
        return false;
    };
    let Some(t) = normalize_resolve_signal(target_path) else {
        return false;
    };
    let Some(p) = normalize_resolve_signal(path) else {
        return false;
    };
    // Exact match (leaf signal)
    if t.eq_ignore_ascii_case(&p) {
        return true;
    }
    // Prefix match (sub-bus selection: path is a parent of target)
    let t_lower = t.to_ascii_lowercase();
    let p_lower = p.to_ascii_lowercase();
    t_lower.starts_with(&format!("{p_lower}."))
}

fn apply_line_resolve_hint(
    line: &Line,
    block_lookup: &HashMap<&str, &Block>,
    target: &mut ConnectionTarget,
) {
    if let Some(signal_name) = explicit_line_signal_name(line) {
        set_signal_resolve(target, Some(signal_name));
        return;
    }

    // The mux input this line ends at – for a branched line that is one of the
    // branch endpoints, not `line.dst`.
    if let Some(dst) = line_destination_endpoints(line).into_iter().find(|dst| {
        block_lookup
            .get(dst.sid.as_str())
            .is_some_and(|block| block.block_type == "Mux")
    }) {
        target.resolve = Some(ConnectionTargetResolve::Index(dst.port_index));
        return;
    }

    if target.resolve.is_none() && target.element_index.is_some() {
        target.resolve = target.element_index.map(ConnectionTargetResolve::Index);
    }
}

fn normalized_path_segment(segment: &str) -> Option<String> {
    let normalized = segment.split_whitespace().collect::<Vec<_>>().join(" ");
    (!normalized.is_empty()).then_some(normalized)
}

fn normalize_path(path: &str) -> String {
    path.trim_matches('/')
        .split('/')
        .filter_map(normalized_path_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn incoming_lines_for_block<'a>(system: &'a System, block: &Block) -> Vec<&'a Line> {
    let Some(block_sid) = block.sid.as_deref() else {
        return Vec::new();
    };
    system
        .lines
        .iter()
        .filter(|line| line_targets_block_sid(line, block_sid))
        .collect()
}

fn outgoing_line_indices_for_block<'a>(
    system: &'a System,
    block: &Block,
) -> Vec<(usize, &'a Line)> {
    let Some(block_sid) = block.sid.as_deref() else {
        return Vec::new();
    };

    system
        .lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.src.as_ref().is_some_and(|src| src.sid == block_sid))
        .collect()
}

fn outgoing_targets_by_port(
    system: &System,
    block: &Block,
    line_targets: &[Vec<ConnectionTarget>],
) -> BTreeMap<u32, Vec<ConnectionTarget>> {
    let mut by_port = BTreeMap::new();
    for (line_index, line) in outgoing_line_indices_for_block(system, block) {
        let port_index = line.src.as_ref().map(|src| src.port_index).unwrap_or(1);
        by_port
            .entry(port_index)
            .or_insert_with(Vec::new)
            .extend(line_targets[line_index].clone());
    }

    for targets in by_port.values_mut() {
        *targets = dedup_targets(std::mem::take(targets));
    }

    by_port
}

fn line_targets_block_sid(line: &Line, block_sid: &str) -> bool {
    line.dst.as_ref().is_some_and(|dst| dst.sid == block_sid)
        || branch_targets_block_sid(&line.branches, block_sid)
}

fn branch_targets_block_sid(branches: &[Branch], block_sid: &str) -> bool {
    branches.iter().any(|branch| {
        branch.dst.as_ref().is_some_and(|dst| dst.sid == block_sid)
            || branch_targets_block_sid(&branch.branches, block_sid)
    })
}

fn port_signal_name(block: &Block, port_type: &str, port_index: u32) -> Option<String> {
    block
        .ports
        .iter()
        .find(|port| port.port_type == port_type && port.index.unwrap_or(0) == port_index)
        .and_then(|port| {
            port.properties
                .get("Name")
                .or_else(|| port.properties.get("name"))
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .and_then(normalized_path_segment)
        })
}

fn port_testpoint(block: &Block, port_type: &str, port_index: u32) -> bool {
    block
        .ports
        .iter()
        .find(|port| port.port_type == port_type && port.index.unwrap_or(0) == port_index)
        .and_then(|port| port.properties.get("TestPoint"))
        .is_some_and(|value| matches!(value.trim(), "on" | "true" | "1" | "On" | "True"))
}

fn line_testpoint(line: &Line) -> bool {
    line.properties
        .get("TestPoint")
        .is_some_and(|value| matches!(value.trim(), "on" | "true" | "1" | "On" | "True"))
}

fn output_port_count(block: &Block) -> u32 {
    block
        .port_counts
        .as_ref()
        .and_then(|counts| counts.outs)
        .unwrap_or_else(|| {
            block
                .ports
                .iter()
                .filter(|port| port.port_type == "out")
                .count() as u32
        })
}

fn subsystem_boundary_paths(
    resolver: &ConnectionTargetResolver,
    system: &System,
    system_path: &[String],
    boundary_type: &str,
) -> BTreeSet<String> {
    system
        .blocks
        .iter()
        .filter(|block| block.block_type == boundary_type)
        .map(|block| resolver.full_block_path(system_path, &block.name))
        .collect()
}

fn explicit_line_signal_name(line: &Line) -> Option<String> {
    line.name
        .as_ref()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .and_then(normalized_path_segment)
}

fn routing_line_signal_name(_system: &System, line: &Line) -> Option<String> {
    explicit_line_signal_name(line)
}

fn block_cache_key(system_path: &[String], block: &Block) -> String {
    if let Some(sid) = &block.sid {
        return format!("sid:{sid}");
    }
    let mut key = system_path.join("/");
    if !key.is_empty() {
        key.push('/');
    }
    key.push_str(&block.name);
    key.push('#');
    key.push_str(&block.block_type);
    key
}

fn line_cache_key(system_path: &[String], line: &Line) -> String {
    let mut key = system_path.join("/");
    key.push('|');
    key.push_str(&line_identity(line));
    key
}

fn line_identity(line: &Line) -> String {
    let src = line
        .src
        .as_ref()
        .map(|src| format!("{}:{}:{}", src.sid, src.port_type, src.port_index))
        .unwrap_or_else(|| "none".to_string());
    let dst = line
        .dst
        .as_ref()
        .map(|dst| format!("{}:{}:{}", dst.sid, dst.port_type, dst.port_index))
        .unwrap_or_else(|| branch_identity(&line.branches));
    format!(
        "{src}->{dst}:{}:{}",
        line.name.as_deref().unwrap_or(""),
        line.points.len()
    )
}

fn branch_identity(branches: &[Branch]) -> String {
    let mut parts = Vec::new();
    collect_branch_identity(branches, &mut parts);
    parts.join(",")
}

fn collect_branch_identity(branches: &[Branch], parts: &mut Vec<String>) {
    for branch in branches {
        if let Some(dst) = &branch.dst {
            parts.push(format!("{}:{}:{}", dst.sid, dst.port_type, dst.port_index));
        }
        collect_branch_identity(&branch.branches, parts);
    }
}

fn same_line(left: &Line, right: &Line) -> bool {
    line_identity(left) == line_identity(right)
}

fn qualify_external_path(model_name: &str, raw_path: &str) -> String {
    let clean = normalize_path(raw_path);
    let normalized_model = normalize_path(model_name);
    if clean.is_empty()
        || normalized_model.is_empty()
        || clean.starts_with(&format!("{normalized_model}/"))
        || clean == normalized_model
    {
        clean
    } else {
        format!("{normalized_model}/{clean}")
    }
}

fn dashboard_binding_block_path(binding: &DashboardBinding) -> &str {
    match binding {
        DashboardBinding::ParamSource { block_path, .. } => block_path,
        DashboardBinding::SignalSpec { block_path, .. } => block_path,
    }
}

fn dashboard_binding_target_path(binding: &DashboardBinding) -> &DashboardTargetPath {
    match binding {
        DashboardBinding::ParamSource { target_path, .. } => target_path,
        DashboardBinding::SignalSpec { target_path, .. } => target_path,
    }
}

#[allow(clippy::type_complexity)]
pub fn dedup_targets(targets: Vec<ConnectionTarget>) -> Vec<ConnectionTarget> {
    let mut seen: BTreeMap<
        (
            String,
            Option<String>,
            Option<ConnectionTargetResolve>,
            Option<u32>,
            ConnectionTargetOrigin,
            bool,
        ),
        usize,
    > = BTreeMap::new();
    let mut out: Vec<ConnectionTarget> = Vec::new();
    for target in targets {
        let key = (
            target.path.clone(),
            target.signal_name.clone(),
            target.resolve.clone(),
            target.element_index,
            target.origin,
            target.signals_only,
        );
        if let Some(index) = seen.get(&key).copied() {
            if let Some(existing) = out.get_mut(index) {
                existing.testpoint = existing.testpoint || target.testpoint;
                merge_signal_aliases(existing, &target.signal_names);
            }
        } else {
            seen.insert(key, out.len());
            out.push(target);
        }
    }
    out
}

/// Property keys that only affect a block's or line's *geometry* (position,
/// stacking, routing waypoints) and therefore never change the resolved signal
/// / target-path graph.  They are skipped by [`model_topology_signature`].
const GEOMETRY_PROPERTY_KEYS: &[&str] = &["Position", "ZOrder", "Points", "SortIndex"];

/// A cheap 64-bit signature of everything in the model that the connection
/// target resolver depends on, deliberately *excluding* geometry (block
/// positions, z-order, line waypoints).
///
/// Building the resolver walks the whole subsystem tree and re-propagates every
/// signal, which is far more expensive than hashing.  Layout-only edits (moving
/// a block, dragging a line waypoint) leave this signature unchanged, so the
/// cached resolver can be reused instead of rebuilt every frame while dragging.
pub fn model_topology_signature(root: &System) -> u64 {
    let mut h = DefaultHasher::new();
    hash_system(root, &mut h);
    h.finish()
}

fn hash_system(sys: &System, h: &mut DefaultHasher) {
    if let Some(name) = sys.properties.get("Name") {
        name.hash(h);
    }
    sys.blocks.len().hash(h);
    for block in &sys.blocks {
        hash_block(block, h);
    }
    sys.lines.len().hash(h);
    for line in &sys.lines {
        hash_line(line, h);
    }
}

fn hash_non_geometry_properties(
    properties: &indexmap::IndexMap<String, String>,
    h: &mut DefaultHasher,
) {
    for (key, value) in properties {
        if GEOMETRY_PROPERTY_KEYS.contains(&key.as_str()) {
            continue;
        }
        key.hash(h);
        value.hash(h);
    }
}

fn hash_block(block: &Block, h: &mut DefaultHasher) {
    block.block_type.hash(h);
    block.name.hash(h);
    block.sid.hash(h);
    block.commented.hash(h);
    block.value.hash(h);
    if let Some(pc) = &block.port_counts {
        pc.ins.hash(h);
        pc.outs.hash(h);
    }
    block.ports.len().hash(h);
    for port in &block.ports {
        port.port_type.hash(h);
        port.index.hash(h);
    }
    hash_non_geometry_properties(&block.properties, h);
    block.dashboard_binding.is_some().hash(h);
    if let Some(subsystem) = &block.subsystem {
        hash_system(subsystem, h);
    }
}

fn hash_endpoint(endpoint: &Option<EndpointRef>, h: &mut DefaultHasher) {
    match endpoint {
        Some(e) => {
            1u8.hash(h);
            e.sid.hash(h);
            e.port_type.hash(h);
            e.port_index.hash(h);
        }
        None => 0u8.hash(h),
    }
}

fn hash_branch(branch: &Branch, h: &mut DefaultHasher) {
    branch.name.hash(h);
    hash_endpoint(&branch.dst, h);
    branch.branches.len().hash(h);
    for child in &branch.branches {
        hash_branch(child, h);
    }
}

fn hash_line(line: &Line, h: &mut DefaultHasher) {
    line.name.hash(h);
    hash_endpoint(&line.src, h);
    hash_endpoint(&line.dst, h);
    line.branches.len().hash(h);
    for branch in &line.branches {
        hash_branch(branch, h);
    }
    hash_non_geometry_properties(&line.properties, h);
}
