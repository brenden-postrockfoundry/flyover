use crate::data::aircraft::{Aircraft, StatesResponse};

const MAX_RADIUS_NM: u32 = 100;

pub fn fetch_nearby(lat: f64, lon: f64) -> Result<Vec<Aircraft>, String> {
    let url = format!("https://api.adsb.lol/v2/point/{lat}/{lon}/{MAX_RADIUS_NM}");
    let mut response = ureq::get(&url)
        .call()
        .map_err(|e| format!("request to adsb.lol failed: {e}"))?;
    let body: StatesResponse = response
        .body_mut()
        .read_json()
        .map_err(|e| format!("could not parse adsb.lol response: {e}"))?;
    Ok(body.ac)
}
