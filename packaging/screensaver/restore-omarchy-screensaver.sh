#!/bin/bash
# Reverts patch-omarchy-screensaver.sh, restoring Omarchy's original
# screensaver script (ttfx --random-effect for everything, no static mode).
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
  exec sudo "$0" "$@"
fi

target=$(readlink -f "$(command -v omarchy-screensaver)")
backup="${target}.pre-flyover.bak"

if [[ ! -f $backup ]]; then
  echo "No backup found at $backup -- nothing to restore." >&2
  exit 1
fi

cp "$backup" "$target"
chmod 755 "$target"
echo "Restored $target from $backup"
