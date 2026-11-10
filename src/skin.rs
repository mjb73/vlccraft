//! Runtime palette + small controls. The icon set lives in `theme` and is
//! reused here with explicit colours so themes and accents can change live.

use egui::{
    pos2, Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, Ui, Vec2,
};

pub use crate::theme::{icon, art, fmt_time, hash01, mix, Icon};

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub panel2: Color32,
    pub hover: Color32,
    pub line: Color32,
    pub line_soft: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub dim: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub good: Color32,
    pub warn: Color32,
    pub bad: Color32,
    pub video_bg: Color32,
    pub light: bool,
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

pub fn palette(theme: &str, accent: [u8; 3]) -> Palette {
    let accent = rgb(accent[0], accent[1], accent[2]);
    let accent_soft = alpha(accent, 46);
    match theme {
        "Graphite" => Palette {
            bg: rgb(15, 15, 16),
            panel: rgb(22, 22, 24),
            panel2: rgb(28, 28, 31),
            hover: rgb(38, 38, 42),
            line: rgb(50, 50, 55),
            line_soft: rgb(36, 36, 40),
            text: rgb(238, 238, 240),
            muted: rgb(150, 150, 156),
            dim: rgb(104, 104, 110),
            accent,
            accent_soft,
            good: rgb(110, 200, 150),
            warn: rgb(235, 190, 100),
            bad: rgb(230, 115, 105),
            video_bg: rgb(6, 6, 7),
            light: false,
        },
        "Paper" => Palette {
            bg: rgb(240, 237, 231),
            panel: rgb(251, 249, 245),
            panel2: rgb(244, 241, 235),
            hover: rgb(232, 227, 219),
            line: rgb(214, 208, 198),
            line_soft: rgb(226, 221, 213),
            text: rgb(34, 32, 29),
            muted: rgb(108, 102, 94),
            dim: rgb(145, 139, 129),
            accent,
            accent_soft: alpha(accent, 44),
            good: rgb(56, 138, 94),
            warn: rgb(170, 120, 30),
            bad: rgb(178, 68, 60),
            video_bg: rgb(18, 18, 20),
            light: true,
        },
        "Ocean" => Palette {
            bg: rgb(10, 17, 24),
            panel: rgb(16, 25, 34),
            panel2: rgb(21, 32, 43),
            hover: rgb(28, 42, 55),
            line: rgb(38, 55, 70),
            line_soft: rgb(27, 41, 54),
            text: rgb(230, 238, 246),
            muted: rgb(138, 158, 176),
            dim: rgb(96, 116, 134),
            accent,
            accent_soft,
            good: rgb(100, 200, 170),
            warn: rgb(232, 190, 105),
            bad: rgb(230, 120, 110),
            video_bg: rgb(4, 8, 12),
            light: false,
        },
        _ => Palette {
            bg: rgb(9, 10, 12),
            panel: rgb(16, 18, 22),
            panel2: rgb(21, 24, 29),
            hover: rgb(30, 34, 41),
            line: rgb(38, 42, 50),
            line_soft: rgb(28, 31, 37),
            text: rgb(236, 238, 242),
            muted: rgb(140, 147, 160),
            dim: rgb(100, 106, 118),
            accent,
            accent_soft,
            good: rgb(104, 200, 150),
            warn: rgb(240, 190, 100),
            bad: rgb(238, 106, 100),
            video_bg: rgb(3, 4, 6),
            light: false,
        },
    }
}

pub const THEMES: &[&str] = &["Midnight", "Graphite", "Ocean", "Paper"];

pub const ACCENTS: &[(&str, [u8; 3])] = &[
    ("Amber", [255, 122, 26]),
    ("Mint", [86, 205, 158]),
    ("Sky", [86, 166, 255]),
    ("Rose", [240, 120, 150]),
    ("Sand", [214, 178, 110]),
];

