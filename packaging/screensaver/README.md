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

## Menu integration (optional)

Omarchy has no pluggable "choose a screensaver" list — it's this one
mechanism, with a `omarchy branding screensaver <image|text|reset>`
command that lets you customize the branding content (pick an image,
edit the text, or reset to the default logo). `patch-omarchy-branding-screensaver.sh`
repurposes that same command as an enable/disable switch for the flyover
screensaver instead:

- `text` — enables it (runs `patch-omarchy-screensaver.sh` above)
- `reset` — disables it and resets the branding content to the default logo
- `image` — untouched

```
sudo bash patch-omarchy-branding-screensaver.sh
```

Same idempotent/backup/revert pattern as above (`restore-omarchy-branding-screensaver.sh`
reverts it). Since this is often triggered from a keybinding or menu with
no terminal attached for a password prompt, also install a narrowly-scoped
sudoers rule (only these two exact scripts, only for your user — not
general sudo access) so it works from anywhere:

```
sudo bash install-screensaver-sudoers.sh
```

Validated with `visudo -c` before it touches anything, since a broken
sudoers file can lock out `sudo` entirely. Revert with
`sudo bash uninstall-screensaver-sudoers.sh`.
