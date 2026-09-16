mod data;

use data::aircraft::Altitude;

fn main() {
    let location = match data::location::load() {
        Ok(loc) => loc,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    println!(
        "Fetching aircraft within 100nm of {} ({:.4}, {:.4})...",
        location.name, location.latitude, location.longitude
    );

    let aircraft = match data::fetch::fetch_nearby(location.latitude, location.longitude) {
        Ok(ac) => ac,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    println!("{} contact(s):", aircraft.len());
    for ac in &aircraft {
        let alt = match ac.alt_baro {
            Some(Altitude::Feet(ft)) => format!("{ft:>6}ft"),
            Some(Altitude::Ground) => "ground".to_string(),
            None => "  ?   ".to_string(),
        };
        let climb = match ac.climb_rate() {
            Some(r) if r > 100.0 => "▲",
            Some(r) if r < -100.0 => "▼",
            _ => " ",
        };
        println!(
            "{:<9} {} {:>6.0}kt  dst={:>6.1}nm dir={:>5.1}deg{}",
            ac.callsign(),
            alt,
            ac.gs.unwrap_or(0.0),
            ac.dst.unwrap_or(0.0),
            ac.dir.unwrap_or(0.0),
            if ac.is_emergency_squawk() { "  EMERGENCY" } else { climb }
        );
    }
}
