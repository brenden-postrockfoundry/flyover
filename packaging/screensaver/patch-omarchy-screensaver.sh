#!/bin/bash
# Patches Omarchy's system screensaver script so that while flyover is
# actively feeding it (screensaver.txt refreshed in the last 45s, tracked via
# ~/.cache/flyover/screensaver-active), it displays the content statically
# -- centered, no ttfx effect -- instead of running it through ttfx's
# --random-effect animation. Any other branding content (or a stale/disabled
# flyover install) still gets the normal animated behavior.
#
# This edits a file owned by the omarchy package (resolved via
# `command -v omarchy-screensaver`, which is a symlink into /usr/bin), so a
# future `omarchy update` can silently revert it. Re-run this script any time
# that happens -- it's idempotent and safe to run repeatedly.
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
  exec sudo "$0" "$@"
fi

target=$(readlink -f "$(command -v omarchy-screensaver)")
marker="# flyover:static-patch v1"

if grep -qF "$marker" "$target" 2>/dev/null; then
  echo "Already patched: $target"
  exit 0
fi

backup="${target}.pre-flyover.bak"
cp "$target" "$backup"
echo "Backed up original to $backup"

cat >"$target" <<'SCRIPT'
#!/bin/bash

# omarchy:summary=Run the Omarchy screensaver using random effects from TTE.
# flyover:static-patch v1 -- see packaging/screensaver/patch-omarchy-screensaver.sh
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
# compositor has told the window how big it is. ttfx measures the terminal once,
# at startup, so starting it before the resize lands sizes an 80x24 canvas and
# paints it into the corner of a fullscreen window.
wait_for_terminal_resize() {
  local deadline=$((SECONDS + 2))
  while ((SECONDS < deadline)) && [[ $(stty size 2>/dev/null) == "24 80" ]]; do
    sleep 0.02
  done
}

wait_for_terminal_resize

content="$HOME/.config/omarchy/branding/screensaver.txt"
flyover_marker="$HOME/.cache/flyover/screensaver-active"

flyover_marker_fresh() {
  [[ -f $flyover_marker ]] || return 1
  local now age
  now=$(date +%s)
  age=$((now - $(stat -c %Y "$flyover_marker" 2>/dev/null || echo 0)))
  ((age < 45))
}

render_static() {
  clear
  local term_rows term_cols content_rows content_cols pad_top pad_left
  read -r term_rows term_cols < <(stty size 2>/dev/null || echo "24 80")
  content_rows=$(wc -l <"$content")
  content_cols=$(awk '{ print length }' "$content" | sort -rn | head -1)
  pad_top=$(((term_rows - content_rows) / 2))
  pad_left=$(((term_cols - content_cols) / 2))
  ((pad_top < 0)) && pad_top=0
  ((pad_left < 0)) && pad_left=0
  for ((i = 0; i < pad_top; i++)); do echo; done
  while IFS= read -r line; do
    printf '%*s%s\n' "$pad_left" "" "$line"
  done <"$content"
}

while true; do
  if flyover_marker_fresh; then
    render_static
    last_mtime=$(stat -c %Y "$content" 2>/dev/null || echo 0)
    while flyover_marker_fresh; do
      if read -n1 -t 1 || ! screensaver_in_focus; then
        exit_screensaver
      fi
      cur_mtime=$(stat -c %Y "$content" 2>/dev/null || echo 0)
      if [[ $cur_mtime != "$last_mtime" ]]; then
        render_static
        last_mtime=$cur_mtime
      fi
    done
    continue
  fi

  ttfx -i "$content" \
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
