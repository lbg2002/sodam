#!/usr/bin/env bash
# Extract the SodaMusic Windows NSIS payload for offline REA analysis.
# Usage: scripts/extract-windows-artifact.sh /path/SodaMusic.exe [output-dir]
set -euo pipefail

input=${1:?usage: extract-windows-artifact.sh /path/SodaMusic.exe [output-dir]}
out=${2:-"$PWD/.private/sodam-windows-extracted"}
command -v 7z >/dev/null 2>&1 || { echo "7z is required (Ubuntu: sudo apt install p7zip-full)" >&2; exit 1; }
[[ -f "$input" ]] || { echo "input not found: $input" >&2; exit 1; }

mkdir -p "$out/installer" "$out/app"
7z x "$input" -o"$out/installer" -y
payload=$(find "$out/installer" -type f -name app.7z -print -quit)
[[ -n "$payload" ]] || { echo "NSIS payload app.7z not found" >&2; exit 1; }
7z x "$payload" -o"$out/app" -y

printf '\nExtracted application files:\n'
find "$out/app" -maxdepth 3 -type f -printf '%p\t%s bytes\n' | sort
printf '\nSHA-256 manifest:\n'
find "$out/app" -type f -print0 | sort -z | xargs -0 sha256sum
