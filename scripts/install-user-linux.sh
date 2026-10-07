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
ext_uuid="sodam-panel-lyrics@lbg2002"
ext_src="packaging/gnome-extension/$ext_uuid"
ext_dir="$HOME/.local/share/gnome-shell/extensions/$ext_uuid"

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

# Ubuntu / GNOME 顶栏歌词。非 GNOME 桌面会自然跳过，不影响播放器本体。
if [[ -d "$ext_src" ]]; then
    # 已启用扩展时，仅覆盖 extension.js 不会让 GNOME Shell 重新载入 JS，
    # 会导致播放器已写入新字段但顶栏仍运行旧逻辑。安装前先 unload，复制后再 enable。
    if command -v gnome-extensions >/dev/null 2>&1; then
        gnome-extensions disable "$ext_uuid" >/dev/null 2>&1 || true
    fi

    mkdir -p "$ext_dir"
    install -m644 "$ext_src/metadata.json" "$ext_dir/metadata.json"
    install -m644 "$ext_src/extension.js" "$ext_dir/extension.js"

    if command -v gnome-extensions >/dev/null 2>&1; then
        # 新装扩展时当前 Shell 可能还没加载 metadata，因此 enable 失败不应阻断安装。
        gnome-extensions enable "$ext_uuid" >/dev/null 2>&1 || true
    fi
fi

# Bump mtimes and desktop database. The absolute icon path also avoids stale
# hicolor-theme lookups from older installations.
touch "$desktop_path" "$icon_path"
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$desktop_dir" >/dev/null 2>&1 || true
fi

echo "已安装：$bin_dir/sodam"
echo "桌面入口：$desktop_path"
echo "图标：$icon_path"
if [[ -d "$ext_dir" ]]; then
    echo "GNOME 顶栏歌词扩展：$ext_dir"
    echo "若顶栏歌词首次安装后未立即出现，请注销并重新登录，然后执行：gnome-extensions enable $ext_uuid"
fi
echo "如果 GNOME 应用网格仍显示旧图标，请注销并重新登录一次。"
