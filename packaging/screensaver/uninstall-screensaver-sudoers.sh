#!/bin/bash
# Reverts install-screensaver-sudoers.sh.
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
  exec sudo "$0" "$@"
fi

sudoers_file=/etc/sudoers.d/flyover-screensaver
if [[ ! -f $sudoers_file ]]; then
  echo "No $sudoers_file -- nothing to remove." >&2
  exit 1
fi

rm -f "$sudoers_file"
echo "Removed $sudoers_file"
