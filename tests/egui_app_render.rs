use eframe::egui::{Pos2, Rect, Vec2};
use rustylink::block_types::IconSpec;
use rustylink::egui_app::{PortLabelMaxWidths, compute_icon_available_rect};

#[test]
fn icon_available_rect_respects_10_percent_margin() {
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 50.0));
    let avail = compute_icon_available_rect(&rect, 1.0, None);
    assert!((avail.left() - 10.0).abs() < 1e-6);
    assert!((avail.right() - 90.0).abs() < 1e-6);
    assert!((avail.top() - 5.0).abs() < 1e-6);
    assert!((avail.bottom() - 45.0).abs() < 1e-6);
}

#[test]
fn icon_available_rect_accounts_for_inside_port_labels() {
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 50.0));
    let avail = compute_icon_available_rect(
        &rect,
        1.0,
        Some(PortLabelMaxWidths {
            left: 30.0,
            right: 0.0,
        }),
    );
    // margin is 10.0, but label inset should win:
    // label_pad=4.0, left=30.0, gap=2.0 => 36.0.
    assert!((avail.left() - 36.0).abs() < 1e-6);
    assert!((avail.right() - 90.0).abs() < 1e-6);
    assert!(avail.center().x > rect.center().x);
}

#[test]
fn icon_available_rect_degenerates_safely_when_insets_exceed_width() {
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(50.0, 20.0));
    let avail = compute_icon_available_rect(
        &rect,
        1.0,
        Some(PortLabelMaxWidths {
            left: 1000.0,
            right: 1000.0,
        }),
    );
    assert!(avail.width() <= 0.0);
    assert!((avail.center().x - rect.center().x).abs() < 1e-6);
}

// -- tests moved from `src/egui_app/render.rs` --

#[test]
fn icon_lookup_prefers_sourceblock_over_block_type() {
    // Simulate a matrix-library block that is internally a generic Product
    // but has a library origin that should override the generic icon.
    let mut b = rustylink::editor::operations::create_default_block(
        "Product",
        "Matrix Multiply",
        0,
        0,
        2,
        1,
    );
    b.properties.insert(
        "SourceBlock".to_string(),
        "matrix_library.slx/Matrix Multiply".to_string(),
    );
    b.library_block_path = None;

    // The matrix library entry now carries its own definition (and typeset
    // icon) instead of an SVG asset, so assert the origin wins the lookup.
    let def = rustylink::simulink_libraries::resolve_definition(&b);
    assert_eq!(def.block_type, "Matrix Multiply");
}

#[test]
fn icon_lookup_accepts_normalized_slx_library_path() {
    let mut b = rustylink::editor::operations::create_default_block(
        "Product",
        "MatrixMultiply",
        0,
        0,
        2,
        1,
    );
    b.library_block_path = Some("matrix_library.slx/MatrixMultiply".to_string());

    let def = rustylink::simulink_libraries::resolve_definition(&b);
    assert_eq!(def.block_type, "Matrix Multiply");
}

/// Blocks whose SLX name uses different capitalisation than the registry key
/// (e.g. "Cross product" with a lowercase 'p') must still resolve to the
/// correct definition via the case-insensitive fallback, and must NOT fall
/// through to the generic block_type icon (the "×" Product icon).
#[test]
fn icon_lookup_cross_product_case_insensitive() {
    let mut b =
        rustylink::editor::operations::create_default_block("Product", "Cross product", 0, 0, 2, 1);
    // Simulate what the parser sets: library_block_path from SourceBlock.
    b.library_block_path = Some("matrix_library/Cross product".to_string());

    let def = rustylink::simulink_libraries::resolve_definition(&b);
    assert_eq!(def.block_type, "Cross Product");
}

