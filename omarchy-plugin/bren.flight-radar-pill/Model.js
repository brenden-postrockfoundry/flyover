// weather.json holds {"name": ..., "latitude": ..., "longitude": ...}, owned
// by omarchy-weather-location. Reused here as the scope's center point
// instead of asking for a location a second time.
function parseLocationFile(raw) {
  var unset = { latitude: NaN, longitude: NaN }
  try {
    var data = JSON.parse(String(raw || ""))
    if (!data || typeof data !== "object") return unset
    var latitude = parseFloat(data.latitude)
    var longitude = parseFloat(data.longitude)
    if (isNaN(latitude) || isNaN(longitude)) return unset
    return { latitude: latitude, longitude: longitude }
  } catch (e) {
    return unset
  }
}

// adsb.lol's point-query response: {"ac": [...]}. -1 signals "couldn't
// parse" so the pill can show a distinct "unknown" state from "zero nearby".
function countAircraft(raw) {
  try {
    var data = JSON.parse(String(raw || "{}"))
    var ac = data.ac
    return Array.isArray(ac) ? ac.length : -1
  } catch (e) {
    return -1
  }
}
