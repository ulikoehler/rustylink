#![cfg(feature = "egui")]

use eframe::egui::{self, Color32, FontId, Stroke, Style as EguiStyle};
use egui::text::LayoutJob;
use quick_xml::events::{BytesStart, Event};
use quick_xml::escape::unescape;
use quick_xml::Reader;
use std::borrow::Cow;

const DEFAULT_FONT_SIZE_PX: f32 = 12.0;

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationRichText {
    pub lines: Vec<AnnotationLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationLine {
    pub spans: Vec<AnnotationSpan>,
    pub resolved_style: ResolvedStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationSpan {
    pub text: String,
    pub style: ResolvedStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedStyle {
    pub font_size_px: f32,
    pub color: Option<Color32>,
    pub background: Option<Color32>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for ResolvedStyle {
    fn default() -> Self {
        Self {
            font_size_px: DEFAULT_FONT_SIZE_PX,
            color: None,
            background: None,
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

impl AnnotationRichText {
    pub fn from_plain_text(raw: &str) -> Self {
        let mut lines: Vec<AnnotationLine> = Vec::new();
        let normalized = raw.replace("\r\n", "\n");
        if normalized.is_empty() {
            lines.push(AnnotationLine::new(ResolvedStyle::default()));
        } else {
            for chunk in normalized.split('\n') {
                let mut line = AnnotationLine::new(ResolvedStyle::default());
                if !chunk.is_empty() {
                    line.push_span(chunk.to_string(), ResolvedStyle::default());
                }
                lines.push(line);
            }
        }
        Self { lines }
    }

    pub fn to_plain_text(&self) -> String {
        let mut out = String::new();
        for (idx, line) in self.lines.iter().enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            let mut buffer = line.text_content();
            if !buffer.is_empty() {
                // retain left whitespace but drop trailing runs for readability
                let trimmed_len = buffer.trim_end().len();
                buffer.truncate(trimmed_len);
                out.push_str(&buffer);
            }
        }
        out
    }

    pub fn to_layout_job(
        &self,
        style: &EguiStyle,
        font_scale: f32,
        default_font_size: f32,
    ) -> LayoutJob {
        let mut job = LayoutJob::default();
        let base_color = style.visuals.text_color();
        let new_line_format = |size: f32| {
            let mut fmt = egui::text::TextFormat::default();
            fmt.font_id = FontId::proportional(size);
            fmt.color = base_color;
            fmt
        };

        for (idx, line) in self.lines.iter().enumerate() {
            if line.spans.is_empty() {
                let fmt = new_line_format(default_font_size * font_scale);
                job.append("\n", 0.0, fmt);
                continue;
            }
            for span in &line.spans {
                let mut fmt = egui::text::TextFormat::default();
                let target_size = span.style.font_size_px.max(1.0) * font_scale;
                fmt.font_id = FontId::proportional(target_size);
                let mut color = span
                    .style
                    .color
                    .or(line.resolved_style.color)
                    .unwrap_or(base_color);
                let bold_source = span.style.bold || line.resolved_style.bold;
                if bold_source && span.style.color.is_none() && line.resolved_style.color.is_none()
                {
                    color = style.visuals.strong_text_color();
                }
                fmt.color = color;
                if let Some(bg) = span.style.background.or(line.resolved_style.background) {
                    fmt.background = bg;
                }
                if span.style.italic || line.resolved_style.italic {
                    fmt.italics = true;
                }
                if span.style.underline || line.resolved_style.underline {
                    fmt.underline = Stroke::new(1.0, color);
                }
                job.append(&span.text, 0.0, fmt);
            }
            if idx + 1 < self.lines.len() {
                let fmt = new_line_format(default_font_size * font_scale);
                job.append("\n", 0.0, fmt);
            }
        }

        job
    }
}

impl AnnotationLine {
    pub fn new(resolved_style: ResolvedStyle) -> Self {
        Self {
            spans: Vec::new(),
            resolved_style,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.spans.iter().all(|span| span.text.trim().is_empty())
    }

    pub fn font_size_px(&self) -> f32 {
        self.resolved_style.font_size_px
    }

    pub fn text_color(&self) -> Option<Color32> {
        self.resolved_style
            .color
            .or_else(|| self.spans.iter().find_map(|span| span.style.color))
    }

    pub fn is_bold(&self) -> bool {
        self.resolved_style.bold || self.spans.iter().any(|span| span.style.bold)
    }

    pub fn is_italic(&self) -> bool {
        self.resolved_style.italic || self.spans.iter().any(|span| span.style.italic)
    }

    pub fn has_underline(&self) -> bool {
        self.resolved_style.underline || self.spans.iter().any(|span| span.style.underline)
    }

    fn text_content(&self) -> String {
        let mut buffer = String::new();
        for span in &self.spans {
            buffer.push_str(&span.text);
        }
        buffer
    }

    fn push_span(&mut self, text: String, style: ResolvedStyle) {
        if text.is_empty() {
            return;
        }
        if let Some(prev) = self.spans.last_mut() {
            if prev.style == style {
                prev.text.push_str(&text);
                return;
            }
        }
        self.spans.push(AnnotationSpan { text, style });
    }
}

/// Convert Simulink/Qt HTML annotations to a structured representation.
pub fn annotation_to_rich_text(raw: &str, interpreter: Option<&str>) -> AnnotationRichText {
    if !looks_like_html(raw, interpreter) {
        return AnnotationRichText::from_plain_text(raw);
    }

    let decoded = html_escape::decode_html_entities(raw).to_string();
    parse_annotation_html(&decoded)
        .unwrap_or_else(|_| AnnotationRichText::from_plain_text(&decoded))
}

/// Backwards-compatible plain-text extraction built on top of the rich parser.
pub fn annotation_to_plain_text(raw: &str, interpreter: Option<&str>) -> String {
    annotation_to_rich_text(raw, interpreter).to_plain_text()
}

fn parse_annotation_html(html: &str) -> Result<AnnotationRichText, ()> {
    let mut reader = Reader::from_str(html);
    // Configure reader to not trim text to keep spaces as-is
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut style_stack: Vec<StyleFrame> = vec![StyleFrame::root()];
    let mut lines: Vec<AnnotationLine> = Vec::new();
    let mut current_line: Option<AnnotationLine> = None;
    let mut skip_depth: usize = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = tag_name(e.name().as_ref())?;
                if skip_depth > 0 {
                    skip_depth += 1;
                    continue;
                }
                if matches!(name.as_str(), "head" | "style" | "script") {
                    skip_depth = 1;
                    continue;
                }
                if name == "br" {
                    handle_break(&mut current_line, &mut lines, &style_stack);
                    continue;
                }
                let attrs = collect_attributes(&e)?;
                let frame = style_from_element(&name, &attrs);
                style_stack.push(frame);
                if name == "p" {
                    finalize_line(&mut current_line, &mut lines);
                    current_line = Some(AnnotationLine::new(resolve_style(&style_stack)));
                }
            }
            Ok(Event::Empty(e)) => {
                let name = tag_name(e.name().as_ref())?;
                if skip_depth > 0 {
                    continue;
                }
                if matches!(name.as_str(), "head" | "style" | "script") {
                    continue;
                }
                if name == "br" {
                    handle_break(&mut current_line, &mut lines, &style_stack);
                    continue;
                }
                let attrs = collect_attributes(&e)?;
                let frame = style_from_element(&name, &attrs);
                style_stack.push(frame.clone());
                if name == "p" {
                    finalize_line(&mut current_line, &mut lines);
                    lines.push(AnnotationLine::new(resolve_style(&style_stack)));
                }
                if style_stack.len() > 1 {
                    style_stack.pop();
                }
            }
            Ok(Event::End(e)) => {
                let name = tag_name(e.name().as_ref())?;
                if skip_depth > 0 {
                    skip_depth = skip_depth.saturating_sub(1);
                    continue;
                }
                if matches!(name.as_str(), "head" | "style" | "script") {
                    continue;
                }
                if name == "br" {
                    continue;
                }
                if name == "p" {
                    finalize_line(&mut current_line, &mut lines);
                }
                if style_stack.len() > 1 {
                    style_stack.pop();
                }
            }
            Ok(Event::Text(e)) => {
                if skip_depth > 0 {
                    continue;
                }
                // Decode text as UTF-8 and unescape XML entities
                let raw = std::str::from_utf8(e.as_ref()).map_err(|_| ())?;
                let unesc = unescape(raw).map_err(|_| ())?;
                push_text_segment(unesc, &mut current_line, &style_stack);
            }
            Ok(Event::CData(e)) => {
                if skip_depth > 0 {
                    continue;
                }
                let text = e.into_inner();
                let owned = String::from_utf8(text.to_vec()).map_err(|_| ())?;
                push_text_segment(Cow::Owned(owned), &mut current_line, &style_stack);
            }
            Ok(Event::Comment(_))
            | Ok(Event::Decl(_))
            | Ok(Event::PI(_))
            | Ok(Event::GeneralRef(_))
            | Ok(Event::DocType(_)) => {}
            Ok(Event::Eof) => break,
            Err(_) => return Err(()),
        }
        buf.clear();
    }

    finalize_line(&mut current_line, &mut lines);

    if lines.is_empty() {
        lines.push(AnnotationLine::new(resolve_style(&style_stack)));
    }

    Ok(AnnotationRichText { lines })
}

fn push_text_segment(
    text: Cow<'_, str>,
    current_line: &mut Option<AnnotationLine>,
    style_stack: &[StyleFrame],
) {
    let mut owned = text.into_owned();
    owned = owned.replace('\r', "").replace('\n', "");
    if owned.is_empty() {
        return;
    }
    if current_line.is_none() {
        current_line.replace(AnnotationLine::new(resolve_style(style_stack)));
    }
    if let Some(line) = current_line.as_mut() {
        let style = resolve_style(style_stack);
        line.push_span(owned, style);
    }
}

fn finalize_line(current_line: &mut Option<AnnotationLine>, lines: &mut Vec<AnnotationLine>) {
    if let Some(line) = current_line.take() {
        lines.push(line);
    }
}

fn handle_break(
    current_line: &mut Option<AnnotationLine>,
    lines: &mut Vec<AnnotationLine>,
    style_stack: &[StyleFrame],
) {
    if current_line.is_some() {
        finalize_line(current_line, lines);
    } else {
        lines.push(AnnotationLine::new(resolve_style(style_stack)));
    }
}

fn looks_like_html(raw: &str, interpreter: Option<&str>) -> bool {
    interpreter
        .map(|i| {
            let i = i.trim();
            i.eq_ignore_ascii_case("html") || i.eq_ignore_ascii_case("rich")
        })
        .unwrap_or_else(|| {
            raw.contains("<html")
                || raw.contains("<!DOCTYPE")
                || raw.contains("<body")
                || raw.contains("<p")
                || raw.contains("<span")
        })
}

fn collect_attributes(tag: &BytesStart<'_>) -> Result<Vec<(String, String)>, ()> {
    let mut attrs: Vec<(String, String)> = Vec::new();
    for attr in tag.attributes() {
        let attr = attr.map_err(|_| ())?;
        let key = std::str::from_utf8(attr.key.as_ref())
            .map_err(|_| ())?
            .to_ascii_lowercase();
        let value = attr.unescape_value().map_err(|_| ())?.into_owned();
        attrs.push((key, value));
    }
    Ok(attrs)
}

fn tag_name(bytes: &[u8]) -> Result<String, ()> {
    Ok(std::str::from_utf8(bytes)
        .map_err(|_| ())?
        .to_ascii_lowercase())
}

#[derive(Debug, Clone, Default)]
struct StyleFrame {
    font_size_px: Option<f32>,
    color: Option<Color32>,
    background: Option<Color32>,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
}

impl StyleFrame {
    fn root() -> Self {
        Self {
            font_size_px: Some(DEFAULT_FONT_SIZE_PX),
            color: None,
            background: None,
            bold: Some(false),
            italic: Some(false),
            underline: Some(false),
        }
    }
}

fn style_from_element(name: &str, attrs: &[(String, String)]) -> StyleFrame {
    let mut frame = StyleFrame::default();
    if let Some(style_attr) = find_attr(attrs, "style") {
        apply_css_declarations(&mut frame, style_attr);
    }
    if let Some(color_attr) = find_attr(attrs, "color") {
        if let Some(color) = parse_color(color_attr) {
            frame.color = Some(color);
        }
    }
    if let Some(bg_attr) = find_attr(attrs, "bgcolor") {
        if let Some(color) = parse_color(bg_attr) {
            frame.background = Some(color);
        }
    }
    if let Some(weight_attr) = find_attr(attrs, "font-weight") {
        if let Some(bold) = parse_font_weight(weight_attr) {
            frame.bold = Some(bold);
        }
    }
    if let Some(size_attr) = find_attr(attrs, "font-size") {
        if let Some(size) = parse_font_size(size_attr) {
            frame.font_size_px = Some(size);
        }
    }

    match name {
        "b" | "strong" => frame.bold = Some(true),
        "i" | "em" | "cite" => frame.italic = Some(true),
        "u" | "ins" => frame.underline = Some(true),
        _ => {}
    }

    frame
}

fn find_attr<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

fn apply_css_declarations(frame: &mut StyleFrame, decls: &str) {
    for decl in decls.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }
        if let Some((raw_key, raw_value)) = decl.split_once(':') {
            let key = raw_key.trim().to_ascii_lowercase();
            let value = clean_css_value(raw_value);
            match key.as_str() {
                "font-size" => {
                    if let Some(size) = parse_font_size(value) {
                        frame.font_size_px = Some(size);
                    }
                }
                "font-weight" => {
                    if let Some(bold) = parse_font_weight(value) {
                        frame.bold = Some(bold);
                    }
                }
                "font-style" => {
                    if let Some(italic) = parse_font_style(value) {
                        frame.italic = Some(italic);
                    }
                }
                "text-decoration" => {
                    if let Some(underline) = parse_text_decoration(value) {
                        frame.underline = Some(underline);
                    }
                }
                "color" => {
                    if let Some(color) = parse_color(value) {
                        frame.color = Some(color);
                    }
                }
                "background" | "background-color" => {
                    if let Some(color) = parse_color(value) {
                        frame.background = Some(color);
                    }
                }
                _ => {}
            }
        }
    }
}

fn clean_css_value<'a>(value: &'a str) -> &'a str {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(pos) = lower.find("!important") {
        trimmed[..pos].trim_end()
    } else {
        trimmed
    }
}

