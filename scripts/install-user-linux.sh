#!/usr/bin/env bash
# Install the locally built SodaM release for the current Linux user.
# This deliberately uses a unique icon name so GNOME does not keep showing a
# stale icon from an older system package.
set -euo pipefail

cd "$(dirname "$0")/.."

binary="target/release/sodam"
[[ -x "$binary" ]] || {
    echo "target/release/sodam 不存在；请先运行 scripts/build.sh --release" >&2
    exit 1
}

bin_dir="$HOME/.local/bin"
desktop_dir="$HOME/.local/share/applications"
icon_dir="$HOME/.local/share/icons"
icon_path="$icon_dir/sodam-user.png"
desktop_path="$desktop_dir/sodam.desktop"

mkdir -p "$bin_dir" "$desktop_dir" "$icon_dir"
install -m755 "$binary" "$bin_dir/sodam"
install -m644 crates/sodam/assets/brand/sodam-logo.png "$icon_path"

cat > "$desktop_path" <<EOF
[Desktop Entry]
Type=Application
Name=SodaM
GenericName=Music Player
Comment=SodaM music client
Exec=$bin_dir/sodam
Icon=$icon_path
Terminal=false
Categories=AudioVideo;Audio;Music;Player;
Keywords=music;qishui;soda;sodam;
StartupWMClass=SodaM
EOF

# Bump mtimes and desktop database. The absolute icon path also avoids stale
# hicolor-theme lookups from older installations.
touch "$desktop_path" "$icon_path"
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$desktop_dir" >/dev/null 2>&1 || true
fi

echo "已安装：$bin_dir/sodam"
echo "桌面入口：$desktop_path"
echo "图标：$icon_path"
echo "如果 GNOME 应用网格仍显示旧图标，请注销并重新登录一次。"
