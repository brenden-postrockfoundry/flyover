#!/bin/bash
# Lets the patched `omarchy branding screensaver text|reset` menu actions
# run patch-omarchy-screensaver.sh / restore-omarchy-screensaver.sh as root
# without a password prompt -- necessary because they can be triggered from
# a keybinding or GUI menu with no terminal attached for `sudo` to prompt
# on, and this system has no polkit GUI agent to fall back to.
#
# Installs a narrowly-scoped sudoers drop-in: only these two exact scripts,
# only for the user who ran this installer -- not general sudo access.
# Validates with `visudo -c` before installing anything, since a broken
# sudoers file can lock out sudo entirely.
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
  exec sudo "$0" "$@"
fi

target_user="${SUDO_USER:-$(logname)}"
if [[ -z $target_user || $target_user == root ]]; then
  echo "Couldn't determine the non-root user to grant this to -- run via sudo as your normal user, not as root directly." >&2
  exit 1
fi

target_home=$(getent passwd "$target_user" | cut -d: -f6)
if [[ -z $target_home ]]; then
  echo "Couldn't determine $target_user's home directory." >&2
  exit 1
fi

# Must match exactly what the patched omarchy-branding-screensaver script
# resolves at invocation time (as $target_user, not root) -- sudoers rules
# match the full command line verbatim.
if [[ -d /usr/share/flyover/screensaver ]]; then
  screensaver_dir=/usr/share/flyover/screensaver
else
  screensaver_dir="$target_home/flyover/packaging/screensaver"
fi

patch_script="$screensaver_dir/patch-omarchy-screensaver.sh"
restore_script="$screensaver_dir/restore-omarchy-screensaver.sh"

for script in "$patch_script" "$restore_script"; do
  if [[ ! -f $script ]]; then
    echo "Expected to find $script but it doesn't exist -- nothing installed." >&2
    exit 1
  fi
done

bash_bin=$(command -v bash)
sudoers_file=/etc/sudoers.d/flyover-screensaver
tmp_file=$(mktemp)
trap 'rm -f "$tmp_file"' EXIT

cat >"$tmp_file" <<EOF
# Installed by flyover's install-screensaver-sudoers.sh -- lets $target_user
# enable/disable the flyover screensaver from a menu or keybinding with no
# terminal attached (no other sudo access is granted).
$target_user ALL=(root) NOPASSWD: $bash_bin $patch_script
$target_user ALL=(root) NOPASSWD: $bash_bin $restore_script
EOF

if ! visudo -c -f "$tmp_file"; then
  echo "Generated sudoers rule failed validation -- not installing anything." >&2
  exit 1
fi

install -m 0440 -o root -g root "$tmp_file" "$sudoers_file"
echo "Installed $sudoers_file for $target_user:"
cat "$sudoers_file"
