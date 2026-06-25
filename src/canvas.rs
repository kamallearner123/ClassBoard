use cairo::Context;
use crate::state::{CanvasImage, Color, Point, Shape, ShapeKind, Stroke, Spray};

pub fn apply_color(cr: &Context, c: &Color) {
    cr.set_source_rgba(c.r, c.g, c.b, c.a);
}

// ── Midpoint Quadratic Bezier Smoothing ─────────────────────────────────────────
// Generates perfectly smooth curves that never loop or overshoot (unlike Catmull-Rom)
// and handles sparse points beautifully (unlike Chaikin).

fn draw_smooth_curve(cr: &Context, pts: &[Point]) {
    if pts.len() < 2 { return; }
    if pts.len() == 2 {
        cr.move_to(pts[0].x, pts[0].y);
        cr.line_to(pts[1].x, pts[1].y);
        return;
    }

    cr.move_to(pts[0].x, pts[0].y);
    
    // Draw line to the first midpoint
    let mut p0_x = (pts[0].x + pts[1].x) / 2.0;
    let mut p0_y = (pts[0].y + pts[1].y) / 2.0;
    cr.line_to(p0_x, p0_y);

    for i in 1..pts.len() - 1 {
        let cp = &pts[i];
        let next = &pts[i + 1];
        
        let p2_x = (cp.x + next.x) / 2.0;
        let p2_y = (cp.y + next.y) / 2.0;

        // Convert Quadratic Bezier (P0, CP, P2) to Cubic Bezier (P0, CP1, CP2, P2)
        // CP1 = P0 + 2/3 * (CP - P0)
        let cp1_x = p0_x + (2.0 / 3.0) * (cp.x - p0_x);
        let cp1_y = p0_y + (2.0 / 3.0) * (cp.y - p0_y);
        
        // CP2 = P2 + 2/3 * (CP - P2)
        let cp2_x = p2_x + (2.0 / 3.0) * (cp.x - p2_x);
        let cp2_y = p2_y + (2.0 / 3.0) * (cp.y - p2_y);

        cr.curve_to(cp1_x, cp1_y, cp2_x, cp2_y, p2_x, p2_y);
        
        p0_x = p2_x;
        p0_y = p2_y;
    }
    
    // Draw line to the final point
    let last = &pts[pts.len() - 1];
    cr.line_to(last.x, last.y);
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

    if stroke.is_highlighter {
        cr.set_operator(cairo::Operator::Multiply);
    } else {
        cr.set_operator(cairo::Operator::Over);
    }

    if raw.len() == 1 {
        cr.arc(raw[0].x, raw[0].y, stroke.width / 2.0, 0.0, std::f64::consts::TAU);
        cr.fill().unwrap();
        return;
    }

    // Draw smooth curve using Catmull-Rom to Bezier
    draw_smooth_curve(cr, raw);
    cr.stroke().unwrap();
    
    if stroke.is_highlighter {
        cr.set_operator(cairo::Operator::Over);
    }
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
            let rx = x1.min(x2);
            let ry = y1.min(y2);
            let w = (x2 - x1).abs();
            let h = (y2 - y1).abs();
            
            if w > 0.0 && h > 0.0 {
                cr.save().unwrap();
                cr.translate(rx, ry);
                cr.scale(w / 100.0, h / 100.0);
                
                cr.move_to(50.0, 30.0);
                cr.curve_to(50.0, 0.0, 0.0, 0.0, 0.0, 40.0);
                cr.curve_to(0.0, 70.0, 50.0, 90.0, 50.0, 100.0);
                cr.curve_to(50.0, 90.0, 100.0, 70.0, 100.0, 40.0);
                cr.curve_to(100.0, 0.0, 50.0, 0.0, 50.0, 30.0);
                cr.close_path();
                
                cr.restore().unwrap();
                cr.stroke().unwrap();
            }
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
        ShapeKind::RoundedRect => {
            let rx = x1.min(x2);
            let ry = y1.min(y2);
            let w = (x2 - x1).abs();
            let h = (y2 - y1).abs();
            let r = 10.0_f64.min(w / 2.0).min(h / 2.0); // radius
            cr.new_sub_path();
            cr.arc(rx + w - r, ry + r, r, -std::f64::consts::PI / 2.0, 0.0);
            cr.arc(rx + w - r, ry + h - r, r, 0.0, std::f64::consts::PI / 2.0);
            cr.arc(rx + r, ry + h - r, r, std::f64::consts::PI / 2.0, std::f64::consts::PI);
            cr.arc(rx + r, ry + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::PI / 2.0);
            cr.close_path();
            cr.stroke().unwrap();
        }
        ShapeKind::UmlClass => {
            let rx = x1.min(x2);
            let ry = y1.min(y2);
            let w = (x2 - x1).abs();
            let h = (y2 - y1).abs();
            cr.rectangle(rx, ry, w, h);
            cr.move_to(rx, ry + 30.0_f64.min(h / 2.0));
            cr.line_to(rx + w, ry + 30.0_f64.min(h / 2.0));
            cr.stroke().unwrap();
        }
        ShapeKind::Actor => {
            let cx = (x1 + x2) / 2.0;
            let top = y1.min(y2);
            let bot = y1.max(y2);
            let w = (x2 - x1).abs();
            let h = bot - top;
            let head_r = (w * 0.2).min(h * 0.15);
            
            // Head
            cr.arc(cx, top + head_r, head_r, 0.0, std::f64::consts::TAU);
            cr.stroke().unwrap();
            
            // Body
            cr.move_to(cx, top + head_r * 2.0);
            cr.line_to(cx, top + h * 0.6);
            
            // Arms
            cr.move_to(cx - w * 0.4, top + h * 0.35);
            cr.line_to(cx + w * 0.4, top + h * 0.35);
            
            // Legs
            cr.move_to(cx, top + h * 0.6);
            cr.line_to(cx - w * 0.3, bot);
            cr.move_to(cx, top + h * 0.6);
            cr.line_to(cx + w * 0.3, bot);
            
            cr.stroke().unwrap();
        }
        ShapeKind::Database => {
            let rx = x1.min(x2);
            let ry = y1.min(y2);
            let w = (x2 - x1).abs();
            let h = (y2 - y1).abs();
            let er = (h * 0.15).max(5.0); // ellipse y-radius
            
            if w > 0.0 && h > 0.0 {
                cr.save().unwrap();
                
                // Top ellipse
                cr.save().unwrap();
                cr.translate(rx + w / 2.0, ry + er);
                cr.scale(w / 2.0, er);
                cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
                cr.restore().unwrap();
                cr.stroke().unwrap();
                
                // Bottom half-ellipse
                cr.save().unwrap();
                cr.translate(rx + w / 2.0, ry + h - er);
                cr.scale(w / 2.0, er);
                cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::PI);
                cr.restore().unwrap();
                
                // Sides
                cr.move_to(rx, ry + er);
                cr.line_to(rx, ry + h - er);
                cr.move_to(rx + w, ry + er);
                cr.line_to(rx + w, ry + h - er);
                
                cr.stroke().unwrap();
                cr.restore().unwrap();
            }
        }
        ShapeKind::Cloud => {
            let rx = x1.min(x2);
            let ry = y1.min(y2);
            let w = (x2 - x1).abs();
            let h = (y2 - y1).abs();
            
            if w > 0.0 && h > 0.0 {
                cr.save().unwrap();
                cr.translate(rx, ry);
                cr.scale(w / 100.0, h / 100.0);
                
                cr.move_to(20.0, 70.0);
                cr.curve_to(0.0, 70.0, 0.0, 40.0, 25.0, 40.0);
                cr.curve_to(25.0, 10.0, 65.0, 10.0, 70.0, 35.0);
                cr.curve_to(100.0, 30.0, 100.0, 70.0, 80.0, 70.0);
                cr.close_path();
                
                cr.restore().unwrap();
                cr.stroke().unwrap();
            }
        }
    }
}

