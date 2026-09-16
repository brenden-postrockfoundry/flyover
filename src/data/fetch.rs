use crate::data::aircraft::{Aircraft, StatesResponse};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const MAX_RADIUS_NM: u32 = 100;
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(10);

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

/// Spawns a background thread that fetches on a fixed interval and pushes
/// each result (success or failure) down the returned channel. The render
/// loop never blocks on network I/O — it just drains this non-blockingly.
pub fn spawn_poller(lat: f64, lon: f64) -> mpsc::Receiver<Result<Vec<Aircraft>, String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || loop {
        let result = fetch_nearby(lat, lon);
        if tx.send(result).is_err() {
            break;
        }
        thread::sleep(REFRESH_INTERVAL);
    });
    rx
}
