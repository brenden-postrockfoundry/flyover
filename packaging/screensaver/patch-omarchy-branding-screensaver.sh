#!/bin/bash
# Patches Omarchy's `omarchy branding screensaver <image|text|reset>` command
# (the same one its branding menu lists as "set from image / edit text /
# reset to default") so:
#   - `text`  enables the flyover screensaver instead of opening the
#             branding file in an editor.
#   - `reset` disables it (restoring Omarchy's stock ttfx-animated
#             screensaver) and resets the branding content back to the
#             default logo, exactly like it always did.
#   - `image` is untouched.
#
# This edits `omarchy-branding-screensaver`, a file owned by the omarchy
# package (resolved via `command -v omarchy-branding-screensaver`, a
# symlink into /usr/bin) -- so it needs sudo, and a future `omarchy update`
# can silently revert it. Re-run this script any time that happens; it's
# idempotent (backs up the original exactly once, safe to run repeatedly).
#
# For `text`/`reset` to actually work when picked from a non-terminal menu
# or keybinding (not just a shell you're sitting at), they need to run
# patch-omarchy-screensaver.sh / restore-omarchy-screensaver.sh without a
# password prompt -- see install-screensaver-sudoers.sh for that.
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
  exec sudo "$0" "$@"
fi

target=$(readlink -f "$(command -v omarchy-branding-screensaver)")
backup="${target}.pre-flyover.bak"

if [[ -f $backup ]]; then
  echo "Original already backed up at $backup"
else
  cp "$target" "$backup"
  echo "Backed up original to $backup"
fi

cat >"$target" <<'SCRIPT'
#!/bin/bash

# omarchy:summary=Enable the flyover screensaver, edit branding, or reset to default
# omarchy:group=branding
# omarchy:name=screensaver
# omarchy:args=<image|text|reset>
# omarchy:examples=omarchy branding screensaver image | omarchy branding screensaver text | omarchy branding screensaver reset
# flyover:branding-patch v1 -- see packaging/screensaver/patch-omarchy-branding-screensaver.sh
# in https://github.com/linuxbren/flyover

set -euo pipefail

# Prefer a packaged install (AUR/cargo + this script), fall back to a git
# clone -- same convention flyover's own binary resolution uses elsewhere.
flyover_screensaver_dir() {
  if [[ -d /usr/share/flyover/screensaver ]]; then
    echo /usr/share/flyover/screensaver
  else
    echo "$HOME/flyover/packaging/screensaver"
  fi
}

case "${1:-}" in
image)
  image=$(omarchy-file-select --title "Pick PNG or SVG for screensaver" --extensions "png svg")
  if omarchy-transcode-ascii "$image" ~/.config/omarchy/branding/screensaver.txt; then
    omarchy-launch-screensaver force >/dev/null 2>&1
  fi
  ;;
text)
  sudo bash "$(flyover_screensaver_dir)/patch-omarchy-screensaver.sh"
  omarchy-notification-send -g ✈ "flyover screensaver enabled"
  omarchy-launch-screensaver force >/dev/null 2>&1
  ;;
reset)
  sudo bash "$(flyover_screensaver_dir)/restore-omarchy-screensaver.sh"
  cp "$OMARCHY_PATH/logo.txt" ~/.config/omarchy/branding/screensaver.txt
  omarchy-notification-send -g 󱄄 "Screensaver reset to default"
  omarchy-launch-screensaver force >/dev/null 2>&1
  ;;
*)
  echo "Usage: omarchy-branding-screensaver <image|text|reset>" >&2
  exit 1
  ;;
esac
SCRIPT

chmod 755 "$target"
echo "Patched $target"
