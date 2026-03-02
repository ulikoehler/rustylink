//! Tests for Simulink Dashboard / UI block detection, parsing, and interaction.
//!
//! These tests verify that:
//! 1. Dashboard block types are correctly identified by `is_dashboard_block_type()`
//! 2. The `Simulink_UI_Test.slx` model is parsed correctly, finding all UI blocks
//! 3. BindingPersistence `Ref` properties are resolved via `blockdiagram.xml.rels`
//! 4. Display blocks have the correct port counts (1 input, 0 outputs)
//! 5. Other dashboard blocks have 0 ports

use rustylink::builtin_libraries::simulink_dashboard::{
    is_dashboard_block_type, is_simulink_dashboard_name, DASHBOARD_BLOCK_TYPES,
};
use rustylink::model::{parse_rels_xml, SlxArchive};
use rustylink::parser::{SimulinkParser, ZipSource};

// ── is_dashboard_block_type ────────────────────────────────────────────────

#[test]
fn recognizes_all_known_dashboard_types() {
    for &bt in DASHBOARD_BLOCK_TYPES {
        assert!(
            is_dashboard_block_type(bt),
            "expected '{}' to be recognized",
            bt
        );
    }
}

#[test]
fn dashboard_type_is_case_insensitive() {
    assert!(is_dashboard_block_type("checkbox"));
    assert!(is_dashboard_block_type("CHECKBOX"));
    assert!(is_dashboard_block_type("CheckBox"));
    assert!(is_dashboard_block_type("dashboardscope"));
    assert!(is_dashboard_block_type("DASHBOARDSCOPE"));
}

#[test]
fn non_dashboard_types_rejected() {
    assert!(!is_dashboard_block_type("Gain"));
    assert!(!is_dashboard_block_type("Sum"));
    assert!(!is_dashboard_block_type("SubSystem"));
    assert!(!is_dashboard_block_type("Constant"));
    assert!(!is_dashboard_block_type(""));
}

// ── is_simulink_dashboard_name ─────────────────────────────────────────────

#[test]
fn dashboard_library_name_matching() {
    assert!(is_simulink_dashboard_name("simulink/Dashboard"));
    assert!(is_simulink_dashboard_name("simulink/dashboard"));
    assert!(is_simulink_dashboard_name("Simulink/Dashboard/Scope"));
    assert!(is_simulink_dashboard_name("simulink\\Dashboard"));
    assert!(!is_simulink_dashboard_name("simulink/Math Operations"));
    assert!(!is_simulink_dashboard_name("other"));
}

// ── parse_rels_xml ─────────────────────────────────────────────────────────

#[test]
fn parse_rels_xml_basic() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="BindingPersistence_151" Target="bdmxdata/BindingPersistence_151.mxarray" Type="http://schemas.mathworks.com/simulinkModel/2015/relationships/modelMxArray"/>
  <Relationship Id="Colors_167" Target="bdmxdata/Colors_167.mxarray" Type="http://schemas.mathworks.com/simulinkModel/2015/relationships/modelMxArray"/>
  <Relationship Id="system_root" Target="systems/system_root.xml" Type="http://schemas.mathworks.com/simulink/2010/relationships/system"/>
</Relationships>"#;

    let rels = parse_rels_xml(xml);
    assert_eq!(rels.len(), 3);

    let bp151 = rels.iter().find(|r| r.id == "BindingPersistence_151");
    assert!(bp151.is_some());
    let bp151 = bp151.unwrap();
    assert_eq!(bp151.target, "bdmxdata/BindingPersistence_151.mxarray");
    assert!(bp151
        .relationship_type
        .contains("modelMxArray"));

    let sys = rels.iter().find(|r| r.id == "system_root").unwrap();
    assert_eq!(sys.target, "systems/system_root.xml");
}

#[test]
fn parse_rels_xml_empty() {
    let xml = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"></Relationships>"#;
    let rels = parse_rels_xml(xml);
    assert!(rels.is_empty());
}

#[test]
fn parse_rels_xml_malformed_returns_empty() {
    let rels = parse_rels_xml("not xml at all");
    assert!(rels.is_empty());
}

// ── SlxArchive parsing of Simulink_UI_Test.slx ────────────────────────────

/// Helper: load the test model. Returns `None` if the file is not present
/// (allows CI without the slx file).
fn load_ui_test_archive() -> Option<SlxArchive> {
    let path = std::path::Path::new("Simulink_UI_Test.slx");
    if !path.exists() {
        return None;
    }
    Some(SlxArchive::from_file(path).expect("failed to load Simulink_UI_Test.slx"))
}

