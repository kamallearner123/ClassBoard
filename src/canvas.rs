use cairo::Context;
use crate::state::{CanvasImage, Color, Point, Shape, ShapeKind, Stroke};

pub fn apply_color(cr: &Context, c: &Color) {
    cr.set_source_rgba(c.r, c.g, c.b, c.a);
}

// ── Chaikin corner-cutting smoothing ─────────────────────────────────────────
// Guaranteed smooth, never overshoots — unlike Catmull-Rom with dense points.

fn chaikin(pts: &[Point], iters: usize) -> Vec<Point> {
    if pts.len() < 3 || iters == 0 {
        return pts.to_vec();
    }
    let mut cur = pts.to_vec();
    for _ in 0..iters {
        let mut next = Vec::with_capacity(cur.len() * 2);
        next.push(cur[0].clone()); // preserve start
        for w in cur.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            next.push(Point { x: 0.75 * a.x + 0.25 * b.x, y: 0.75 * a.y + 0.25 * b.y });
            next.push(Point { x: 0.25 * a.x + 0.75 * b.x, y: 0.25 * a.y + 0.75 * b.y });
        }
        next.push(cur[cur.len() - 1].clone()); // preserve end
        cur = next;
    }
    cur
}

// ── Stroke rendering ─────────────────────────────────────────────────────────

pub fn draw_stroke(cr: &Context, stroke: &Stroke) {
    let raw = &stroke.points;
    if raw.is_empty() { return; }

    apply_color(cr, &stroke.color);
    cr.set_line_width(stroke.width);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
    cr.set_antialias(cairo::Antialias::Best);

    if raw.len() == 1 {
        cr.arc(raw[0].x, raw[0].y, stroke.width / 2.0, 0.0, std::f64::consts::TAU);
        cr.fill().unwrap();
        return;
    }

    // Apply Chaikin smoothing — 3 iterations gives very smooth curves
    let pts = chaikin(raw, 3);

    cr.move_to(pts[0].x, pts[0].y);
    for p in pts.iter().skip(1) {
        cr.line_to(p.x, p.y);
    }
    cr.stroke().unwrap();
}

// ── Shape rendering ───────────────────────────────────────────────────────────

pub fn draw_shape(cr: &Context, shape: &Shape) {
    apply_color(cr, &shape.color);
    cr.set_line_width(shape.width);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_antialias(cairo::Antialias::Best);

    let (x1, y1, x2, y2) = (shape.x1, shape.y1, shape.x2, shape.y2);
    match shape.kind {
        ShapeKind::Line => {
            cr.move_to(x1, y1);
            cr.line_to(x2, y2);
            cr.stroke().unwrap();
        }
        ShapeKind::Rectangle => {
            let (rx, ry) = (x1.min(x2), y1.min(y2));
            let (rw, rh) = ((x2 - x1).abs(), (y2 - y1).abs());
            cr.rectangle(rx, ry, rw, rh);
            cr.stroke().unwrap();
        }
        ShapeKind::Circle => {
            let cx = (x1 + x2) / 2.0;
            let cy = (y1 + y2) / 2.0;
            let rx = (x2 - x1).abs() / 2.0;
            let ry = (y2 - y1).abs() / 2.0;
            if rx > 0.0 && ry > 0.0 {
                cr.save().unwrap();
                cr.translate(cx, cy);
                cr.scale(rx, ry);
                cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
                cr.restore().unwrap();
                cr.stroke().unwrap();
            }
        }
        ShapeKind::Arrow => {
            cr.move_to(x1, y1);
            cr.line_to(x2, y2);
            cr.stroke().unwrap();
            let angle = (y2 - y1).atan2(x2 - x1);
            let head = 14.0_f64;
            let spread = std::f64::consts::PI / 6.0;
            for side in [-1.0_f64, 1.0] {
                cr.move_to(x2, y2);
                cr.line_to(
                    x2 - head * (angle + side * spread).cos(),
                    y2 - head * (angle + side * spread).sin(),
                );
                cr.stroke().unwrap();
            }
        }
        ShapeKind::Star => {
            let cx = (x1 + x2) / 2.0;
            let cy = (y1 + y2) / 2.0;
            let outer = (x2 - x1).abs().min((y2 - y1).abs()) / 2.0;
            let inner = outer * 0.42;
            let n = 5usize;
            let offset = -std::f64::consts::PI / 2.0;
            let mut first = true;
            for i in 0..n * 2 {
                let a = i as f64 * std::f64::consts::PI / n as f64 + offset;
                let r = if i % 2 == 0 { outer } else { inner };
                let (px, py) = (cx + r * a.cos(), cy + r * a.sin());
                if first { cr.move_to(px, py); first = false; } else { cr.line_to(px, py); }
            }
            cr.close_path();
            cr.stroke().unwrap();
        }
        ShapeKind::Heart => {
            let cx = (x1 + x2) / 2.0;
            let cy = (y1 + y2) / 2.0;
            let top = y1.min(y2);
            let bot = y1.max(y2);
            let lft = x1.min(x2);
            let rgt = x1.max(x2);
            let w = rgt - lft;
            let h = bot - top;
            cr.move_to(cx, bot);
            cr.curve_to(lft - w*0.1, cy, lft, top + h*0.1, cx - w*0.25, top + h*0.1);
            cr.curve_to(cx, top - h*0.2, cx, top - h*0.2, cx + w*0.25, top + h*0.1);
            cr.curve_to(rgt, top + h*0.1, rgt + w*0.1, cy, cx, bot);
            cr.close_path();
            cr.stroke().unwrap();
        }

        ShapeKind::Triangle => {
            let cx = (x1 + x2) / 2.0;
            let top = y1.min(y2);
            let bot = y1.max(y2);
            cr.move_to(cx, top);
            cr.line_to(x1.max(x2), bot);
            cr.line_to(x1.min(x2), bot);
            cr.close_path();
            cr.stroke().unwrap();
        }
        ShapeKind::Diamond => {
            let cx = (x1 + x2) / 2.0;
            let cy = (y1 + y2) / 2.0;
            cr.move_to(cx, y1.min(y2));
            cr.line_to(x1.max(x2), cy);
            cr.line_to(cx, y1.max(y2));
            cr.line_to(x1.min(x2), cy);
            cr.close_path();
            cr.stroke().unwrap();
        }

    }
}

