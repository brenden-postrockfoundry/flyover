/// adsb.lol gives distance (nm) and bearing (degrees, 0 = north, clockwise)
/// from the query point directly, so plotting a contact is just polar-to-
/// cartesian — no lat/lon projection math needed.
pub fn bearing_to_xy(dst_nm: f64, dir_deg: f64) -> (f64, f64) {
    let rad = dir_deg.to_radians();
    (dst_nm * rad.sin(), dst_nm * rad.cos())
}

// Shared by both render modes so switching modes doesn't also change the
// sweep's timing.
const SWEEP_PERIOD: std::time::Duration = std::time::Duration::from_secs(40);

pub fn sweep_angle_deg(sweep_start: std::time::Instant) -> f64 {
    let elapsed = sweep_start.elapsed().as_secs_f64();
    let period = SWEEP_PERIOD.as_secs_f64();
    (elapsed / period * 360.0) % 360.0
}