pub fn install(ctx: &egui::Context, p: &Palette, scale: f32) {
    let mut style = (*ctx.style()).clone();
    let mut v = if p.light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    v.override_text_color = Some(p.text);
    v.panel_fill = p.bg;
    v.window_fill = p.panel;
    v.extreme_bg_color = if p.light { rgb(235, 231, 224) } else { rgb(11, 12, 15) };
    v.faint_bg_color = p.panel2;
    v.window_stroke = Stroke::new(1.0, p.line);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(10);
    v.selection.bg_fill = p.accent_soft;
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.hyperlink_color = p.accent;
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 12],
        blur: 32,
        spread: 0,
        color: Color32::from_black_alpha(if p.light { 40 } else { 140 }),
    };
    v.popup_shadow = v.window_shadow;
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = CornerRadius::same(6);
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.line_soft);
    v.widgets.noninteractive.bg_fill = p.panel;
    v.widgets.inactive.bg_fill = p.panel2;
    v.widgets.inactive.weak_bg_fill = p.panel2;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.line);
    v.widgets.hovered.bg_fill = p.hover;
    v.widgets.hovered.weak_bg_fill = p.hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.line);
    v.widgets.active.bg_fill = p.accent_soft;
    v.widgets.active.weak_bg_fill = p.accent_soft;
    v.widgets.open.bg_fill = p.hover;
    v.widgets.open.weak_bg_fill = p.hover;
    style.visuals = v;
    style.spacing.item_spacing = Vec2::new(8.0, 7.0);
    style.spacing.button_padding = Vec2::new(11.0, 5.0);
    style.spacing.scroll.bar_width = 6.0;
    style.spacing.scroll.floating = true;
    style.spacing.interact_size.y = 26.0;
    style.spacing.slider_width = 160.0;
    style.spacing.slider_rail_height = 4.0;
    style.visuals.slider_trailing_fill = true;
    style.visuals.widgets.inactive.bg_fill = if p.light {
        Color32::from_rgb(220, 214, 204)
    } else {
        mix(p.panel2, p.text, 0.18)
    };
    style.text_styles = [
        (egui::TextStyle::Body, FontId::proportional(13.5 * scale)),
        (egui::TextStyle::Button, FontId::proportional(13.0 * scale)),
        (egui::TextStyle::Small, FontId::proportional(11.5 * scale)),
        (egui::TextStyle::Heading, FontId::proportional(21.0 * scale)),
        (egui::TextStyle::Monospace, FontId::monospace(12.5 * scale)),
    ]
    .into();
    ctx.set_style(style);
}

pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

pub fn icon_button(ui: &mut Ui, ic: Icon, size: f32, active: bool, tip: &str, p: &Palette) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    if active || resp.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(6),
            if active { p.accent_soft } else { p.hover },
        );
    }
    let col = if active {
        p.accent
    } else if resp.hovered() {
        p.text
    } else {
        p.muted
    };
    icon(ui.painter(), ic, rect.shrink(size * 0.14), col, 1.5);
    if tip.is_empty() {
        resp
    } else {
        resp.on_hover_ui(|ui| {
            ui.label(egui::RichText::new(tip).size(12.0));
        })
    }
}

pub fn primary(ui: &mut Ui, text: &str, p: &Palette) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(12.8), p.text);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 30.0, 31.0), Sense::click());
    let col = if resp.hovered() {
        mix(p.accent, Color32::WHITE, 0.14)
    } else {
        p.accent
    };
    ui.painter().rect_filled(rect, CornerRadius::same(7), col);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font(12.8),
        if p.light { Color32::WHITE } else { rgb(20, 12, 6) },
    );
    resp
}

pub fn ghost(ui: &mut Ui, text: &str, p: &Palette) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(12.5), p.text);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 24.0, 30.0), Sense::click());
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(7),
        Stroke::new(1.0, if resp.hovered() { p.muted } else { p.line }),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font(12.5),
        if resp.hovered() { p.text } else { p.muted },
    );
    resp
}

pub fn chip(ui: &mut Ui, text: &str, active: bool, p: &Palette) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(11.5), p.text);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 18.0, 24.0), Sense::click());
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(12),
        if active {
            p.accent_soft
        } else if resp.hovered() {
            p.hover
        } else {
            p.panel2
        },
    );
    if active {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(12),
            Stroke::new(1.0, p.accent),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font(11.5),
        if active { p.accent } else { p.muted },
    );
    resp
}

pub fn text_tab(ui: &mut Ui, text: &str, active: bool, p: &Palette) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(13.0), p.text);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 20.0, 30.0), Sense::click());
    if resp.hovered() && !active {
        ui.painter().rect_filled(rect, CornerRadius::same(6), p.hover);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font(13.0),
        if active { p.text } else { p.muted },
    );
    if active {
        let y = rect.bottom() - 3.0;
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + 6.0, y),
                egui::pos2(rect.right() - 6.0, y),
            ],
            Stroke::new(2.0, p.accent),
        );
    }
    resp
}