#[test]
fn archive_contains_blockdiagram_rels() {
    let archive = match load_ui_test_archive() {
        Some(a) => a,
        None => return, // skip when file absent
    };
    assert!(
        !archive.relationships.is_empty(),
        "expected parsed relationships from blockdiagram.xml.rels"
    );
    // Spot-check a known relationship id.
    assert!(archive.relationships.contains_key("BindingPersistence_151"));
    assert!(archive.relationships.contains_key("system_root"));
}

#[test]
fn archive_resolves_binding_persistence_refs() {
    let archive = match load_ui_test_archive() {
        Some(a) => a,
        None => return,
    };

    // resolve_ref should map a bdmxdata:ID to a file path
    let path = archive.resolve_ref("bdmxdata:BindingPersistence_151");
    assert_eq!(
        path.as_deref(),
        Some("simulink/bdmxdata/BindingPersistence_151.mxarray")
    );

    // resolve_binding_persistence should locate the raw bytes
    let data = archive.resolve_binding_persistence("bdmxdata:BindingPersistence_151");
    assert!(data.is_some(), "Expected raw .mxarray data");
    assert!(data.unwrap().len() > 0);
}

#[test]
fn archive_resolves_colors_ref() {
    let archive = match load_ui_test_archive() {
        Some(a) => a,
        None => return,
    };
    let path = archive.resolve_ref("bdmxdata:Colors_167");
    assert_eq!(
        path.as_deref(),
        Some("simulink/bdmxdata/Colors_167.mxarray")
    );
}

// ── System-level UI block detection ────────────────────────────────────────

#[test]
fn root_system_contains_all_ui_block_types() {
    let archive = match load_ui_test_archive() {
        Some(a) => a,
        None => return,
    };
    let system = archive
        .root_system()
        .expect("expected root system in archive");

    // Collect all block types
    let block_types: Vec<&str> = system
        .blocks
        .iter()
        .map(|b| b.block_type.as_str())
        .collect();

    // Every expected dashboard block type from the model should be present
    let expected = [
        "Checkbox",
        "ComboBox",
        "DashboardScope",
        "Display",
        "DisplayBlock",
        "EditField",
        "CircularGaugeBlock",
        "SemiCircularGaugeBlock",
        "KnobBlock",
        "LampBlock",
        "LinearGaugeBlock",
        "PushButtonBlock",
        "QuarterGaugeBlock",
        "RadioButtonGroup",
        "RockerSwitchBlock",
        "RotarySwitchBlock",
        "SliderBlock",
        "SliderSwitchBlock",
        "ToggleSwitchBlock",
    ];

    for &bt in &expected {
        assert!(
            block_types.contains(&bt),
            "Block type '{}' not found in model. Found types: {:?}",
            bt,
            block_types
        );
    }
}

#[test]
fn display_blocks_have_one_input() {
    let archive = match load_ui_test_archive() {
        Some(a) => a,
        None => return,
    };
    let system = archive.root_system().unwrap();

    for blk in &system.blocks {
        if blk.block_type == "Display" {
            let ins = blk
                .port_counts
                .as_ref()
                .and_then(|pc| pc.ins)
                .unwrap_or(0);
            let outs = blk
                .port_counts
                .as_ref()
                .and_then(|pc| pc.outs)
                .unwrap_or(0);
            assert_eq!(
                ins, 1,
                "Display block '{}' should have 1 input but has {}",
                blk.name, ins
            );
            assert_eq!(
                outs, 0,
                "Display block '{}' should have 0 outputs but has {}",
                blk.name, outs
            );
        }
    }
}

#[test]
fn dashboard_blocks_have_zero_ports_except_display() {
    let archive = match load_ui_test_archive() {
        Some(a) => a,
        None => return,
    };
    let system = archive.root_system().unwrap();

    let zero_port_types = [
        "Checkbox",
        "ComboBox",
        "DashboardScope",
        "DisplayBlock",
        "EditField",
        "CircularGaugeBlock",
        "SemiCircularGaugeBlock",
        "KnobBlock",
        "LampBlock",
        "LinearGaugeBlock",
        "PushButtonBlock",
        "QuarterGaugeBlock",
        "RadioButtonGroup",
        "RockerSwitchBlock",
        "RotarySwitchBlock",
        "SliderBlock",
        "SliderSwitchBlock",
        "ToggleSwitchBlock",
    ];

    for blk in &system.blocks {
        if zero_port_types.contains(&blk.block_type.as_str()) {
            let ins = blk
                .port_counts
                .as_ref()
                .and_then(|pc| pc.ins)
                .unwrap_or(0);
            let outs = blk
                .port_counts
                .as_ref()
                .and_then(|pc| pc.outs)
                .unwrap_or(0);
            assert_eq!(
                ins, 0,
                "Dashboard block '{}' (type={}) should have 0 inputs but has {}",
                blk.name, blk.block_type, ins
            );
            assert_eq!(
                outs, 0,
                "Dashboard block '{}' (type={}) should have 0 outputs but has {}",
                blk.name, blk.block_type, outs
            );
        }
    }
}

