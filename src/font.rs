use std::process::Command;

/// Resolves the system monospace font the same way Omarchy's terminals do
/// (`fc-match monospace`), so the rasterized scope text tracks `omarchy font
/// set` automatically instead of hardcoding a family that could drift out of
/// sync with the bar.
pub fn load_monospace() -> Result<fontdue::Font, String> {
    let path = resolve_monospace_font_path()
        .ok_or("fc-match couldn't resolve a monospace font (is fontconfig installed?)")?;
    let bytes = std::fs::read(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
        .map_err(|e| format!("parsing font {}: {e}", path.display()))
}

fn resolve_monospace_font_path() -> Option<std::path::PathBuf> {
    let output = Command::new("fc-match")
        .args(["-f", "%{file}", "monospace"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?;
    let path = path.trim();
    if path.is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(path))
    }
}
