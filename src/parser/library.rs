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
/// Initial set:
/// - `simulink.slx`
/// - `matrix_library.slx`
pub const SPECIAL_VIRTUAL_LIBRARIES: [&str; 3] = [
    "simulink.slx",
    "matrix_library.slx",
    // Some blocks are referenced with the full path "simulink/Logic and Bit/...";
    // treat that prefix as a virtual library as well.
    "simulink/Logic and Bit",
];

/// Return true if the given library name should be treated as a virtual library.
///
/// Accepts both with and without a `.slx` suffix and matches case-insensitively.
pub fn is_virtual_library(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    // lowercase first so that suffix stripping is case-insensitive
    let mut normalized = name.to_ascii_lowercase();
    normalized = normalized
        .strip_suffix(".slx")
        .unwrap_or(&normalized)
        .to_string();
    // collapse multiple slashes for consistency (not strictly needed but harmless)
    normalized = normalized.replace("\\\\", "/");

    SPECIAL_VIRTUAL_LIBRARIES.iter().any(|s| {
        let mut s_norm = s.to_ascii_lowercase();
        s_norm = s_norm
            .strip_suffix(".slx")
            .unwrap_or(&s_norm)
            .to_string();
        s_norm = s_norm.replace("\\\\", "/");
        // match if the names are equal, or if the candidate is a prefix of the
        // normalized name followed by a slash.  This lets entries like
        // "simulink/Logic and Bit" cover "simulink/Logic and Bit/SomeBlock".
        normalized == s_norm || normalized.starts_with(&format!("{}/", s_norm))
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
            if lib.is_empty() || !seen.insert(lib.to_string()) {
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
