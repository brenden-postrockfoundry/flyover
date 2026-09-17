#!/bin/bash
# Reverts patch-omarchy-branding-screensaver.sh, restoring Omarchy's
# original `omarchy branding screensaver <image|text|reset>` behavior
# (text opens an editor, reset just resets the branding content).
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
  exec sudo "$0" "$@"
fi

target=$(readlink -f "$(command -v omarchy-branding-screensaver)")
backup="${target}.pre-flyover.bak"

if [[ ! -f $backup ]]; then
  echo "No backup found at $backup -- nothing to restore." >&2
  exit 1
fi

cp "$backup" "$target"
chmod 755 "$target"
echo "Restored $target from $backup"