#[test]
fn icon_lookup_diagonal_matrix_alias() {
    // using the shorter/legacy name as a library path should still hit
    // the same definition.  this exercises the alias support in the
    // matrix library.
    let mut blk =
        rustylink::editor::operations::create_default_block("SubSystem", "Foo", 0, 0, 1, 1);
    blk.library_block_path = Some("matrix_library/DiagonalMatrix".to_string());
    let cfg = rustylink::egui_app::get_block_type_cfg(&blk);
    let diagonal = cfg.icon.expect("diagonal matrix icon");
    assert!(matches!(diagonal, IconSpec::Plot(_)));

    // and the generic fallback via block_type (used by the catalog) also works
    let blk2 =
        rustylink::editor::operations::create_default_block("DiagonalMatrix", "Bar", 0, 0, 1, 1);
    let cfg2 = rustylink::egui_app::get_block_type_cfg(&blk2);
    assert_eq!(cfg2.icon, Some(diagonal));

    // check extract-diagonal alias as well (library path variant)
    let mut blk3 =
        rustylink::editor::operations::create_default_block("SubSystem", "Qux", 0, 0, 1, 1);
    blk3.library_block_path = Some("matrix_library/ExtractDiag".to_string());
    let cfg3 = rustylink::egui_app::get_block_type_cfg(&blk3);
    assert!(matches!(cfg3.icon, Some(IconSpec::Plot(_))));
}

#[test]
fn icon_lookup_product_matrix_multiplication_uses_matrix_definition() {
    let mut b =
        rustylink::editor::operations::create_default_block("Product", "Product1", 0, 0, 2, 1);
    b.properties
        .insert("Multiplication".to_string(), "Matrix(*)".to_string());

    // `Multiplication="Matrix(*)"` is how Simulink encodes a matrix multiply,
    // so the block resolves to the matrix library definition.
    let def = rustylink::simulink_libraries::resolve_definition(&b);
    assert_eq!(def.block_type, "Matrix Multiply");
}

#[test]
fn icon_lookup_simulink_discrete_derivative() {
    let mut b = rustylink::editor::operations::create_default_block(
        "SubSystem",
        "Discrete Derivative",
        0,
        0,
        1,
        1,
    );
    b.library_block_path = Some("simulink/Discrete/Discrete Derivative".to_string());

    // The Discrete Derivative now uses a typeset fraction icon (K(z-1)/(Ts z))
    // instead of the flat SVG, matching Simulink's block mask.
    let cfg = rustylink::egui_app::get_block_type_cfg(&b);
    assert_eq!(cfg.icon, Some(IconSpec::Math("frac:K(z-1)/Ts z")));
}

#[test]
fn icon_lookup_matrix_square_alias_square() {
    let mut b =
        rustylink::editor::operations::create_default_block("SubSystem", "Square", 0, 0, 1, 1);
    b.library_block_path = Some("matrix_library/Square".to_string());

    let cfg = rustylink::egui_app::get_block_type_cfg(&b);
    assert_eq!(cfg.icon, Some(IconSpec::Math("sup:A^H A")));
}

/// SLX XML can embed line-breaks inside long property values, e.g.
/// `SourceBlock` becomes `"matrix_library/Matrix\nSquare"`.
/// After replacing the newline with a space the path normalises to
/// `"matrix_library/Matrix Square"` whose last segment matches the registry.
#[test]
fn icon_lookup_matrix_square_newline_in_source_block() {
    let mut b = rustylink::editor::operations::create_default_block(
        "Reference",
        "Matrix Square",
        0,
        0,
        1,
        1,
    );
    // This is what the parser reads verbatim from the SLX XML.
    b.properties.insert(
        "SourceBlock".to_string(),
        "matrix_library/Matrix\nSquare".to_string(),
    );
    b.library_block_path = Some("matrix_library/Matrix\nSquare".to_string());

    let cfg = rustylink::egui_app::get_block_type_cfg(&b);
    assert_eq!(cfg.icon, Some(IconSpec::Math("sup:A^H A")));
}