fn parse_font_size(value: &str) -> Option<f32> {
    let lower = value.to_ascii_lowercase();
    if lower.ends_with('%') {
        return None; // relative sizes not handled yet
    }
    if let Some(number) = lower.strip_suffix("px") {
        number.trim().parse::<f32>().ok()
    } else if let Some(number) = lower.strip_suffix("pt") {
        number
            .trim()
            .parse::<f32>()
            .ok()
            .map(|pt| pt * (96.0 / 72.0))
    } else {
        lower.trim().parse::<f32>().ok()
    }
}

fn parse_font_weight(value: &str) -> Option<bool> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("bold") {
        Some(true)
    } else if lower.contains("normal") || lower.contains("lighter") {
        Some(false)
    } else if let Ok(weight) = lower.trim().parse::<i32>() {
        Some(weight >= 600)
    } else {
        None
    }
}

fn parse_font_style(value: &str) -> Option<bool> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("italic") || lower.contains("oblique") {
        Some(true)
    } else if lower.contains("normal") {
        Some(false)
    } else {
        None
    }
}

fn parse_text_decoration(value: &str) -> Option<bool> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("underline") {
        Some(true)
    } else if lower.contains("none") {
        Some(false)
    } else {
        None
    }
}

fn parse_color(value: &str) -> Option<Color32> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(hex) = trimmed.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("rgb(") {
        return parse_rgb_function(rest, false);
    }
    if let Some(rest) = lower.strip_prefix("rgba(") {
        return parse_rgb_function(rest, true);
    }
    parse_named_color(&lower)
}

