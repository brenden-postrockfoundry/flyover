# Screensaver wiring

Wires `flyover --ascii-snapshot` into Omarchy's real screensaver (`ttfx`
animating `~/.config/omarchy/branding/screensaver.txt`, re-read fresh each
effect cycle — see the main README's "Screensaver" section for the
mechanism itself).

`flyover-screensaver-refresh` only regenerates the snapshot while the
screensaver is actually active, checked via the real Omarchy idle-service
IPC (`omarchy-shell idle status` → `.screensaverStarted`) — not our own
idle-detection guesswork. During normal use it's a single cheap local IPC
call and nothing else; no adsb.lol fetch happens unless you're genuinely
idle.

## Install

```
mkdir -p ~/.local/bin ~/.config/systemd/user
cp flyover-screensaver-refresh ~/.local/bin/
chmod +x ~/.local/bin/flyover-screensaver-refresh
cp flyover-screensaver-refresh.service flyover-screensaver-refresh.timer ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now flyover-screensaver-refresh.timer
```

## Verify

```
systemctl --user status flyover-screensaver-refresh.timer
journalctl --user -u flyover-screensaver-refresh.service -n 20
```

The service only actually runs `flyover` (and writes the file) once a
screensaver cycle is genuinely active — check `omarchy-shell idle status`
yourself to confirm `screensaverStarted` before expecting the file to
change.

## Static display (optional)

By default Omarchy's screensaver puts every refresh through a random `ttfx`
effect (matrix rain, fireworks, etc.), restarting with a new one every time
the previous one finishes — a few seconds, independent of our refresh
cadence. `patch-omarchy-screensaver.sh` swaps that for a plain, centered,
static render whenever flyover is actively feeding the file (tracked via
`~/.cache/flyover/screensaver-active`, touched by the refresh script above
on every successful write, and considered fresh for 45s). Any other
branding content, or a stale/disabled flyover install, still gets the
normal animated behavior.

This edits `omarchy-screensaver`, a file owned by the `omarchy` package
(resolved via `command -v omarchy-screensaver`, a symlink into `/usr/bin`)
— so it needs `sudo`, and a future `omarchy update` can silently revert it.
The script is idempotent (backs up the original once, detects its own
marker on repeat runs), so it's safe to re-run any time that happens.

```
sudo bash patch-omarchy-screensaver.sh
```

To revert to Omarchy's stock animated-only behavior:

```
sudo bash restore-omarchy-screensaver.sh
```
