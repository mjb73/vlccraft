use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontFamily, FontId, Mesh, Pos2, Rect, Response, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};

pub const BG: Color32 = Color32::from_rgb(9, 10, 12);
pub const PANEL: Color32 = Color32::from_rgb(16, 18, 22);
pub const PANEL2: Color32 = Color32::from_rgb(21, 24, 29);
pub const HOVER: Color32 = Color32::from_rgb(30, 34, 41);
pub const LINE: Color32 = Color32::from_rgb(38, 42, 50);
pub const LINE_SOFT: Color32 = Color32::from_rgb(28, 31, 37);
pub const TEXT: Color32 = Color32::from_rgb(236, 238, 242);
pub const MUTED: Color32 = Color32::from_rgb(140, 147, 160);
pub const DIM: Color32 = Color32::from_rgb(100, 106, 118);
pub const ACCENT: Color32 = Color32::from_rgb(255, 122, 26);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(122, 60, 18);
pub const GOOD: Color32 = Color32::from_rgb(74, 222, 128);
pub const WARN: Color32 = Color32::from_rgb(245, 190, 80);
pub const BAD: Color32 = Color32::from_rgb(240, 100, 100);
pub const BLACK: Color32 = Color32::from_rgb(0, 0, 0);

pub fn install(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(TEXT);
    v.panel_fill = BG;
    v.window_fill = PANEL;
    v.extreme_bg_color = Color32::from_rgb(12, 13, 16);
    v.faint_bg_color = PANEL2;
    v.window_stroke = Stroke::new(1.0, LINE);
    v.selection.bg_fill = ACCENT_SOFT;
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(10);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = CornerRadius::same(6);
        w.bg_stroke = Stroke::NONE;
        w.fg_stroke = Stroke::NONE;
    }
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.inactive.bg_fill = PANEL2;
    v.widgets.inactive.weak_bg_fill = PANEL2;
    v.widgets.hovered.bg_fill = HOVER;
    v.widgets.hovered.weak_bg_fill = HOVER;
    v.widgets.active.bg_fill = Color32::from_rgb(44, 48, 56);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(44, 48, 56);
    v.widgets.open.bg_fill = HOVER;
    v.widgets.open.weak_bg_fill = HOVER;
    style.visuals = v;

    style.spacing.item_spacing = Vec2::new(8.0, 7.0);
    style.spacing.button_padding = Vec2::new(10.0, 5.0);
    style.spacing.scroll.bar_width = 6.0;
    style.spacing.scroll.floating = true;
    style.spacing.interact_size.y = 24.0;

    use egui::{FontId, TextStyle::*};
    style.text_styles.insert(Body, FontId::new(13.5, egui::FontFamily::Proportional));
    style.text_styles.insert(Button, FontId::new(13.0, egui::FontFamily::Proportional));
    style.text_styles.insert(Small, FontId::new(11.5, egui::FontFamily::Proportional));
    style.text_styles.insert(Heading, FontId::new(21.0, egui::FontFamily::Proportional));
    style.text_styles.insert(Monospace, FontId::new(12.5, egui::FontFamily::Monospace));
    ctx.set_style(style);
}

pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Play,
    Pause,
    Prev,
    Next,
    Volume,
    Mute,
    Subtitle,
    Queue,
    Library,
    Panel,
    Pip,
    Fullscreen,
    ExitFullscreen,
    Settings,
    Search,
    Plus,
    Close,
    ChevronDown,
    ChevronUp,
    ChevronLeft,
    ChevronRight,
    Check,
    Pin,
    Trash,
    Drag,
    Bookmark,
    Folder,
    Film,
    Music,
    Stream,
    Clock,
    Info,
    Download,
    Rotate,
    FlipH,
    FlipV,
    Zoom,
    Disc,
    Link,
    Devices,
    ArrowLeft,
    Stop,
    Sort,
    Grid,
    List,
    Details,
    More,
    Speaker,
    Wave,
    Sparkle,
    Screen,
    Edit,
}

fn tri(painter: &egui::Painter, pts: [Pos2; 3], color: Color32) {
    painter.add(egui::Shape::convex_polygon(
        pts.to_vec(),
        color,
        Stroke::NONE,
    ));
}