// ── Spray rendering ─────────────────────────────────────────────────────────────

pub fn draw_spray(cr: &Context, spray: &Spray) {
    apply_color(cr, &spray.color);
    cr.set_antialias(cairo::Antialias::None); // For performance with many particles
    for p in &spray.points {
        cr.rectangle(p.x, p.y, 1.5, 1.5);
    }
    cr.fill().unwrap();
    cr.set_antialias(cairo::Antialias::Best);
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
    
    // Choose font generic family
    let slant = cairo::FontSlant::Normal;
    let weight = cairo::FontWeight::Normal;
    cr.select_font_face(&txt.font_family, slant, weight);
    cr.set_font_size(txt.font_size);
    
    let extents = cr.font_extents().unwrap();
    let lines: Vec<&str> = txt.text.split('\n').collect();
    let mut max_w: f64 = 0.0;
    for line in &lines {
        if let Ok(text_extents) = cr.text_extents(line) {
            if text_extents.width() > max_w { max_w = text_extents.width(); }
        }
    }

    if let Some(bg) = &txt.bg_color {
        let pad_x = 12.0;
        let pad_y = 12.0;
        let w = max_w.max(100.0);
        let h = (extents.height() * lines.len().max(1) as f64).max(50.0);
        
        // Draw drop shadow
        cr.save().unwrap();
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.2);
        cr.rectangle(txt.x - pad_x + 3.0, txt.y - pad_y + 3.0, w + pad_x * 2.0, h + pad_y * 2.0);
        cr.fill().unwrap();
        cr.restore().unwrap();
        
        // Draw sticky background
        cr.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
        cr.rectangle(txt.x - pad_x, txt.y - pad_y, w + pad_x * 2.0, h + pad_y * 2.0);
        cr.fill().unwrap();
    }
    
    apply_color(cr, &txt.color);

    // Cairo text starts drawing from the bottom-left baseline. We want x,y to be top-left.
    let mut current_y = txt.y + extents.ascent();
    let line_height = extents.height();

    for line in lines {
        if !line.is_empty() {
            cr.move_to(txt.x, current_y);
            let _ = cr.show_text(line);
        }
        current_y += line_height;
    }
    
    cr.restore().unwrap();
}

// Custom cursor removed for native performance
