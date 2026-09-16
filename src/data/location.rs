use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Location {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
}

pub fn load() -> Result<Location, String> {
    let path = weather_json_path();
    let contents = std::fs::read_to_string(&path).map_err(|_| {
        format!(
            "No location configured. Set one with `omarchy-weather-location --set \"<name>\" <lat,lon>`, then rerun.\n(expected {})",
            path.display()
        )
    })?;
    serde_json::from_str(&contents).map_err(|e| format!("could not parse {}: {e}", path.display()))
}

fn weather_json_path() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home).join(".local/state/omarchy/settings/weather.json")
}