#[test]
fn ui_blocks_have_binding_persistence_ref() {
    let archive = match load_ui_test_archive() {
        Some(a) => a,
        None => return,
    };
    let system = archive.root_system().unwrap();

    // All these block types should have a BindingPersistence property with Ref attr
    let binding_types = [
        "Checkbox",
        "ComboBox",
        "DashboardScope",
        "DisplayBlock",
        "EditField",
        "CircularGaugeBlock",
        "SemiCircularGaugeBlock",
        "KnobBlock",
        "LampBlock",
        "LinearGaugeBlock",
        "PushButtonBlock",
        "QuarterGaugeBlock",
        "RadioButtonGroup",
        "RockerSwitchBlock",
        "RotarySwitchBlock",
        "SliderBlock",
        "SliderSwitchBlock",
        "ToggleSwitchBlock",
    ];

    for blk in &system.blocks {
        if binding_types.contains(&blk.block_type.as_str()) {
            assert!(
                blk.properties.contains_key("BindingPersistence"),
                "Dashboard block '{}' (type={}) should have BindingPersistence property",
                blk.name,
                blk.block_type,
            );
            assert!(
                blk.ref_properties.contains("BindingPersistence"),
                "Dashboard block '{}' (type={}) BindingPersistence should be a Ref property",
                blk.name,
                blk.block_type,
            );
            // The value should start with "bdmxdata:"
            let val = blk.properties.get("BindingPersistence").unwrap();
            assert!(
                val.starts_with("bdmxdata:"),
                "BindingPersistence ref for '{}' should start with 'bdmxdata:', got '{}'",
                blk.name,
                val,
            );
            // And it should resolve through the archive relationships
            let resolved = archive.resolve_ref(val);
            assert!(
                resolved.is_some(),
                "Failed to resolve BindingPersistence ref '{}' for block '{}'",
                val,
                blk.name,
            );
        }
    }
}

// ── Full parser-based detection test ───────────────────────────────────────

#[test]
fn parser_detects_ui_blocks_from_slx() {
    let path = std::path::Path::new("Simulink_UI_Test.slx");
    if !path.exists() {
        return;
    }
    let file = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);
    let mut parser =
        SimulinkParser::new("", ZipSource::new(reader).unwrap());
    let root = camino::Utf8PathBuf::from("simulink/systems/system_root.xml");
    let system = parser.parse_system_file(&root).unwrap();

    // Should find at least one of each dashboard block type
    let mut found: std::collections::HashSet<String> = std::collections::HashSet::new();
    for blk in &system.blocks {
        if is_dashboard_block_type(&blk.block_type) {
            found.insert(blk.block_type.clone());
        }
    }
    // We expect to find all types listed in the test model
    let expected = [
        "Checkbox",
        "ComboBox",
        "DashboardScope",
        "Display",
        "DisplayBlock",
        "EditField",
        "CircularGaugeBlock",
        "SemiCircularGaugeBlock",
        "KnobBlock",
        "LampBlock",
        "LinearGaugeBlock",
        "PushButtonBlock",
        "QuarterGaugeBlock",
        "RadioButtonGroup",
        "RockerSwitchBlock",
        "RotarySwitchBlock",
        "SliderBlock",
        "SliderSwitchBlock",
        "ToggleSwitchBlock",
    ];
    for &bt in &expected {
        assert!(
            found.contains(bt),
            "Parser did not detect dashboard block type '{}'. Found: {:?}",
            bt,
            found
        );
    }
}

// ── Interaction: PushButton, RadioButton, ComboBox values ──────────────────

#[test]
fn push_button_has_button_text() {
    let archive = match load_ui_test_archive() {
        Some(a) => a,
        None => return,
    };
    let system = archive.root_system().unwrap();

    let btn = system
        .blocks
        .iter()
        .find(|b| b.block_type == "PushButtonBlock")
        .expect("PushButtonBlock not found");
    assert_eq!(
        btn.properties.get("ButtonText").map(|s| s.as_str()),
        Some("Reset"),
        "PushButton should have ButtonText='Reset'"
    );
}

