use eframe::egui::{Pos2, Rect, Vec2};
use rustylink::block_types::IconSpec;
use rustylink::egui_app::{PortLabelMaxWidths, compute_icon_available_rect, icon_assets};

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

#[test]
fn embedded_svg_assets_exist() {
    for path in &[
        "matrix/identity_matrix.svg",
        "matrix/is_triangular.svg",
        "matrix/is_symmetric.svg",
        "matrix/matrix_product.svg",
        "matrix/cross_product.svg",
        "matrix/submatrix.svg",
        "matrix/create_diagonal_matrix.svg",
        "matrix/expand_scalar_to_matrix.svg",
        "matrix/extract_diagonal.svg",
    ] {
        let bytes = icon_assets::get(path);
        assert!(bytes.is_some(), "missing asset {}", path);
    }
}

#[test]
fn svg_parse_extract_diagonal_embedded() {
    // Ensure the new icon actually parses, catching any embedding or SVG errors.
    let bytes = icon_assets::get("matrix/extract_diagonal.svg").unwrap();
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(&bytes, &options).unwrap();
    assert!(tree.size().width() > 0.0 && tree.size().height() > 0.0);
}

#[test]
fn block_type_registry_contains_matrix_library_icons() {
    use rustylink::block_types::IconSpec;
    use rustylink::simulink_libraries::types::SimulinkIcon;
    let map = rustylink::block_types::get_block_type_config_map();
    let r = map.read().unwrap();
    for def in rustylink::simulink_libraries::libraries::matrix::BLOCKS {
        if let Some(SimulinkIcon::Svg(icon)) = def.icon {
            assert_eq!(
                r.get(def.block_type).and_then(|c| c.icon),
                Some(IconSpec::Svg(icon)),
                "registry entry for {}",
                def.block_type,
            );
        }
    }
}

/// Verify that `create_stub_block` blocks (as produced by `initial_system()`)
/// resolve to their SVG icon via Phase 4 (block_type lookup) when
/// `library_block_path` is `None`.  This is the code path exercised when the
/// virtual-library browser displays the library grid directly.  The three
/// blocks that prompted this test are "Identity Matrix", "Is Triangular", and
/// "Is Symmetric".
#[test]
fn icon_lookup_stub_block_initial_system_no_library_path() {
    use rustylink::block_types::IconSpec;
    use rustylink::simulink_libraries::stubs;
    use rustylink::simulink_libraries::types::SimulinkIcon;
    for def in rustylink::simulink_libraries::libraries::matrix::BLOCKS {
        if let Some(SimulinkIcon::Svg(icon)) = def.icon {
            let (ins, outs) = stubs::matrix_port_counts_if_known(def.block_type).unwrap_or((1, 1));
            let blk = stubs::create_stub_block(def.block_type, ins, outs);
            assert!(
                blk.library_block_path.is_none(),
                "stub block should have no library_block_path"
            );
            let cfg = rustylink::egui_app::get_block_type_cfg(&blk);
            assert_eq!(
                cfg.icon,
                Some(IconSpec::Svg(icon)),
                "stub block '{}' (Phase 4 lookup) should resolve to icon '{}'",
                def.block_type,
                icon
            );
        }
    }
}

/// Ensure identity_matrix.svg (which contains <text> elements, unlike the
/// purely-geometric sibling icons) parses and rasterizes without error.
#[test]
fn svg_rasterization_identity_matrix() {
    let bytes = icon_assets::get("matrix/identity_matrix.svg").unwrap();
    let options = usvg::Options::default();
    // Parsing must succeed.
    let tree = usvg::Tree::from_data(&bytes, &options).unwrap();
    assert!(tree.size().width() > 0.0 && tree.size().height() > 0.0);
    // Rasterisation must also succeed.
    let image = egui_extras::image::load_svg_bytes_with_size(
        &bytes,
        egui::SizeHint::Size {
            width: 128,
            height: 128,
            maintain_aspect_ratio: true,
        },
        &options,
    )
    .unwrap();
    assert!(image.size[0] > 0 && image.size[1] > 0);
}

