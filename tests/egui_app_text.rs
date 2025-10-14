#![cfg(feature = "egui")]

use rustylink::egui_app::highlight_query_job;

#[test]
fn test_highlight_job() {
    let job = highlight_query_job("/A/B", "b");
    assert!(job.sections.len() >= 1);
}
