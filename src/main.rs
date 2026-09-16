mod data;
mod geometry;
mod scope;
mod theme;
mod trail;
mod tui;

use crossterm::event::{self, Event, KeyCode};
use data::aircraft::Aircraft;
use std::time::{Duration, Instant};
use theme::ThemeWatcher;
use trail::TrailStore;

const ZOOM_STEP_NM: f64 = 5.0;
const DEFAULT_ZOOM_NM: f64 = 40.0;
const THEME_POLL_INTERVAL: Duration = Duration::from_secs(1);

fn main() -> std::io::Result<()> {
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

    let mut terminal = tui::init()?;

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
            );
        })?;
    }

    tui::restore()?;
    Ok(())
}