pub fn section(ui: &mut Ui, text: &str, p: &Palette) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .size(10.5)
            .color(p.dim)
            .strong(),
    );
}

pub fn divider(ui: &mut Ui, p: &Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, p.line_soft),
    );
}

pub fn frame(p: &Palette, selected: bool) -> egui::Frame {
    egui::Frame::new()
        .fill(if selected { p.panel2 } else { p.panel })
        .stroke(Stroke::new(1.0, if selected { p.accent } else { p.line_soft }))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(12))
}

pub fn toggle(ui: &mut Ui, on: &mut bool, p: &Palette) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(38.0, 21.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
    }
    let t = if *on { 1.0 } else { 0.0 };
    let bg = if *on {
        p.accent
    } else if resp.hovered() {
        mix(p.panel2, p.text, 0.12)
    } else {
        p.line
    };
    ui.painter().rect_filled(rect, CornerRadius::same(11), bg);
    let cx = rect.left() + 10.5 + t * 17.0;
    ui.painter().circle_filled(egui::pos2(cx, rect.center().y), 7.5, Color32::WHITE);
    resp
}

pub fn slider(
    ui: &mut Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    width: f32,
    p: &Palette,
) -> Response {
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, 22.0), Sense::click_and_drag());
    let (min, max) = (*range.start(), *range.end());
    let track = Rect::from_center_size(rect.center(), Vec2::new(rect.width(), 3.0));
    ui.painter().rect_filled(track, CornerRadius::same(2), p.line);
    let frac = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    if resp.hovered() || resp.dragged() {
        let knob = pos2(track.left() + track.width() * frac, track.center().y);
        ui.painter().circle_filled(knob, 7.0, p.accent_soft);
    }
    let filled = Rect::from_min_size(track.min, Vec2::new(track.width() * frac, 3.0));
    ui.painter().rect_filled(filled, CornerRadius::same(2), p.accent);
    let knob = pos2(track.left() + track.width() * frac, track.center().y);
    ui.painter().circle_filled(knob, if resp.hovered() { 5.5 } else { 4.5 }, p.accent);
    if resp.clicked() || resp.dragged() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let f = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
            *value = min + (max - min) * f;
            resp.mark_changed();
        }
    }
    resp
}

pub fn progress(ui: &mut Ui, value: f32, p: &Palette, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 4.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(2), p.line_soft);
    if value > 0.0 {
        let filled = Rect::from_min_size(rect.min, Vec2::new(rect.width() * value.clamp(0.0, 1.0), 3.0));
        ui.painter().rect_filled(filled, CornerRadius::same(2), p.accent);
    }
}

impl Palette {
    pub fn warn_or(&self, _p: &Palette) -> Color32 {
        self.warn
    }
}

pub fn label(ui: &mut Ui, text: impl Into<String>, size: f32, color: Color32) -> Response {
    ui.label(egui::RichText::new(text.into()).size(size).color(color))
}

/// Stable seed from a path/title, so every item keeps its own art.
pub fn seed_for(text: &str) -> f32 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    hash01(h)
}

