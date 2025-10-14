#![cfg(feature = "egui")]

use egui::text::LayoutJob;
use eframe::egui::{self, Color32};

/// Convert Simulink/Qt rich text HTML into plain text suitable for egui labels.
/// We implement a tiny HTML "parser":
/// - Detect common rich-text markers like <p>, <br> and convert them to newlines
/// - Strip all remaining tags
/// - Decode common HTML entities (via html_escape)
/// This is intentionally conservative and fast; egui does not render HTML, so
/// we aim for a readable plain-text fallback instead of showing raw tags.
pub fn annotation_to_plain_text(raw: &str, interpreter: Option<&str>) -> String {
    // Heuristic: consider it HTML if interpreter suggests so or if we see
    // typical markers. If it's not HTML, just return the string as-is.
    let likely_html = interpreter
        .map(|i| {
            let i = i.trim();
            i.eq_ignore_ascii_case("html") || i.eq_ignore_ascii_case("rich")
        })
        .unwrap_or_else(|| {
            let s = raw;
            s.contains("<html")
                || s.contains("<!DOCTYPE")
                || s.contains("<body")
                || s.contains("<p")
                || s.contains("<span")
        });

    if !likely_html {
        return raw.to_string();
    }

    // 1) Remove head/style/script sections (including contents)
    fn strip_block(mut s: String, tag: &str) -> String {
        // Case-insensitive search by working on a lowercase mirror
        let open_pat = format!("<{}", tag);
        let close_pat = format!("</{}>", tag);
        let mut lower = s.to_lowercase();
        loop {
            if let Some(start_open) = lower.find(&open_pat) {
                // find the end of the opening tag '>'
                if let Some(end_open_rel) = lower[start_open..].find('>') {
                    let end_open = start_open + end_open_rel + 1;
                    if let Some(end_close) = lower[end_open..].find(&close_pat) {
                        let end_close_abs = end_open + end_close + close_pat.len();
                        s.replace_range(start_open..end_close_abs, "");
                        lower.replace_range(start_open..end_close_abs, "");
                        continue;
                    } else {
                        // no closing tag; drop from opening to end
                        s.truncate(start_open);
                        lower.truncate(start_open);
                        break;
                    }
                } else {
                    // malformed; break to avoid infinite loop
                    break;
                }
            } else {
                break;
            }
        }
        s
    }
    let mut s = raw.to_string();
    for tag in ["head", "style", "script"] { s = strip_block(s, tag); }

    // 2) Normalize common line breaks from HTML. (Cannot use multi-pattern replace on stable.)
    for pat in ["<br>", "<br/>", "<br />", "<BR>", "<BR/>", "<BR />"] { s = s.replace(pat, "\n"); }
    // Close paragraph becomes a newline; opening paragraph removed
    for pat in ["</p>", "</P>"] { s = s.replace(pat, "\n"); }
    for pat in ["<p>", "<P>"] { s = s.replace(pat, ""); }
    s = s.replace("\r\n", "\n");

    // 3) Strip remaining tags with a tiny state machine (avoid heavy HTML deps)
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ => {
                if !in_tag { out.push(ch); }
            }
        }
    }

    // 4) Decode a few common entities (use crate dependency)
    let decoded = html_escape::decode_html_entities(&out).to_string();

    // 5) Remove empty lines introduced by HTML formatting and trim trailing spaces.
    //    Simulink/Qt often formats each <p> on its own line in the source. After we
    //    convert </p> to a newline, those source newlines would yield blank lines.
    //    We therefore drop blank-only lines entirely to get one line per paragraph.
    let mut cleaned = String::new();
    for line in decoded.lines() {
        if line.trim().is_empty() { continue; }
        if !cleaned.is_empty() { cleaned.push('\n'); }
        cleaned.push_str(line.trim_end());
    }
    cleaned
}

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
