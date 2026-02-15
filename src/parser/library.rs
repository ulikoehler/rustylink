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
