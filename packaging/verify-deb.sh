#!/usr/bin/env bash
# Verify a SodaM Debian artifact without installing system-wide.
# Usage: packaging/verify-deb.sh [path/to/package.deb]
set -euo pipefail
cd "$(dirname "$0")/.."

pkg=${1:-dist/sodam_$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)_$(dpkg --print-architecture 2>/dev/null || echo amd64).deb}
[[ -f "$pkg" ]] || { echo "package not found: $pkg" >&2; exit 1; }
sha_file="$pkg.sha256"
[[ -f "$sha_file" ]] && sha256sum -c "$sha_file"

control=$(dpkg-deb -f "$pkg" Package Version Architecture Depends)
case "$control" in
  *$'Package: sodam'*) ;; *) echo "unexpected package name" >&2; exit 1 ;; esac
for path in \
  ./usr/bin/sodam \
  ./usr/share/applications/sodam.desktop \
  ./usr/share/icons/hicolor/256x256/apps/sodam.png \
  ./usr/share/gnome-shell/extensions/sodam-panel-lyrics@lbg2002/metadata.json; do
  dpkg-deb -c "$pkg" | awk '{print $NF}' | grep -Fx "$path" >/dev/null || {
    echo "missing package path: $path" >&2
    exit 1
  }
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
dpkg-deb --extract "$pkg" "$tmp"
[[ -x "$tmp/usr/bin/sodam" ]]
file "$tmp/usr/bin/sodam" | grep -q 'ELF'
printf '%s\n' "$control"
echo "Debian artifact verification passed"