fn parse_hex_color(hex: &str) -> Option<Color32> {
    match hex.len() {
        3 => {
            let mut chars = hex.chars();
            let r = chars.next()?;
            let g = chars.next()?;
            let b = chars.next()?;
            Some(Color32::from_rgb(
                short_hex(r)?,
                short_hex(g)?,
                short_hex(b)?,
            ))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color32::from_rgb(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(Color32::from_rgba_unmultiplied(r, g, b, a))
        }
        _ => None,
    }
}

fn short_hex(c: char) -> Option<u8> {
    c.to_digit(16).map(|d| (d as u8) * 17)
}

fn parse_rgb_function(value: &str, include_alpha: bool) -> Option<Color32> {
    let cleaned = value.trim_end_matches(')');
    let mut parts = cleaned.split(',').map(|s| s.trim());
    let r = parse_rgb_number(parts.next()?)?;
    let g = parse_rgb_number(parts.next()?)?;
    let b = parse_rgb_number(parts.next()?)?;
    if include_alpha {
        let alpha_part = parts.next()?;
        let a = parse_alpha(alpha_part)?;
        Some(Color32::from_rgba_unmultiplied(r, g, b, a))
    } else {
        Some(Color32::from_rgb(r, g, b))
    }
}

fn parse_rgb_number(value: &str) -> Option<u8> {
    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent.parse::<f32>().ok()?;
        Some((percent.clamp(0.0, 100.0) * 2.55).round() as u8)
    } else {
        value.parse::<u8>().ok()
    }
}

