mod data;
mod scope;
mod tui;

use crossterm::event::{self, Event, KeyCode};
use data::aircraft::Aircraft;
use std::time::{Duration, Instant};

const ZOOM_STEP_NM: f64 = 5.0;

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
    let mut last_error: Option<String> = None;
    let mut last_update: Option<Instant> = None;
    let mut zoom_radius_nm: f64 = 40.0;
    let sweep_start = Instant::now();

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
                    _ => {}
                }
            }
        }

        while let Ok(result) = rx.try_recv() {
            match result {
                Ok(list) => {
                    aircraft = list;
                    last_error = None;
                    last_update = Some(Instant::now());
                }
                Err(err) => last_error = Some(err),
            }
        }

        let title = match (last_update, &last_error) {
            (_, Some(err)) => format!(" flyover — {} — error: {err} ", location.name),
            (Some(t), None) => format!(
                " flyover — {} — updated {}s ago — {} contact(s) — {:.0}nm range — +/- zoom, q to quit ",
                location.name,
                t.elapsed().as_secs(),
                aircraft.len(),
                zoom_radius_nm
            ),
            (None, None) => format!(" flyover — {} — waiting for first update... ", location.name),
        };

        terminal.draw(|frame| {
            scope::render(frame, frame.area(), title, &aircraft, zoom_radius_nm, sweep_start);
        })?;
    }

    tui::restore()?;
    Ok(())
}
