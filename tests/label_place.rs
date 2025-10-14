use rustylink::label_place::*;

struct FixedMeasurer(f32, f32);
impl Measurer for FixedMeasurer {
    fn measure(&self, _text: &str) -> (f32, f32) {
        (self.0, self.1)
    }
}

#[test]
fn places_on_longest_horizontal_segment() {
    let poly = vec![
        Vec2f::new(0.0, 0.0),
        Vec2f::new(200.0, 0.0),
        Vec2f::new(200.0, 50.0),
    ];
    let meas = FixedMeasurer(60.0, 12.0);
    let res = place_label(&poly, "Label", &meas, Config::default(), &[]).unwrap();
    assert!(res.horizontal);
    assert!(
        res.rect.min.x >= 70.0 && res.rect.max.x <= 130.0,
        "centered within long segment"
    );
    assert!(res.rect.min.y < 0.0, "placed with upward offset");
}

#[test]
fn respects_vertical_orientation() {
    let poly = vec![Vec2f::new(0.0, 0.0), Vec2f::new(0.0, 200.0)];
    let meas = FixedMeasurer(10.0, 80.0); // tall when vertical
    let res = place_label(&poly, "AB", &meas, Config::default(), &[]).unwrap();
    assert!(!res.horizontal);
    assert!(
        res.rect.min.y >= 60.0 && res.rect.max.y <= 140.0,
        "within vertical segment"
    );
    assert!(res.rect.min.x > 0.0, "placed with rightward offset");
}

#[test]
fn avoids_collisions_with_expanded_boxes() {
    let poly = vec![Vec2f::new(0.0, 0.0), Vec2f::new(200.0, 0.0)];
    let meas = FixedMeasurer(80.0, 12.0);
    let cfg = Config {
        expand_factor: 1.5,
        ..Default::default()
    };
    let first = place_label(&poly, "A", &meas, cfg, &[]).unwrap();
    let placed = vec![first.rect];
    let second = place_label(&poly, "B", &meas, cfg, &placed).unwrap();
    // Expanded rectangles must not intersect (recompute expansion here using public RectF)
    fn expand(r: RectF, f: f32) -> RectF {
        let c = r.center();
        let hw = r.width() * 0.5 * f;
        let hh = r.height() * 0.5 * f;
        RectF::from_min_max(
            Vec2f::new(c.x - hw, c.y - hh),
            Vec2f::new(c.x + hw, c.y + hh),
        )
    }
    let a = expand(first.rect, cfg.expand_factor);
    let b = expand(second.rect, cfg.expand_factor);
    assert!(!a.intersects(b));
}

#[test]
fn penalizes_spill_prefers_other_segment() {
    // Two segments: one too short for label, one long enough
    let poly = vec![
        Vec2f::new(0.0, 0.0),
        Vec2f::new(40.0, 0.0),
        Vec2f::new(120.0, 0.0),
    ];
    let meas = FixedMeasurer(80.0, 12.0);
    let res = place_label(&poly, "Label", &meas, Config::default(), &[]).unwrap();
    // Expect it to pick the long second segment (center near 80)
    assert!(res.rect.min.x >= 40.0 && res.rect.max.x <= 120.0);
}

#[test]
fn deterministic_results() {
    let poly = vec![Vec2f::new(10.0, 10.0), Vec2f::new(110.0, 10.0)];
    let meas = FixedMeasurer(40.0, 10.0);
    let cfg = Config::default();
    let r1 = place_label(&poly, "X", &meas, cfg, &[]).unwrap();
    let r2 = place_label(&poly, "X", &meas, cfg, &[]).unwrap();
    assert_eq!(r1.rect.min, r2.rect.min);
    assert_eq!(r1.rect.max, r2.rect.max);
}