/// A loudspeaker: the small driver on the left, the cone flaring to `tip`.
/// Drawn as one solid shape so it stays a speaker at 18 px.
fn speaker_cone(painter: &egui::Painter, r: Rect, color: Color32, tip: f32) {
    let x = |t: f32| r.left() + r.width() * t;
    let y = |t: f32| r.top() + r.height() * t;
    painter.rect_filled(
        Rect::from_min_max(Pos2::new(x(0.10), y(0.33)), Pos2::new(x(0.33), y(0.67))),
        0.5,
        color,
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(x(0.32), y(0.33)),
            Pos2::new(x(tip), y(0.11)),
            Pos2::new(x(tip), y(0.89)),
            Pos2::new(x(0.32), y(0.67)),
        ],
        color,
        Stroke::NONE,
    ));
}

pub fn icon(painter: &egui::Painter, ic: Icon, rect: Rect, color: Color32, w: f32) {
    let r = rect.shrink(rect.width() * 0.08);
    let x = |t: f32| r.left() + r.width() * t;
    let y = |t: f32| r.top() + r.height() * t;
    let p = |a: f32, b: f32| Pos2::new(x(a), y(b));
    let s = Stroke::new(w, color);
    match ic {
        Icon::Play => tri(painter, [p(0.30, 0.20), p(0.30, 0.80), p(0.80, 0.50)], color),
        Icon::Pause => {
            painter.rect_filled(Rect::from_min_max(p(0.28, 0.20), p(0.43, 0.80)), 1.5, color);
            painter.rect_filled(Rect::from_min_max(p(0.57, 0.20), p(0.72, 0.80)), 1.5, color);
        }
        Icon::Prev => {
            tri(painter, [p(0.72, 0.20), p(0.72, 0.80), p(0.28, 0.50)], color);
            painter.line_segment([p(0.24, 0.22), p(0.24, 0.78)], s);
        }
        Icon::Next => {
            tri(painter, [p(0.28, 0.20), p(0.28, 0.80), p(0.72, 0.50)], color);
            painter.line_segment([p(0.76, 0.22), p(0.76, 0.78)], s);
        }
        Icon::Stop => {
            painter.rect_filled(Rect::from_min_max(p(0.28, 0.28), p(0.72, 0.72)), 2.0, color);
        }
        // The speaker is one shape: a small driver on the left and a cone that
        // flares to the right. The old drawing mixed a wedge above a bar, which
        // read as a bent needle at bar size.
        Icon::Volume => {
            speaker_cone(painter, r, color, 0.50);
            for rad in [0.15f32, 0.29] {
                let pts: Vec<Pos2> = (0..18)
                    .map(|i| {
                        let t = -0.78 + 1.56 * (i as f32 / 17.0);
                        Pos2::new(
                            x(0.58) + rad * r.width() * t.cos(),
                            y(0.50) + rad * r.width() * t.sin(),
                        )
                    })
                    .collect();
                painter.add(egui::Shape::line(pts, Stroke::new(w * 0.95, color)));
            }
        }
        Icon::Mute => {
            speaker_cone(painter, r, color, 0.50);
            painter.line_segment([p(0.64, 0.37), p(0.87, 0.63)], s);
            painter.line_segment([p(0.87, 0.37), p(0.64, 0.63)], s);
        }
        // Audio tracks and output: an equaliser, so this button never looks
        // like the volume speaker beside the slider.
        Icon::Speaker => {
            let bw = (r.width() * 0.11).max(1.8);
            for (cx, top, bottom) in [
                (0.11f32, 0.40f32, 0.60f32),
                (0.30, 0.24, 0.76),
                (0.50, 0.10, 0.90),
                (0.70, 0.26, 0.74),
                (0.89, 0.38, 0.62),
            ] {
                let bar = Rect::from_min_max(
                    Pos2::new(x(cx) - bw / 2.0, y(top)),
                    Pos2::new(x(cx) + bw / 2.0, y(bottom)),
                );
                painter.rect_filled(bar, CornerRadius::same((bw / 2.0) as u8 + 1), color);
            }
        }
        Icon::Subtitle => {
            let rr = Rect::from_min_max(p(0.12, 0.26), p(0.88, 0.74));
            painter.rect_stroke(rr, CornerRadius::same(2), Stroke::new(w, color), StrokeKind::Inside);
            painter.line_segment([p(0.22, 0.56), p(0.46, 0.56)], Stroke::new(w, color));
            painter.line_segment([p(0.54, 0.56), p(0.78, 0.56)], Stroke::new(w, color));
            painter.line_segment([p(0.22, 0.66), p(0.62, 0.66)], Stroke::new(w * 0.9, color));
        }
        Icon::Queue | Icon::List => {
            for (i, len) in [0.62f32, 0.50, 0.40].iter().enumerate() {
                let yy = 0.28 + i as f32 * 0.22;
                painter.circle_filled(p(0.18, yy), w * 0.9, color);
                painter.line_segment([p(0.32, yy), p(0.32 + len, yy)], Stroke::new(w, color));
            }
        }
        Icon::Library | Icon::Grid => {
            let sz = 0.26;
            for (a, b) in [(0.16, 0.16), (0.58, 0.16), (0.16, 0.58), (0.58, 0.58)] {
                painter.rect_filled(
                    Rect::from_min_max(p(a, b), p(a + sz, b + sz)),
                    2.0,
                    color,
                );
            }
        }
        Icon::Details => {
            painter.rect_filled(Rect::from_min_max(p(0.14, 0.18), p(0.44, 0.48)), 2.0, color);
            for i in 0..3 {
                let yy = 0.26 + i as f32 * 0.18;
                painter.line_segment([p(0.54, yy), p(0.86, yy)], Stroke::new(w, color));
            }
            painter.line_segment([p(0.14, 0.62), p(0.44, 0.62)], Stroke::new(w, color));
            painter.line_segment([p(0.14, 0.76), p(0.44, 0.76)], Stroke::new(w, color));
        }
        Icon::Panel => {
            painter.rect_stroke(
                Rect::from_min_max(p(0.12, 0.20), p(0.88, 0.80)),
                CornerRadius::same(2),
                Stroke::new(w, color),
                StrokeKind::Inside,
            );
            painter.line_segment([p(0.62, 0.20), p(0.62, 0.80)], Stroke::new(w, color));
        }
        Icon::Pip => {
            painter.rect_stroke(
                Rect::from_min_max(p(0.10, 0.22), p(0.90, 0.78)),
                CornerRadius::same(2),
                Stroke::new(w, color),
                StrokeKind::Inside,
            );
            painter.rect_filled(Rect::from_min_max(p(0.52, 0.50), p(0.82, 0.72)), 1.5, color);
        }
        Icon::Fullscreen | Icon::ExitFullscreen => {
            let l = 0.22;
            for (cx, cy, dx, dy) in [
                (0.14, 0.18, 1.0, 1.0),
                (0.86, 0.18, -1.0, 1.0),
                (0.14, 0.82, 1.0, -1.0),
                (0.86, 0.82, -1.0, -1.0),
            ] {
                painter.line_segment([Pos2::new(x(cx), y(cy)), Pos2::new(x(cx + dx * l), y(cy))], s);
                painter.line_segment([Pos2::new(x(cx), y(cy)), Pos2::new(x(cx), y(cy + dy * l))], s);
            }
            if ic == Icon::ExitFullscreen {
                painter.rect_stroke(
                    Rect::from_min_max(p(0.36, 0.36), p(0.64, 0.64)),
                    CornerRadius::same(1),
                    Stroke::new(w, color),
                    StrokeKind::Inside,
                );
            }
        }
        Icon::Settings => {
            painter.circle_stroke(p(0.5, 0.5), r.width() * 0.19, Stroke::new(w, color));
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let c = Pos2::new(x(0.5), y(0.5));
                painter.line_segment(
                    [
                        Pos2::new(c.x + a.cos() * r.width() * 0.26, c.y + a.sin() * r.width() * 0.26),
                        Pos2::new(c.x + a.cos() * r.width() * 0.38, c.y + a.sin() * r.width() * 0.38),
                    ],
                    Stroke::new(w, color),
                );
            }
        }
        Icon::Search => {
            painter.circle_stroke(p(0.44, 0.44), r.width() * 0.22, Stroke::new(w, color));
            painter.line_segment([p(0.62, 0.62), p(0.84, 0.84)], Stroke::new(w, color));
        }
        Icon::Plus | Icon::Edit => {
            painter.line_segment([p(0.22, 0.5), p(0.78, 0.5)], s);
            painter.line_segment([p(0.5, 0.22), p(0.5, 0.78)], s);
        }
        Icon::Close => {
            painter.line_segment([p(0.26, 0.26), p(0.74, 0.74)], s);
            painter.line_segment([p(0.74, 0.26), p(0.26, 0.74)], s);
        }
        Icon::ChevronDown => {
            painter.line_segment([p(0.26, 0.40), p(0.50, 0.64)], s);
            painter.line_segment([p(0.50, 0.64), p(0.74, 0.40)], s);
        }
        Icon::ChevronUp => {
            painter.line_segment([p(0.26, 0.60), p(0.50, 0.36)], s);
            painter.line_segment([p(0.50, 0.36), p(0.74, 0.60)], s);
        }
        Icon::ChevronRight => {
            painter.line_segment([p(0.40, 0.26), p(0.64, 0.50)], s);
            painter.line_segment([p(0.64, 0.50), p(0.40, 0.74)], s);
        }
        Icon::ChevronLeft => {
            painter.line_segment([p(0.60, 0.26), p(0.36, 0.50)], s);
            painter.line_segment([p(0.36, 0.50), p(0.60, 0.74)], s);
        }
        Icon::ArrowLeft => {
            painter.line_segment([p(0.22, 0.50), p(0.80, 0.50)], s);
            painter.line_segment([p(0.22, 0.50), p(0.44, 0.28)], s);
            painter.line_segment([p(0.22, 0.50), p(0.44, 0.72)], s);
        }
        Icon::Check => {
            painter.line_segment([p(0.22, 0.52), p(0.42, 0.72)], s);
            painter.line_segment([p(0.42, 0.72), p(0.80, 0.30)], s);
        }
        Icon::Pin => {
            painter.circle_stroke(p(0.5, 0.38), r.width() * 0.16, Stroke::new(w, color));
            painter.line_segment([p(0.5, 0.54), p(0.5, 0.84)], s);
            painter.line_segment([p(0.32, 0.22), p(0.68, 0.22)], Stroke::new(w * 0.9, color));
        }
        Icon::Trash => {
            painter.line_segment([p(0.22, 0.30), p(0.78, 0.30)], s);
            painter.line_segment([p(0.40, 0.22), p(0.60, 0.22)], s);
            painter.rect_stroke(
                Rect::from_min_max(p(0.28, 0.34), p(0.72, 0.80)),
                CornerRadius::same(1),
                Stroke::new(w, color),
                StrokeKind::Inside,
            );
            painter.line_segment([p(0.43, 0.44), p(0.43, 0.70)], Stroke::new(w * 0.8, color));
            painter.line_segment([p(0.57, 0.44), p(0.57, 0.70)], Stroke::new(w * 0.8, color));
        }
        Icon::Drag => {
            for row in 0..3 {
                for col in 0..2 {
                    painter.circle_filled(
                        p(0.38 + col as f32 * 0.22, 0.28 + row as f32 * 0.22),
                        w * 1.1,
                        color,
                    );
                }
            }
        }
        Icon::Bookmark => {
            let pts = vec![p(0.30, 0.18), p(0.70, 0.18), p(0.70, 0.82), p(0.50, 0.66), p(0.30, 0.82)];
            painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
        }
        Icon::Folder => {
            // A tab and a body, so it still reads as a folder at 14 px where a
            // single notched silhouette collapses into a grey box.
            let radius = CornerRadius::same((r.width() * 0.05).clamp(1.0, 4.0) as u8);
            painter.rect_filled(
                Rect::from_min_max(p(0.13, 0.27), p(0.48, 0.40)),
                radius,
                color,
            );
            painter.rect_filled(
                Rect::from_min_max(p(0.13, 0.35), p(0.87, 0.78)),
                radius,
                color,
            );
        }
        Icon::Film => {
            painter.rect_stroke(
                Rect::from_min_max(p(0.12, 0.22), p(0.88, 0.78)),
                CornerRadius::same(2),
                Stroke::new(w, color),
                StrokeKind::Inside,
            );
            for i in 0..3 {
                let xx = 0.20 + i as f32 * 0.24;
                painter.rect_filled(Rect::from_min_max(p(xx, 0.28), p(xx + 0.08, 0.36)), 0.5, color);
                painter.rect_filled(Rect::from_min_max(p(xx, 0.64), p(xx + 0.08, 0.72)), 0.5, color);
            }
        }
        Icon::Music => {
            painter.circle_filled(p(0.34, 0.70), r.width() * 0.10, color);
            painter.line_segment([p(0.44, 0.68), p(0.44, 0.24)], Stroke::new(w, color));
            painter.line_segment([p(0.44, 0.24), p(0.76, 0.32)], Stroke::new(w, color));
            painter.circle_filled(p(0.66, 0.62), r.width() * 0.09, color);
        }
        Icon::Stream => {
            painter.circle_filled(p(0.5, 0.72), r.width() * 0.07, color);
            for (i, rad) in [0.22f32, 0.36, 0.5].iter().enumerate() {
                let a0 = -2.35;
                let a1 = -0.79;
                let pts: Vec<Pos2> = (0..12)
                    .map(|k| {
                        let t = a0 + (a1 - a0) * (k as f32 / 11.0);
                        Pos2::new(
                            p(0.5, 0.72).x + rad * r.width() * t.cos(),
                            p(0.5, 0.72).y + rad * r.width() * t.sin(),
                        )
                    })
                    .collect();
                painter.add(egui::Shape::line(pts, Stroke::new(w * (0.9 - i as f32 * 0.15), color)));
            }
        }
        Icon::Clock => {
            painter.circle_stroke(p(0.5, 0.5), r.width() * 0.32, Stroke::new(w, color));
            painter.line_segment([p(0.5, 0.5), p(0.5, 0.30)], Stroke::new(w, color));
            painter.line_segment([p(0.5, 0.5), p(0.66, 0.58)], Stroke::new(w, color));
        }
        Icon::Info => {
            painter.circle_stroke(p(0.5, 0.5), r.width() * 0.32, Stroke::new(w, color));
            painter.circle_filled(p(0.5, 0.32), w * 0.9, color);
            painter.line_segment([p(0.5, 0.46), p(0.5, 0.70)], Stroke::new(w, color));
        }
        Icon::Download => {
            painter.line_segment([p(0.5, 0.20), p(0.5, 0.62)], s);
            painter.line_segment([p(0.32, 0.46), p(0.5, 0.64)], s);
            painter.line_segment([p(0.68, 0.46), p(0.5, 0.64)], s);
            painter.line_segment([p(0.22, 0.78), p(0.78, 0.78)], s);
        }
        Icon::Rotate => {
            let pts: Vec<Pos2> = (0..18)
                .map(|i| {
                    let t = -2.6 + 4.6 * (i as f32 / 17.0);
                    Pos2::new(x(0.5) + 0.3 * r.width() * t.cos(), y(0.52) + 0.3 * r.width() * t.sin())
                })
                .collect();
            painter.add(egui::Shape::line(pts, Stroke::new(w, color)));
            tri(painter, [p(0.74, 0.20), p(0.86, 0.34), p(0.66, 0.36)], color);
        }
        Icon::FlipH => {
            painter.line_segment([p(0.5, 0.16), p(0.5, 0.84)], Stroke::new(w * 0.7, color));
            tri(painter, [p(0.40, 0.30), p(0.40, 0.70), p(0.16, 0.50)], color);
            tri(painter, [p(0.60, 0.30), p(0.60, 0.70), p(0.84, 0.50)], color);
        }
        Icon::FlipV => {
            painter.line_segment([p(0.16, 0.5), p(0.84, 0.5)], Stroke::new(w * 0.7, color));
            tri(painter, [p(0.30, 0.40), p(0.70, 0.40), p(0.50, 0.16)], color);
            tri(painter, [p(0.30, 0.60), p(0.70, 0.60), p(0.50, 0.84)], color);
        }
        Icon::Zoom => {
            painter.circle_stroke(p(0.44, 0.44), r.width() * 0.22, Stroke::new(w, color));
            painter.line_segment([p(0.62, 0.62), p(0.84, 0.84)], Stroke::new(w, color));
            painter.line_segment([p(0.34, 0.44), p(0.54, 0.44)], Stroke::new(w * 0.8, color));
            painter.line_segment([p(0.44, 0.34), p(0.44, 0.54)], Stroke::new(w * 0.8, color));
        }
        Icon::Disc => {
            painter.circle_stroke(p(0.5, 0.5), r.width() * 0.33, Stroke::new(w, color));
            painter.circle_stroke(p(0.5, 0.5), r.width() * 0.09, Stroke::new(w, color));
            painter.line_segment([p(0.5, 0.17), p(0.5, 0.26)], Stroke::new(w * 0.8, color));
        }
        Icon::Link => {
            let rr1 = Rect::from_min_max(p(0.14, 0.40), p(0.50, 0.60));
            let rr2 = Rect::from_min_max(p(0.50, 0.40), p(0.86, 0.60));
            painter.rect_stroke(rr1, CornerRadius::same(6), Stroke::new(w, color), StrokeKind::Inside);
            painter.rect_stroke(rr2, CornerRadius::same(6), Stroke::new(w, color), StrokeKind::Inside);
        }
        Icon::Devices => {
            painter.rect_stroke(
                Rect::from_min_max(p(0.10, 0.24), p(0.66, 0.66)),
                CornerRadius::same(2),
                Stroke::new(w, color),
                StrokeKind::Inside,
            );
            painter.line_segment([p(0.26, 0.76), p(0.52, 0.76)], Stroke::new(w, color));
            painter.rect_stroke(
                Rect::from_min_max(p(0.66, 0.42), p(0.86, 0.76)),
                CornerRadius::same(2),
                Stroke::new(w, color),
                StrokeKind::Inside,
            );
        }
        Icon::Sort => {
            painter.line_segment([p(0.24, 0.28), p(0.72, 0.28)], s);
            painter.line_segment([p(0.24, 0.50), p(0.58, 0.50)], s);
            painter.line_segment([p(0.24, 0.72), p(0.44, 0.72)], s);
        }
        Icon::More => {
            for i in 0..3 {
                painter.circle_filled(p(0.26 + (i as f32) * 0.24, 0.5), w * 1.2, color);
            }
        }
        Icon::Wave => {
            let mut pts = Vec::new();
            for i in 0..=24 {
                let t = i as f32 / 24.0;
                pts.push(Pos2::new(x(0.10 + 0.80 * t), y(0.5 + 0.28 * (t * 18.0).sin() * (0.4 + 0.6 * t))));
            }
            painter.add(egui::Shape::line(pts, Stroke::new(w, color)));
        }
        Icon::Sparkle => {
            let c = p(0.5, 0.5);
            let pts = vec![
                Pos2::new(c.x, y(0.14)),
                Pos2::new(c.x + r.width() * 0.10, c.y - r.width() * 0.10),
                Pos2::new(x(0.86), c.y),
                Pos2::new(c.x + r.width() * 0.10, c.y + r.width() * 0.10),
                Pos2::new(c.x, y(0.86)),
                Pos2::new(c.x - r.width() * 0.10, c.y + r.width() * 0.10),
                Pos2::new(x(0.14), c.y),
                Pos2::new(c.x - r.width() * 0.10, c.y - r.width() * 0.10),
            ];
            painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
        }
        Icon::Screen => {
            painter.rect_stroke(
                Rect::from_min_max(p(0.10, 0.20), p(0.90, 0.68)),
                CornerRadius::same(2),
                Stroke::new(w, color),
                StrokeKind::Inside,
            );
            painter.line_segment([p(0.38, 0.82), p(0.62, 0.82)], Stroke::new(w, color));
            painter.line_segment([p(0.5, 0.68), p(0.5, 0.82)], Stroke::new(w, color));
        }
    }
}

