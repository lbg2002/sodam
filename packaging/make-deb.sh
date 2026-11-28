#!/usr/bin/env bash
# Build a Debian package from an already-built Linux binary.
# Usage: packaging/make-deb.sh [version] [arch]
set -euo pipefail
cd "$(dirname "$0")/.."

version=${1:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)}
arch=${2:-$(dpkg --print-architecture 2>/dev/null || echo amd64)}
case "$arch" in
  x86_64) arch=amd64 ;; aarch64) arch=arm64 ;; amd64|arm64) ;; *) echo "unsupported architecture: $arch" >&2; exit 1 ;; esac
[[ -x target/release/sodam ]] || { echo "target/release/sodam is missing; run cargo build --release first" >&2; exit 1; }

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
install -Dm755 target/release/sodam "$stage/usr/bin/sodam"
install -Dm644 packaging/arch/sodam.desktop "$stage/usr/share/applications/sodam.desktop"
install -Dm644 crates/sodam/assets/brand/sodam-logo.png "$stage/usr/share/icons/hicolor/256x256/apps/sodam.png"
install -Dm644 LICENSE "$stage/usr/share/doc/sodam/copyright"
mkdir -p "$stage/DEBIAN"
cat > "$stage/DEBIAN/control" <<CONTROL
Package: sodam
Version: ${version}
Section: sound
Priority: optional
Architecture: ${arch}
Maintainer: Lumina contributors <sodam@users.noreply.github.com>
Depends: libc6 | libc6t64, libasound2 | libasound2t64, libdbus-1-3 | libdbus-1-3t64, libfontconfig1, libfreetype6 | libfreetype6t64, libxkbcommon0, libxkbcommon-x11-0, libxcb1
Description: Lumina native multi-source lossless music player
 Lumina (formerly SodaM) is a native GPUI/Rust desktop client supporting Qishui and Migu Music.
 It supports Linux Wayland/X11, lossless playback, karaoke lyrics, queues and rolling cache.
CONTROL
mkdir -p dist
out="dist/sodam_${version}_${arch}.deb"
dpkg-deb --build --root-owner-group "$stage" "$out" >/dev/null
sha256sum "$out" > "$out.sha256"
printf '%s\n' "$out"
