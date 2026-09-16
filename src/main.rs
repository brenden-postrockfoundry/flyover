mod data;
mod font;
mod geometry;
mod raster;
mod scope;
mod theme;
mod trail;
mod tui;

use crossterm::event::{self, Event, KeyCode};
use data::aircraft::{Aircraft, Altitude};
use ratatui_image::picker::Picker;
use std::time::{Duration, Instant};
use theme::ThemeWatcher;
use trail::TrailStore;

const ZOOM_STEP_NM: f64 = 5.0;
const DEFAULT_ZOOM_NM: f64 = 40.0;
const THEME_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Dev-only: `flyover --preview out.png` renders a synthetic scene straight
/// to a PNG, bypassing the terminal/network entirely — useful for checking
/// the raster output without a live TTY (which this can't assume exists).
fn run_preview(out_path: &str) -> std::io::Result<()> {
    let font = font::load_monospace().map_err(std::io::Error::other)?;
    let palette = ThemeWatcher::new().palette;

    fn ac(hex: &str, flight: &str, alt_ft: i64, gs: f64, rate: f64, squawk: &str, dst: f64, dir: f64) -> Aircraft {
        Aircraft {
            hex: hex.to_string(),
            flight: Some(flight.to_string()),
            r: None,
            t: None,
            alt_baro: Some(Altitude::Feet(alt_ft)),
            gs: Some(gs),
            track: Some(dir),
            baro_rate: Some(rate),
            geom_rate: None,
            squawk: Some(squawk.to_string()),
            lat: None,
            lon: None,
            dst: Some(dst),
            dir: Some(dir),
        }
    }

    let aircraft = vec![
        ac("a1", "UAL1234", 35000, 420.0, 0.0, "1200", 20.0, 45.0),
        ac("a2", "SWA1563", 8000, 250.0, -1800.0, "1200", 12.0, 200.0),
        ac("a3", "ENY3937", 34000, 445.0, 900.0, "1200", 30.0, 300.0),
        ac("a4", "N247JH", 5000, 150.0, 0.0, "7700", 15.0, 130.0),
        ac("a5", "AAL2159", 36000, 406.0, 0.0, "1200", 5.0, 5.0),
    ];

    let mut trails = TrailStore::default();
    // Feed a few slightly-shifted snapshots so each contact has real trail
    // history to render (a single point wouldn't show the comet fade).
    for step in 0..5 {
        let shifted: Vec<Aircraft> = aircraft
            .iter()
            .map(|a| {
                let mut a = a.clone();
                a.dst = a.dst.map(|d| d - f64::from(4 - step) * 0.8);
                a
            })
            .collect();
        trails.update(&shifted);
    }
    trails.update(&aircraft);

    let scene = raster::Scene {
        width_px: 900,
        height_px: 900,
        aircraft: &aircraft,
        trails: &trails,
        zoom_radius_nm: 40.0,
        sweep_angle_deg: 50.0,
        palette: &palette,
        font: &font,
    };
    let image = raster::render(&scene);
    image
        .save(out_path)
        .map_err(std::io::Error::other)?;
    println!("wrote {out_path}");
    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--preview") {
        let out_path = args.next().unwrap_or_else(|| "preview.png".to_string());
        return run_preview(&out_path);
    }

    let location = match data::location::load() {
        Ok(loc) => loc,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    let rx = data::fetch::spawn_poller(location.latitude, location.longitude);
    let mut aircraft: Vec<Aircraft> = Vec::new();
    let mut trails = TrailStore::default();
    let mut last_error: Option<String> = None;
    let mut last_update: Option<Instant> = None;
    let mut zoom_radius_nm: f64 = DEFAULT_ZOOM_NM;
    let sweep_start = Instant::now();
    let mut theme = ThemeWatcher::new();
    let mut last_theme_check = Instant::now();
    let font = match font::load_monospace() {
        Ok(f) => f,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    let mut terminal = tui::init()?;
    let picker = match Picker::from_query_stdio() {
        Ok(p) => p,
        Err(err) => {
            tui::restore()?;
            eprintln!("couldn't detect terminal image support: {err}");
            std::process::exit(1);
        }
    };

    loop {
        if event::poll(Duration::from_millis(80))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Up => {
                        zoom_radius_nm =
                            (zoom_radius_nm - ZOOM_STEP_NM).max(scope::MIN_ZOOM_NM);
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::Down => {
                        zoom_radius_nm =
                            (zoom_radius_nm + ZOOM_STEP_NM).min(scope::MAX_ZOOM_NM);
                    }
                    KeyCode::Char('0') => zoom_radius_nm = DEFAULT_ZOOM_NM,
                    _ => {}
                }
            }
        }

        if last_theme_check.elapsed() >= THEME_POLL_INTERVAL {
            theme.poll();
            last_theme_check = Instant::now();
        }

        while let Ok(result) = rx.try_recv() {
            match result {
                Ok(list) => {
                    trails.update(&list);
                    aircraft = list;
                    last_error = None;
                    last_update = Some(Instant::now());
                }
                Err(err) => last_error = Some(err),
            }
        }

        let title = format!(
            " flyover — {} — {} contact(s) — {:.0}nm range ",
            location.name,
            aircraft.len(),
            zoom_radius_nm
        );
        let status = match (last_update, &last_error) {
            (_, Some(_)) => "err".to_string(),
            (Some(t), None) => format!("↻{}s", t.elapsed().as_secs()),
            (None, None) => "…".to_string(),
        };

        terminal.draw(|frame| {
            scope::render(
                frame,
                frame.area(),
                title,
                status,
                &aircraft,
                &trails,
                zoom_radius_nm,
                sweep_start,
                &theme.palette,
                &picker,
                &font,
            );
        })?;
    }

    tui::restore()?;
    Ok(())
}
