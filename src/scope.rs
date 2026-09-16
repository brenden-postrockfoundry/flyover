use crate::data::aircraft::Aircraft;
use crate::raster::{self, Scene};
use crate::theme::Palette;
use crate::trail::TrailStore;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::StatefulImage;
use std::time::{Duration, Instant};

pub const MIN_ZOOM_NM: f64 = 5.0;
pub const MAX_ZOOM_NM: f64 = 100.0;
const SWEEP_PERIOD: Duration = Duration::from_secs(4);
const CONTROLS: &str = "q   +/-   0";

fn sweep_angle_deg(sweep_start: Instant) -> f64 {
    let elapsed = sweep_start.elapsed().as_secs_f64();
    let period = SWEEP_PERIOD.as_secs_f64();
    (elapsed / period * 360.0) % 360.0
}

#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut Frame,
    area: Rect,
    title: String,
    status: String,
    aircraft: &[Aircraft],
    trails: &TrailStore,
    zoom_radius_nm: f64,
    sweep_start: Instant,
    palette: &Palette,
    picker: &Picker,
    font: &fontdue::Font,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);
    let scope_area = chunks[0];
    let footer_area = chunks[1];

    let footer_text = if footer_area.width as usize >= CONTROLS.len() + status.len() + 4 {
        format!("{CONTROLS}   {status} ")
    } else {
        format!("{CONTROLS} ")
    };
    frame.render_widget(
        Paragraph::new(footer_text)
            .style(Style::default().fg(palette.muted))
            .alignment(Alignment::Right),
        footer_area,
    );

    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(scope_area);
    frame.render_widget(block, scope_area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let font_size = picker.font_size();
    let width_px = u32::from(inner.width) * u32::from(font_size.width);
    let height_px = u32::from(inner.height) * u32::from(font_size.height);

    let scene = Scene {
        width_px,
        height_px,
        aircraft,
        trails,
        zoom_radius_nm,
        sweep_angle_deg: sweep_angle_deg(sweep_start),
        palette,
        font,
    };
    let image = raster::render(&scene);
    let mut protocol = picker.new_resize_protocol(image::DynamicImage::ImageRgba8(image));
    frame.render_stateful_widget(StatefulImage::new(), inner, &mut protocol);
}