#[test]
fn svg_rasterization_preserves_aspect_ratio() {
    let bytes = icon_assets::get("matrix/is_triangular.svg").unwrap();
    let options = usvg::Options::default();

    let tree = usvg::Tree::from_data(&bytes, &options).unwrap();
    let src_w = tree.size().width();
    let src_h = tree.size().height();
    let src_ratio = src_w / src_h;

    let image = egui_extras::image::load_svg_bytes_with_size(
        &bytes,
        egui::SizeHint::Size {
            width: 128,
            height: 64,
            maintain_aspect_ratio: true,
        },
        &options,
    )
    .unwrap();

    assert!(image.size[0] <= 128);
    assert!(image.size[1] <= 64);
    let out_ratio = image.size[0] as f32 / image.size[1] as f32;
    assert!((out_ratio - src_ratio).abs() < 0.02);
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
/// correct SVG icon via the case-insensitive fallback, and must NOT fall
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
fn icon_lookup_matrix_library_icons() {
    use rustylink::simulink_libraries::types::SimulinkIcon;
    for def in rustylink::simulink_libraries::libraries::matrix::BLOCKS {
        if let Some(SimulinkIcon::Svg(icon)) = def.icon {
            let mut blk = rustylink::editor::operations::create_default_block(
                "SubSystem",
                def.block_type,
                0,
                0,
                1,
                1,
            );
            blk.library_block_path = Some(format!("matrix_library/{}", def.block_type));
            let cfg = rustylink::egui_app::get_block_type_cfg(&blk);
            assert_eq!(
                cfg.icon,
                Some(IconSpec::Svg(icon)),
                "block {}",
                def.block_type
            );
        }
    }
}

#[test]
fn icon_lookup_diagonal_matrix_alias() {
    // using the shorter/legacy name as a library path should still hit
    // the same SVG icon.  this exercises the alias support we just added
    // to the matrix library.
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
fn svg_parse_matrix_square_embedded() {
    let bytes = rustylink::egui_app::icon_assets::get("matrix/matrix_square.svg")
        .expect("matrix_square.svg must be embedded");

    let mut options = usvg::Options::default();
    // Keep consistent with runtime behavior: populate the font DB if possible.
    if let Some(db) = rustylink::egui_app::embedded_egui_sans_fontdb() {
        options.fontdb = db;
        options.font_family = "sans-serif".to_owned();
    }

    let image = egui_extras::image::load_svg_bytes_with_size(
        &bytes,
        egui::SizeHint::Size {
            width: 256,
            height: 256,
            maintain_aspect_ratio: true,
        },
        &options,
    )
    .expect("matrix_square.svg must parse");
    assert!(image.size[0] > 0 && image.size[1] > 0);
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

#[test]
fn display_still_hides_input_port_labels() {
    let block =
        rustylink::editor::operations::create_default_block("Display", "Display", 0, 0, 1, 0);
    let cfg = rustylink::egui_app::get_block_type_cfg(&block);

    assert!(!cfg.show_input_port_labels);
}

/// The subsystem variants of the reference model must expose exactly the
/// top-edge control ports their contents ask for: an enable and a trigger port
/// for the enabled+triggered subsystem, a reset port for the resettable ones,
/// and the reinitialize event port of a nested function subsystem.
#[test]
fn subsystem_control_ports_come_from_the_contained_port_blocks() {
    use rustylink::model::SlxArchive;
    use rustylink::simulink_libraries::renderers::subsystem_control_port_count;

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
    assert_eq!(count_of("Atomic Subsystem with Reinit"), 1);
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

    let enable = endpoint_pos(rect, &control("enable"), &port_counts, false);
    let trigger = endpoint_pos(rect, &control("trigger"), &port_counts, false);

    assert_eq!(enable.y, rect.top());
    assert_eq!(trigger.y, rect.top());
    assert_eq!(enable.x, 30.0);
    assert_eq!(trigger.x, 60.0);
}
