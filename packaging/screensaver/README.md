# Screensaver wiring

Patches Omarchy's real screensaver script to run flyover's own live scope
(sweep, fading trails, theme sync — the same rendering as the interactive
TUI) directly, in place of `ttfx` animating static branding text.

By default, Omarchy's screensaver puts whatever's in
`~/.config/omarchy/branding/screensaver.txt` through a random `ttfx`
effect (matrix rain, fireworks, etc.), restarting with a new one every time
the previous one finishes. flyover doesn't fit that model — it's a live
process, not a static frame — so this replaces the effect step entirely:
the screensaver script launches `flyover --screensaver` and lets it run,
falling back to the stock `ttfx` behavior only if flyover isn't
installed/built.

flyover runs in the foreground and owns its own exit logic in this mode —
any keypress or losing focus (checked via `hyprctl`) ends it, and the
script cleans up once it returns. (An earlier version had the script
background flyover and watch the same tty itself, the way it already
watches `ttfx` — but that meant two processes racing to read the same
stdin, which sporadically broke flyover's terminal-capability query at
startup. Only the `ttfx` fallback below still uses that pattern, since
`ttfx` itself never reads input.) Because the process only exists while
the screensaver window is actually open, it naturally never fetches from
adsb.lol outside of genuine idle time — no separate idle-detection timer
needed.

## Install

This edits `omarchy-screensaver`, a file owned by the `omarchy` package
(resolved via `command -v omarchy-screensaver`, a symlink into `/usr/bin`)
— so it needs `sudo`, and a future `omarchy update` can silently revert it.
The script is idempotent (backs up the original exactly once, safe to
re-run any time that happens, including to pick up a newer version of this
same patch):

```
sudo bash patch-omarchy-screensaver.sh
```

## Verify

Force-trigger the screensaver and confirm flyover (not `ttfx`) is running:

```
omarchy-launch-screensaver force
pgrep -af 'flyover --screensaver'
```

## Revert

```
sudo bash restore-omarchy-screensaver.sh
```