pub fn icon_button(ui: &mut Ui, ic: Icon, size: f32, active: bool, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let hovered = resp.hovered();
    if active || hovered {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(6),
            if active { ACCENT_SOFT } else { HOVER },
        );
    }
    let col = if active {
        ACCENT
    } else if hovered {
        TEXT
    } else {
        MUTED
    };
    icon(ui.painter(), ic, rect.shrink(size * 0.22), col, 1.5);
    if !tip.is_empty() {
        resp.clone().on_hover_text(tip)
    } else {
        resp
    }
}

pub fn icon_text_button(ui: &mut Ui, ic: Icon, text: &str, active: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(12.5), TEXT);
    let w = galley.size().x + 34.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::click());
    if active || resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), if active { ACCENT_SOFT } else { HOVER });
    }
    let col = if active { ACCENT } else { MUTED };
    icon(ui.painter(), ic, Rect::from_min_size(Pos2::new(rect.left() + 7.0, rect.top() + 6.0), Vec2::splat(16.0)), col, 1.4);
    ui.painter().text(
        Pos2::new(rect.left() + 27.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        font(12.5),
        if active { TEXT } else { MUTED },
    );
    resp
}

pub fn text_button(ui: &mut Ui, text: &str, active: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(13.0), TEXT);
    let w = galley.size().x + 22.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::click());
    if active {
        ui.painter().rect_filled(rect, CornerRadius::same(6), HOVER);
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(6), Color32::from_rgb(24, 27, 33));
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        font(13.0),
        if active { TEXT } else { MUTED },
    );
    resp
}

pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(13.0), BLACK);
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(galley.size().x + 30.0, 32.0),
        Sense::click(),
    );
    let col = if resp.hovered() {
        Color32::from_rgb(255, 145, 64)
    } else {
        ACCENT
    };
    ui.painter().rect_filled(rect, CornerRadius::same(7), col);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        font(13.0),
        Color32::from_rgb(20, 12, 6),
    );
    resp
}

pub fn ghost_button(ui: &mut Ui, text: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(12.5), TEXT);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 24.0, 30.0), Sense::click());
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(7),
        Stroke::new(1.0, if resp.hovered() { MUTED } else { LINE }),
        StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        font(12.5),
        if resp.hovered() { TEXT } else { Color32::from_rgb(200, 205, 214) },
    );
    resp
}

pub fn chip(ui: &mut Ui, text: &str, active: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font(11.5), TEXT);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 18.0, 24.0), Sense::click());
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(12),
        if active {
            ACCENT_SOFT
        } else if resp.hovered() {
            HOVER
        } else {
            PANEL2
        },
    );
    if active {
        ui.painter().rect_stroke(rect, CornerRadius::same(12), Stroke::new(1.0, ACCENT_SOFT), StrokeKind::Inside);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        font(11.5),
        if active { ACCENT } else { MUTED },
    );
    resp
}