#[test]
fn signal_routing_blocks_have_explicit_visible_configs() {
    for block_type in ["Mux", "Demux", "BusCreator", "BusSelector"] {
        let block =
            rustylink::editor::operations::create_default_block(block_type, block_type, 0, 0, 1, 1);
        let cfg = rustylink::egui_app::get_block_type_cfg(&block);
        assert!(cfg.known, "{block_type} should be known");
        // Simulink draws bus/mux blocks as a solid bar, so they carry the
        // FilledBlack shape and no interior icon.
        assert_eq!(cfg.icon, None, "{block_type} should have no interior icon");
        assert_eq!(
            cfg.shape,
            rustylink::simulink_libraries::types::SimulinkShape::FilledBlack
        );
    }
}

#[test]
fn complex_to_real_imag_is_drawn_by_its_renderer() {
    let block = rustylink::editor::operations::create_default_block(
        "ComplexToRealImag",
        "ComplexToRealImag",
        0,
        0,
        1,
        2,
    );
    let cfg = rustylink::egui_app::get_block_type_cfg(&block);

    assert!(cfg.known);
    // The `Re`/`Im` fork is part of the drawn icon, so there is neither a
    // static icon nor a set of fixed port labels.
    assert_eq!(cfg.icon, None);
    assert!(cfg.input_port_names.is_empty());
    assert!(cfg.output_port_names.is_empty());
    let def = rustylink::simulink_libraries::resolve_definition(&block);
    assert!(def.static_renderer.is_some());
}

/// The standalone lifecycle/control port blocks draw the same pictogram their
/// containing subsystem shows at the port they add, so none of them may fall
/// back to a text icon.
#[test]
fn lifecycle_port_blocks_draw_pictograms() {
    for block_type in ["EnablePort", "TriggerPort", "ResetPort", "EventListener"] {
        let block =
            rustylink::editor::operations::create_default_block(block_type, block_type, 0, 0, 0, 0);
        let def = rustylink::simulink_libraries::resolve_definition(&block);
        assert!(
            def.static_renderer.is_some(),
            "{block_type} must draw a pictogram"
        );
        assert_eq!(def.icon, None, "{block_type} must not draw a text icon");
    }
}

#[test]
fn display_still_hides_input_port_labels() {
    let block =
        rustylink::editor::operations::create_default_block("Display", "Display", 0, 0, 1, 0);
    let cfg = rustylink::egui_app::get_block_type_cfg(&block);

    assert!(!cfg.show_input_port_labels);
}

/// The subsystem variants of the reference model must expose exactly the
/// top-edge control ports their contents ask for: an enable and a trigger port
/// for the enabled+triggered subsystem and a reset port for the resettable
/// ones.  The reinitialize event port of a nested function subsystem is *not*
/// a top-edge port – Simulink puts it on the input side.
#[test]
fn subsystem_control_ports_come_from_the_contained_port_blocks() {
    use rustylink::model::SlxArchive;
    use rustylink::simulink_libraries::renderers::{
        subsystem_control_port_count, subsystem_event_input_count,
    };

    let file = std::fs::File::open("simulink_test_models/Simulink_Blocks.slx")
        .expect("open Simulink_Blocks.slx");
    let archive = SlxArchive::from_reader(std::io::BufReader::new(file)).expect("read archive");
    let system = archive.assembled_root_system().expect("assemble root");

    let count_of = |name: &str| {
        system
            .blocks
            .iter()
            .find(|b| b.name == name)
            .map(subsystem_control_port_count)
            .unwrap_or_else(|| panic!("{name} missing from the model"))
    };

    assert_eq!(count_of("Subsystem"), 0);
    assert_eq!(count_of("Enabled Subsystem"), 1);
    assert_eq!(count_of("Enabled and Triggered Subsystem"), 2);
    assert_eq!(count_of("Resettable Subsystem"), 1);
    assert_eq!(count_of("Atomic Subsystem with Reinit"), 0);

    let events_of = |name: &str| {
        system
            .blocks
            .iter()
            .find(|b| b.name == name)
            .map(subsystem_event_input_count)
            .unwrap_or_else(|| panic!("{name} missing from the model"))
    };
    assert_eq!(events_of("Atomic Subsystem with Reinit"), 1);
    assert_eq!(events_of("Subsystem"), 0);
}

