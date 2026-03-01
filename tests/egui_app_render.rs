use eframe::egui::{Pos2, Rect, Vec2};
use rustylink::block_types;
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
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(&bytes, &options).unwrap();
    assert!(tree.size().width() > 0.0 && tree.size().height() > 0.0);
}

#[test]
fn block_type_registry_contains_matrix_library_icons() {
    use rustylink::block_types::IconSpec;
    let map = rustylink::block_types::get_block_type_config_map();
    let r = map.read().unwrap();
    for b in rustylink::builtin_libraries::matrix_library::BLOCKS {
        if let Some(icon) = b.icon {
            assert_eq!(
                r.get(b.name).and_then(|c| c.icon),
                Some(IconSpec::Svg(icon)),
                "registry entry for {}",
                b.name,
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
    for b in rustylink::builtin_libraries::matrix_library::BLOCKS {
        if let Some(icon) = b.icon {
            let blk = rustylink::builtin_libraries::virtual_library::create_stub_block(
                b.name, b.ins, b.outs,
            );
            assert!(
                blk.library_block_path.is_none(),
                "stub block should have no library_block_path"
            );
            let cfg = rustylink::egui_app::get_block_type_cfg(&blk);
            assert_eq!(
                cfg.icon,
                Some(IconSpec::Svg(icon)),
                "stub block '{}' (Phase 4 lookup) should resolve to icon '{}'",
                b.name,
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
    let options = resvg::usvg::Options::default();
    // Parsing must succeed.
    let tree = resvg::usvg::Tree::from_data(&bytes, &options).unwrap();
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
    let options = resvg::usvg::Options::default();

    let tree = resvg::usvg::Tree::from_data(&bytes, &options).unwrap();
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