// ── Pasted image rendering ────────────────────────────────────────────────────

pub fn draw_canvas_image(cr: &Context, img: &CanvasImage) {
    let mut cursor = std::io::Cursor::new(&img.png_data);
    if let Ok(surface) = cairo::ImageSurface::create_from_png(&mut cursor) {
        let sw = surface.width() as f64;
        let sh = surface.height() as f64;
        cr.save().unwrap();
        cr.translate(img.x, img.y);
        if sw > 0.0 && sh > 0.0 {
            cr.scale(img.width / sw, img.height / sh);
        }
        cr.set_source_surface(&surface, 0.0, 0.0).unwrap();
        cr.paint().unwrap();
        cr.restore().unwrap();
    }
}

// ── Text rendering ────────────────────────────────────────────────────────────

pub fn draw_canvas_text(cr: &Context, txt: &crate::state::CanvasText) {
    cr.save().unwrap();
    apply_color(cr, &txt.color);
    
    // Choose font generic family
    let slant = cairo::FontSlant::Normal;
    let weight = cairo::FontWeight::Normal;
    cr.select_font_face(&txt.font_family, slant, weight);
    cr.set_font_size(txt.font_size);
    
    // Cairo text starts drawing from the bottom-left baseline. We want x,y to be top-left.
    let extents = cr.font_extents().unwrap();
    cr.move_to(txt.x, txt.y + extents.ascent());
    
    cr.show_text(&txt.text).unwrap();
    cr.restore().unwrap();
}

// ── Custom cursor ─────────────────────────────────────────────────────────────

pub fn draw_cursor(cr: &Context, cx: f64, cy: f64, drawing: bool) {
    let radius = if drawing { 5.0 } else { 9.0 };
    cr.set_antialias(cairo::Antialias::Best);
    cr.set_source_rgba(0.18, 0.42, 0.95, 0.85);
    cr.set_line_width(1.5);
    cr.arc(cx, cy, radius, 0.0, std::f64::consts::TAU);
    cr.stroke().unwrap();
    cr.set_source_rgba(0.18, 0.42, 0.95, 1.0);
    cr.arc(cx, cy, 1.8, 0.0, std::f64::consts::TAU);
    cr.fill().unwrap();
    cr.set_source_rgba(0.18, 0.42, 0.95, 0.45);
    cr.set_line_width(1.0);
    let gap = radius + 3.5;
    let arm = 5.0;
    for (ax, ay, bx, by) in [
        (cx - gap - arm, cy, cx - gap, cy),
        (cx + gap, cy, cx + gap + arm, cy),
        (cx, cy - gap - arm, cx, cy - gap),
        (cx, cy + gap, cx, cy + gap + arm),
    ] {
        cr.move_to(ax, ay);
        cr.line_to(bx, by);
        cr.stroke().unwrap();
    }
}
