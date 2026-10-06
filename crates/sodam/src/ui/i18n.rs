//! 极简中英文案层。中文原文作为 key，便于在 UI 里渐进接入。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    System,
    #[default]
    Chinese,
    English,
}

impl Language {
    pub fn from_key(key: &str) -> Option<Self> {
        match key.trim().to_ascii_lowercase().as_str() {
            "auto" | "system" => Some(Self::System),
            "zh" | "zh-cn" | "chinese" => Some(Self::Chinese),
            "en" | "en-us" | "english" => Some(Self::English),
            _ => None,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::System => "auto",
            Self::Chinese => "zh",
            Self::English => "en",
        }
    }

    pub fn system_locale() -> Self {
        sys_locale::get_locale()
            .as_deref()
            .map(|locale| {
                if locale.to_ascii_lowercase().starts_with("zh") {
                    Self::Chinese
                } else {
                    Self::English
                }
            })
            .unwrap_or(Self::Chinese)
    }

    pub fn resolved(self) -> Self {
        match self {
            Self::System => Self::system_locale(),
            language => language,
        }
    }

    pub fn is_zh(self) -> bool {
        self.resolved() == Self::Chinese
    }

    /// 静态文案翻译；中文原文作为稳定 key。
    pub fn text(self, key: &'static str) -> &'static str {
        let language = self.resolved();
        if language.is_zh() {
            return key;
        }
        #[allow(unreachable_patterns)]
        match key {
            "推荐" => "Discover",
            "听歌模式" => "Listening Modes",
            "搜索" => "Search",
            "我喜欢的音乐" => "Liked Music",
            "我的歌单" => "My Playlists",
            "下载管理" => "Downloads",
            "下载" => "Download",
            "音乐人" => "Artists",
            "专辑" => "Albums",
            "设置" => "Settings",
            "自适应预加载" => "Adaptive Prefetch",
            "同时满足最低曲目数和目标分钟数，最长预取 8 首" => {
                "Keep both a minimum track count and a target listening duration, up to 8 tracks"
            }
            "5 分钟" => "5 minutes",
            "12 分钟（推荐）" => "12 minutes (Recommended)",
            "20 分钟" => "20 minutes",
            "30 分钟" => "30 minutes",
            "已开启" => "On",
            "已关闭" => "Off",
            "播放开始后后台逐步缓存后续歌曲；自适应模式按未来播放时长动态决定缓存深度" => {
                "Cache upcoming tracks in the background; adaptive mode sizes the buffer by upcoming listening time"
            }
            "离线播放" => "Offline Playback",
            "离线模式 / 缓存优先" => "Offline / Cache First",
            "开启后只播放当前音质档位已经缓存的歌曲，不发起音频网络请求" => {
                "Only play songs already cached for the current quality and make no new audio network requests"
            }
            "优先使用本地缓存；离线模式开启后完全禁止新的音频网络请求" => {
                "Prefer local cache; offline mode blocks all new audio network requests"
            }
            "在线" => "Online",
            "离线" => "Offline",
            "自适应预加载：至少 {} 分钟" => "Adaptive prefetch: at least {} minutes",
            "固定预加载：前方 {} 首" => "Fixed prefetch: {} tracks ahead",
            "已进入离线模式：只播放本地缓存" => "Offline mode enabled: local cache only",
            "已退出离线模式" => "Offline mode disabled",
            "离线模式保存失败：{err}" => "Failed to save offline mode: {err}",
            "离线不可播放：{} 尚未缓存" => "Unavailable offline: {} is not cached",
            "离线模式：队列中没有更多已缓存歌曲" => "Offline mode: no more cached songs in the queue",
            "离线模式：队列中没有其他已缓存歌曲" => "Offline mode: no other cached songs in the queue",
            "离线模式：这首歌没有本地歌词缓存" => {
                "Offline mode: this track has no local lyric cache"
            }
            "离线模式：未加载的歌词不会联网获取" => {
                "Offline mode: lyrics that are not already loaded will not be fetched"
            }
            "播放队列顺序已更新" => "Queue order updated",
            "睡眠定时结束，已暂停播放" => "Sleep timer ended; playback paused",
            "当前歌曲播放结束，已暂停" => "Current song finished; playback paused",
            "睡眠定时已关闭" => "Sleep timer disabled",
            "睡眠定时：{} 分钟" => "Sleep timer: {} minutes",
            "将在当前歌曲结束后暂停" => "Playback will pause after the current song",
            "播完当前歌曲" => "After current song",
            "剩余 {} 分钟" => "{} minutes remaining",
            "睡眠定时" => "Sleep Timer",
            "15 分钟" => "15 minutes",
            "60 分钟" => "60 minutes",
            "90 分钟" => "90 minutes",

            "关闭睡眠定时" => "Turn Off Sleep Timer",
            "已打开迷你播放器" => "Mini player opened",
            "已打开桌面歌词" => "Desktop lyrics opened",
            "暂无歌词" => "No lyrics",
            "批量下载" => "Batch Download",
            "下载全部" => "Download All",
            "批量下载：{} 首已加入任务，{} 首等待处理" => {
                "Batch download: {} tracks selected, {} waiting"
            }
            "批量下载已暂停" => "Batch download paused",
            "批量下载已继续" => "Batch download resumed",
            "已取消批量下载任务" => "Batch download cancelled",
            "批量下载任务已完成" => "Batch download completed",
            "继续" => "Resume",
            "取消任务" => "Cancel Task",
            "{} / {} 首" => "{} / {} tracks",

            "外观、播放、歌词、下载、存储与账户" => "Appearance, playback, lyrics, downloads, storage, and account",
            "导出时写入歌曲名、歌手、专辑；封面缓存可用时一并嵌入" => {
                "Write title, artist, and album tags on export; embed cached artwork when available"
            }

            "最近播放" => "Recently Played",
            "还没有最近播放记录" => "No recent playback yet",
            "清空最近播放" => "Clear Recent",
            "已清空最近播放" => "Recent playback cleared",
            "清空最近播放失败：{err}" => "Failed to clear recent playback: {err}",
            "歌词字号" => "Lyric Font Size",
            "调整播放页歌词文字大小" => "Adjust lyric text size on the playback page",
            "歌词行距" => "Lyric Spacing",
            "调整每行歌词之间的垂直间距" => "Adjust vertical spacing between lyric lines",
            "歌词时间偏移" => "Lyric Timing Offset",
            "负值让歌词更早出现，正值让歌词更晚出现" => {
                "Negative values show lyrics earlier; positive values show them later"
            }
            "小" => "Small",
            "大" => "Large",
            "特大" => "Extra Large",
            "紧凑" => "Compact",
            "宽松" => "Relaxed",
            "很宽" => "Extra Relaxed",
            "歌词字号设置已保存" => "Lyric font size saved",
            "歌词行距设置已保存" => "Lyric spacing saved",
            "歌词偏移：{} ms" => "Lyric offset: {} ms",
            "歌词设置保存失败：{err}" => "Failed to save lyric settings: {err}",
            "当前保存到：{}" => "Current save location: {}",
            "选择目录" => "Choose Folder",
            "打开目录" => "Open Folder",
            "恢复默认" => "Restore Default",
            "下载目录已更新" => "Download folder updated",
            "有下载任务正在处理，请稍后更改下载目录" => {
                "A download is currently being processed; change the download folder after it finishes"
            }
            "下载目录被 SODAM_DOWNLOAD_DIR 环境变量覆盖，请先取消该变量" => {
                "The download folder is overridden by SODAM_DOWNLOAD_DIR; unset it before changing this setting"
            }

            "下载目录已恢复默认" => "Download folder restored to default",
            "更改下载目录失败：{err}" => "Failed to change download folder: {err}",
            "恢复默认下载目录失败：{err}" => "Failed to restore default download folder: {err}",

            "常规" => "General",
            "存储" => "Storage",
            "账户" => "Account",

            "等待扫码" => "Waiting for scan",
            "已扫码" => "Scanned",
            "登录成功" => "Signed in",
            "已过期" => "Expired",
            "失败" => "Failed",
            "已配置登录与签名服务，可以去「搜索」或「推荐」了" => "Signed in and signer ready. Try Discover or Search.",
            "还没配好：请在「设置」里填 Cookie 与签名服务地址（或设 SODA_COOKIE / QISHUI_SIGNER_URL）" => "Not ready: set Cookie and signer URL in Settings (or use SODA_COOKIE / QISHUI_SIGNER_URL).",
            "推荐队列" => "Recommendation queue",
            "请先在「设置 → 账户」扫码登录" => "Sign in with QR code in Settings → Account.",
            "语言已切换为中文" => "Language switched to Chinese",
            "语言已切换，但保存失败：{err}" => "Language switched, but saving failed: {err}",
            "就绪" => "Ready",
            "创建二维码失败：{err}" => "Failed to create QR code: {err}",
            "二维码编码失败：{err}" => "Failed to encode QR code: {err}",
            "等待扫码…" => "Waiting for scan…",
            "轮询异常（会自动重试）：{err}" => "Polling error (retrying): {err}",
            "登录成功但保存失败：{err}" => "Signed in, but saving failed: {err}",
            "已打开二次验证窗口，请在其中完成验证" => {
                "Second-verification window opened. Complete the verification there."
            }
            "打开二次验证窗口失败：{err}，请改用官方客户端导出 Cookie" => {
                "Failed to open the verification window: {err}. Export a cookie from the official client instead."
            }
            "正在播放的曲目不能从队列移除，可直接点「下一首」" => {
                "The playing track can't be removed from the queue. Use Next instead."
            }
            "已登录：{}（{}）" => "Signed in: {} ({})",
            "账号" => "Account",
            "{}；音质已按权益自动设为 {quality}" => "{}; quality set to {}",
            "音质偏好保存失败：{err}" => "Failed to save audio quality: {err}",
            "读取账号信息失败：{err}" => "Failed to load account: {err}",
            "主题已切换为{}" => "Theme switched to {}",
            "浅色" => "Light",
            "深色" => "Dark",
            "主题已切换，但保存失败：{err}" => "Theme switched, but saving failed: {err}",
            "缓存管理" => "Cache Management",
            "缓存使用情况" => "Cache Usage",
            "达到上限后按最近使用顺序自动清理播放缓存；不会删除下载管理里的歌曲" => {
                "When the limit is reached, old playback cache is removed by recent use; downloaded songs are never deleted."
            }
            "适合磁盘空间较小的设备" => "For devices with limited disk space",
            "3 GB（推荐）" => "3 GB (Recommended)",
            "兼顾无感切歌与磁盘占用" => "Balanced for seamless switching and disk usage",
            "适合经常连续听歌，保留更多本地缓存" => {
                "Keeps more local cache for long listening sessions"
            }
            "不限制" => "Unlimited",
            "不自动清理播放缓存" => "Do not automatically prune playback cache",
            "播放缓存上限：不限制" => "Playback cache limit: unlimited",
            "播放缓存上限：{} GB" => "Playback cache limit: {} GB",
            "缓存设置保存失败：{err}" => "Failed to save cache setting: {err}",
            "已自动清理 {} 个旧缓存" => "Automatically removed {} old cache items",
            "已恢复上次播放队列" => "Restored previous playback queue",
            "已恢复：{}（暂停）" => "Restored: {} (paused)",
            "已停止" => "Stopped",
            "智能预加载" => "Smart Prefetch",
            "播放开始后后台逐步缓存后续歌曲；任务完成会立即补位，减少切歌等待" => {
                "Cache upcoming tracks progressively after playback starts and refill immediately to reduce track-switch delays."
            }
            "前方 1 首" => "1 Track Ahead",
            "前方 3 首（推荐）" => "3 Tracks Ahead (Recommended)",
            "前方 5 首" => "5 Tracks Ahead",
            "不提前缓存后续歌曲；切歌时可能需要等待加载" => {
                "Do not pre-cache upcoming tracks; switching may require loading."
            }
            "最省流量，只保证下一首优先缓存" => {
                "Lowest bandwidth usage; prioritize only the next track."
            }
            "兼顾无感切歌、网络占用与缓存空间" => {
                "Balanced for seamless switching, bandwidth, and cache usage."
            }
            "网络稳定时切歌更从容，但会增加缓存和流量" => {
                "More headroom on stable networks, with higher cache and bandwidth use."
            }
            "智能预加载已关闭" => "Smart prefetch disabled",
            "智能预加载：保持前方 {} 首" => "Smart prefetch: keep {} tracks ahead",
            "预加载设置保存失败：{err}" => "Failed to save prefetch setting: {err}",
            "播放音质偏好" => "Playback Quality",
            "下载设置" => "Download Settings",
            "下载音质" => "Download Quality",
            "下载格式" => "Download Format",
            "跟随播放设置" => "Follow Playback Setting",
            "使用当前播放音质偏好；播放为自动时选择已有缓存中的最高档" => {
                "Use the playback quality preference; when playback is Auto, use the highest cached quality."
            }
            "只导出无损缓存；没有对应缓存时保持待下载" => {
                "Export only lossless cache; remain pending until that cache exists."
            }
            "只导出极高缓存（≈320k）" => "Export only the high-quality cache (≈320k).",
            "只导出较高缓存" => "Export only the medium-quality cache.",
            "只导出标准缓存" => "Export only the standard-quality cache.",
            "原始格式" => "Original Format",
            "直接保存 SodaM 实际播放缓存，速度最快且不二次编码" => {
                "Save SodaM's playback cache directly for the fastest export without re-encoding."
            }
            "使用 ffmpeg 转为高质量 MP3，兼容性最好" => {
                "Convert to high-quality MP3 with ffmpeg for maximum compatibility."
            }
            "使用 ffmpeg 转为 FLAC；有损源不会因此变成真正无损" => {
                "Convert to FLAC with ffmpeg; a lossy source does not become truly lossless."
            }
            "默认保存到：{}" => "Default save location: {}",
            "下载音质设置已保存" => "Download quality setting saved",
            "下载音质保存失败：{err}" => "Failed to save download quality: {err}",
            "下载格式设置已保存" => "Download format setting saved",
            "下载格式保存失败：{err}" => "Failed to save download format: {err}",
            "保存待下载队列失败：{err}" => "Failed to save pending download queue: {err}",
            "下载已完成：{}" => "Download completed: {}",
            "仍在等待播放缓存：{}" => "Still waiting for playback cache: {}",
            "已打开下载文件" => "Opened downloaded file",
            "打开下载文件失败：{err}" => "Failed to open downloaded file: {err}",
            "立即重试" => "Retry Now",
            "打开" => "Open",
            "搜索已下载歌曲 / 歌手" => "Search downloaded songs / artists",
            "打开下载目录" => "Open Download Folder",
            "正在读取下载列表…" => "Loading downloads…",
            "还没有下载的歌曲" => "No downloaded songs yet",
            "没有找到匹配的下载" => "No matching downloads",
            "{} 首 · {} 待下载" => "{} tracks · {} pending",
            "待下载" => "Pending",
            "等待播放缓存" => "Waiting for playback cache",
            "取消" => "Cancel",
            "读取下载列表失败：{err}" => "Failed to load downloads: {err}",
            "已下载：{}" => "Downloaded: {}",
            "已取消待下载：{}" => "Cancelled pending download: {}",
            "已加入待下载：{}；正常播放产生缓存后会自动保存" => {
                "Queued for download: {}. It will be saved after normal playback creates a cache."
            }
            "待下载已完成：{}" => "Pending download completed: {}",
            "下载失败：{err}" => "Download failed: {err}",
            "已删除下载：{}" => "Deleted download: {}",
            "删除下载失败：{err}" => "Failed to delete download: {err}",
            "已打开下载目录" => "Opened download folder",
            "打开下载目录失败：{err}" => "Failed to open download folder: {err}",
            "已用系统默认应用打开配置文件" => "Opened config with the default app",
            "打开配置文件失败：{err}" => "Failed to open config: {err}",
            "已在浏览器打开 GitHub 仓库" => "Opened GitHub repository",
            "打开 GitHub 仓库失败：{err}" => "Failed to open GitHub repository: {err}",
            "「{}」没有可用歌词" => "No lyrics available for {}",
            "「{}」歌词已加载" => "Lyrics loaded for {}",
            "已切换到{}队列" => "Switched to {} queue",
            "正在读取听歌模式…" => "Loading listening modes…",
            "正在加载更多探索歌单…" => "Loading more discovery playlists…",
            "没有更多探索歌单" => "No more discovery playlists",
            "探索歌单已追加 {added} 张" => "Added {} discovery playlists",
            "读取听歌模式失败：{err}" => "Failed to load listening modes: {err}",
            "正在加载「{}」队列…" => "Loading {} queue…",
            "加载听歌模式队列失败：{err}" => "Failed to load listening-mode queue: {err}",
            "推荐" => "Discover",
            "加载推荐队列失败：{err}" => "Failed to load recommendation queue: {err}",
            "正在加载电台「{}」…" => "Loading {} radio…",
            "电台「{}」没有可播放曲目" => "Radio {} has no playable tracks",
            "加载电台失败：{err}" => "Failed to load radio: {err}",
            "歌单「{}」没有可播放曲目" => "Playlist {} has no playable tracks",
            "加载歌单失败：{err}" => "Failed to load playlist: {err}",
            "推荐队列已追加 {added} 首" => "Added {} recommendation tracks",
            "加载更多推荐失败：{err}" => "Failed to load more recommendations: {err}",
            "已退出登录" => "Signed out",
            "已退出，但保存配置失败：{err}" => "Signed out, but saving failed: {err}",
            "读取歌单失败：{err}" => "Failed to load playlist: {err}",
            "已收藏：{}" => "Liked: {}",
            "已取消收藏：{}" => "Unliked: {}",
            "读取收藏失败：{err}" => "Failed to load liked music: {err}",
            "已入队：{title}" => "Queued: {title}",
            "搜索歌单共 {count} 首" => "Search playlist: {} tracks",
            "读取搜索歌单失败：{err}" => "Failed to load search playlist: {err}",
            "下一首将播放：{title}" => "Play next: {}",
            "播放中" => "Playing",
            "已暂停" => "Paused",
            "拉流失败" => "Stream failed",
            "「{}」拉流失败：{err}" => "Stream failed for {}: {err}",
            "连续 3 首拉流失败，已暂停（检查网络或签名服务）" => "Paused after 3 stream failures. Check network or signer.",
            "保存失败：{err}" => "Save failed: {err}",
            "暂停" => "Pause",
            "播放" => "Play",
            "上一首" => "Previous Track",
            "下一首" => "Next Track",
            "显示主窗口" => "Show Main Window",
            "退出" => "Quit",
            "「{}」共 {} 条结果" => "{}: {} results",
            "读取音乐人失败：{err}" => "Failed to load artist: {err}",
            "加载更多音乐人歌曲失败：{err}" => "Failed to load more artist tracks: {err}",
            "读取专辑失败：{err}" => "Failed to load album: {err}",
            "回车搜索歌曲、歌手或专辑" => "Press Enter to search songs, artists or albums",
            "来自汽水的推荐" => "Recommended for you",
            "按场景选歌" => "Choose by scene",
            "音乐人详情" => "Artist Details",
            "专辑详情" => "Album Details",
            "我喜欢的音乐" => "Liked Music",
            "{} 首 · {}" => "{} tracks · {}",
            "搜索歌曲 / 歌手（点这里后输入，回车搜索）" => "Search songs / artists (click, type, press Enter)",
            "亿" => "B",
            "万" => "K",
            "{}人关注 · {}首歌" => "{} followers · {} songs",
            "回车搜索歌曲、音乐人、专辑或歌单" => "Search songs, artists, albums or playlists",
            "{} 首" => "{} tracks",
            "用户 {} · {}" => "User {} · {}",
            "语言已恢复为跟随系统" => "Language restored to system",
            "共 {}（歌曲 {} 首 {} / 封面 {} 张 {}）" => "Total {} ({} songs {}, {} covers {})",
            "签名服务：{}" => "Signer: {}",
            "配置：{}" => "Config: {}",
            "加载更多探索歌单（当前 {} 张）" => "Load More Playlists ({} loaded)",
            "会话 {}…" => "Session {}…",
            "下一首播放" => "Play Next",
            "关闭" => "Close",
            "已固定主题；可随时切换深色或浅色" => "Theme pinned; switch anytime",
            "已固定语言；可随时切换中文或 English" => {
                "Language pinned; switch anytime"
            }
            "扫码登录" => "QR Login",
            "封顶无损；单曲没有无损时自动降级" => {
                "Up to lossless; downgrades automatically"
            }
            "封顶极高（≈320k）" => "Up to high (≈320k)",
            "封顶较高" => "Up to medium",
            "正在准备推荐队列…" => "Preparing recommendations…",
            "正在准备播放…" => "Preparing playback…",
            "正在创建二维码…" => "Creating QR code…",
            "正在播放" => "Now Playing",
            "搜索歌曲 / 歌手（点这里后输入，回车搜索）" => {
                "Search songs / artists (click, type, press Enter)"
            }
            "综合" => "All",
            "歌曲" => "Songs",
            "歌单" => "Playlists",
            "输入关键词进行搜索" => "Type a keyword to search",
            "没有找到相关内容" => "No matching results",
            "没有找到相关歌曲" => "No matching songs",
            "没有找到相关音乐人" => "No matching artists",
            "没有找到相关专辑" => "No matching albums",
            "没有找到相关歌单" => "No matching playlists",
            "搜索中…" => "Searching…",
            "歌单已关闭" => "Playlist closed",
            "还没有选择音乐人" => "No artist selected",
            "正在读取音乐人热歌…" => "Loading artist hits…",
            "暂时没有读到音乐人热歌" => "Artist hits unavailable",
            "还没有选择专辑" => "No album selected",
            "正在读取专辑歌曲…" => "Loading album tracks…",
            "暂时没有读到专辑歌曲" => "Album tracks unavailable",
            "加载更多歌曲" => "Load more songs",
            "播放全部" => "Play All",
            "加载更多探索歌单" => "Load More Playlists",
            "加载更多探索歌单（当前 {n} 张）" => "Load More Playlists ({n} loaded)",
            "正在加载…" => "Loading…",
            "重试" => "Retry",
            "重试加载" => "Retry",
            "常用模式" => "Frequently Used",
            "探索更多新模式" => "Explore More Modes",
            "正在读取常用模式…" => "Loading modes…",
            "常用模式暂不可用" => "Modes unavailable",
            "正在读取探索歌单…" => "Loading playlists…",
            "暂无探索歌单" => "No discovery playlists",
            "电台" => "Radio",
            "首" => "tracks",
            "账号" => "Account",
            "账号、签名服务与音质偏好" => "Account, signer and audio quality",
            "已登录" => "Signed in",
            "未登录" => "Signed out",
            "用户 {} · {}" => "User {} · {}",
            "VIP" => "VIP",
            "非 VIP" => "Non-VIP",
            "正在读取账号信息…" => "Loading account…",
            "登录后才能使用搜索、歌单与播放" => {
                "Sign in to use search, playlists and playback"
            }
            "去登录" => "Sign In",
            "退出登录" => "Sign Out",
            "主题" => "Theme",
            "当前跟随系统偏好，手动选择后会固定主题" => {
                "Following system; choose a theme to pin it"
            }
            "深色" => "Dark",
            "浅色" => "Light",
            "语言" => "Language",
            "跟随系统" => "System",
            "音质偏好" => "Audio Quality",
            "按账号权益和单曲实际可用的档位择优" => {
                "Choose the best usable tier for your account and each track"
            }
            "当前账号是 VIP：登录时默认无损，单曲没有无损会自动降级" => {
                "VIP account: lossless by default; downgrades automatically when unavailable"
            }
            "当前账号非 VIP：默认自动，取免费档里实际可用的最高一档" => {
                "Non-VIP account: Auto by default; uses the best free tier available"
            }
            "自动" => "Auto",
            "无损" => "Lossless",
            "极高" => "High",
            "较高" => "Medium",
            "标准" => "Low",
            "省流" => "Data saver",
            "缓存" => "Cache",
            "共 {}（歌曲 {} 首 {} / 封面 {} 张 {}）" => {
                "Total {} ({} songs {}, {} covers {})"
            }
            "清除歌曲缓存" => "Clear Song Cache",
            "其他" => "Other",
            "签名服务：{}" => "Signer: {}",
            "（未配置）" => "(not configured)",
            "配置：{}" => "Config: {}",
            "Edit" => "Edit",
            "GitHub: {}" => "GitHub: {}",
            "Author: ZephyrCheung" => "Author: ZephyrCheung",
            "歌手" => "Artist",
            "时长" => "Time",
            "这里还没有内容" => "Nothing here yet",
            "未登录：先在「登录」页扫码" => "Sign in with QR code first",
            "正在读取…" => "Loading…",
            "来自汽水的推荐" => "Recommended for you",
            "按场景选歌" => "Choose by scene",
            "音乐人详情" => "Artist Details",
            "专辑详情" => "Album Details",
            "还没有歌单" => "No playlists",
            "登录后这里会显示你的歌单" => "Your playlists appear after sign-in",
            "正在读取歌单…" => "Loading playlists…",
            "登录后这里会显示你收藏的歌曲" => "Liked songs appear after sign-in",
            "还没有喜欢的歌曲" => "No liked songs yet",
            "正在读取我喜欢的音乐…" => "Loading liked music…",
            "正在读取歌单曲目…" => "Loading playlist tracks…",
            "播放中" => "Playing",
            "已暂停" => "Paused",
            "请使用汽水音乐 App 扫码登录" => "Scan with the Qishui Music app",
            "会话 {}…" => "Session {}…",
            "等待扫码" => "Waiting for scan",
            "已扫码" => "Scanned",
            "登录成功" => "Signed in",
            "已过期" => "Expired",
            "失败" => "Failed",
            "现在播放" => "Now Playing",
            "歌词" => "Lyrics",
            "队列" => "Queue",
            "播放队列 · {} 首" => "Queue · {} tracks",
            "已播放 · {} 首" => "Played · {}",
            "接下来 · {} 首" => "Up Next · {}",
            "没有可用歌词" => "No lyrics available",
            "正在读取歌词…" => "Loading lyrics…",
            "设置已保存" => "Settings saved",
            "已配置登录与签名服务，可以去「搜索」或「推荐」了" => "Signed in and signer ready. Try Discover or Search.",
            "还没配好：请在「设置」里填 Cookie 与签名服务地址（或设 SODA_COOKIE / QISHUI_SIGNER_URL）" => "Not ready: set Cookie and signer URL in Settings (or use SODA_COOKIE / QISHUI_SIGNER_URL).",
            "推荐队列" => "Recommendation queue",
            "请先在「设置 → 账户」扫码登录" => "Sign in with QR code in Settings → Account.",
            "语言已切换为中文" => "Language switched to Chinese",
            "语言已切换，但保存失败：{err}" => "Language switched, but saving failed: {err}",
            "就绪" => "Ready",
            "创建二维码失败：{err}" => "Failed to create QR code: {err}",
            "二维码编码失败：{err}" => "Failed to encode QR code: {err}",
            "等待扫码…" => "Waiting for scan…",
            "轮询异常（会自动重试）：{err}" => "Polling error (retrying): {err}",
            "登录成功但保存失败：{err}" => "Signed in, but saving failed: {err}",
            "已打开二次验证窗口，请在其中完成验证" => {
                "Second-verification window opened. Complete the verification there."
            }
            "打开二次验证窗口失败：{err}，请改用官方客户端导出 Cookie" => {
                "Failed to open the verification window: {err}. Export a cookie from the official client instead."
            }
            "正在播放的曲目不能从队列移除，可直接点「下一首」" => {
                "The playing track can't be removed from the queue. Use Next instead."
            }
            "已登录：{}（{}）" => "Signed in: {} ({})",
            "非 VIP" => "Non-VIP",
            "{}；音质已按权益自动设为 {quality}" => "{}; quality set to {quality}",
            "音质偏好保存失败：{err}" => "Failed to save audio quality: {err}",
            "读取账号信息失败：{err}" => "Failed to load account: {err}",
            "主题已切换为{}" => "Theme switched to {}",
            "浅色" => "light",
            "深色" => "dark",
            "主题已切换，但保存失败：{err}" => "Theme switched, but saving failed: {err}",
            "已用系统默认应用打开配置文件" => "Opened config with the default app",
            "打开配置文件失败：{err}" => "Failed to open config: {err}",
            "已在浏览器打开 GitHub 仓库" => "Opened GitHub repository",
            "打开 GitHub 仓库失败：{err}" => "Failed to open GitHub repository: {err}",
            "正在读取「{}」歌词…" => "Loading lyrics for “{}”…",
            "「{}」没有可用歌词" => "No lyrics available for “{}”",
            "「{}」歌词已加载" => "Lyrics loaded for “{}”",
            "已切换到{}队列" => "Switched to {} queue",
            "正在读取听歌模式…" => "Loading listening modes…",
            "正在加载更多探索歌单…" => "Loading more discovery playlists…",
            "没有更多探索歌单" => "No more discovery playlists",
            "探索歌单已追加 {} 张" => "Added {} discovery playlists",
            "读取听歌模式失败：{err}" => "Failed to load listening modes: {err}",
            "「{}」队列" => "{} queue",
            "正在加载「{}」队列…" => "Loading {} queue…",
            "加载听歌模式队列失败：{err}" => "Failed to load listening-mode queue: {err}",
            "已准备播放：{}" => "Ready to play: {}",
            "加载推荐队列失败：{err}" => "Failed to load recommendation queue: {err}",
            "「{}」电台" => "{} radio",
            "正在加载电台「{}」…" => "Loading radio “{}”…",
            "电台「{}」没有可播放曲目" => "Radio “{}” has no playable tracks",
            "加载电台失败：{err}" => "Failed to load radio: {err}",
            "正在加载歌单「{}」…" => "Loading playlist “{}”…",
            "歌单「{}」没有可播放曲目" => "Playlist “{}” has no playable tracks",
            "加载歌单失败：{err}" => "Failed to load playlist: {err}",
            "推荐队列已追加 {} 首" => "Added {} recommendation tracks",
            "加载更多推荐失败：{err}" => "Failed to load more recommendations: {err}",
            "已退出登录" => "Signed out",
            "已退出，但保存配置失败：{err}" => "Signed out, but saving failed: {err}",
            "共 {} 个歌单" => "{} playlists",
            "已收藏：{}" => "Liked: {}",
            "已取消收藏：{}" => "Unliked: {}",
            "我喜欢的音乐：{} 首" => "Liked Music: {} tracks",
            "读取收藏失败：{err}" => "Failed to load liked music: {err}",
            "已入队：{}" => "Queued: {}",
            "正在读取歌单「{}」…" => "Loading playlist “{}”…",
            "搜索歌单共 {} 首" => "Search playlist: {} tracks",
            "读取搜索歌单失败：{err}" => "Failed to load search playlist: {err}",
            "下一首将播放：{}" => "Play next: {}",
            "歌单" => "Playlist",
            "歌单共 {} 首" => "Playlist: {} tracks",
            "播放中" => "Playing",
            "已暂停" => "Paused",
            "正在准备播放：{}…" => "Preparing playback: {}…",
            "播放中：{}" => "Playing: {}",
            "「{}」拉流失败：{err}" => "Stream failed for “{}”: {err}",
            "连续 3 首拉流失败，已暂停（检查网络或签名服务）" => "Paused after 3 stream failures. Check network or signer.",
            "保存失败：{err}" => "Save failed: {err}",
            "先输入关键词再回车" => "Type a keyword and press Enter",
            "正在搜索「{}」…" => "Searching “{}”…",
            "「{}」共 {} 条结果" => "“{}”: {} results",
            "搜索失败：{err}" => "Search failed: {err}",
            "音乐人已加载" => "Artist loaded",
            "读取音乐人失败：{err}" => "Failed to load artist: {err}",
            "音乐人歌曲已加载" => "Artist tracks loaded",
            "加载更多音乐人歌曲失败：{err}" => "Failed to load more artist tracks: {err}",
            "专辑共 {} 首" => "Album: {} tracks",
            "读取专辑失败：{err}" => "Failed to load album: {err}",
            "暂停" => "Pause",
            "播放" => "Play",
            "上一首" => "Previous Track",
            "下一首" => "Next Track",
            "显示主窗口" => "Show Main Window",
            "退出" => "Quit",
            "未在播放" => "Not playing",
            "选择一首歌开始播放吧" => "Choose a song to start playback",
            "已播放 · {} 首" => "Played · {}",
            "接下来 · {} 首" => "Up Next · {}",
            "播放队列 · {} 首" => "Play Queue · {} tracks",
            "我的音乐" => "My Music",
            "会话 {}…" => "Session {}…",
            "登录失败：{err}" => "Sign-in failed: {err}",
            "搜索中…" => "Searching…",
            "回车搜索歌曲、歌手或专辑" => "Press Enter to search songs, artists or albums",
            "{} 条结果" => "{} results",
            "未登录：先在「登录」页扫码" => "Sign in with QR code first",
            "正在读取…" => "Loading…",
            "{} 首" => "{} tracks",
            "{} 个歌单" => "{} playlists",
            "账号、签名服务与音质偏好" => "Account, signer and audio quality",
            "来自汽水的推荐" => "Recommended for you",
            "按场景选歌" => "Choose by scene",
            "音乐人详情" => "Artist Details",
            "专辑详情" => "Album Details",
            "搜索歌曲 / 歌手（点这里后输入，回车搜索）" => "Search songs / artists (click, type, press Enter)",
            "亿" => "B",
            "万" => "K",
            "综合" => "All",
            "音乐人" => "Artists",
            "专辑" => "Albums",
            "{}人关注 · {}首歌" => "{} followers · {} songs",
            "搜索" => "Search",
            "回车搜索歌曲、音乐人、专辑或歌单" => "Search songs, artists, albums or playlists",
            "电台" => "Radio",
            "正在加载…" => "Loading…",
            "加载更多探索歌单（当前 {} 张）" => "Load More Playlists ({} loaded)",
            "用户 {} · {}" => "User {} · {}",
            "中文" => "中文",
            "语言已恢复为跟随系统" => "Language restored to system",
            "共 {}（歌曲 {} 首 {} / 封面 {} 张 {}）" => "Total {} ({} songs {}, {} covers {})",
            "已清理缓存：{} 个文件" => "Cleared {} cache files",
            "签名服务：{}" => "Signer: {}",
            "配置：{}" => "Config: {}",
            "创建二维码失败，可点「重新获取二维码」重试" => {
                "Failed to create QR code. Try again."
            }
            "请用汽水音乐 App 扫码并确认" => {
                "Scan and confirm with the Qishui Music app."
            }
            "登录成功，正在读取账号信息…" => "Signed in. Loading account…",
            "登录成功（会话未初始化）" => "Signed in (session not initialized)",
            "二维码已过期，正在自动换新码…" => "QR code expired. Refreshing…",
            "还没有正在播放的歌曲" => "No song is playing",
            "正在读取听歌模式…" => "Loading listening modes…",
            "正在加载更多探索歌单…" => "Loading more discovery playlists…",
            "没有更多探索歌单" => "No more discovery playlists",
            "读取听歌模式失败：" => "Failed to load listening modes: ",
            "正在加载推荐队列…" => "Loading recommendation queue…",
            "加载推荐队列失败：" => "Failed to load recommendation queue: ",
            "加载电台失败：" => "Failed to load radio queue: ",
            "正在加载更多推荐…" => "Loading more recommendations…",
            "加载更多推荐失败：" => "Failed to load more recommendations: ",
            "已退出登录" => "Signed out",
            "需要先登录才能读取歌单" => "Sign in to load playlists",
            "读取歌单失败：" => "Failed to load playlists: ",
            "需要先登录才能读取收藏" => "Sign in to load liked music",
            "读取收藏失败：" => "Failed to load liked music: ",
            "队列是空的：先去搜索或打开一张歌单" => {
                "Queue is empty. Search or open a playlist."
            }
            "连续 3 首拉流失败，已暂停（检查网络或签名服务）" => {
                "Paused after 3 stream failures. Check network or signer."
            }
            "SodaM" => "SodaM",
            "正在搜索…" => "Searching…",
            "从队列移除" => "Remove from Queue",
            "这首歌暂时没有歌词" => "No lyrics for this track",
            "当前跟随系统语言；手动选择后会固定语言" => {
                "Following system; choose a language to pin it"
            }
            "{}；音质已按权益自动设为 {}" => "{}; quality set to {}",
            "收藏失败：{err}" => "Failed to update liked music: {err}",
            _ => key,
        }
    }
}

impl Language {
    fn normalize_placeholders(value: String) -> String {
        let chars: Vec<char> = value.chars().collect();
        let mut out = String::with_capacity(value.capacity());
        let mut index = 0;
        while index < chars.len() {
            if chars[index] != '{' {
                out.push(chars[index]);
                index += 1;
                continue;
            }
            if chars.get(index + 1) == Some(&'}') {
                out.push_str("{}");
                index += 2;
                continue;
            }
            let mut cursor = index + 1;
            while cursor < chars.len()
                && (chars[cursor].is_ascii_alphanumeric() || chars[cursor] == '_')
            {
                cursor += 1;
            }
            if cursor > index + 1 && chars.get(cursor) == Some(&'}') {
                out.push_str("{}");
                index = cursor + 1;
            } else {
                let end = (cursor + 1).min(chars.len());
                out.extend(&chars[index..end]);
                index = end;
            }
        }
        out
    }

