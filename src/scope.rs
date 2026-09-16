use crate::data::aircraft::Aircraft;
use crate::geometry::sweep_angle_deg;
use crate::theme::Palette;
use crate::trail::TrailStore;
use crate::{braille_scope, sixel_scope};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui_image::picker::Picker;
use std::time::Instant;

pub const MIN_ZOOM_NM: f64 = 5.0;
pub const MAX_ZOOM_NM: f64 = 100.0;
const CONTROLS: &str = "q   +/-   0   v";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Anti-aliased CRT-glow graphics via an off-screen rasterizer + Sixel.
    /// Better looking, but real-world per-frame cost (mostly the terminal's
    /// own Sixel decode, not this app) can make animation less smooth.
    Sixel,
    /// The original braille-Canvas renderer: character-based, no image
    /// encoding, so it's inherently fast and animates smoothly.
    Braille,
}

impl RenderMode {
    pub fn toggled(self) -> Self {
        match self {
            RenderMode::Sixel => RenderMode::Braille,
            RenderMode::Braille => RenderMode::Sixel,
        }
    }
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
    mode: RenderMode,
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

    let angle = sweep_angle_deg(sweep_start);
    match mode {
        RenderMode::Sixel => sixel_scope::render(
            frame,
            inner,
            aircraft,
            trails,
            zoom_radius_nm,
            angle,
            palette,
            picker,
            font,
        ),
        RenderMode::Braille => braille_scope::render(
            frame,
            inner,
            aircraft,
            trails,
            zoom_radius_nm,
            angle,
            palette,
        ),
    }
}
