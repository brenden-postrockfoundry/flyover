use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone)]
pub enum Altitude {
    Feet(i64),
    Ground,
}

impl<'de> Deserialize<'de> for Altitude {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Number(n) => n
                .as_i64()
                .map(Altitude::Feet)
                .ok_or_else(|| DeError::custom("alt_baro number out of range")),
            // adsb.lol (and the underlying readsb/dump1090 format) reports "ground"
            // as a literal string instead of a number for aircraft on the ground.
            serde_json::Value::String(s) if s == "ground" => Ok(Altitude::Ground),
            other => Err(DeError::custom(format!(
                "unexpected alt_baro value: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Aircraft {
    pub hex: String,
    pub flight: Option<String>,
    pub r: Option<String>,
    pub t: Option<String>,
    pub alt_baro: Option<Altitude>,
    pub gs: Option<f64>,
    pub track: Option<f64>,
    pub baro_rate: Option<f64>,
    pub geom_rate: Option<f64>,
    pub squawk: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// Distance from the query point in nautical miles, precomputed by adsb.lol.
    pub dst: Option<f64>,
    /// Bearing from the query point in degrees, precomputed by adsb.lol.
    pub dir: Option<f64>,
}

impl Aircraft {
    pub fn callsign(&self) -> &str {
        self.flight.as_deref().unwrap_or(&self.hex).trim()
    }

    pub fn climb_rate(&self) -> Option<f64> {
        self.baro_rate.or(self.geom_rate)
    }

    pub fn is_emergency_squawk(&self) -> bool {
        matches!(self.squawk.as_deref(), Some("7500") | Some("7600") | Some("7700"))
    }
}

#[derive(Debug, Deserialize)]
pub struct StatesResponse {
    pub ac: Vec<Aircraft>,
}
