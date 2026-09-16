use crate::data::aircraft::{Aircraft, Altitude};
use crate::geometry::bearing_to_xy;
use crate::theme::Palette;
use crate::trail::TrailStore;
use image::RgbaImage;
use ratatui::style::Color;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, PremultipliedColorU8, Shader, Stroke, Transform};

const RING_COUNT: u32 = 4;
const LABEL_ROWS: usize = 3;

pub struct Scene<'a> {
    pub width_px: u32,
    pub height_px: u32,
    pub aircraft: &'a [Aircraft],
    pub trails: &'a TrailStore,
    pub zoom_radius_nm: f64,
    pub sweep_angle_deg: f64,
    pub palette: &'a Palette,
    pub font: &'a fontdue::Font,
    /// Label text size in px, derived from the terminal's actual detected
    /// cell height so labels read at the same size as the surrounding
    /// terminal/bar text instead of an arbitrary guessed constant.
    pub label_font_px: f32,
}

pub fn render(scene: &Scene) -> RgbaImage {
    let mut pixmap =
        Pixmap::new(scene.width_px.max(1), scene.height_px.max(1)).expect("nonzero pixmap size");
    pixmap.fill(to_skia(scene.palette.background, 255));

    let cx = scene.width_px as f32 / 2.0;
    let cy = scene.height_px as f32 / 2.0;
    // Pixels are square, unlike terminal cells, so no aspect correction is
    // needed here for the rings to actually look circular.
    let radius_px = (scene.width_px.min(scene.height_px) as f32 / 2.0) * 0.94;
    let px_per_nm = radius_px / scene.zoom_radius_nm.max(0.001) as f32;

    let to_px = |nm_x: f64, nm_y: f64| -> (f32, f32) {
        (
            cx + nm_x as f32 * px_per_nm,
            cy - nm_y as f32 * px_per_nm, // screen y grows downward; nm y is north-up
        )
    };

    draw_rings(&mut pixmap, cx, cy, radius_px, scene.palette.muted);
    draw_sweep(
        &mut pixmap,
        cx,
        cy,
        radius_px,
        scene.sweep_angle_deg,
        scene.palette.accent,
    );
    draw_trails(&mut pixmap, scene, &to_px);
    draw_contacts(&mut pixmap, scene, &to_px, px_per_nm);

    pixmap_to_image(pixmap)
}

fn to_skia(color: Color, alpha: u8) -> tiny_skia::Color {
    let (r, g, b) = match color {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (255, 255, 255),
    };
    tiny_skia::Color::from_rgba8(r, g, b, alpha)
}

fn solid_paint(color: tiny_skia::Color) -> Paint<'static> {
    Paint {
        shader: Shader::SolidColor(color),
        anti_alias: true,
        ..Default::default()
    }
}

