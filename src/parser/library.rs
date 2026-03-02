//! Library resolution – locate `.slx` library files on disk.

use camino::{Utf8Path, Utf8PathBuf};

/// Result for library resolution: which libraries were found (with path)
/// and which were not found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryLookupResult {
    pub found: Vec<(String, Utf8PathBuf)>,
    pub not_found: Vec<String>,
}

/// Resolver that searches for `LIBNAME.slx` files in an ordered list of
/// directories (first match wins).
#[derive(Debug, Clone)]
pub struct LibraryResolver {
    search_paths: Vec<Utf8PathBuf>,
}

/// Libraries that are treated specially when resolving library blocks.
///
/// For these, rustylink creates a "virtual" in-memory library instead of
/// requiring the `.slx` file to be present on disk.
///
/// Note: Some Simulink models refer to libraries as `NAME.slx/...`. Both
/// [`is_virtual_library`] and [`split_source_block_reference`] treat `.slx`
/// suffixes as optional and match case-insensitively.
pub const SPECIAL_VIRTUAL_LIBRARIES: [&str; 8] = [
    "simulink/Math Operations",
    // The built-in Simulink library is referenced as `simulink/...` or `simulink.slx/...`.
    "simulink",
    // Built-in rustylink virtual library.
    "matrix_library",
    // Some blocks are referenced with the full prefix "simulink/Logic and Bit Operations/...".
    "simulink/Logic and Bit Operations",
    // Some blocks are referenced with the full prefix "simulink/Logic and Bit/...".
    "simulink/Logic and Bit",
    // Discrete library blocks are referenced as "simulink/Discrete/...".
    "simulink/Discrete",
    // Signal Routing blocks (BusCreator, BusSelector, …).
    "simulink/Signal Routing",
    // Dashboard / UI blocks (Scope, Gauge, Switch, Slider, …).
    "simulink/Dashboard",
];

fn normalize_segment(seg: &str) -> String {
    let seg = seg.trim();
    let seg = seg
        .strip_suffix(".slx")
        .or_else(|| seg.strip_suffix(".SLX"))
        .unwrap_or(seg);
    seg.to_ascii_lowercase()
}

fn split_path_segments(path: &str) -> Vec<String> {
    let path = path.replace('\\', "/");
    path.split('/')
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Split a `SourceBlock`-style reference (`Library/Block/...`) into
/// `(library_name, block_path)`.
///
/// Unlike a simple `split_once('/')`, this function supports "virtual libraries"
/// whose logical names include sub-paths (e.g. `simulink/Discrete`). It performs
/// a longest-prefix match against [`SPECIAL_VIRTUAL_LIBRARIES`].
///
/// Returns `None` if the input does not contain at least one `/` separator.
pub fn split_source_block_reference(source_block: &str) -> Option<(String, String)> {
    let source_block = source_block.trim();
    if source_block.is_empty() {
        return None;
    }
    let segs = split_path_segments(source_block);
    if segs.len() < 2 {
        return None;
    }

    // Longest-prefix match against known virtual library prefixes.
    let mut best_prefix: Option<&'static str> = None;
    let mut best_len: usize = 0;
    for prefix in SPECIAL_VIRTUAL_LIBRARIES {
        let p_segs = split_path_segments(prefix);
        if p_segs.is_empty() || segs.len() <= p_segs.len() {
            continue;
        }
        let mut ok = true;
        for (i, p) in p_segs.iter().enumerate() {
            if normalize_segment(&segs[i]) != normalize_segment(p) {
                ok = false;
                break;
            }
        }
        if ok && p_segs.len() > best_len {
            best_prefix = Some(prefix);
            best_len = p_segs.len();
        }
    }

    if let Some(prefix) = best_prefix {
        let rest = segs[best_len..].join("/");
        return Some((prefix.to_string(), rest));
    }

    // Fallback: treat the first segment as the library name.
    let lib = segs[0].trim();
    let lib = lib
        .strip_suffix(".slx")
        .or_else(|| lib.strip_suffix(".SLX"))
        .unwrap_or(lib);
    let rest = segs[1..].join("/");
    Some((lib.to_string(), rest))
}

/// Return true if the given library name should be treated as a virtual library.
///
/// Accepts both with and without a `.slx` suffix and matches case-insensitively.
pub fn is_virtual_library(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    let segs = split_path_segments(name);
    if segs.is_empty() {
        return false;
    }

    SPECIAL_VIRTUAL_LIBRARIES.iter().any(|prefix| {
        let p_segs = split_path_segments(prefix);
        if p_segs.is_empty() {
            return false;
        }
        if segs.len() < p_segs.len() {
            return false;
        }
        for (i, p) in p_segs.iter().enumerate() {
            if normalize_segment(&segs[i]) != normalize_segment(p) {
                return false;
            }
        }
        // exact match or prefix match
        segs.len() == p_segs.len() || (segs.len() > p_segs.len())
    })
}

impl LibraryResolver {
    /// Create a resolver that will search the provided directories in order.
    pub fn new<P: AsRef<Utf8Path>>(paths: impl IntoIterator<Item = P>) -> Self {
        Self {
            search_paths: paths
                .into_iter()
                .map(|p| p.as_ref().to_path_buf())
                .collect(),
        }
    }

    /// Locate the given library names (e.g. `Regler`) by looking for
    /// `Regler.slx` under the configured search paths.
    pub fn locate<'a, I>(&self, libs: I) -> LibraryLookupResult
    where
        I: IntoIterator<Item = &'a str>,
    {
        use std::collections::HashSet;
        let mut found = Vec::new();
        let mut not_found = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for lib in libs {
            let lib = lib.trim();
            if lib.is_empty() {
                continue;
            }

            // virtual libraries are handled entirely in-memory and have no
            // corresponding `.slx` file on disk.  Consumers often call
            // `LibraryResolver::locate` simply to report which libraries are
            // missing; including virtual libs in the results would be noisy and
            // misleading.  Skip them here so that the returned `LibraryLookupResult`
            // only contains non-virtual entries.  See also
            // `is_virtual_library` which has the matching logic.
            if is_virtual_library(lib) {
                continue;
            }

            if !seen.insert(lib.to_string()) {
                continue;
            }
            let file_name = format!("{}.slx", lib);
            let mut matched: Option<Utf8PathBuf> = None;
            for dir in &self.search_paths {
                let candidate = dir.join(&file_name);
                if candidate.exists() {
                    matched = Some(candidate);
                    break;
                }
            }
            if let Some(p) = matched {
                found.push((lib.to_string(), p));
            } else {
                not_found.push(lib.to_string());
            }
        }
        LibraryLookupResult { found, not_found }
    }
}
