#![cfg(feature = "egui")]

use egui::text::LayoutJob;
use eframe::egui::{self, Color32};

/// Simple case-insensitive highlighter that builds a LayoutJob for `text`,
/// highlighting occurrences of `query`.
pub fn highlight_query_job(text: &str, query: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    let t = text;
    let tl = t.to_lowercase();
    let ql = query.to_lowercase();
    if ql.is_empty() {
        job.append(t, 0.0, egui::TextFormat::default());
        return job;
    }
    let mut i = 0;
    while let Some(pos) = tl[i..].find(&ql) {
        let start = i + pos;
        if start > i {
            job.append(&t[i..start], 0.0, egui::TextFormat::default());
        }
        let end = start + ql.len();
        let mut fmt = egui::TextFormat::default();
        fmt.background = Color32::YELLOW.into();
        job.append(&t[start..end], 0.0, fmt);
        i = end;
    }
    if i < t.len() {
        job.append(&t[i..], 0.0, egui::TextFormat::default());
    }
    job
}

/// MATLAB syntax highlighter using syntect. Lazily loads the syntax set and theme.
pub fn matlab_syntax_job(script: &str) -> LayoutJob {
    use egui::{FontId, TextFormat};
    use once_cell::sync::OnceCell;
    use syntect::easy::HighlightLines;
    use syntect::highlighting::{Style, ThemeSet};
    use syntect::parsing::SyntaxSet;
    use syntect::util::LinesWithEndings;

    static SYNTAX_SET: OnceCell<SyntaxSet> = OnceCell::new();
    static THEME_SET: OnceCell<ThemeSet> = OnceCell::new();

    let ss = SYNTAX_SET.get_or_init(|| SyntaxSet::load_defaults_newlines());
    let ts = THEME_SET.get_or_init(|| ThemeSet::load_defaults());
    // Important: Don't select by ".m" file extension as syntect often resolves that to Objective‑C.
    // Prefer the explicit MATLAB scope or well-known names and only then fall back to plain text.
    let syntax = {
        use syntect::parsing::Scope;
        // Try by scope first (most reliable)
        let by_scope = Scope::new("source.matlab").ok().and_then(|s| ss.find_syntax_by_scope(s));
        if let Some(s) = by_scope { s } else {
            // Try a few common names that appear across sublime grammars
            ss.find_syntax_by_name("Matlab")
                .or_else(|| ss.find_syntax_by_name("MATLAB"))
                .or_else(|| ss.find_syntax_by_name("Matlab (Octave)"))
                .or_else(|| ss.find_syntax_by_name("MATLAB (Octave)"))
                .unwrap_or_else(|| ss.find_syntax_plain_text())
        }
    };
    let theme = ts
        .themes
        .get("InspiredGitHub")
        .or_else(|| ts.themes.values().next())
        .unwrap();

    let mut h = HighlightLines::new(syntax, theme);
    let mut job = LayoutJob::default();
    let mono = FontId::monospace(14.0);

    for line in LinesWithEndings::from(script) {
        let regions: Vec<(Style, &str)> = h.highlight(line, ss);
        for (style, text) in regions {
            let color = Color32::from_rgba_premultiplied(
                style.foreground.r,
                style.foreground.g,
                style.foreground.b,
                style.foreground.a,
            );
            let tf = TextFormat { font_id: mono.clone(), color, ..Default::default() };
            job.append(text, 0.0, tf);
        }
    }
    job
}

// tests moved to tests/ module