fn fill_circle(pixmap: &mut Pixmap, x: f32, y: f32, r: f32, color: tiny_skia::Color) {
    let Some(path) = PathBuilder::from_circle(x, y, r) else {
        return;
    };
    pixmap.fill_path(
        &path,
        &solid_paint(color),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn draw_rings(pixmap: &mut Pixmap, cx: f32, cy: f32, radius_px: f32, color: Color) {
    let paint = solid_paint(to_skia(color, 140));
    let stroke = Stroke {
        width: 1.0,
        ..Default::default()
    };
    for i in 1..=RING_COUNT {
        let r = radius_px * f32::from(i as u16) / f32::from(RING_COUNT as u16);
        if let Some(path) = PathBuilder::from_circle(cx, cy, r) {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
}

fn draw_sweep(pixmap: &mut Pixmap, cx: f32, cy: f32, radius_px: f32, angle_deg: f64, color: Color) {
    let rad = angle_deg.to_radians();
    let ex = cx + radius_px * rad.sin() as f32;
    let ey = cy - radius_px * rad.cos() as f32;

    // Layered strokes, wide+dim to narrow+bright, simulate a phosphor glow.
    const LAYERS: [(f32, u8); 3] = [(8.0, 30), (3.5, 90), (1.2, 220)];
    for (width, alpha) in LAYERS {
        let mut pb = PathBuilder::new();
        pb.move_to(cx, cy);
        pb.line_to(ex, ey);
        let Some(path) = pb.finish() else { continue };
        let paint = solid_paint(to_skia(color, alpha));
        let stroke = Stroke {
            width,
            ..Default::default()
        };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

fn draw_trails(pixmap: &mut Pixmap, scene: &Scene, to_px: &dyn Fn(f64, f64) -> (f32, f32)) {
    for ac in scene.aircraft {
        let Some(trail) = scene.trails.get(&ac.hex) else {
            continue;
        };
        let base = if ac.is_emergency_squawk() {
            scene.palette.alert
        } else {
            scene.palette.foreground
        };
        let len = trail.len();
        for (i, (tx, ty)) in trail.iter().enumerate() {
            let t = (i + 1) as f64 / (len + 1) as f64;
            let (px, py) = to_px(*tx, *ty);
            let alpha = (30.0 + t * 140.0) as u8;
            fill_circle(pixmap, px, py, 1.6, to_skia(base, alpha));
        }
    }
}

struct LabelBox {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

fn boxes_overlap(a: &LabelBox, b: &LabelBox) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

fn draw_contacts(
    pixmap: &mut Pixmap,
    scene: &Scene,
    to_px: &dyn Fn(f64, f64) -> (f32, f32),
    px_per_nm: f32,
) {
    let mut contacts: Vec<&Aircraft> = scene
        .aircraft
        .iter()
        .filter(|ac| matches!(ac.dst, Some(d) if d <= scene.zoom_radius_nm))
        .collect();
    // Stable order keeps label placement from flickering between frames.
    contacts.sort_by(|a, b| a.hex.cmp(&b.hex));

    let mut placed: Vec<LabelBox> = Vec::with_capacity(contacts.len());
    let gap = (px_per_nm * 0.6).max(4.0);

    for ac in &contacts {
        let (Some(dst), Some(dir)) = (ac.dst, ac.dir) else {
            continue;
        };
        let (nx, ny) = bearing_to_xy(dst, dir);
        let (px, py) = to_px(nx, ny);
        let base = if ac.is_emergency_squawk() {
            scene.palette.alert
        } else {
            scene.palette.foreground
        };

        // Soft glow halo behind a bright core dot.
        fill_circle(pixmap, px, py, 4.5, to_skia(base, 45));
        fill_circle(pixmap, px, py, 1.4, to_skia(base, 255));

        let climb = match ac.climb_rate() {
            Some(r) if r > 100.0 => " ▲",
            Some(r) if r < -100.0 => " ▼",
            _ => "",
        };
        let alt = match ac.alt_baro {
            Some(Altitude::Feet(ft)) => format!("FL{:03}{climb}", ft / 100),
            Some(Altitude::Ground) => format!("GND{climb}"),
            None => format!("?{climb}"),
        };
        let lines = [
            ac.callsign().to_string(),
            alt,
            format!("{:.0}kt", ac.gs.unwrap_or(0.0)),
        ];

        let (width_px, line_h) = measure(scene.font, scene.label_font_px, &lines);
        let height_px = line_h * LABEL_ROWS as f32;

        let candidates = [
            (px + gap, py - gap - height_px),
            (px + gap, py + gap),
            (px - gap - width_px, py - gap - height_px),
            (px - gap - width_px, py + gap),
            (px + gap * 3.0, py - gap * 3.0 - height_px),
            (px + gap * 3.0, py + gap * 3.0),
            (px - gap * 3.0 - width_px, py - gap * 3.0 - height_px),
            (px - gap * 3.0 - width_px, py + gap * 3.0),
        ];

        let mut chosen = candidates[0];
        for candidate in candidates {
            let candidate_box = LabelBox {
                x: candidate.0,
                y: candidate.1,
                w: width_px,
                h: height_px,
            };
            if !placed.iter().any(|p| boxes_overlap(p, &candidate_box)) {
                chosen = candidate;
                break;
            }
        }

        // Keep the label fully on-screen even when its contact is near the
        // edge of the visible range — sliding it back in reads much better
        // than letting text run off the image and get clipped. Only clamps
        // the label's own position; the blip itself is untouched, so this
        // stays correct as the aircraft keeps moving toward/along the edge.
        let margin = gap;
        chosen.0 = chosen
            .0
            .max(margin)
            .min((scene.width_px as f32 - width_px - margin).max(margin));
        chosen.1 = chosen
            .1
            .max(margin)
            .min((scene.height_px as f32 - height_px - margin).max(margin));

        placed.push(LabelBox {
            x: chosen.0,
            y: chosen.1,
            w: width_px,
            h: height_px,
        });

        // draw_text's y is a text baseline, not the top of the glyph — the
        // ascent sits above it. chosen.1/LabelBox treat the label as
        // starting at its visual top (for collision-avoidance and edge
        // clamping), so only the actual draw call needs the baseline
        // conversion, via an approximate ascent fraction of the row height.
        let baseline_offset = line_h * 0.8;
        for (row, line) in lines.iter().enumerate() {
            draw_text(
                pixmap,
                scene.font,
                scene.label_font_px,
                line,
                chosen.0,
                chosen.1 + row as f32 * line_h + baseline_offset,
                base,
            );
        }
    }
}

fn measure(font: &fontdue::Font, font_px: f32, lines: &[String; LABEL_ROWS]) -> (f32, f32) {
    let mut max_w = 0.0f32;
    for line in lines {
        let mut w = 0.0f32;
        for ch in line.chars() {
            w += font.metrics(ch, font_px).advance_width;
        }
        max_w = max_w.max(w);
    }
    (max_w, font_px * 1.25)
}

#[allow(clippy::too_many_arguments)]
fn draw_text(
    pixmap: &mut Pixmap,
    font: &fontdue::Font,
    font_px: f32,
    text: &str,
    x: f32,
    y: f32,
    color: Color,
) {
    let (r, g, b) = match color {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (255, 255, 255),
    };
    let mut pen_x = x;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, font_px);
        let glyph_x = pen_x + metrics.xmin as f32;
        let glyph_y = y - metrics.ymin as f32 - metrics.height as f32;
        blit_glyph(pixmap, &bitmap, metrics.width, metrics.height, glyph_x, glyph_y, (r, g, b));
        pen_x += metrics.advance_width;
    }
}

/// Alpha-composites (standard "over", in premultiplied space) a fontdue
/// coverage bitmap onto the pixmap with a solid color.
fn blit_glyph(
    pixmap: &mut Pixmap,
    bitmap: &[u8],
    w: usize,
    h: usize,
    x0: f32,
    y0: f32,
    (r, g, b): (u8, u8, u8),
) {
    if w == 0 || h == 0 {
        return;
    }
    let pw = pixmap.width() as i32;
    let ph = pixmap.height() as i32;
    let x0i = x0.round() as i32;
    let y0i = y0.round() as i32;
    let data = pixmap.pixels_mut();
    for row in 0..h as i32 {
        let py = y0i + row;
        if py < 0 || py >= ph {
            continue;
        }
        for col in 0..w as i32 {
            let px = x0i + col;
            if px < 0 || px >= pw {
                continue;
            }
            let coverage = u32::from(bitmap[row as usize * w + col as usize]);
            if coverage == 0 {
                continue;
            }
            let idx = (py * pw + px) as usize;
            let dst = data[idx];
            let inv = 255 - coverage;
            let out_r = (((r as u32 * coverage) + dst.red() as u32 * inv) / 255) as u8;
            let out_g = (((g as u32 * coverage) + dst.green() as u32 * inv) / 255) as u8;
            let out_b = (((b as u32 * coverage) + dst.blue() as u32 * inv) / 255) as u8;
            let out_a = ((coverage * 255 + dst.alpha() as u32 * inv) / 255) as u8;
            if let Some(color) = PremultipliedColorU8::from_rgba(out_r, out_g, out_b, out_a) {
                data[idx] = color;
            }
        }
    }
}

fn pixmap_to_image(pixmap: Pixmap) -> RgbaImage {
    let width = pixmap.width();
    let height = pixmap.height();
    let mut out = RgbaImage::new(width, height);
    for (i, px) in pixmap.pixels().iter().enumerate() {
        let x = i as u32 % width;
        let y = i as u32 / width;
        let straight = px.demultiply();
        out.put_pixel(
            x,
            y,
            image::Rgba([straight.red(), straight.green(), straight.blue(), px.alpha()]),
        );
    }
    out
}
