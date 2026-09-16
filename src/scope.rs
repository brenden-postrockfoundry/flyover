use crate::data::aircraft::{Aircraft, Altitude};
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

/// adsb.lol gives distance (nm) and bearing (degrees, 0 = north, clockwise)
/// from the query point directly, so plotting a contact is just polar-to-
/// cartesian — no lat/lon projection math needed.
fn bearing_to_xy(dst_nm: f64, dir_deg: f64) -> (f64, f64) {
    let rad = dir_deg.to_radians();
    (dst_nm * rad.sin(), dst_nm * rad.cos())
}

fn sweep_angle_deg(sweep_start: Instant) -> f64 {
    let elapsed = sweep_start.elapsed().as_secs_f64();
    let period = SWEEP_PERIOD.as_secs_f64();
    (elapsed / period * 360.0) % 360.0
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    title: String,
    aircraft: &[Aircraft],
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

    let angle = sweep_angle_deg(sweep_start);
    // Scaled per-axis (rather than via bearing_to_xy) so the sweep line reaches
    // the visual edge on both axes even though x/y bounds differ after the
    // aspect correction above.
    let angle_rad = angle.to_radians();
    let sweep_x = x_reach * angle_rad.sin();
    let sweep_y = zoom_radius_nm * angle_rad.cos();

    let label_offset_x = x_reach * 0.02;
    let label_offset_y = zoom_radius_nm * 0.02;

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

            for ac in aircraft {
                let (Some(dst), Some(dir)) = (ac.dst, ac.dir) else {
                    continue;
                };
                if dst > zoom_radius_nm {
                    continue;
                }
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

                let alt = match ac.alt_baro {
                    Some(Altitude::Feet(ft)) => format!("FL{:03}", ft / 100),
                    Some(Altitude::Ground) => "GND".to_string(),
                    None => "?".to_string(),
                };
                let climb = match ac.climb_rate() {
                    Some(r) if r > 100.0 => "^",
                    Some(r) if r < -100.0 => "v",
                    _ => "",
                };
                let tag = format!(
                    "{} {} {:.0}kt{}",
                    ac.callsign(),
                    alt,
                    ac.gs.unwrap_or(0.0),
                    climb
                );

                ctx.print(
                    x + label_offset_x,
                    y + label_offset_y,
                    TextLine::styled(tag, Style::default().fg(color)),
                );
            }
        });

    frame.render_widget(canvas, area);
}