#[test]
fn radio_button_has_values_property() {
    let archive = match load_ui_test_archive() {
        Some(a) => a,
        None => return,
    };
    let system = archive.root_system().unwrap();

    let rb = system
        .blocks
        .iter()
        .find(|b| b.block_type == "RadioButtonGroup")
        .expect("RadioButtonGroup not found");
    // RadioButtonGroup uses an Array/Cell structure for Values.
    // The block name should be "RadioButton UI".
    assert_eq!(rb.name, "RadioButton UI");
    // It should have a ButtonGroupName property.
    assert_eq!(
        rb.properties.get("ButtonGroupName").map(|s| s.as_str()),
        Some("Group")
    );
}

#[test]
fn dashboard_scope_has_foreground_and_font_color() {
    let archive = match load_ui_test_archive() {
        Some(a) => a,
        None => return,
    };
    let system = archive.root_system().unwrap();

    let scope = system
        .blocks
        .iter()
        .find(|b| b.block_type == "DashboardScope")
        .expect("DashboardScope not found");
    assert!(
        scope.properties.contains_key("ForegroundColor"),
        "DashboardScope should have ForegroundColor"
    );
    // FontColor uses Class="double" attribute but value is stored
    assert!(
        scope.properties.contains_key("FontColor"),
        "DashboardScope should have FontColor"
    );
}

#[test]
fn lamp_block_has_states() {
    let archive = match load_ui_test_archive() {
        Some(a) => a,
        None => return,
    };
    let system = archive.root_system().unwrap();

    let lamp = system
        .blocks
        .iter()
        .find(|b| b.block_type == "LampBlock")
        .expect("LampBlock not found");
    assert_eq!(lamp.name, "Lamp");
    assert!(
        lamp.properties.contains_key("DefaultColor"),
        "LampBlock should have DefaultColor"
    );
}

// ── Virtual library name matching for parser detection ─────────────────────

#[test]
fn dashboard_virtual_library_is_recognized() {
    use rustylink::parser::is_virtual_library;
    assert!(is_virtual_library("simulink/Dashboard"));
    assert!(is_virtual_library("simulink/dashboard"));
    assert!(is_virtual_library("Simulink/Dashboard/Foo"));
}
// ── Visualization: custom renderers registered for all dashboard blocks ────

#[cfg(feature = "egui")]
mod visualization_tests {
    use rustylink::egui_app::dashboard_widgets;

    /// All dashboard block types that are "UI elements" (i.e. should have a
    /// dedicated custom renderer instead of displaying a "?" fallback).
    const UI_BLOCK_TYPES: &[&str] = &[
        "PushButtonBlock",
        "SliderSwitchBlock",
        "RadioButtonGroup",
        "ComboBox",
        "Checkbox",
        "SliderBlock",
        "EditField",
        "ToggleSwitchBlock",
        "KnobBlock",
        "RockerSwitchBlock",
        "RotarySwitchBlock",
        "QuarterGaugeBlock",
        "SemiCircularGaugeBlock",
        "LinearGaugeBlock",
        "DashboardScope",
        "DisplayBlock",
        "CircularGaugeBlock",
        "LampBlock",
    ];

    #[test]
    fn all_ui_block_types_have_custom_renderer() {
        for &bt in UI_BLOCK_TYPES {
            assert!(
                dashboard_widgets::is_dashboard_rendered(bt),
                "Block type '{}' should have a custom dashboard renderer but none is registered",
                bt
            );
        }
    }

    #[test]
    fn all_ui_block_types_registered_in_interior_registry() {
        use rustylink::egui_app::get_interior_renderer;
        for &bt in UI_BLOCK_TYPES {
            assert!(
                get_interior_renderer(bt).is_some(),
                "Block type '{}' should be in the interior renderer registry",
                bt
            );
        }
    }

    #[test]
    fn dashboard_renderers_table_has_correct_count() {
        // We expect exactly 18 dashboard renderers (all UI block types above)
        assert_eq!(
            dashboard_widgets::DASHBOARD_RENDERERS.len(),
            UI_BLOCK_TYPES.len(),
            "DASHBOARD_RENDERERS should contain exactly {} entries (one per UI block type)",
            UI_BLOCK_TYPES.len()
        );
    }

    #[test]
    fn all_declared_renderers_match_known_block_types() {
        // Every entry in DASHBOARD_RENDERERS should be a known dashboard block type
        for &(bt, _) in dashboard_widgets::DASHBOARD_RENDERERS {
            assert!(
                rustylink::builtin_libraries::simulink_dashboard::is_dashboard_block_type(bt),
                "Renderer registered for '{}' but it's not a known dashboard block type",
                bt
            );
        }
    }

