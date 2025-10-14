#![cfg(feature = "egui")]

use eframe::egui::Color32;

use rustylink::egui_app::highlight_query_job;
use rustylink::egui_app::text::{annotation_to_plain_text, annotation_to_rich_text};

#[test]
fn test_highlight_job() {
    let job = highlight_query_job("/A/B", "b");
    assert!(job.sections.len() >= 1);
}

#[test]
fn test_annotation_to_plain_text_removes_blank_lines() {
    let html = r#"
<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 4.0//EN" "http://www.w3.org/TR/REC-html40/strict.dtd">
<html><head><meta name="qrichtext" content="1" /><style type="text/css">
p, li { white-space: pre-wrap; }
</style></head><body style=" font-family:'Helvetica'; font-size:10px; font-weight:400; font-style:normal;">
<p style=" margin-top:0px; margin-bottom:0px; margin-left:0px; margin-right:0px; -qt-block-indent:0; text-indent:0px;"><span style=" font-size:10px; background-color:#ffff00;">(5): </span></p>
<p style=" margin-top:0px; margin-bottom:0px; margin-left:0px; margin-right:0px; -qt-block-indent:0; text-indent:0px;"><span style=" font-size:10px; background-color:#00ffff;">enable WBC</span></p>
<p style=" margin-top:0px; margin-bottom:0px; margin-left:0px; margin-right:0px; -qt-block-indent:0; text-indent:0px;"><span style=" font-size:10px; background-color:#00ffff;">(whole-body control)</span></p></body></html>
"#;
    let result = annotation_to_plain_text(html, Some("rich"));
    let expected = "(5):\nenable WBC\n(whole-body control)";
    assert_eq!(result, expected);
}

#[test]
fn test_annotation_rich_text_detects_styles() {
    let html = r#"
<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 4.0//EN" "http://www.w3.org/TR/REC-html40/strict.dtd">
<html><head><meta name="qrichtext" content="1" /></head>
<body style=" font-family:'Helvetica'; font-size:10px; font-weight:400; font-style:normal;">
<p><span style=" font-size:20px; color:#ff0000; font-weight:bold;">Hello</span><span style=" font-style:italic;"> world</span></p>
<p style="-qt-paragraph-type:empty; margin-top:0px; margin-bottom:0px;"><br/></p>
<p><span style=" background-color:#00ff00;">Done</span></p>
</body></html>
"#;

    let parsed = annotation_to_rich_text(html, Some("rich"));
    assert_eq!(parsed.lines.len(), 3);

    let first = &parsed.lines[0];
    assert!(first.is_bold());
    assert!(first.is_italic());
    assert_eq!(first.resolved_style.font_size_px, 10.0);
    assert_eq!(first.spans.len(), 2);
    assert_eq!(first.spans[0].text, "Hello");
    assert_eq!(first.spans[0].style.font_size_px, 20.0);
    assert_eq!(
        first.spans[0].style.color,
        Some(Color32::from_rgb(255, 0, 0))
    );
    assert!(first.spans[0].style.bold);
    assert!(first.spans[1].style.italic);
    assert_eq!(first.spans[1].style.font_size_px, 10.0);

    let second = &parsed.lines[1];
    assert!(second.is_empty());

    let third = &parsed.lines[2];
    assert_eq!(third.spans.len(), 1);
    assert_eq!(
        third.spans[0].style.background,
        Some(Color32::from_rgb(0, 255, 0))
    );

    let plain = annotation_to_plain_text(html, Some("rich"));
    assert_eq!(plain, "Hello world\n\nDone");
}