    /// 用顺序占位符格式化；词典里的 `{err}` 这类命名占位符也会按顺序填充。
    pub fn textf(self, key: &'static str, args: &[String]) -> String {
        let mut value = Self::normalize_placeholders(self.text(key).to_string());
        for arg in args {
            value = value.replacen("{}", arg, 1);
        }
        value
    }
}

/// 官方 47 种听歌模式是固定集合；服务端只下发中文名，
/// 这里按稳定的 `sub_queue_type` 提供英文展示名。
pub fn scene_mode_text(language: Language, sub_queue_type: &str, fallback: &str) -> String {
    if language.resolved().is_zh() {
        return fallback.to_string();
    }
    // 服务端常见格式是 `scene_mode_emo`；本地熟悉/新鲜没有前缀。
    let sub_queue_type = sub_queue_type
        .strip_prefix("scene_mode_")
        .unwrap_or(sub_queue_type);
    match sub_queue_type {
        "familiar" => "Familiar".to_string(),
        "fresh" => "Fresh".to_string(),
        "bath" => "Bath Time".to_string(),
        "beach" => "Beach".to_string(),
        "bedtime" => "Bedtime".to_string(),
        "breakup" => "Breakup".to_string(),
        "calm" => "Calm".to_string(),
        "cantonese" => "Cantonese".to_string(),
        "car_mode" => "Car Mode".to_string(),
        "child" => "Kids".to_string(),
        "chill" => "Chill".to_string(),
        "chinese_style" => "Chinese Style".to_string(),
        "classic" => "Classical".to_string(),
        "clean_up" => "Clean Up".to_string(),
        "commute" => "Commute".to_string(),
        "country" => "Country".to_string(),
        "dj" => "DJ".to_string(),
        "douyin_roam" => "Douyin Roaming".to_string(),
        "drunk" => "Tipsy".to_string(),
        "electronic" => "Electronic".to_string(),
        "emo" => "Emo".to_string(),
        "english" => "English Songs".to_string(),
        "fish" => "Fishing".to_string(),
        "focus" => "Focus".to_string(),
        "folk" => "Folk".to_string(),
        "game" => "Gaming".to_string(),
        "get_up" => "Wake Up".to_string(),
        "happy" => "Happy".to_string(),
        "heal" => "Healing".to_string(),
        "jpop" => "J-Pop".to_string(),
        "kpop" => "K-Pop".to_string(),
        "ktv" => "KTV".to_string(),
        "library" => "Library".to_string(),
        "love_song" => "Love Songs".to_string(),
        "lucky" => "Lucky".to_string(),
        "lying_flat" => "Lying Flat".to_string(),
        "night_time" => "Late Night".to_string(),
        "non_vocal" => "Non-Vocal".to_string(),
        "nostalgic" => "Nostalgia".to_string(),
        "rain" => "Rain".to_string(),
        "rap" => "Rap".to_string(),
        "rnb" => "R&B".to_string(),
        "rock" => "Rock".to_string(),
        "slow_motion" => "Slow Motion".to_string(),
        "sport" => "Workout".to_string(),
        "sweet_girl" => "Sweet Girl".to_string(),
        "travel" => "Travel".to_string(),
        _ => fallback.to_string(),
    }
}
