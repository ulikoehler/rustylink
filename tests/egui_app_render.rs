use rustylink::egui_app::{compute_icon_available_rect, PortLabelMaxWidths, icon_assets};
use eframe::egui::{Pos2, Rect, Vec2};

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
        Some(PortLabelMaxWidths { left: 1000.0, right: 1000.0 }),
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
    ] {
        let bytes = icon_assets::get(path);
        assert!(bytes.is_some(), "missing asset {}", path);
    }
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
