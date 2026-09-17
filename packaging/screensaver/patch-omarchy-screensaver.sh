#!/bin/bash
# Patches Omarchy's system screensaver script to run flyover's own live
# scope (sweep, fading trails, theme sync) directly, instead of running
# static branding text through ttfx's --random-effect animation. Falls back
# to the stock ttfx behavior if flyover isn't installed/built.
#
# This edits `omarchy-screensaver`, a file owned by the omarchy package
# (resolved via `command -v omarchy-screensaver`, a symlink into /usr/bin)
# -- so it needs sudo, and a future `omarchy update` can silently revert it.
# Re-run this script any time that happens; it's idempotent (backs up the
# original exactly once, safe to run repeatedly after that, including to
# upgrade an older version of this same patch).
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
  exec sudo "$0" "$@"
fi

target=$(readlink -f "$(command -v omarchy-screensaver)")
backup="${target}.pre-flyover.bak"

if [[ -f $backup ]]; then
  echo "Original already backed up at $backup"
else
  cp "$target" "$backup"
  echo "Backed up original to $backup"
fi

cat >"$target" <<'SCRIPT'
#!/bin/bash

# omarchy:summary=Run the Omarchy screensaver using random effects from TTE.
# flyover:live-patch v3 -- see packaging/screensaver/patch-omarchy-screensaver.sh
# in https://github.com/linuxbren/flyover

screensaver_in_focus() {
  hyprctl activewindow -j | jq -e '.class == "org.omarchy.screensaver"' >/dev/null 2>&1
}

exit_screensaver() {
  hyprctl eval 'hl.config({ cursor = { invisible = false } })' &>/dev/null || hyprctl keyword cursor:invisible false &>/dev/null || true
  pkill -x ttfx 2>/dev/null
  pkill -f '[o]rg.omarchy.screensaver' 2>/dev/null
  exit 0
}

# Exit the screensaver on signals and input from keyboard and mouse
trap exit_screensaver SIGINT SIGTERM SIGHUP SIGQUIT

printf '\033]11;rgb:00/00/00\007'  # Set background color to black

hyprctl eval 'hl.config({ cursor = { invisible = true } })' &>/dev/null || hyprctl keyword cursor:invisible true &>/dev/null

tty=$(tty 2>/dev/null)

# Terminals allocate the pty at the default 80x24 and only resize it once the
# compositor has told the window how big it is. Both flyover and ttfx measure
# the terminal once, at startup, so starting either before the resize lands
# sizes an 80x24 canvas and paints it into the corner of a fullscreen window.
wait_for_terminal_resize() {
  local deadline=$((SECONDS + 2))
  while ((SECONDS < deadline)) && [[ $(stty size 2>/dev/null) == "24 80" ]]; do
    sleep 0.02
  done
}

wait_for_terminal_resize

flyover_bin=$(command -v flyover || true)
[[ -z $flyover_bin ]] && flyover_bin="$HOME/flyover/target/release/flyover"

if [[ -x $flyover_bin ]]; then
  # Runs in the foreground and owns its own exit-on-keypress/focus-loss
  # logic -- unlike ttfx below, nothing else here reads this tty, so there's
  # no risk of stealing bytes from its terminal-capability query at startup.
  "$flyover_bin" --screensaver
  exit_screensaver
fi

# flyover isn't installed/built -- fall back to Omarchy's stock effect.
while true; do
  ttfx -i ~/.config/omarchy/branding/screensaver.txt \
    --frame-rate 120 --canvas-width 0 --canvas-height 0 --reuse-canvas --anchor-canvas c --anchor-text c\
    --random-effect --no-eol --no-restore-cursor &

  while pgrep -t "${tty#/dev/}" -x ttfx >/dev/null; do
    if read -n1 -t 1 || ! screensaver_in_focus; then
      exit_screensaver
    fi
  done
done
SCRIPT

chmod 755 "$target"
echo "Patched $target"
