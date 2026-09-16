use crate::data::aircraft::{Aircraft, Altitude};
use crate::geometry::bearing_to_xy;
use crate::trail::TrailStore;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::symbols::Marker;
use ratatui::text::Line as TextLine;
use ratatui::widgets::canvas::{Canvas, Circle, Line as CanvasLine, Points};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;
use std::time::{Duration, Instant};

pub const MIN_ZOOM_NM: f64 = 5.0;
pub const MAX_ZOOM_NM: f64 = 100.0;
const SWEEP_PERIOD: Duration = Duration::from_secs(4);
const RING_COUNT: u32 = 4;
const LABEL_ROWS: usize = 3;

fn sweep_angle_deg(sweep_start: Instant) -> f64 {
    let elapsed = sweep_start.elapsed().as_secs_f64();
    let period = SWEEP_PERIOD.as_secs_f64();
    (elapsed / period * 360.0) % 360.0
}

/// Fades from a dim tail color (t=0, oldest) toward bright green (t close to
/// 1) without ever reaching the full brightness reserved for the live blip.
fn trail_color(t: f64) -> Color {
    let g = (40.0 + t.clamp(0.0, 1.0) * 175.0).round() as u8;
    Color::Rgb(0, g, 0)
}

/// Axis-aligned box anchored at its top-left corner (x, y), extending right
/// by `w` and down by `h` — "down" meaning decreasing y, since canvas space
/// is y-up (north-up).
struct LabelBox {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

fn boxes_overlap(a: &LabelBox, b: &LabelBox) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && (a.y - a.h) < b.y && (b.y - b.h) < a.y
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    title: String,
    aircraft: &[Aircraft],
    trails: &TrailStore,
    zoom_radius_nm: f64,
    sweep_start: Instant,
) {
    // Terminal cells are roughly twice as tall as they are wide, so without
    // this correction the range rings render as ellipses, not circles.
    let aspect = if area.height > 0 {
        (area.width as f64) / (area.height as f64 * 2.0)
    } else {
        1.0
    };
    let x_reach = zoom_radius_nm * aspect.max(0.5);

    let angle_rad = sweep_angle_deg(sweep_start).to_radians();
    let sweep_x = x_reach * angle_rad.sin();
    let sweep_y = zoom_radius_nm * angle_rad.cos();

    // Size a label's bounding box in nm using the actual terminal cell size,
    // since that's the granularity text is drawn at regardless of the
    // higher-resolution braille marker used for shapes.
    let nm_per_col = (2.0 * x_reach) / f64::from(area.width.max(1));
    let nm_per_row = (2.0 * zoom_radius_nm) / f64::from(area.height.max(1));
    let gap_x = nm_per_col;
    let gap_y = nm_per_row;

    let mut contacts: Vec<&Aircraft> = aircraft
        .iter()
        .filter(|ac| matches!((ac.dst, ac.dir), (Some(d), _) if d <= zoom_radius_nm))
        .collect();
    // Stable processing order keeps label placement from flickering between
    // frames as aircraft positions shift by tiny amounts.
    contacts.sort_by(|a, b| a.hex.cmp(&b.hex));

    let canvas = Canvas::default()
        .block(Block::default().borders(Borders::ALL).title(title))
        .marker(Marker::Braille)
        .x_bounds([-x_reach, x_reach])
        .y_bounds([-zoom_radius_nm, zoom_radius_nm])
        .paint(move |ctx| {
            for i in 1..=RING_COUNT {
                let frac = f64::from(i) / f64::from(RING_COUNT);
                ctx.draw(&Circle {
                    x: 0.0,
                    y: 0.0,
                    radius: zoom_radius_nm * frac,
                    color: Color::DarkGray,
                });
            }

            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: 0.0,
                x2: sweep_x,
                y2: sweep_y,
                color: Color::Green,
            });

            for ac in &contacts {
                if let Some(trail) = trails.get(&ac.hex) {
                    let len = trail.len();
                    for (i, (tx, ty)) in trail.iter().enumerate() {
                        let t = (i + 1) as f64 / (len + 1) as f64;
                        ctx.draw(&Points {
                            coords: &[(*tx, *ty)],
                            color: trail_color(t),
                        });
                    }
                }
            }

            let mut placed: Vec<LabelBox> = Vec::with_capacity(contacts.len());

            for ac in &contacts {
                let (dst, dir) = match (ac.dst, ac.dir) {
                    (Some(d), Some(b)) => (d, b),
                    _ => continue,
                };
                let (x, y) = bearing_to_xy(dst, dir);
                let color = if ac.is_emergency_squawk() {
                    Color::Red
                } else {
                    Color::Green
                };

                ctx.draw(&Points {
                    coords: &[(x, y)],
                    color,
                });

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

                let width_nm = lines
                    .iter()
                    .map(|l| l.chars().count())
                    .max()
                    .unwrap_or(0) as f64
                    * nm_per_col;
                let height_nm = LABEL_ROWS as f64 * nm_per_row;

                let candidates = [
                    (x + gap_x, y + gap_y + height_nm),                 // NE
                    (x + gap_x, y - gap_y),                              // SE
                    (x - gap_x - width_nm, y + gap_y + height_nm),       // NW
                    (x - gap_x - width_nm, y - gap_y),                   // SW
                    (x + gap_x * 3.0, y + gap_y * 3.0 + height_nm),      // far NE
                    (x + gap_x * 3.0, y - gap_y * 3.0),                  // far SE
                    (x - gap_x * 3.0 - width_nm, y + gap_y * 3.0 + height_nm), // far NW
                    (x - gap_x * 3.0 - width_nm, y - gap_y * 3.0),       // far SW
                ];

                let mut chosen = candidates[0];
                for candidate in candidates {
                    let candidate_box = LabelBox {
                        x: candidate.0,
                        y: candidate.1,
                        w: width_nm,
                        h: height_nm,
                    };
                    if !placed.iter().any(|p| boxes_overlap(p, &candidate_box)) {
                        chosen = candidate;
                        break;
                    }
                }
                placed.push(LabelBox {
                    x: chosen.0,
                    y: chosen.1,
                    w: width_nm,
                    h: height_nm,
                });

                let style = Style::default().fg(color);
                for (row, line) in lines.iter().enumerate() {
                    ctx.print(
                        chosen.0,
                        chosen.1 - row as f64 * nm_per_row,
                        TextLine::styled(line.clone(), style),
                    );
                }
            }
        });

    frame.render_widget(canvas, area);
}
