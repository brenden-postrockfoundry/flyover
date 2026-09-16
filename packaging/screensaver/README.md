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
