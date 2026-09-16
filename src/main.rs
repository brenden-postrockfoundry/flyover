mod data;
mod tui;

use crossterm::event::{self, Event, KeyCode};
use data::aircraft::{Aircraft, Altitude};
use ratatui::layout::Constraint;
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, Row, Table};
use ratatui::Frame;
use std::time::{Duration, Instant};

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

    let mut terminal = tui::init()?;

    loop {
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                    break;
                }
            }
        }

        while let Ok(result) = rx.try_recv() {
            match result {
                Ok(mut list) => {
                    list.sort_by(|a, b| {
                        a.dst
                            .unwrap_or(f64::MAX)
                            .partial_cmp(&b.dst.unwrap_or(f64::MAX))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    aircraft = list;
                    last_error = None;
                    last_update = Some(Instant::now());
                }
                Err(err) => last_error = Some(err),
            }
        }

        terminal.draw(|frame| render(frame, &location.name, &aircraft, last_update, &last_error))?;
    }

    tui::restore()?;
    Ok(())
}

fn render(
    frame: &mut Frame,
    location_name: &str,
    aircraft: &[Aircraft],
    last_update: Option<Instant>,
    last_error: &Option<String>,
) {
    let title = match (last_update, last_error) {
        (_, Some(err)) => format!(" Flyover — {location_name} — error: {err} "),
        (Some(t), None) => format!(
            " Flyover — {location_name} — updated {}s ago — {} contact(s) — q to quit ",
            t.elapsed().as_secs(),
            aircraft.len()
        ),
        (None, None) => format!(" Flyover — {location_name} — waiting for first update... "),
    };

    let header = Row::new(["CALLSIGN", "ALT", "SPEED", "DIST", "DIR", ""]).bold();
    let rows = aircraft.iter().map(|ac| {
        let alt = match ac.alt_baro {
            Some(Altitude::Feet(ft)) => format!("{ft}ft"),
            Some(Altitude::Ground) => "ground".to_string(),
            None => "?".to_string(),
        };
        let climb = match ac.climb_rate() {
            Some(r) if r > 100.0 => "▲",
            Some(r) if r < -100.0 => "▼",
            _ => "",
        };
        let flag = if ac.is_emergency_squawk() {
            "EMERGENCY"
        } else {
            climb
        };
        Row::new([
            ac.callsign().to_string(),
            alt,
            format!("{:.0}kt", ac.gs.unwrap_or(0.0)),
            format!("{:.1}nm", ac.dst.unwrap_or(0.0)),
            format!("{:.0}°", ac.dir.unwrap_or(0.0)),
            flag.to_string(),
        ])
    });

    let widths = [
        Constraint::Length(10),
        Constraint::Length(9),
        Constraint::Length(8),
        Constraint::Length(9),
        Constraint::Length(6),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(table, frame.area());
}
