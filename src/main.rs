mod braille_scope;
mod data;
mod font;
mod geometry;
mod raster;
mod scope;
mod sixel_scope;
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

    // A synthetic-fixture builder for this dev-only preview scene doesn't
    // need the ergonomics a real API would; one extra arg over clippy's
    // default threshold isn't worth a builder struct here.
    #[allow(clippy::too_many_arguments)]
    fn ac(
        hex: &str,
        flight: &str,
        alt_ft: i64,
        gs: f64,
        rate: f64,
        squawk: &str,
        dst: f64,
        dir: f64,
    ) -> Aircraft {
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
        ac("a6", "EDGE001", 41000, 480.0, 0.0, "1200", 39.5, 2.0),
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
        label_font_px: 18.0 * 0.85,
    };
    let image = raster::render(&scene);
    image.save(out_path).map_err(std::io::Error::other)?;
    println!("wrote {out_path}");
    Ok(())
}

/// Dev-only: `flyover --bench` times the raster + Sixel-encode pipeline
/// against an in-memory TestBackend (no real TTY needed) to diagnose actual
/// per-frame cost, since this environment can't be used to eyeball a live
/// frame rate.
fn run_bench() -> Result<(), Box<dyn std::error::Error>> {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui_image::FontSize;
    use ratatui_image::picker::Picker;

    let font = font::load_monospace()?;
    let palette = ThemeWatcher::new().palette;
    let trails = TrailStore::default();
    let aircraft: Vec<Aircraft> = Vec::new();

    // Representative of a bigger tiled window: ~160 cols x 50 rows at a typical
    // JetBrainsMono cell size.
    let cols = 160u16;
    let rows = 50u16;
    #[allow(deprecated)]
    let mut picker = Picker::from_fontsize(FontSize {
        width: 9,
        height: 18,
    });
    picker.set_protocol_type(ratatui_image::picker::ProtocolType::Sixel);

    let backend = TestBackend::new(cols, rows);
    let mut terminal = Terminal::new(backend)?;
    let sweep_start = Instant::now();

    const N: u32 = 10;
    let mut raster_total = Duration::ZERO;
    let mut draw_total = Duration::ZERO;

    for _ in 0..N {
        let t0 = Instant::now();
        let font_size = picker.font_size();
        let width_px = u32::from(cols) * u32::from(font_size.width);
        let height_px = u32::from(rows) * u32::from(font_size.height);
        let scene = raster::Scene {
            width_px,
            height_px,
            aircraft: &aircraft,
            trails: &trails,
            zoom_radius_nm: 40.0,
            sweep_angle_deg: 10.0,
            palette: &palette,
            font: &font,
            label_font_px: f32::from(font_size.height) * 0.85,
        };
        let image = raster::render(&scene);
        raster_total += t0.elapsed();

        let t1 = Instant::now();
        terminal.draw(|frame| {
            scope::render(
                frame,
                frame.area(),
                "bench".to_string(),
                "".to_string(),
                &aircraft,
                &trails,
                40.0,
                sweep_start,
                &palette,
                &picker,
                &font,
                scope::RenderMode::Sixel,
            );
        })?;
        draw_total += t1.elapsed();
        std::hint::black_box(&image);
    }

    println!(
        "sixel: raster::render {:.1}ms/frame, full terminal.draw {:.1}ms/frame",
        raster_total.as_secs_f64() * 1000.0 / f64::from(N),
        draw_total.as_secs_f64() * 1000.0 / f64::from(N)
    );

    let mut braille_total = Duration::ZERO;
    for _ in 0..N {
        let t0 = Instant::now();
        terminal.draw(|frame| {
            scope::render(
                frame,
                frame.area(),
                "bench".to_string(),
                "".to_string(),
                &aircraft,
                &trails,
                40.0,
                sweep_start,
                &palette,
                &picker,
                &font,
                scope::RenderMode::Braille,
            );
        })?;
        braille_total += t0.elapsed();
    }
    println!(
        "braille: full terminal.draw {:.1}ms/frame",
        braille_total.as_secs_f64() * 1000.0 / f64::from(N)
    );
    Ok(())
}

/// Screensaver-mode focus check: true once Hyprland's active window is no
/// longer the screensaver itself. Shelled out rather than piped in from the
/// wrapping script — an earlier version had Omarchy's screensaver script
/// background this process and poll `hyprctl`/read stdin itself, but that
/// made two processes race to read the same tty (this process's own
/// terminal-capability query at startup vs. the script's exit-on-keypress
/// read), which sporadically broke Sixel detection. Now this process owns
/// both checks itself and runs in the foreground with nothing else reading
/// its stdin.
fn screensaver_lost_focus() -> bool {
    let Ok(output) = std::process::Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
    else {
        return false;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return false;
    };
    value.get("class").and_then(|c| c.as_str()) != Some("org.omarchy.screensaver")
}

