#![cfg(feature = "egui")]

use rustylink::egui_app::highlight_query_job;

use rustylink::egui_app::text::annotation_to_plain_text;

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