pub fn toggle_switch(ui: &mut Ui, on: &mut bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(38.0, 21.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
    }
    let t = if *on { 1.0 } else { 0.0 };
    let bg = if *on {
        ACCENT
    } else if resp.hovered() {
        Color32::from_rgb(55, 60, 70)
    } else {
        Color32::from_rgb(45, 49, 57)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(11), bg);
    let cx = rect.left() + 10.5 + t * 17.0;
    ui.painter().circle_filled(Pos2::new(cx, rect.center().y), 7.5, Color32::WHITE);
    resp
}

pub fn section_label(ui: &mut Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .size(10.5)
            .color(DIM)
            .strong(),
    );
}

pub fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, LINE_SOFT),
    );
}

/// Deterministic soft poster art, used wherever real artwork would live.
pub fn art(painter: &egui::Painter, rect: Rect, seed: u64, tint: Color32) {
    if rect.width() < 2.0 || rect.height() < 2.0 {
        return;
    }
    let a = hash01(seed);
    let b = hash01(seed ^ 0x9e37_79b9);
    let c = hash01(seed.wrapping_mul(6364136223846793005).wrapping_add(1));
    let d = hash01(seed ^ 0x85eb_ca6b);

    // Deep vertical gradient: night sky above, tinted haze at the horizon.
    let sky_top = mix(Color32::from_rgb(7, 9, 15), tint, 0.05 + 0.06 * a);
    let haze = mix(Color32::from_rgb(18, 20, 27), tint, 0.30 + 0.22 * b);
    let ground = Color32::from_rgb(5, 6, 9);
    let horizon = 0.56 + 0.18 * c;
    let mut mesh = Mesh::default();
    let uv = Pos2::new(0.0, 0.0);
    let _ = uv;
    let cols = [
        (rect.left_top(), sky_top),
        (rect.right_top(), mix(sky_top, Color32::BLACK, 0.25)),
        (rect.right_bottom(), mix(haze, Color32::BLACK, 0.55)),
        (rect.left_bottom(), mix(haze, ground, 0.35)),
    ];
    for (i, (p, col)) in cols.iter().enumerate() {
        let u = if i == 0 || i == 3 { 0.0 } else { 1.0 };
        let v = if i < 2 { 0.0 } else { 1.0 };
        mesh.vertices.push(egui::epaint::Vertex {
            pos: *p,
            uv: Pos2::new(u, v),
            color: *col,
        });
    }
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    painter.add(egui::Shape::mesh(mesh));

    // A band of low light along the horizon, built from soft strips so it
    // reads as haze rather than a hard circle.
    let hz = rect.top() + rect.height() * horizon;
    for i in 0..18 {
        let t = i as f32 / 18.0;
        let alpha = ((1.0 - t).powf(1.6) * 26.0) as u8;
        let h = rect.height() * (0.02 + 0.05 * t);
        let col = mix(tint, Color32::WHITE, 0.10 + 0.20 * a);
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(rect.left(), hz - h * 0.5),
                Pos2::new(rect.right(), hz + h * 0.5),
            ),
            CornerRadius::ZERO,
            Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), alpha),
        );
    }

    // Off-centre light bloom: many small discs make a soft falloff.
    let lx = rect.left() + rect.width() * (0.24 + 0.52 * d);
    let ly = hz - rect.height() * (0.03 + 0.16 * b);
    let rad = rect.height() * (0.13 + 0.10 * a);
    let core = mix(tint, Color32::WHITE, 0.55);
    for i in 0..40 {
        let t = i as f32 / 40.0;
        let rr = rad * (1.0 - t * 0.96);
        let alpha = (2.0 + 6.0 * (1.0 - t)) as u8;
        painter.circle_filled(
            Pos2::new(lx, ly),
            rr.max(1.0),
            Color32::from_rgba_unmultiplied(core.r(), core.g(), core.b(), alpha),
        );
    }

    // Distant ridge, then a nearer one: a still frame, not an icon.
    let ridge = |base: f32, height: f32, shade: Color32, seed_shift: u64| {
        let mut pts: Vec<Pos2> = Vec::new();
        pts.push(Pos2::new(rect.left(), rect.bottom()));
        let steps = 9;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let n = hash01(seed.wrapping_add(seed_shift).wrapping_add(i as u64 * 7919));
            let y = rect.top() + rect.height() * (base + (n - 0.5) * height);
            pts.push(Pos2::new(rect.left() + rect.width() * t, y));
        }
        pts.push(Pos2::new(rect.right(), rect.bottom()));
        painter.add(egui::Shape::convex_polygon(pts, shade, Stroke::NONE));
    };
    ridge(horizon + 0.04, 0.10, Color32::from_rgba_unmultiplied(10, 12, 18, 170), 31);
    ridge(horizon + 0.13, 0.08, Color32::from_rgb(6, 7, 10), 97);

    // A thin rim of light on the far ridge.
    let mut rim: Vec<Pos2> = Vec::new();
    for i in 0..=9 {
        let t = i as f32 / 9.0;
        let n = hash01(seed.wrapping_add(31).wrapping_add(i as u64 * 7919));
        let y = rect.top() + rect.height() * (horizon + 0.04 + (n - 0.5) * 0.10);
        rim.push(Pos2::new(rect.left() + rect.width() * t, y));
    }
    painter.add(egui::Shape::line(
        rim,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), 55)),
    ));

    // Stars in the upper third, deterministic per seed.
    let stars = 26 + (b * 40.0) as usize;
    for i in 0..stars {
        let n1 = hash01(seed.wrapping_add(i as u64 * 104_729));
        let n2 = hash01(seed.wrapping_add(i as u64 * 130_363));
        let x = rect.left() + rect.width() * n1;
        let y = rect.top() + rect.height() * n2 * horizon * 0.85;
        let r = 0.5 + 0.9 * hash01(seed.wrapping_add(i as u64 * 31));
        let alpha = (18.0 + 52.0 * hash01(seed.wrapping_add(i as u64 * 17))) as u8;
        painter.circle_filled(
            Pos2::new(x, y),
            r,
            Color32::from_rgba_unmultiplied(235, 238, 248, alpha),
        );
    }

    // Gentle vignette: corners fall away from the subject.
    for i in 0..6 {
        let pad = i as f32 * 3.0;
        painter.rect_stroke(
            rect.shrink(pad),
            CornerRadius::same(4),
            Stroke::new(2.0, Color32::from_rgba_unmultiplied(0, 0, 0, 16)),
            StrokeKind::Inside,
        );
    }
}

pub fn hash01(mut x: u64) -> f32 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^= x >> 33;
    (x % 10_000) as f32 / 10_000.0
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 * (1.0 - t) + b.r() as f32 * t) as u8,
        (a.g() as f32 * (1.0 - t) + b.g() as f32 * t) as u8,
        (a.b() as f32 * (1.0 - t) + b.b() as f32 * t) as u8,
    )
}

pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

pub fn fmt_time(ms: i64) -> String {
    let ms = ms.max(0);
    let total = ms / 1000;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}