/// The reinitialize port of `Atomic Subsystem with Reinit` (which carries
/// `ShowSubsystemReinitializePorts = on`) sits at a fixed position in its own
/// top section, the data inputs are distributed below the separator line, and
/// all three are on the left edge.
#[test]
fn reinit_event_port_sits_above_the_data_inputs() {
    use eframe::egui::{Pos2, Rect};
    use rustylink::egui_app::ui::signal_routing::{
        compute_port_info, endpoint_pos, is_reinit_subsystem_counts, REINIT_PORT_FRAC,
        REINIT_SEP_FRAC,
    };
    use rustylink::model::{EndpointRef, SlxArchive};

    let file = std::fs::File::open("simulink_test_models/Simulink_Blocks.slx")
        .expect("open Simulink_Blocks.slx");
    let archive = SlxArchive::from_reader(std::io::BufReader::new(file)).expect("read archive");
    let system = archive.assembled_root_system().expect("assemble root");

    let block = system
        .blocks
        .iter()
        .find(|b| b.name == "Atomic Subsystem with Reinit")
        .expect("subsystem missing from the model");
    let sid = block.sid.clone().expect("subsystem has a SID");
    let (counts, _) = compute_port_info(&system.lines, &system.blocks);
    assert!(
        is_reinit_subsystem_counts(&counts, &sid),
        "block should be flagged as a reinit subsystem"
    );
    let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 120.0));
    let at = |port_type: &str, index: u32| {
        endpoint_pos(
            rect,
            &EndpointRef {
                sid: sid.clone(),
                port_index: index,
                port_type: port_type.to_string(),
            },
            &counts,
            false,
            &[],
        )
    };

    let event = at("event", 1);
    let in1 = at("in", 1);
    let in2 = at("in", 2);
    // All three on the left edge.
    for p in [event, in1, in2] {
        assert_eq!(p.x, rect.left());
    }
    // The reinit port sits at the fixed fraction in the top section.
    let expected_event_y = rect.top() + REINIT_PORT_FRAC * rect.height();
    assert!(
        (event.y - expected_event_y).abs() < 0.5,
        "event {event:?} should be at y={expected_event_y}"
    );
    // Data inputs are below the separator line.
    let sep_y = rect.top() + REINIT_SEP_FRAC * rect.height();
    assert!(in1.y > sep_y, "in1 {in1:?} must be below separator y={sep_y}");
    assert!(in2.y > sep_y, "in2 {in2:?} must be below separator y={sep_y}");
    assert!(in1.y < in2.y, "in1 {in1:?} must sit above {in2:?}");
}

/// Control ports are numbered per type, so an `enable:1` and a `trigger:1`
/// endpoint carry the same index while sitting on different slots of the top
/// edge.  They must land on the pictogram of their own port, left to right.
#[test]
fn enable_and_trigger_endpoints_land_on_different_top_edge_slots() {
    use eframe::egui::{Pos2, Rect};
    use rustylink::egui_app::ui::signal_routing::{compute_port_info, endpoint_pos};
    use rustylink::model::{EndpointRef, SlxArchive};

    let file = std::fs::File::open("simulink_test_models/Simulink_Blocks.slx")
        .expect("open Simulink_Blocks.slx");
    let archive = SlxArchive::from_reader(std::io::BufReader::new(file)).expect("read archive");
    let system = archive.assembled_root_system().expect("assemble root");

    let block = system
        .blocks
        .iter()
        .find(|b| b.name == "Enabled and Triggered Subsystem")
        .expect("enabled and triggered subsystem missing from the model");
    let sid = block.sid.clone().expect("subsystem has a SID");

    let (port_counts, _connected) = compute_port_info(&[], std::slice::from_ref(block));
    let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(90.0, 60.0));
    let control = |port_type: &str| EndpointRef {
        sid: sid.clone(),
        port_type: port_type.to_string(),
        port_index: 1,
    };

    let enable = endpoint_pos(rect, &control("enable"), &port_counts, false, &[]);
    let trigger = endpoint_pos(rect, &control("trigger"), &port_counts, false, &[]);

    assert_eq!(enable.y, rect.top());
    assert_eq!(trigger.y, rect.top());
    assert_eq!(enable.x, 30.0);
    assert_eq!(trigger.x, 60.0);
}

