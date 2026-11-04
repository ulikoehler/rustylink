//! `egui::RawInput` gesture synthesizers.
//!
//! Every helper returns one `RawInput` per *frame* — the only reliable way to
//! reproduce egui's `drag_started`/`dragged`/`drag_stopped` transitions:
//!
//! ```text
//! frame k:   pointer down at p0            → press
//! frame k+1: pointer moves p0→p1           → drag_started (DragMode armed)
//! frame k+2..n-1: moves                    → dragged
//! frame n:   pointer up                    → drag_stopped (commit)
//! ```

#![allow(dead_code)]

use eframe::egui::{self, Pos2, Vec2};

/// Fixed virtual screen for all harness frames (deterministic `avail` rects).
pub const SCREEN_W: f32 = 1400.0;
pub const SCREEN_H: f32 = 900.0;

fn base(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Pos2::ZERO,
            Vec2::new(SCREEN_W, SCREEN_H),
        )),
        events,
        time: Some(0.0),
        ..Default::default()
    }
}

fn base_with(events: Vec<egui::Event>, modifiers: egui::Modifiers) -> egui::RawInput {
    // egui 0.37 carries modifier state via Event::ModifierChanged, not a
    // RawInput field — prepend it so `i.modifiers` reflects the intent.
    let mut ev = vec![egui::Event::ModifiersChanged(modifiers)];
    ev.extend(events);
    base(ev)
}

/// Clear held modifiers at the end of a gesture.
pub fn mod_cleanup() -> egui::RawInput {
    base_with(vec![], egui::Modifiers::NONE)
}

/// One frame with no events (idle frame; also used to let repaint settle).
pub fn idle() -> egui::RawInput {
    base(vec![])
}

pub fn press(pos: Pos2) -> egui::RawInput {
    base(vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
    ])
}

pub fn press_with(pos: Pos2, modifiers: egui::Modifiers) -> egui::RawInput {
    base_with(
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers,
            },
        ],
        modifiers,
    )
}

pub fn press_middle(pos: Pos2) -> egui::RawInput {
    base(vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Middle,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
    ])
}

pub fn move_to(pos: Pos2) -> egui::RawInput {
    base(vec![egui::Event::PointerMoved(pos)])
}

pub fn release(pos: Pos2) -> egui::RawInput {
    release_button(pos, egui::PointerButton::Primary)
}

pub fn release_button(pos: Pos2, button: egui::PointerButton) -> egui::RawInput {
    base(vec![egui::Event::PointerButton {
        pos,
        button,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    }])
}

/// Frames for a linear drag `from → to`: [move(p1)…move(pn), release(to)].
/// Caller emits [`press`] first.
pub fn drag_path(from: Pos2, to: Pos2, steps: usize) -> Vec<egui::RawInput> {
    let steps = steps.max(1);
    let mut out = Vec::with_capacity(steps + 1);
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        out.push(move_to(from + (to - from) * t));
    }
    out.push(release(to));
    out
}

/// Like [`drag_path`] but holding `modifiers` on every move frame
/// (egui reads `RawInput.modifiers` per frame, e.g. shift-marquee).
pub fn drag_path_with(
    from: Pos2,
    to: Pos2,
    steps: usize,
    modifiers: egui::Modifiers,
) -> Vec<egui::RawInput> {
    let steps = steps.max(1);
    let mut out = Vec::with_capacity(steps + 1);
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        out.push(base_with(
            vec![egui::Event::PointerMoved(from + (to - from) * t)],
            modifiers,
        ));
    }
    out.push(release(to));
    out.push(mod_cleanup());
    out
}

/// Full click (press+release at same position): press frame, release frame.
pub fn click(pos: Pos2) -> Vec<egui::RawInput> {
    click_with(pos, egui::Modifiers::NONE)
}

pub fn click_with(pos: Pos2, modifiers: egui::Modifiers) -> Vec<egui::RawInput> {
    vec![
        base_with(vec![egui::Event::PointerMoved(pos)], modifiers),
        base_with(
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers,
            }],
            modifiers,
        ),
        base_with(
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers,
            }],
            modifiers,
        ),
        mod_cleanup(),
    ]
}

pub fn ctrl_click(pos: Pos2) -> Vec<egui::RawInput> {
    click_with(pos, egui::Modifiers::CTRL)
}

pub fn double_click(pos: Pos2) -> Vec<egui::RawInput> {
    let mut v = click(pos);
    v.extend(click(pos));
    v
}

/// Single key press (down only) with modifiers.
pub fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::RawInput {
    base_with(
        vec![egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        }],
        modifiers,
    )
}

pub fn esc() -> egui::RawInput {
    key(egui::Key::Escape, egui::Modifiers::NONE)
}

pub fn key_with_mods(key: egui::Key, ctrl: bool, shift: bool) -> egui::RawInput {
    self::key(
        key,
        egui::Modifiers {
            ctrl,
            shift,
            ..Default::default()
        },
    )
}

/// Text input, one `Event::Text` frame per char batch (splits into chunks).
pub fn type_text(s: &str) -> Vec<egui::RawInput> {
    s.chars()
        .map(|c| base(vec![egui::Event::Text(c.to_string())]))
        .collect()
}

/// Scroll wheel at a position (delta in points).
pub fn scroll_at(pos: Pos2, delta: Vec2) -> egui::RawInput {
    base(vec![
        egui::Event::PointerMoved(pos),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::NONE,
        },
    ])
}