fn parse_alpha(value: &str) -> Option<u8> {
    if let Ok(num) = value.parse::<f32>() {
        Some((num.clamp(0.0, 1.0) * 255.0).round() as u8)
    } else {
        parse_rgb_number(value)
    }
}

fn parse_named_color(name: &str) -> Option<Color32> {
    match name {
        "black" => Some(Color32::BLACK),
        "white" => Some(Color32::WHITE),
        "red" => Some(Color32::from_rgb(255, 0, 0)),
        "green" => Some(Color32::from_rgb(0, 128, 0)),
        "blue" => Some(Color32::from_rgb(0, 0, 255)),
        "yellow" => Some(Color32::from_rgb(255, 255, 0)),
        "cyan" => Some(Color32::from_rgb(0, 255, 255)),
        "magenta" => Some(Color32::from_rgb(255, 0, 255)),
        "gray" | "grey" => Some(Color32::from_gray(128)),
        _ => None,
    }
}

fn resolve_style(stack: &[StyleFrame]) -> ResolvedStyle {
    let font_size_px = stack
        .iter()
        .rev()
        .find_map(|s| s.font_size_px)
        .unwrap_or(DEFAULT_FONT_SIZE_PX);
    let color = stack.iter().rev().find_map(|s| s.color);
    let background = stack.iter().rev().find_map(|s| s.background);
    let bold = stack.iter().rev().find_map(|s| s.bold).unwrap_or(false);
    let italic = stack.iter().rev().find_map(|s| s.italic).unwrap_or(false);
    let underline = stack
        .iter()
        .rev()
        .find_map(|s| s.underline)
        .unwrap_or(false);
    ResolvedStyle {
        font_size_px,
        color,
        background,
        bold,
        italic,
        underline,
    }
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
    use egui::FontId;
    use egui::text::TextFormat;
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
        let by_scope = Scope::new("source.matlab")
            .ok()
            .and_then(|s| ss.find_syntax_by_scope(s));
        if let Some(s) = by_scope {
            s
        } else {
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
            let tf = TextFormat {
                font_id: mono.clone(),
                color,
                ..Default::default()
            };
            job.append(text, 0.0, tf);
        }
    }
    job
}

// tests moved to tests/ module