/// A round Sum places its last input on the bottom edge via a
/// `PortPositionOverride`.  `endpoint_pos` must honour that override so line
/// endpoints land on the bottom edge, not the default left-edge slot.
#[test]
fn round_sum_last_input_endpoint_uses_bottom_override() {
    use eframe::egui::{Pos2, Rect};
    use rustylink::egui_app::ui::signal_routing::{compute_port_info, endpoint_pos};
    use rustylink::model::EndpointRef;
    use rustylink::simulink_libraries::libraries::core::ROUND_SUM_PORT_OVERRIDES;

    // Build a synthetic round-Sum-like block with 4 inputs.
    let mut block = rustylink::editor::operations::create_default_block("Sum", "Sum4", 0, 0, 4, 1);
    block.sid = Some("479".to_string());
    block.properties
        .insert("IconShape".to_string(), "round".to_string());
    block.properties
        .insert("Inputs".to_string(), "-+++".to_string());

    let (port_counts, _) = compute_port_info(&[], std::slice::from_ref(&block));
    let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(20.0, 20.0));

    // Port 4 is the last input; the override places it on the bottom edge at
    // fraction 0.5, i.e. (10, 20).
    let ep = EndpointRef {
        sid: "479".to_string(),
        port_type: "in".to_string(),
        port_index: 4,
    };
    let pos = endpoint_pos(rect, &ep, &port_counts, false, ROUND_SUM_PORT_OVERRIDES);
    assert_eq!(pos.x, 10.0, "bottom port x should be centered");
    assert_eq!(pos.y, 20.0, "bottom port y should be at rect.bottom()");

    // Without the override, port 4 would be on the left edge.
    let pos_no_override = endpoint_pos(rect, &ep, &port_counts, false, &[]);
    assert_eq!(
        pos_no_override.x, 0.0,
        "without override, port 4 should be on the left edge"
    );
}

/// Control port endpoints without an explicit port index (e.g. `"480#enable"`)
/// must parse successfully and default to port index 1.
#[test]
fn parse_endpoint_control_port_without_index_defaults_to_1() {
    use rustylink::parser::helpers::parse_endpoint;

    let ep = parse_endpoint("480#enable").expect("control port endpoint should parse");
    assert_eq!(ep.sid, "480");
    assert_eq!(ep.port_type, "enable");
    assert_eq!(ep.port_index, 1);

    // Standard format still works.
    let ep2 = parse_endpoint("18#out:1").expect("standard endpoint should parse");
    assert_eq!(ep2.sid, "18");
    assert_eq!(ep2.port_type, "out");
    assert_eq!(ep2.port_index, 1);
}

#[test]
fn inport_shadow_resolves_to_same_definition_as_inport() {
    use rustylink::simulink_libraries::resolve_definition;
    use rustylink::simulink_libraries::types::SimulinkShape;

    let block = rustylink::editor::operations::create_default_block(
        "InportShadow",
        "shadow",
        0,
        0,
        0,
        1,
    );
    let def = resolve_definition(&block);
    assert_eq!(def.block_type, "InportShadow");
    // Same shape as Inport
    assert_eq!(def.shape, SimulinkShape::Obround);
    // Same port topology: 0 inputs, 1 output
    assert_eq!(def.inputs.default_count(), 0);
    assert_eq!(def.outputs.default_count(), 1);
}
