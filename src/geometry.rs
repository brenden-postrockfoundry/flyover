/// adsb.lol gives distance (nm) and bearing (degrees, 0 = north, clockwise)
/// from the query point directly, so plotting a contact is just polar-to-
/// cartesian — no lat/lon projection math needed.
pub fn bearing_to_xy(dst_nm: f64, dir_deg: f64) -> (f64, f64) {
    let rad = dir_deg.to_radians();
    (dst_nm * rad.sin(), dst_nm * rad.cos())
}
