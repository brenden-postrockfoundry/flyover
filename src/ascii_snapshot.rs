use crate::data::aircraft::{Aircraft, Altitude};
use crate::geometry::bearing_to_xy;

const RING_COUNT: u32 = 4;
const LABEL_ROWS: usize = 3;

/// Renders one static frame of the radar scope as plain ASCII text (a
/// character grid, not braille or Unicode block art) — meant for Omarchy's
/// `ttfx`-driven screensaver, which reads `~/.config/omarchy/branding/
/// screensaver.txt` fresh each animation cycle and applies its own visual
/// effect (matrix rain, fireworks, etc.) on top. No color is embedded here
/// since ttfx recolors per-effect regardless of input styling, and no
/// trails/sweep-animation either — this is a one-shot snapshot (each
/// invocation is a fresh process, no persistent state across runs), so the
/// "motion" comes entirely from ttfx's own effect, not from us.
pub fn render(cols: usize, rows: usize, aircraft: &[Aircraft], zoom_radius_nm: f64) -> String {
    let mut grid = vec![vec![' '; cols]; rows];

    // Terminal cells are roughly twice as tall as they are wide, so without
    // this correction the range rings render as ellipses, not circles.
    let aspect = if rows > 0 {
        cols as f64 / (rows as f64 * 2.0)
    } else {
        1.0
    };
    let x_reach = zoom_radius_nm * aspect.max(0.5);

    let center_col = cols as f64 / 2.0;
    let center_row = rows as f64 / 2.0;

    let to_cell = |nm_x: f64, nm_y: f64| -> Option<(usize, usize)> {
        let col = center_col + (nm_x / x_reach) * center_col;
        let row = center_row - (nm_y / zoom_radius_nm) * center_row;
        if col < 0.0 || row < 0.0 {
            return None;
        }
        let (col, row) = (col.round() as usize, row.round() as usize);
        if col < cols && row < rows {
            Some((col, row))
        } else {
            None
        }
    };

    // Rings only fill blank cells (harmless either way, since adjacent
    // sampled ring points all write the same char).
    fn set_if_blank(grid: &mut [Vec<char>], col: usize, row: usize, ch: char) {
        if row < grid.len() && col < grid[row].len() && grid[row][col] == ' ' {
            grid[row][col] = ch;
        }
    }

    // Blips must win over a ring dot that happens to land on the same cell
    // (e.g. a contact sitting exactly at a ring's radius) — an aircraft
    // there should show as the aircraft, not the ring.
    fn set_force(grid: &mut [Vec<char>], col: usize, row: usize, ch: char) {
        if row < grid.len() && col < grid[row].len() {
            grid[row][col] = ch;
        }
    }

    // Range rings: sample the circumference densely enough that adjacent
    // points always land on neighboring cells (no gaps) at any zoom level.
    for i in 1..=RING_COUNT {
        let r = zoom_radius_nm * f64::from(i) / f64::from(RING_COUNT);
        let steps = 720;
        for s in 0..steps {
            let deg = 360.0 * f64::from(s) / f64::from(steps);
            let (nx, ny) = bearing_to_xy(r, deg);
            if let Some((c, row)) = to_cell(nx, ny) {
                set_if_blank(&mut grid, c, row, '.');
            }
        }
    }

    struct LabelBox {
        col: i64,
        row: i64,
        w: i64,
        h: i64,
    }
    fn boxes_overlap(a: &LabelBox, b: &LabelBox) -> bool {
        a.col < b.col + b.w && b.col < a.col + a.w && a.row < b.row + b.h && b.row < a.row + a.h
    }
    fn write_str(grid: &mut [Vec<char>], col: i64, row: i64, text: &str) {
        if row < 0 || row as usize >= grid.len() {
            return;
        }
        let row_vec = &mut grid[row as usize];
        for (i, ch) in text.chars().enumerate() {
            let c = col + i as i64;
            if c >= 0 && (c as usize) < row_vec.len() {
                row_vec[c as usize] = ch;
            }
        }
    }

    let mut contacts: Vec<&Aircraft> = aircraft
        .iter()
        .filter(|ac| matches!(ac.dst, Some(d) if d <= zoom_radius_nm))
        .collect();
    contacts.sort_by(|a, b| a.hex.cmp(&b.hex));

    let mut placed: Vec<LabelBox> = Vec::with_capacity(contacts.len());
    let gap: i64 = 1;

    for ac in &contacts {
        let (Some(dst), Some(dir)) = (ac.dst, ac.dir) else {
            continue;
        };
        let (nx, ny) = bearing_to_xy(dst, dir);
        let Some((col, row)) = to_cell(nx, ny) else {
            continue;
        };
        let (col, row) = (col as i64, row as i64);
        set_force(
            &mut grid,
            col as usize,
            row as usize,
            if ac.is_emergency_squawk() { '!' } else { '@' },
        );

        let climb = match ac.climb_rate() {
            Some(r) if r > 100.0 => "^",
            Some(r) if r < -100.0 => "v",
            _ => "",
        };
        let alt = match ac.alt_baro {
            Some(Altitude::Feet(ft)) => format!("FL{:03}{climb}", ft / 100),
            Some(Altitude::Ground) => format!("GND{climb}"),
            None => format!("?{climb}"),
        };
        let lines = [
            ac.callsign().to_string(),
            alt,
            format!("{:.0}kt", ac.gs.unwrap_or(0.0)),
        ];
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as i64;
        let height = LABEL_ROWS as i64;

        let candidates = [
            (col + gap, row - gap - height + 1),
            (col + gap, row + gap),
            (col - gap - width, row - gap - height + 1),
            (col - gap - width, row + gap),
        ];

        let mut chosen = candidates[0];
        for candidate in candidates {
            let candidate_box = LabelBox {
                col: candidate.0,
                row: candidate.1,
                w: width,
                h: height,
            };
            if !placed.iter().any(|p| boxes_overlap(p, &candidate_box)) {
                chosen = candidate;
                break;
            }
        }
        // Keep on-grid even when the contact is near the edge.
        chosen.0 = chosen.0.max(0).min((cols as i64 - width).max(0));
        chosen.1 = chosen.1.max(0).min((rows as i64 - height).max(0));

        placed.push(LabelBox {
            col: chosen.0,
            row: chosen.1,
            w: width,
            h: height,
        });
        for (i, line) in lines.iter().enumerate() {
            write_str(&mut grid, chosen.0, chosen.1 + i as i64, line);
        }
    }

    grid.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ac(
        hex: &str,
        flight: &str,
        alt_ft: i64,
        gs: f64,
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
            baro_rate: Some(0.0),
            geom_rate: None,
            squawk: Some(squawk.to_string()),
            lat: None,
            lon: None,
            dst: Some(dst),
            dir: Some(dir),
        }
    }

    #[test]
    fn renders_rings_blips_and_labels() {
        let aircraft = vec![
            ac("a1", "UAL1234", 35000, 420.0, "1200", 20.0, 45.0),
            ac("a2", "N247JH", 5000, 150.0, "7700", 15.0, 130.0),
        ];
        let out = render(70, 35, &aircraft, 40.0);

        // Print it so `cargo test -- --nocapture` doubles as a visual check.
        println!("{out}");

        assert!(out.contains('.'), "expected range ring dots");
        assert!(out.contains('@'), "expected a normal-squawk blip");
        assert!(out.contains('!'), "expected an emergency-squawk blip");
        assert!(out.contains("UAL1234"), "expected the callsign label");
        assert!(out.contains("FL350"), "expected the altitude label");
        assert!(out.contains("420kt"), "expected the speed label");
        let rows: Vec<&str> = out.split('\n').collect();
        assert_eq!(rows.len(), 35, "expected exactly 35 rows");
        assert!(
            rows.iter().all(|r| r.chars().count() == 70),
            "expected every row to be exactly 70 cols"
        );
    }
}