/// `flyover --screensaver [--ascii]`: runs the same live scope (sweep,
/// fading trails, theme sync) as the interactive TUI, meant to be launched
/// by Omarchy's screensaver script in place of `ttfx`. Exits on any
/// keypress or when the screensaver window loses focus, so the wrapping
/// script only needs to run it in the foreground and clean up once it
/// returns. `--ascii` selects the Braille render mode, which — unlike
/// Sixel — never queries the terminal for image support, so it works
/// without ever touching stdout/stdin at startup.
fn run_screensaver(mode: scope::RenderMode) -> std::io::Result<()> {
    if cfg!(debug_assertions) {
        eprintln!("flyover: running a debug build — the scope will look choppy.");
        eprintln!("         use `cargo run --release` for smooth animation.");
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
    let picker = match mode {
        scope::RenderMode::Sixel => match Picker::from_query_stdio() {
            Ok(p) => p,
            Err(err) => {
                tui::restore()?;
                eprintln!("couldn't detect terminal image support: {err}");
                std::process::exit(1);
            }
        },
        // Braille mode never draws an image, so the real query (and its
        // startup round-trip over stdin/stdout) is unnecessary — these
        // placeholder metrics are never used for anything.
        scope::RenderMode::Braille =>
        {
            #[allow(deprecated)]
            Picker::from_fontsize(ratatui_image::FontSize {
                width: 9,
                height: 18,
            })
        }
    };

    let mut last_focus_check = Instant::now();
    const FOCUS_POLL_INTERVAL: Duration = Duration::from_secs(1);

    loop {
        if event::poll(Duration::from_millis(80))? {
            // Any input at all ends the screensaver — consume it and exit
            // rather than trying to interpret it.
            let _ = event::read()?;
            break;
        }

        if last_focus_check.elapsed() >= FOCUS_POLL_INTERVAL {
            if screensaver_lost_focus() {
                break;
            }
            last_focus_check = Instant::now();
        }

        if last_theme_check.elapsed() >= THEME_POLL_INTERVAL {
            theme.poll();
            last_theme_check = Instant::now();
        }

        while let Ok(result) = rx.try_recv()
            && let Ok(list) = result
        {
            trails.update(&list);
            aircraft = list;
        }

        let title = format!(
            " flyover — {} — {} contact(s) ",
            location.name,
            aircraft.len()
        );

        terminal.draw(|frame| {
            scope::render(
                frame,
                frame.area(),
                title,
                String::new(),
                &aircraft,
                &trails,
                DEFAULT_ZOOM_NM,
                sweep_start,
                &theme.palette,
                &picker,
                &font,
                mode,
            );
        })?;
    }

    tui::restore()?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--preview") => {
            let out_path = args.next().unwrap_or_else(|| "preview.png".to_string());
            return run_preview(&out_path);
        }
        Some("--bench") => {
            return run_bench().map_err(|e| std::io::Error::other(e.to_string()));
        }
        Some("--screensaver") => {
            let mode = if args.next().as_deref() == Some("--ascii") {
                scope::RenderMode::Braille
            } else {
                scope::RenderMode::Sixel
            };
            return run_screensaver(mode);
        }
        _ => {}
    }

    // Sixel encoding is real per-frame work; in a debug build it dominates
    // frame time so badly (~400ms/frame measured vs ~10ms in release) that
    // the sweep animation looks like it's jumping every few seconds instead
    // of rotating smoothly. `cargo run` defaults to debug, so warn plainly
    // rather than let that read as a rendering bug.
    if cfg!(debug_assertions) {
        eprintln!("flyover: running a debug build — the scope will look choppy.");
        eprintln!("         use `cargo run --release` for smooth animation.");
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
    let mut render_mode = scope::RenderMode::Sixel;
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
        if event::poll(Duration::from_millis(80))?
            && let Event::Key(key) = event::read()?
        {
            // Letter keys reflect Caps Lock (crossterm reports the actual
            // character produced, so 'q' becomes 'Q' with Caps Lock on) —
            // lowercase before matching so shortcuts work regardless.
            let code = match key.code {
                KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
                other => other,
            };
            match code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Up => {
                    zoom_radius_nm = (zoom_radius_nm - ZOOM_STEP_NM).max(scope::MIN_ZOOM_NM);
                }
                KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::Down => {
                    zoom_radius_nm = (zoom_radius_nm + ZOOM_STEP_NM).min(scope::MAX_ZOOM_NM);
                }
                KeyCode::Char('0') => zoom_radius_nm = DEFAULT_ZOOM_NM,
                KeyCode::Char('v') => render_mode = render_mode.toggled(),
                _ => {}
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
            (Some(t), None) => {
                let remaining = data::fetch::REFRESH_INTERVAL.saturating_sub(t.elapsed());
                format!("↻{}s", remaining.as_secs())
            }
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
                render_mode,
            );
        })?;
    }

    tui::restore()?;
    Ok(())
}