    #[test]
    fn block_type_config_has_known_true_for_all_dashboard_types() {
        let map = rustylink::block_types::get_block_type_config_map();
        let guard = map.read().unwrap();
        for &bt in UI_BLOCK_TYPES {
            let cfg = guard.get(bt);
            assert!(
                cfg.is_some(),
                "Block type '{}' should be in the config map",
                bt
            );
            assert!(
                cfg.unwrap().known,
                "Block type '{}' should have known=true in config map",
                bt
            );
        }
    }

    #[test]
    fn block_type_config_preserves_icon_after_virtual_lib_registration() {
        // This tests the icon overwrite fix: dashboard blocks have UTF-8 icons
        // set in default_registry, and the virtual library registration loop
        // should preserve them instead of overwriting with None.
        let map = rustylink::block_types::get_block_type_config_map();
        let guard = map.read().unwrap();

        // All dashboard blocks (except Display which has separate special handling)
        // should still have their UTF-8 icon set from the default_registry.
        for &bt in UI_BLOCK_TYPES {
            let cfg = guard.get(bt);
            assert!(
                cfg.is_some(),
                "Block type '{}' should be in the config map",
                bt
            );
            let cfg = cfg.unwrap();
            assert!(
                cfg.icon.is_some(),
                "Block type '{}' should still have an icon (the virtual library registration should not have overwritten it with None)",
                bt
            );
        }
    }

    #[test]
    fn no_dashboard_block_renders_as_question_mark() {
        // Build a fake block for each dashboard type and verify it would not
        // use the "?" fallback path in the rendering logic.
        //
        // The "?" path is reached when:
        //   1. The block is not Constant / masked / has value / Display
        //   2. The block type has no custom interior renderer
        //   3. render_block_icon finds no icon in the config
        //
        // With our changes, all dashboard types have both:
        //   - A custom interior renderer (checked above)
        //   - A non-None icon in the config (checked above)
        //
        // This is a structural check: verify both conditions hold.
        use rustylink::egui_app::get_interior_renderer;
        let map = rustylink::block_types::get_block_type_config_map();
        let guard = map.read().unwrap();

        for &bt in UI_BLOCK_TYPES {
            let has_renderer = get_interior_renderer(bt).is_some();
            let has_icon = guard.get(bt).and_then(|c| c.icon.as_ref()).is_some();

            assert!(
                has_renderer || has_icon,
                "Block type '{}' would render as '?' - it has neither a custom renderer nor an icon",
                bt
            );
            // Prefer renderer (our primary fix)
            assert!(
                has_renderer,
                "Block type '{}' should have a custom interior renderer for proper visualization",
                bt
            );
        }
    }

    #[test]
    fn display_block_type_also_has_icon() {
        // The standard "Display" block (not DisplayBlock) has special handling
        // in update.rs that shows the connected signal label, but it should
        // still have a fallback icon.
        let map = rustylink::block_types::get_block_type_config_map();
        let guard = map.read().unwrap();
        let cfg = guard.get("Display").expect("Display should be in config map");
        assert!(cfg.icon.is_some(), "Display should have an icon");
        assert!(cfg.known, "Display should be known");
        assert_eq!(cfg.default_ins, 1, "Display should have 1 default input");
    }

    #[test]
    fn sum_block_renderer_still_works() {
        // Sanity: the pre-existing Sum renderer should not be affected
        // by adding dashboard renderers.
        use rustylink::egui_app::get_interior_renderer;
        assert!(
            get_interior_renderer("Sum").is_some(),
            "Sum block should still have its custom interior renderer"
        );
    }

    /// Verify that the model's actual block types from the SLX file will
    /// go through the custom renderer path (not the "?" path) by checking
    /// all dashboard blocks parsed from the model.
    #[test]
    fn parsed_model_blocks_have_renderers() {
        use rustylink::egui_app::get_interior_renderer;

        let path = std::path::Path::new("Simulink_UI_Test.slx");
        if !path.exists() {
            return;
        }
        let archive = rustylink::model::SlxArchive::from_file(path)
            .expect("failed to load SLX");
        let system = archive.root_system().expect("no root system");

        for blk in &system.blocks {
            if rustylink::builtin_libraries::simulink_dashboard::is_dashboard_block_type(
                &blk.block_type,
            ) && blk.block_type != "Display"
            {
                assert!(
                    get_interior_renderer(&blk.block_type).is_some(),
                    "Model block '{}' (type={}) should have a custom renderer, \
                     but none is registered. It will render as '?'!",
                    blk.name,
                    blk.block_type
                );
            }
        }
    }
}