use crate::scope::RenderMode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persists the user's chosen render mode across runs, so `--screensaver`
/// (which never reads the 'v' toggle key itself) shows whatever mode the
/// interactive TUI was last set to instead of always defaulting to Sixel.
#[derive(Serialize, Deserialize)]
struct Settings {
    render_mode: RenderMode,
}

fn path() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home).join(".config/flyover/settings.toml")
}

pub fn load_render_mode(default: RenderMode) -> RenderMode {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|s| toml::from_str::<Settings>(&s).ok())
        .map_or(default, |s| s.render_mode)
}

pub fn save_render_mode(render_mode: RenderMode) {
    let path = path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(contents) = toml::to_string(&Settings { render_mode }) {
        let _ = std::fs::write(path, contents);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // `path()` reads $HOME process-wide, and Rust runs tests in parallel
    // threads by default -- this guards against two tests racing to set it.
    static HOME_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn round_trips_through_a_fresh_home() {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp =
            std::env::temp_dir().join(format!("flyover-settings-test-{}", std::process::id()));
        let original_home = std::env::var("HOME").ok();
        // SAFETY: serialized by HOME_LOCK against the other test in this
        // module, and nothing else in this crate's test suite reads $HOME.
        unsafe {
            std::env::set_var("HOME", &tmp);
        }

        assert_eq!(
            load_render_mode(RenderMode::Sixel),
            RenderMode::Sixel,
            "no settings file yet -- should fall back to the given default"
        );

        save_render_mode(RenderMode::Braille);
        assert_eq!(load_render_mode(RenderMode::Sixel), RenderMode::Braille);

        let _ = std::fs::remove_dir_all(&tmp);
        // SAFETY: see above.
        unsafe {
            match original_home {
                Some(h) => std::env::set_var("HOME", h),
                None => std::env::remove_var("HOME"),
            }
        }
    }
}
