use crate::data::aircraft::Aircraft;
use crate::raster::{self, Scene};
use crate::theme::Palette;
use crate::trail::TrailStore;
use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::StatefulImage;

#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut Frame,
    inner: Rect,
    aircraft: &[Aircraft],
    trails: &TrailStore,
    zoom_radius_nm: f64,
    sweep_angle_deg: f64,
    palette: &Palette,
    picker: &Picker,
    font: &fontdue::Font,
) {
    let font_size = picker.font_size();
    let width_px = u32::from(inner.width) * u32::from(font_size.width);
    let height_px = u32::from(inner.height) * u32::from(font_size.height);
    // Match the terminal's own text size instead of an arbitrary constant,
    // per feedback that the labels read smaller than the surrounding UI.
    let label_font_px = f32::from(font_size.height) * 0.85;

    let scene = Scene {
        width_px,
        height_px,
        aircraft,
        trails,
        zoom_radius_nm,
        label_font_px,
        sweep_angle_deg,
        palette,
        font,
    };
    let image = raster::render(&scene);
    let mut protocol = picker.new_resize_protocol(image::DynamicImage::ImageRgba8(image));
    frame.render_stateful_widget(StatefulImage::new(), inner, &mut protocol);
}