/// A quiet abstract poster: dusk sky, a low sun, two ridges. No photographs,
/// no logos — just a composition that changes with the item.
pub fn poster(painter: &egui::Painter, rect: Rect, seed: f32, p: &Palette) {
    let k = (seed.max(0.0) * 1_000_000.0) as u64;
    let h1 = hash01(k);
    let h2 = hash01(k ^ 0x9e37_79b9_7f4a_7c15);
    let h3 = hash01(k.wrapping_add(31));

    // Palette: a cool-to-warm dusk, pulled toward the chosen accent.
    let night = Color32::from_rgb(7, 11, 16);
    let cool = mix(Color32::from_rgb(24, 38, 52), p.accent, 0.12 + 0.10 * h1);
    let warm = mix(p.accent, Color32::from_rgb(120, 56, 34), 0.55);
    let sky_top = mix(cool, night, 0.45);
    let sky_low = mix(warm, night, 0.45 + 0.25 * h2);

    // Painted as horizontal bands: no textured mesh, so no font-atlas artefacts.
    let bands = 28;
    for i in 0..bands {
        let t0 = i as f32 / bands as f32;
        let t1 = (i + 1) as f32 / bands as f32;
        let col = mix(sky_top, sky_low, t0);
        let y0 = rect.top() + rect.height() * t0;
        let y1 = rect.top() + rect.height() * t1;
        painter.rect_filled(
            Rect::from_min_max(pos2(rect.left(), y0), pos2(rect.right(), y1 + 1.0)),
            CornerRadius::ZERO,
            col,
        );
    }

    // Sun with two soft halos.
    let sun = pos2(
        rect.left() + rect.width() * (0.20 + 0.58 * h1),
        rect.top() + rect.height() * (0.26 + 0.26 * h3),
    );
    let sun_r = rect.width() * (0.055 + 0.045 * h2);
    for (scale, alpha) in [(2.6, 10u8), (1.7, 18)] {
        painter.circle_filled(
            sun,
            sun_r * scale,
            Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), alpha),
        );
    }
    painter.circle_filled(sun, sun_r, mix(warm, Color32::WHITE, 0.35));

    // Distant ridge.
    let ridge_y = rect.top() + rect.height() * (0.60 + 0.06 * h2);
    let far = vec![
        rect.left_bottom(),
        pos2(rect.left(), ridge_y),
        pos2(rect.left() + rect.width() * 0.22, ridge_y - rect.height() * (0.04 + 0.05 * h1)),
        pos2(rect.left() + rect.width() * 0.44, ridge_y - rect.height() * 0.01),
        pos2(rect.left() + rect.width() * 0.66, ridge_y - rect.height() * (0.07 + 0.04 * h3)),
        pos2(rect.right(), ridge_y - rect.height() * 0.02),
        rect.right_bottom(),
    ];
    fill_ridge(painter, &far, rect.bottom(), Color32::from_rgba_unmultiplied(6, 9, 13, 235));

    // Nearer ridge, a little warmer.
    let near_y = rect.top() + rect.height() * (0.76 + 0.08 * h1);
    let near = vec![
        rect.left_bottom(),
        pos2(rect.left(), near_y),
        pos2(rect.left() + rect.width() * 0.30, near_y - rect.height() * (0.03 + 0.06 * h2)),
        pos2(rect.left() + rect.width() * 0.58, near_y + rect.height() * 0.015),
        pos2(rect.left() + rect.width() * 0.80, near_y - rect.height() * (0.05 + 0.03 * h3)),
        pos2(rect.right(), near_y),
        rect.right_bottom(),
    ];
    fill_ridge(painter, &near, rect.bottom(), Color32::from_rgba_unmultiplied(4, 6, 9, 245));

    // Water line catching the light.
    painter.rect_filled(
        Rect::from_min_max(
            pos2(rect.left(), near_y + rect.height() * 0.16),
            pos2(rect.right(), near_y + rect.height() * 0.16 + 1.0),
        ),
            CornerRadius::ZERO,
        Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 36),
    );

    painter.rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(1.0, Color32::from_white_alpha(12)),
        egui::StrokeKind::Inside,
    );
}

/// Fill the area under a ridge line with convex quads, avoiding the
/// triangulation artefacts a concave polygon would produce.
fn fill_ridge(painter: &egui::Painter, points: &[egui::Pos2], bottom: f32, color: Color32) {
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let quad = vec![
            a,
            b,
            pos2(b.x, bottom.max(b.y)),
            pos2(a.x, bottom.max(a.y)),
        ];
        painter.add(egui::Shape::convex_polygon(quad, color, Stroke::NONE));
    }
}

pub fn badge(ui: &mut Ui, label: &str, p: &Palette) {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font(9.5), p.muted);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(galley.size().x + 12.0, 18.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(5), p.panel2);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, p.muted);
}

pub fn play_badge(painter: &egui::Painter, center: egui::Pos2, radius: f32, p: &Palette) {
    painter.circle_filled(center, radius, Color32::from_black_alpha(120));
    painter.circle_stroke(center, radius, Stroke::new(1.0, Color32::from_white_alpha(60)));
    let r = Rect::from_center_size(center, Vec2::splat(radius * 1.1));
    icon(
        painter,
        Icon::Play,
        r,
        Color32::from_white_alpha(230),
        1.6,
    );
}
