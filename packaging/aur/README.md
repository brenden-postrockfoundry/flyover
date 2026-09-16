# AUR packaging

`PKGBUILD` + `.SRCINFO` for the `flyover` AUR package, kept here for
visibility/maintenance; the actual submission lives in a separate
`ssh://aur@aur.archlinux.org/flyover.git` repo (AUR convention — it only
accepts a repo with `PKGBUILD` at its root).

Verified working end-to-end via a real `makepkg -s` build (not just written
and assumed): built the package, extracted the binary, ran it. One real gotcha
found and fixed along the way — see the comment in `PKGBUILD` next to
`options=('!lto')`: Arch's default LTO breaks linking against `ring`'s
precompiled assembly objects (a transitive dep via rustls/ureq) with
undefined `ring_core_*` symbols. Confirmed by diffing `makepkg`'s build
environment against a plain `cargo build --release`, which succeeds fine —
`LDFLAGS`'s `-flto=auto` was the only relevant difference.

## Publishing (or updating) the AUR package

1. Create an AUR account at https://aur.archlinux.org/register (if you
   don't have one) and add an SSH public key to it under "My Account" —
   the same key already used for GitHub works fine, or generate a
   dedicated one.
2. Bump `pkgver`/`pkgrel` here, regenerate `.SRCINFO`
   (`makepkg --printsrcinfo > .SRCINFO`), and re-verify with a real
   `makepkg -s` build before pushing.
3. Clone the (empty, on first publish) AUR repo, copy `PKGBUILD` and
   `.SRCINFO` into it, commit, and push:
   ```
   git clone ssh://aur@aur.archlinux.org/flyover.git
   cp PKGBUILD .SRCINFO flyover/
   cd flyover
   git add PKGBUILD .SRCINFO
   git commit -m "flyover 0.1.0-1"
   git push
   ```
