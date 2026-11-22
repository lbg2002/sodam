//! 咪咕音乐（Migu Music）纯 Rust 协议客户端。
//!
//! 职责：
//! - 响应体 `[171, 205, 1, t, ...]` 解密与统一 HTTP 请求头
//! - 多音质流解析（`ZQ24` 24bit 母带 / `SQ` 无损 FLAC / `HQ` 320k / `PQ` 128k / `LQ` 64k）
//! - 逐字歌词 `MRC` 64-bit TEA 解密与 `LRC` 回退
//! - 搜索（综合 / 单曲 / 歌手 / 专辑 / 歌单）、推荐、排行榜、电台、详情页
//! - Cookie / `pacmtoken` 导入、移动 SIM 快捷认证、短信动态码登录、用户歌单与红心收藏

use std::io::Read;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::audio;
use crate::models::{
    parse_lrc, AlbumDetail, AlbumItem, ArtistDetail, ArtistItem, ArtistTracksPage, LyricLine,
    PlaylistItem, SceneItem, SearchEntry, SearchGroup, SearchResults, TrackItem,
};
use crate::session::{AccountInfo, CachedTrack};

const MIGU_SECURE_KEY: &[u8] = b"Jk8qzuePiJ1qE3mDYhLQ3T73DtDoAhLP";
const MRC_TEA_DELTA: i64 = 2_654_435_769;
const MRC_MIN_LENGTH: usize = 32;
const MRC_KEY_RAW: &[u8] = b"karakal@123Qcomyidongtiantianhaoting";
const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// 咪咕推荐探索卡（复用通用 `PlaylistItem`）。
#[derive(Debug, Clone, PartialEq)]
pub struct MiguDiscoverCard {
    pub block_id: String,
    pub playlist: PlaylistItem,
}

/// 咪咕流解析结果。
#[derive(Debug, Clone, PartialEq)]
pub struct MiguStreamResolved {
    pub url: String,
    pub ext: &'static str,
    pub quality_label: String,
    pub expected_size: i64,
    pub is_preview: bool,
}

/// 移动 SIM 认证轮询结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiguSimPoll {
    Waiting,
    Success { cookie: String },
    Failed(String),
}

/// 编码咪咕曲目 ID：`migu_{contentId}_{copyrightId}`。
///
/// 使用下划线分隔，保证缓存文件名与 Windows/macOS/Linux 路径完全兼容。
pub fn encode_track_id(content_id: &str, copyright_id: &str) -> String {
    format!(
        "migu_{}_{}",
        sanitize_id(content_id),
        sanitize_id(copyright_id)
    )
}

/// 从 `migu_{contentId}_{copyrightId}` 解出 `(contentId, copyrightId)`。
pub fn parse_track_id(track_id: &str) -> Option<(String, String)> {
    let rest = track_id.trim().strip_prefix("migu_")?;
    let (content_id, copyright_id) = rest.split_once('_')?;
    let content_id = content_id.trim();
    if content_id.is_empty() {
        return None;
    }
    Some((content_id.to_string(), copyright_id.trim().to_string()))
}

/// 编码咪咕实体 ID（歌单 / 专辑 / 歌手）。
pub fn encode_entity_id(raw_id: &str) -> String {
    let trimmed = raw_id.trim();
    if trimmed.is_empty() {
        String::new()
    } else if trimmed.starts_with("migu_") {
        trimmed.to_string()
    } else {
        format!("migu_{trimmed}")
    }
}

/// 去除 `migu_` 前缀还原咪咕原始实体 ID。
pub fn decode_entity_id(id: &str) -> &str {
    let trimmed = id.trim();
    trimmed.strip_prefix("migu_").unwrap_or(trimmed)
}

fn sanitize_id(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

/// 补全咪咕相对路径图片地址。
pub fn normalize_img_url(raw: &str) -> String {
    let url = raw.trim();
    if url.is_empty() {
        return String::new();
    }
    if url.starts_with("https://") || url.starts_with("http://") {
        return url.to_string();
    }
    if url.starts_with("//") {
        return format!("https:{url}");
    }
    if url.starts_with('/') {
        return format!("https://d.musicapp.migu.cn{url}");
    }
    url.to_string()
}

/// 从用户输入（裸 `pacmtoken`、`Cookie:` 请求头或 DevTools cURL）提取 `(pacmtoken, cookie_header)`。
pub fn parse_migu_credentials(raw: &str) -> (String, String) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (String::new(), String::new());
    }

    let mut extracted_cookie = String::new();
    let mut extracted_pacm = String::new();

    if trimmed.contains("curl ") || trimmed.contains("-H ") || trimmed.contains("-b ") {
        for segment in extract_shell_quoted_tokens(trimmed) {
            let lower = segment.to_ascii_lowercase();
            if let Some(rest) = strip_prefix_ignore_ascii(&segment, "cookie:") {
                extracted_cookie = rest.trim().to_string();
            } else if let Some(rest) = strip_prefix_ignore_ascii(&segment, "pacmtoken:") {
                extracted_pacm = rest.trim().to_string();
            } else if lower.contains("pacmtoken=") && extracted_cookie.is_empty() {
                extracted_cookie = segment.trim().to_string();
            }
        }
    }

    if extracted_cookie.is_empty() {
        let mut candidate = trimmed;
        if let Some(rest) = strip_prefix_ignore_ascii(candidate, "cookie:") {
            candidate = rest.trim();
        }
        if candidate.contains('=') || candidate.contains(';') {
            extracted_cookie = candidate.to_string();
        } else if extracted_pacm.is_empty() {
            extracted_pacm = candidate.to_string();
        }
    }

    if extracted_pacm.is_empty() && !extracted_cookie.is_empty() {
        for pair in extracted_cookie.split(';') {
            let pair = pair.trim();
            if let Some((k, v)) = pair.split_once('=') {
                if k.trim().eq_ignore_ascii_case("pacmtoken") {
                    let val = v.trim();
                    if !val.is_empty() {
                        extracted_pacm = val.to_string();
                        break;
                    }
                }
            }
        }
    }

    if extracted_cookie.is_empty() && !extracted_pacm.is_empty() {
        extracted_cookie = format!("pacmtoken={extracted_pacm}");
    } else if extracted_pacm.is_empty() && !extracted_cookie.is_empty() {
        // 没有明确 pacmtoken 字段时保留 cookie_header 供接口鉴权
    }

    (extracted_pacm, extracted_cookie)
}

fn strip_prefix_ignore_ascii<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    let bytes = input.as_bytes();
    let pbytes = prefix.as_bytes();
    if bytes.len() < pbytes.len() {
        return None;
    }
    if bytes[..pbytes.len()].eq_ignore_ascii_case(pbytes) {
        Some(&input[pbytes.len()..])
    } else {
        None
    }
}

fn extract_shell_quoted_tokens(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let quote = chars[i];
        if quote == '\'' || quote == '"' {
            i += 1;
            let mut buf = String::new();
            while i < chars.len() {
                if quote == '"' && chars[i] == '\\' && i + 1 < chars.len() {
                    buf.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if chars[i] == quote {
                    break;
                }
                buf.push(chars[i]);
                i += 1;
            }
            out.push(buf);
        }
        i += 1;
    }
    out
}

/// 解密咪咕 `[171, 205, 1, t, ...]` 二进制封装或直接解析明文 JSON。
pub fn decode_migu_bytes(bytes: &[u8]) -> Result<Value> {
    if bytes.len() >= 4 && bytes[0] == 171 && bytes[1] == 205 && bytes[2] == 1 {
        let offset = bytes[3];
        let mut out = Vec::with_capacity(bytes.len() - 4);
        for (idx, &byte) in bytes[4..].iter().enumerate() {
            let decoded = byte
                .wrapping_add(offset)
                .wrapping_sub(MIGU_SECURE_KEY[idx % MIGU_SECURE_KEY.len()]);
            out.push(decoded);
        }
        let text = String::from_utf8(out).context("咪咕解密结果不是合法 UTF-8")?;
        return serde_json::from_str(&text).context("解析咪咕解密 JSON 失败");
    }
    serde_json::from_slice(bytes).context("解析咪咕响应 JSON 失败")
}

/// 解密咪咕 `MRC` 逐字歌词十六进制密文（64-bit signed TEA -> UTF-16LE）。
pub fn decrypt_mrc(raw_hex: &str) -> Result<String> {
    let hex = raw_hex.trim();
    if hex.len() < MRC_MIN_LENGTH {
        return Ok(hex.to_string());
    }
    let block_count = hex.len() / 16;
    if block_count == 0 {
        return Ok(String::new());
    }

    let mut data = Vec::with_capacity(block_count);
    for i in 0..block_count {
        let slice = &hex[i * 16..(i + 1) * 16];
        let val = u64::from_str_radix(slice, 16)
            .with_context(|| format!("MRC 歌词十六进制块解析失败: {slice}"))?;
        data.push(val as i64);
    }

    // 密钥固定使用 UTF-8 字符串按字节零扩展为 UTF-16LE，再按小端 8 字节打包为 i64 数组。
    let mut padded_key = MRC_KEY_RAW.to_vec();
    while padded_key.len() < MRC_MIN_LENGTH {
        padded_key.push(b'0');
    }
    let mut utf16_bytes = vec![0u8; padded_key.len() * 2];
    for (idx, &b) in padded_key.iter().enumerate() {
        utf16_bytes[idx * 2] = b;
    }
    let key_len = utf16_bytes.len().div_ceil(8);
    let mut key = Vec::with_capacity(key_len);
    for chunk_idx in 0..key_len {
        let mut buf = [0u8; 8];
        let start = chunk_idx * 8;
        let end = (start + 8).min(utf16_bytes.len());
        buf[..end - start].copy_from_slice(&utf16_bytes[start..end]);
        key.push(i64::from_le_bytes(buf));
    }

    let n = data.len();
    let mut j3 = data[0];
    let rounds = (6 + 52 / n) as i64;
    let mut j4 = rounds.wrapping_mul(MRC_TEA_DELTA);
    while j4 != 0 {
        let j5 = j4;
        let j6 = 3 & (j5 >> 2);
        for j7 in (1..n).rev() {
            let j8 = data[j7 - 1];
            let key_idx = ((3 & (j7 as i64)) ^ j6) as usize % key.len();
            let lhs = (j3 ^ j5).wrapping_add(j8 ^ key[key_idx]);
            let rhs = ((j8 >> 5) ^ j3.wrapping_shl(2)).wrapping_add((j3 >> 3) ^ j8.wrapping_shl(4));
            j3 = data[j7].wrapping_sub(lhs ^ rhs);
            data[j7] = j3;
        }
        let j9 = data[n - 1];
        let key_idx = (j6 as usize) % key.len();
        let lhs = (key[key_idx] ^ j9).wrapping_add(j3 ^ j5);
        let rhs = ((j9 >> 5) ^ j3.wrapping_shl(2)).wrapping_add((j3 >> 3) ^ j9.wrapping_shl(4));
        j3 = data[0].wrapping_sub(lhs ^ rhs);
        data[0] = j3;
        j4 = j5.wrapping_sub(MRC_TEA_DELTA);
    }

    let mut plain_bytes = Vec::with_capacity(n * 8);
    for word in data {
        plain_bytes.extend_from_slice(&word.to_le_bytes());
    }
    while plain_bytes.last() == Some(&b'0') {
        plain_bytes.pop();
    }
    if plain_bytes.len() % 2 == 1 {
        plain_bytes.push(0);
    }
    let units: Vec<u16> = plain_bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    let decoded = char::decode_utf16(units)
        .map(|ch| ch.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect::<String>();
    Ok(decoded.trim_end_matches('\0').to_string())
}

/// 将解密后的 `MRC` 文本转换成 SodaM 可保留逐字时间轴的增强 LRC 格式。
pub fn mrc_to_enhanced_lrc(mrc: &str) -> String {
    let mut out = String::new();
    let mut zero_line_seq = 0u64;

    for raw_line in mrc.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with('[') {
            continue;
        }
        let Some((tag, rest)) = line[1..].split_once(']') else {
            continue;
        };
        let Some((start_raw, dur_raw)) = tag.split_once(',') else {
            // [ti:...] / [ar:...] / [00:12.34] 原样保留
            out.push_str(line);
            out.push('\n');
            continue;
        };
        let Ok(mut line_start) = start_raw.trim().parse::<u64>() else {
            out.push_str(line);
            out.push('\n');
            continue;
        };
        let line_dur = dur_raw.trim().parse::<u64>().unwrap_or(0);
        if line_start == 0 && line_dur == 0 {
            line_start = zero_line_seq;
            zero_line_seq += 1;
        }

        let words = parse_mrc_word_tokens(rest);
        if words.is_empty() {
            let plain = rest.trim().replace('<', "‹").replace('>', "›");
            if !plain.is_empty() {
                out.push_str(&format!("[{line_start},{line_dur}]{plain}\n"));
            }
            continue;
        }

        out.push_str(&format!("[{line_start},{line_dur}]"));
        for (word_text, w_start, w_dur) in words {
            let safe_word = word_text.replace('<', "‹").replace('>', "›");
            let mut effective_start = w_start.max(line_start);
            if line_dur == 0 && effective_start == 0 {
                effective_start = line_start;
            }
            let effective_end = effective_start.saturating_add(w_dur);
            out.push_str(&format!(
                "<{effective_start},{w_dur}>{safe_word}</{effective_end},{w_dur}>"
            ));
        }
        out.push('\n');
    }

    out
}

fn parse_mrc_word_tokens(rest: &str) -> Vec<(String, u64, u64)> {
    let mut tokens = Vec::new();
    let bytes = rest.as_bytes();
    let mut seg_start = 0usize;
    let mut i = 0usize;
    let mut last_time = 0u64;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            if let Some(rel_close) = rest[i + 1..].find(')') {
                let inside = &rest[i + 1..i + 1 + rel_close];
                if let Some((w_start, w_dur)) = parse_mrc_pair(inside) {
                    let word = &rest[seg_start..i];
                    if !word.is_empty() {
                        tokens.push((word.to_string(), w_start, w_dur));
                    }
                    last_time = w_start.saturating_add(w_dur);
                    i = i + 1 + rel_close + 1;
                    seg_start = i;
                    continue;
                }
            }
        }
        i += 1;
    }
    if seg_start < rest.len() {
        let tail = &rest[seg_start..];
        if !tail.trim().is_empty() {
            tokens.push((tail.to_string(), last_time, 0));
        }
    }
    tokens
}

fn parse_mrc_pair(inside: &str) -> Option<(u64, u64)> {
    let (a, b) = inside.split_once(',')?;
    let a = a.trim();
    let b = b.trim();
    if a.is_empty()
        || b.is_empty()
        || !a.bytes().all(|c| c.is_ascii_digit())
        || !b.bytes().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    Some((a.parse().ok()?, b.parse().ok()?))
}

/// 咪咕协议客户端（无状态读取配置快照，全部请求均为同步阻塞，由调用方放到后台线程）。
#[derive(Debug, Clone)]
pub struct MiguClient {
    pub device_id: String,
    pub pacmtoken: String,
    pub cookie_header: String,
}

impl MiguClient {
    pub fn new(raw_cookie: &str, device_id: &str) -> Self {
        let (pacmtoken, cookie_header) = parse_migu_credentials(raw_cookie);
        let dev = if device_id.trim().is_empty() {
            "2f4c8f10-7c4d-4c8d-9b30-1c82b6f38a01".to_string()
        } else {
            device_id.trim().to_string()
        };
        Self {
            device_id: dev,
            pacmtoken,
            cookie_header,
        }
    }

    pub fn is_logged_in(&self) -> bool {
        !self.pacmtoken.is_empty() || !self.cookie_header.is_empty()
    }

    fn apply_headers(&self, mut req: ureq::Request) -> ureq::Request {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        req = req
            .set("channel", "014X031")
            .set("subchannel", "014X031")
            .set("ua", "Android_migu")
            .set("version", "6.8.8")
            .set("imei", &self.device_id)
            .set("deviceId", &self.device_id)
            .set("activityId", "MUSIC-WWW")
            .set("birth", "h5page")
            .set("signature", "1")
            .set("timestamp", &now_ms.to_string())
            .set("Referer", "https://music.migu.cn/")
            .set("User-Agent", USER_AGENT);
        if !self.pacmtoken.is_empty() {
            req = req.set("pacmtoken", &self.pacmtoken);
        }
        if !self.cookie_header.is_empty() {
            req = req.set("Cookie", &self.cookie_header);
        }
        req
    }

    fn get_bytes(&self, url: &str, timeout_secs: u64) -> Result<Vec<u8>> {
        let req = self.apply_headers(ureq::get(url).timeout(Duration::from_secs(timeout_secs)));
        let resp = req
            .call()
            .map_err(|err| anyhow::anyhow!("咪咕请求失败: {err}"))?;
        let mut bytes = Vec::new();
        resp.into_reader()
            .take(16 * 1024 * 1024)
            .read_to_end(&mut bytes)
            .context("读取咪咕响应失败")?;
        Ok(bytes)
    }

    fn get_json(&self, url: &str) -> Result<Value> {
        let bytes = self.get_bytes(url, 15)?;
        decode_migu_bytes(&bytes)
    }

    fn post_json(&self, url: &str, body: &Value) -> Result<Value> {
        let req = self
            .apply_headers(ureq::post(url).timeout(Duration::from_secs(15)))
            .set("Content-Type", "application/json");
        let resp = req
            .send_string(&body.to_string())
            .map_err(|err| anyhow::anyhow!("咪咕提交请求失败: {err}"))?;
        let mut bytes = Vec::new();
        resp.into_reader()
            .take(8 * 1024 * 1024)
            .read_to_end(&mut bytes)
            .context("读取咪咕响应失败")?;
        decode_migu_bytes(&bytes)
    }

    /// 搜索：综合 / 歌曲 / 音乐人 / 专辑 / 歌单。
    pub fn search(&self, keyword: &str, mode: &str) -> Result<SearchResults> {
        let trimmed = keyword.trim();
        let encoded = url_encode(trimmed);
        match mode {
            "tracks" => {
                let tracks = self.search_songs(&encoded, 30)?;
                Ok(build_scoped_search_results(
                    trimmed.to_string(),
                    tracks,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                ))
            }
            "artists" => {
                let artists = self.search_artists(&encoded, 20)?;
                Ok(build_scoped_search_results(
                    trimmed.to_string(),
                    Vec::new(),
                    artists,
                    Vec::new(),
                    Vec::new(),
                ))
            }
            "albums" => {
                let albums = self.search_albums(&encoded, 20)?;
                Ok(build_scoped_search_results(
                    trimmed.to_string(),
                    Vec::new(),
                    Vec::new(),
                    albums,
                    Vec::new(),
                ))
            }
            "playlists" => {
                let playlists = self.search_playlists(&encoded, 20)?;
                Ok(build_scoped_search_results(
                    trimmed.to_string(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    playlists,
                ))
            }
            _ => {
                let (tracks, artists, albums, playlists) = std::thread::scope(|scope| {
                    let h_tracks = scope.spawn(|| self.search_songs(&encoded, 15));
                    let h_artists = scope.spawn(|| self.search_artists(&encoded, 6));
                    let h_albums = scope.spawn(|| self.search_albums(&encoded, 6));
                    let h_playlists = scope.spawn(|| self.search_playlists(&encoded, 6));
                    (
                        h_tracks
                            .join()
                            .ok()
                            .and_then(Result::ok)
                            .unwrap_or_default(),
                        h_artists
                            .join()
                            .ok()
                            .and_then(Result::ok)
                            .unwrap_or_default(),
                        h_albums
                            .join()
                            .ok()
                            .and_then(Result::ok)
                            .unwrap_or_default(),
                        h_playlists
                            .join()
                            .ok()
                            .and_then(Result::ok)
                            .unwrap_or_default(),
                    )
                });
                if tracks.is_empty()
                    && artists.is_empty()
                    && albums.is_empty()
                    && playlists.is_empty()
                {
                    // 保底再跑一次单曲搜索以返回真实网络/接口错误
                    let tracks = self.search_songs(&encoded, 20)?;
                    return Ok(build_scoped_search_results(
                        trimmed.to_string(),
                        tracks,
                        artists,
                        albums,
                        playlists,
                    ));
                }
                Ok(build_scoped_search_results(
                    trimmed.to_string(),
                    tracks,
                    artists,
                    albums,
                    playlists,
                ))
            }
        }
    }

    pub fn search_songs(&self, encoded_kw: &str, limit: usize) -> Result<Vec<TrackItem>> {
        let url =
            format!("https://app.c.nf.migu.cn/bmw/search/song/v1.0?pageNo=1&text={encoded_kw}");
        let value = self.get_json(&url)?;
        ensure_migu_ok(&value, "搜索咪咕歌曲失败")?;
        let mut out = Vec::new();
        for item in value
            .pointer("/data/items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(track) = parse_migu_song(item) {
                out.push(track);
                if out.len() >= limit {
                    break;
                }
            }
        }
        Ok(out)
    }

    fn search_artists(&self, encoded_kw: &str, limit: usize) -> Result<Vec<ArtistItem>> {
        let url =
            format!("https://app.c.nf.migu.cn/bmw/search/singer/v2.0?pageNo=1&text={encoded_kw}");
        let value = self.get_json(&url)?;
        ensure_migu_ok(&value, "搜索咪咕音乐人失败")?;
        let mut out = Vec::new();
        for item in value
            .pointer("/data/items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let singer = item.get("singer").unwrap_or(item);
            let raw_id = str_field(singer, &["singerId", "id"]);
            if raw_id.is_empty() {
                continue;
            }
            let name = str_field(singer, &["singer", "name", "singerName"]);
            let avatar = extract_img_from_array(singer.get("imgs"))
                .or_else(|| {
                    let s = str_field(singer, &["artis5gImg", "img"]);
                    (!s.is_empty()).then_some(s)
                })
                .map(|s| normalize_img_url(&s))
                .unwrap_or_default();
            out.push(ArtistItem {
                id: encode_entity_id(&raw_id),
                name,
                avatar,
                track_count: int_field(singer, &["songNum", "songCount"]),
                follower_count: int_field(singer, &["followNums", "followerCount"]),
            });
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }

    fn search_albums(&self, encoded_kw: &str, limit: usize) -> Result<Vec<AlbumItem>> {
        let url = format!(
            "https://app.c.nf.migu.cn/bmw/search/album/v1.0?pageNo=1&typeOrder=0&text={encoded_kw}"
        );
        let value = self.get_json(&url)?;
        ensure_migu_ok(&value, "搜索咪咕专辑失败")?;
        let mut out = Vec::new();
        for item in value
            .pointer("/data/items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let album = item.get("album").unwrap_or(item);
            let raw_id = str_field(album, &["albumId", "id"]);
            if raw_id.is_empty() {
                continue;
            }
            let cover = extract_img_from_array(album.get("imgItems"))
                .or_else(|| {
                    let s = str_field(album, &["img", "cover"]);
                    (!s.is_empty()).then_some(s)
                })
                .map(|s| normalize_img_url(&s))
                .unwrap_or_default();
            let release_str = str_field(album, &["publishDate", "publishTime"]);
            out.push(AlbumItem {
                id: encode_entity_id(&raw_id),
                title: str_field(album, &["title", "albumName", "name"]),
                cover,
                artist: str_field(album, &["singer", "singerName", "artist"]),
                track_count: int_field(album, &["totalCount", "songNum"]),
                release_date: parse_ymd_timestamp(&release_str),
                company: str_field(album, &["publishCompany", "publishCorp"]),
            });
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }

    fn search_playlists(&self, encoded_kw: &str, limit: usize) -> Result<Vec<PlaylistItem>> {
        let url = format!(
            "https://app.c.nf.migu.cn/bmw/search/music-list/v1.0?pageNo=1&typeOrder=0&text={encoded_kw}"
        );
        let value = self.get_json(&url)?;
        ensure_migu_ok(&value, "搜索咪咕歌单失败")?;
        let mut out = Vec::new();
        for item in value
            .pointer("/data/items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let pl = item.get("musicList").unwrap_or(item);
            if let Some(playlist) = parse_migu_playlist(pl) {
                out.push(playlist);
                if out.len() >= limit {
                    break;
                }
            }
        }
        Ok(out)
    }

    /// 首页每日推荐歌曲或场景队列。
    pub fn recommend_tracks(&self, scene: Option<&SceneItem>, page: u32) -> Result<Vec<TrackItem>> {
        let page = page.max(1);
        if let Some(scene) = scene {
            let sq = scene.sub_queue_type.trim();
            if let Some(rank_id) = sq.strip_prefix("migu_rank:") {
                return self.rank_tracks(rank_id, page);
            }
            if sq == "familiar" || sq == "fresh" {
                let radio_type = if sq == "fresh" { 2 } else { 1 };
                let url = format!(
                    "https://app.c.nf.migu.cn/pc/column/music-radio/recommend/song/v2.0?remain=0&actionId=1&type={radio_type}"
                );
                if let Ok(value) = self.get_json(&url) {
                    let tracks = extract_tracks_from_list_value(&value);
                    if !tracks.is_empty() {
                        return Ok(tracks);
                    }
                }
                if sq == "fresh" {
                    // 新鲜模式兜底读取榜单列表里的第二个榜单（新歌榜）或热歌榜第 2 页
                    if let Ok(tracks) = self.rank_tracks("83049001", page) {
                        if !tracks.is_empty() {
                            return Ok(tracks);
                        }
                    }
                    return self.rank_tracks("83048887", page.saturating_add(1));
                }
            }
        }

        let url = format!(
            "https://app.c.nf.migu.cn/pc/resource-dataloader/recommend-song/v1.0?scene=TODAY_RECOMMEND&action={page}&size=25"
        );
        if let Ok(value) = self.get_json(&url) {
            let tracks = extract_tracks_from_list_value(&value);
            if !tracks.is_empty() {
                return Ok(tracks);
            }
        }
        // 回落到咪咕全网热歌榜，确保无登录或区域波动时仍可正常听歌
        self.rank_tracks("83048887", page)
    }

    pub fn rank_tracks(&self, rank_id: &str, page: u32) -> Result<Vec<TrackItem>> {
        let raw_id = decode_entity_id(rank_id);
        let url = format!(
            "https://app.c.nf.migu.cn/bmw/rank/rank-info/v1.0?pageNo={}&rankId={raw_id}",
            page.max(1)
        );
        let value = self.get_json(&url)?;
        ensure_migu_ok(&value, "读取咪咕榜单失败")?;
        let mut tracks = Vec::new();
        for item in value
            .pointer("/data/contents")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(song_data_str) = item.get("songData").and_then(Value::as_str) {
                if let Ok(inner) = serde_json::from_str::<Value>(song_data_str) {
                    if let Some(track) = parse_migu_song(&inner) {
                        tracks.push(track);
                        continue;
                    }
                }
            }
            if let Some(track) = parse_migu_song(item) {
                tracks.push(track);
            }
        }
        Ok(tracks)
    }

    /// 听歌模式 & 官方榜单。
    pub fn scene_modes(&self) -> Result<Vec<SceneItem>> {
        let mut items = vec![
            SceneItem {
                text: "熟悉模式".to_string(),
                entry_type: "scene_mode".to_string(),
                scene_mode_id: -1,
                sub_queue_type: "familiar".to_string(),
                cover: String::new(),
            },
            SceneItem {
                text: "新鲜模式".to_string(),
                entry_type: "scene_mode".to_string(),
                scene_mode_id: -1,
                sub_queue_type: "fresh".to_string(),
                cover: String::new(),
            },
        ];

        let value = self.get_json("https://app.c.nf.migu.cn/pc/bmw/rank/rank-index/v1.0")?;
        ensure_migu_ok(&value, "读取咪咕排行榜单失败")?;
        for block in value
            .pointer("/data/contents")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            for entry in block
                .get("contents")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let rank_id = str_field(entry, &["rankId"]);
                let name = str_field(entry, &["rankName", "txt", "title"]);
                if rank_id.is_empty() || name.is_empty() {
                    continue;
                }
                let cover = normalize_img_url(&str_field(entry, &["imageUrl", "img"]));
                items.push(SceneItem {
                    text: name,
                    entry_type: "scene_mode".to_string(),
                    scene_mode_id: 0,
                    sub_queue_type: format!("migu_rank:{rank_id}"),
                    cover,
                });
            }
        }
        Ok(items)
    }

    /// 首页/探索页推荐歌单。
    pub fn discover_playlists(&self) -> Result<Vec<MiguDiscoverCard>> {
        let value =
            self.get_json("https://app.c.nf.migu.cn/bmw/index-show/recommend-playlist/v3.0")?;
        ensure_migu_ok(&value, "读取咪咕推荐歌单失败")?;
        let mut out = Vec::new();
        for item in value
            .pointer("/data/playLists")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let raw_id =
                str_field(item, &["resId", "playListId", "musicListId"]).if_empty_then(|| {
                    item.pointer("/logEvent/contentId")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string()
                });
            if raw_id.is_empty() {
                continue;
            }
            let title = str_field(item, &["txt", "title", "name"]).if_empty_then(|| {
                item.pointer("/logEvent/contentName")
                    .and_then(Value::as_str)
                    .unwrap_or("推荐歌单")
                    .to_string()
            });
            let cover = normalize_img_url(&str_field(item, &["img", "imageUrl"]));
            let encoded_id = encode_entity_id(&raw_id);
            out.push(MiguDiscoverCard {
                block_id: raw_id,
                playlist: PlaylistItem {
                    id: encoded_id,
                    title,
                    cover,
                    track_count: int_field(item, &["musicNum", "songCount"]),
                    creator: str_field(item, &["ownerName"]),
                },
            });
        }
        Ok(out)
    }

    /// 歌单内全部曲目（优先公共歌单接口，必要时回落个人歌单接口）。
    pub fn playlist_tracks(&self, playlist_id: &str) -> Result<Vec<TrackItem>> {
        let raw_id = decode_entity_id(playlist_id);
        let mut tracks = Vec::new();
        let mut total_count = 0usize;

        for page in 1..=3 {
            let url = format!(
                "https://app.c.nf.migu.cn/MIGUM3.0/resource/playlist/song/v2.0?pageNo={page}&pageSize=50&playlistId={raw_id}"
            );
            let value = self.get_json(&url)?;
            let code = value
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if code != "000000" {
                if page == 1 {
                    break;
                }
                break;
            }
            if page == 1 {
                total_count = value
                    .pointer("/data/totalCount")
                    .and_then(|v| {
                        v.as_i64()
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                    })
                    .unwrap_or(0)
                    .max(0) as usize;
            }
            let batch = value
                .pointer("/data/songList")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(parse_migu_song)
                        .collect::<Vec<TrackItem>>()
                })
                .unwrap_or_default();
            if batch.is_empty() {
                break;
            }
            let batch_len = batch.len();
            tracks.extend(batch);
            if batch_len < 50 || (total_count > 0 && tracks.len() >= total_count) {
                break;
            }
        }

        if tracks.is_empty() && self.is_logged_in() {
            let url = format!(
                "https://app.c.nf.migu.cn/resource/h5/my/playlist/song/v1.0?pageNo=1&pageSize=200&playlistId={raw_id}"
            );
            if let Ok(value) = self.get_json(&url) {
                tracks = extract_tracks_from_list_value(&value);
            }
        }

        Ok(tracks)
    }

    /// 专辑详情 + 曲目。
    pub fn album_detail(&self, album_id: &str) -> Result<AlbumDetail> {
        let raw_id = decode_entity_id(album_id);
        let info_url =
            format!("https://app.c.nf.migu.cn/MIGUM3.0/resource/album/v2.0?albumId={raw_id}");
        let songs_url = format!(
            "https://app.c.nf.migu.cn/MIGUM3.0/resource/album/song/v2.0?albumId={raw_id}&pageNo=1"
        );
        let (info_res, songs_res) = std::thread::scope(|s| {
            let h1 = s.spawn(|| self.get_json(&info_url));
            let h2 = s.spawn(|| self.get_json(&songs_url));
            (
                h1.join()
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("线程异常"))),
                h2.join()
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("线程异常"))),
            )
        });
        let info_val = info_res?;
        ensure_migu_ok(&info_val, "读取咪咕专辑详情失败")?;
        let songs_val = songs_res?;
        ensure_migu_ok(&songs_val, "读取咪咕专辑曲目失败")?;

        let data = info_val.get("data").unwrap_or(&Value::Null);
        let cover = extract_img_from_array(data.get("imgItems"))
            .or_else(|| {
                let s = str_field(data, &["img", "cover"]);
                (!s.is_empty()).then_some(s)
            })
            .map(|s| normalize_img_url(&s))
            .unwrap_or_default();
        let release_str = str_field(data, &["publishDate", "publishTime"]);
        let tracks = extract_tracks_from_list_value(&songs_val);
        let album = AlbumItem {
            id: encode_entity_id(raw_id),
            title: str_field(data, &["title", "albumName", "name"]),
            cover,
            artist: str_field(data, &["singer", "singerName", "artist"]),
            track_count: int_field(data, &["totalCount", "songNum"]).max(tracks.len() as i64),
            release_date: parse_ymd_timestamp(&release_str),
            company: str_field(data, &["publishCompany", "publishCorp"]),
        };
        Ok(AlbumDetail { album, tracks })
    }

    /// 音乐人详情 + 热歌首页 + 专辑。
    pub fn artist_detail(&self, artist_id: &str) -> Result<ArtistDetail> {
        let raw_id = decode_entity_id(artist_id);
        let idx_url = format!("https://app.c.nf.migu.cn/bmw/singer/index/v1.0?singerId={raw_id}");
        let song_url = format!(
            "https://app.c.nf.migu.cn/bmw/singer/song/v1.0?pageNo=1&singerId={raw_id}&type=1"
        );
        let alb_url =
            format!("https://app.c.nf.migu.cn/bmw/singer/album/v1.0?singerId={raw_id}&pageNo=1");

        let (idx_res, song_res, alb_res) = std::thread::scope(|s| {
            let h1 = s.spawn(|| self.get_json(&idx_url));
            let h2 = s.spawn(|| self.get_json(&song_url));
            let h3 = s.spawn(|| self.get_json(&alb_url));
            (
                h1.join().ok().and_then(Result::ok),
                h2.join()
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("线程异常"))),
                h3.join().ok().and_then(Result::ok),
            )
        });

        let mut description = String::new();
        if let Some(idx_val) = idx_res {
            for block in idx_val
                .pointer("/data/contents")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if block.get("view").and_then(Value::as_str) == Some("ZJ-Singer-Intro-Scroll") {
                    if let Some(txt) = block.pointer("/contents/0/txt2").and_then(Value::as_str) {
                        description = txt.trim().to_string();
                        break;
                    }
                }
            }
        }

        let song_val = song_res?;
        ensure_migu_ok(&song_val, "读取咪咕音乐人歌曲失败")?;
        let (tracks, mut artist_name, mut artist_avatar) =
            extract_singer_page_tracks(&song_val, raw_id);

        let mut albums = Vec::new();
        if let Some(alb_val) = alb_res {
            for item in alb_val
                .pointer("/data/contents")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if item.get("resType").and_then(Value::as_str) == Some("1") {
                    // 数字专辑跳转 H5 页，非标准专辑资源
                    continue;
                }
                let aid = str_field(item, &["resId", "albumId", "id"]);
                let title = str_field(item, &["txt", "title"]);
                if aid.is_empty() || title.is_empty() {
                    continue;
                }
                albums.push(AlbumItem {
                    id: encode_entity_id(&aid),
                    title,
                    cover: normalize_img_url(&str_field(item, &["img"])),
                    artist: str_field(item, &["txt2"]),
                    track_count: 0,
                    release_date: parse_ymd_timestamp(&str_field(item, &["txt3"])),
                    company: String::new(),
                });
            }
        }

        if artist_name.is_empty() {
            artist_name = tracks
                .first()
                .map(|t| t.artist.clone())
                .unwrap_or_else(|| "音乐人".to_string());
        }
        if artist_avatar.is_empty() {
            artist_avatar = tracks.first().map(|t| t.cover.clone()).unwrap_or_default();
        }

        let tracks_has_more = !tracks.is_empty();
        Ok(ArtistDetail {
            artist: ArtistItem {
                id: encode_entity_id(raw_id),
                name: artist_name,
                avatar: artist_avatar,
                track_count: tracks.len() as i64,
                follower_count: 0,
            },
            description,
            tracks,
            albums,
            tracks_cursor: "2".to_string(),
            tracks_has_more,
        })
    }

    pub fn artist_tracks(&self, artist_id: &str, cursor: &str) -> Result<ArtistTracksPage> {
        let raw_id = decode_entity_id(artist_id);
        let page = cursor.trim().parse::<u32>().unwrap_or(2).max(2);
        let url = format!(
            "https://app.c.nf.migu.cn/bmw/singer/song/v1.0?pageNo={page}&singerId={raw_id}&type=1"
        );
        let value = self.get_json(&url)?;
        ensure_migu_ok(&value, "读取更多咪咕音乐人歌曲失败")?;
        let (tracks, _, _) = extract_singer_page_tracks(&value, raw_id);
        let has_more = tracks.len() >= 15;
        Ok(ArtistTracksPage {
            tracks,
            next_cursor: (page + 1).to_string(),
            has_more,
        })
    }

    /// 获取歌词（优先 MRC 逐字歌词并转为增强 LRC，其次标准 LRC）。
    pub fn fetch_lyrics_lrc(&self, track_id: &str) -> Result<String> {
        let (content_id, _) = parse_track_id(track_id)
            .ok_or_else(|| anyhow::anyhow!("非法的咪咕曲目 ID：{track_id}"))?;
        let info_url = format!(
            "https://app.u.nf.migu.cn/MIGUM2.0/v1.0/content/resourceinfo.do?resourceId={content_id}&resourceType=2"
        );
        let value = self.get_json(&info_url)?;
        let res0 = value
            .pointer("/resource/0")
            .ok_or_else(|| anyhow::anyhow!("咪咕歌词资源信息为空"))?;
        let mrc_url = str_field(res0, &["mrcUrl"]);
        let lrc_url = str_field(res0, &["lrcUrl"]);

        if !mrc_url.is_empty() {
            if let Ok(raw_bytes) = self.get_bytes(&mrc_url, 15) {
                let hex = String::from_utf8_lossy(&raw_bytes).trim().to_string();
                if let Ok(decrypted) = decrypt_mrc(&hex) {
                    let converted = mrc_to_enhanced_lrc(&decrypted);
                    if !parse_lrc(&converted).is_empty() {
                        return Ok(converted);
                    }
                }
            }
        }

        if !lrc_url.is_empty() {
            let raw_bytes = self.get_bytes(&lrc_url, 15)?;
            let text = String::from_utf8_lossy(&raw_bytes).into_owned();
            if !text.trim().is_empty() {
                return Ok(text);
            }
        }

        Ok(String::new())
    }

    pub fn lyrics(&self, track_id: &str) -> Result<Vec<LyricLine>> {
        let raw = self.fetch_lyrics_lrc(track_id)?;
        Ok(parse_lrc(&raw))
    }

    /// 根据用户偏好与账号权限级联解析可用最高音质音频流。
    pub fn resolve_stream(&self, track_id: &str, quality_pref: &str) -> Result<MiguStreamResolved> {
        let (content_id, mut copyright_id) = parse_track_id(track_id)
            .ok_or_else(|| anyhow::anyhow!("非法的咪咕曲目 ID：{track_id}"))?;

        if copyright_id.is_empty() {
            if let Ok((filled_cpr, _)) = self.lookup_copyright_id(&content_id) {
                copyright_id = filled_cpr;
            }
        }
        if copyright_id.is_empty() {
            anyhow::bail!("咪咕曲目缺少版权 ID（copyrightId），无法请求播放流");
        }

        let logged_in = self.is_logged_in();
        let tiers: Vec<&'static str> = match quality_pref.trim().to_ascii_lowercase().as_str() {
            "zq24" => vec!["ZQ24", "SQ", "HQ", "PQ"],
            "lossless" => vec!["SQ", "HQ", "PQ"],
            "highest" | "medium" => vec!["HQ", "PQ"],
            "low" => vec!["PQ", "LQ"],
            _ => {
                if logged_in {
                    vec!["ZQ24", "SQ", "HQ", "PQ"]
                } else {
                    vec!["HQ", "PQ"]
                }
            }
        };

        let mut last_dialog = String::new();
        for tone in tiers {
            let resource_types: &[&str] = if tone == "SQ" && logged_in {
                &["2", "E"]
            } else {
                &["2"]
            };
            for rt in resource_types {
                let url = format!(
                    "https://app.c.nf.migu.cn/strategy/pc/listen/v2.0?contentId={content_id}&copyrightId={copyright_id}&scene=&netType=01&resourceType={rt}&toneFlag={tone}"
                );
                if let Ok(value) = self.get_json(&url) {
                    if let Some(stream) = parse_listen_v2_response(&value, tone) {
                        return Ok(stream);
                    }
                    if last_dialog.is_empty() {
                        last_dialog = value
                            .pointer("/data/dialogInfo/text")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .trim()
                            .to_string();
                    }
                }
            }
        }

        // H5 v2.4 与 PC v1.0 兜底
        let h5_url = format!(
            "https://c.musicapp.migu.cn/strategy/listen-url/h5/v2.4?contentId={content_id}&copyrightId={copyright_id}&resourceType=2&netType=01&toneFlag=PQ&scene=&lowerQualityContentId={content_id}"
        );
        if let Ok(resp) = self
            .apply_headers(ureq::get(&h5_url).timeout(Duration::from_secs(12)))
            .set("Referer", "https://y.migu.cn/")
            .set("location-data", "30.6698676660,104.1229614820")
            .call()
        {
            let mut bytes = Vec::new();
            if resp
                .into_reader()
                .take(4 * 1024 * 1024)
                .read_to_end(&mut bytes)
                .is_ok()
            {
                if let Ok(value) = decode_migu_bytes(&bytes) {
                    if let Some(stream) = parse_listen_v2_response(&value, "PQ") {
                        return Ok(stream);
                    }
                }
            }
        }

        let v1_url = format!(
            "https://app.c.nf.migu.cn/MIGUM3.0/strategy/pc/listen/v1.0?contentId={content_id}&copyrightId={copyright_id}&resourceType=2&toneFlag=PQ"
        );
        if let Ok(value) = self.get_json(&v1_url) {
            if let Some(stream) = parse_listen_v2_response(&value, "PQ") {
                return Ok(stream);
            }
        }

        if !last_dialog.is_empty() {
            if !logged_in {
                anyhow::bail!(
                    "咪咕限制：{last_dialog}（可在「设置 → 账号与会员」登录咪咕音乐会员）"
                );
            }
            anyhow::bail!("咪咕暂未下发可播放地址：{last_dialog}");
        }
        anyhow::bail!("未能解析咪咕播放流，请检查网络或重新登录咪咕账号")
    }

    fn lookup_copyright_id(&self, content_id: &str) -> Result<(String, String)> {
        let url = format!(
            "https://app.c.nf.migu.cn/resource/song/by-contentids/v2.0?contentId={content_id}"
        );
        let value = self.get_json(&url)?;
        let item = value
            .pointer("/data/0")
            .or_else(|| value.pointer("/resource/0"))
            .unwrap_or(&Value::Null);
        Ok((
            str_field(item, &["copyrightId", "ringCopyrightId"]),
            str_field(item, &["songName"]),
        ))
    }

    /// 下载音频至本地缓存目录，并返回 `CachedTrack`。
    pub fn download_to_cache(
        &self,
        track: &TrackItem,
        quality_pref: &str,
        timeout_seconds: u32,
    ) -> Result<CachedTrack> {
        let stream = self.resolve_stream(&track.id, quality_pref)?;
        let bytes = self.download_stream_bytes(&stream.url, timeout_seconds)?;
        let actual_ext = detect_audio_extension(&bytes, stream.ext);
        let dir = audio::cache_dir();
        std::fs::create_dir_all(&dir).context("创建音频缓存目录失败")?;

        let tag = if quality_pref.trim().is_empty() {
            "auto".to_string()
        } else {
            quality_pref.trim().to_ascii_lowercase()
        };
        let path = dir.join(format!("{}-{tag}.{actual_ext}", track.id));
        let quality_path = path.with_extension("quality");
        let part = dir.join(unique_part_name(&track.id, &tag, actual_ext));

        std::fs::write(&part, &bytes).context("写入咪咕音频缓存失败")?;
        let written = bytes.len() as u64;
        if stream.expected_size > 0 && written < stream.expected_size as u64 {
            let _ = std::fs::remove_file(&part);
            anyhow::bail!(
                "下载不完整（至少需要 {} 字节，实际 {written} 字节）",
                stream.expected_size
            );
        }
        std::fs::rename(&part, &path).context("落盘咪咕音频缓存失败")?;
        let quality = if stream.is_preview {
            format!("{} · 试听", stream.quality_label)
        } else {
            stream.quality_label
        };
        let _ = std::fs::write(&quality_path, format!("{quality}\t{written}"));
        audio::touch_audio_cache(&path);
        Ok(CachedTrack { path, quality })
    }

    fn download_stream_bytes(&self, url: &str, timeout_seconds: u32) -> Result<Vec<u8>> {
        let mut last_error = String::new();
        for attempt in 0..3 {
            let response = ureq::get(url.trim())
                .timeout(Duration::from_secs(timeout_seconds.clamp(15, 600) as u64))
                .set("User-Agent", USER_AGENT)
                .set("Referer", "https://music.migu.cn/")
                .set("Accept", "*/*")
                .set("Connection", "close")
                .call();
            let response = match response {
                Ok(r) => r,
                Err(err) => {
                    last_error = format!("咪咕音频响应失败：{err}");
                    if attempt < 2 {
                        std::thread::sleep(Duration::from_millis(350 * (attempt + 1)));
                        continue;
                    }
                    anyhow::bail!("{last_error}");
                }
            };
            let mut body = Vec::new();
            match response.into_reader().read_to_end(&mut body) {
                Ok(_) if !body.is_empty() => return Ok(body),
                Ok(_) => last_error = "咪咕音频响应为空".to_string(),
                Err(err) => last_error = format!("读取咪咕音频失败：{err}"),
            }
            if attempt < 2 {
                std::thread::sleep(Duration::from_millis(350 * (attempt + 1)));
            }
        }
        anyhow::bail!("{last_error}");
    }

    /// 读取当前登录的咪咕账号信息（含 VIP 状态验证）。
    pub fn fetch_account(&self) -> Result<AccountInfo> {
        if !self.is_logged_in() {
            anyhow::bail!("尚未登录咪咕音乐");
        }
        let value = self.get_json("https://app.c.nf.migu.cn/pc/user/h5/queryUserInfo/v1.0")?;
        let code = value
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if code != "000000" {
            let info = str_field(&value, &["info"]);
            anyhow::bail!("咪咕会话无效或已过期：{info}");
        }
        let data = value.get("data").unwrap_or(&Value::Null);
        let nickname = str_field(data, &["nickname", "nickName", "userName", "msisdn"]);
        let user_id = str_field(data, &["userId", "uid", "id", "msisdn"]);
        let avatar_url = normalize_img_url(&str_field(
            data,
            &["avatar", "headImg", "middleAvatar", "smallAvatar", "img"],
        ));

        let mut vip = matches!(
            str_field(data, &["vipFlag", "isVip", "vip", "memberType"])
                .to_ascii_lowercase()
                .as_str(),
            "1" | "true" | "vip" | "svip"
        ) || data
            .get("vipInfo")
            .and_then(|v| v.get("isVip").or_else(|| v.get("vipFlag")))
            .is_some_and(|v| v == true || v == 1 || v == "1");

        // 部分版本 queryUserInfo 字段名不同，用一首标准 VIP 曲目探测 SQ 权限兜底
        if !vip {
            let probe = self
                .get_json("https://app.c.nf.migu.cn/strategy/pc/listen/v2.0?contentId=600929000000096577&copyrightId=60054704965&scene=&netType=01&resourceType=2&toneFlag=SQ")
                .ok();
            if let Some(p) = probe {
                vip = p
                    .pointer("/data/url")
                    .and_then(Value::as_str)
                    .is_some_and(|u| !u.trim().is_empty());
            }
        }

        Ok(AccountInfo {
            nickname: if nickname.is_empty() {
                "咪咕音乐用户".to_string()
            } else {
                nickname
            },
            user_id,
            vip,
            avatar_url,
        })
    }

    /// 我的歌单 + 收藏的歌单。
    pub fn user_playlists(&self) -> Result<Vec<PlaylistItem>> {
        if !self.is_logged_in() {
            anyhow::bail!("需要先登录咪咕音乐才能读取我的歌单");
        }
        let my_url =
            "https://app.c.nf.migu.cn/user/h5/my-music-list/v1.0?queryType=0&pageNo=1&pageSize=50";
        let col_url =
            "https://app.c.nf.migu.cn/user/h5/user/collection/v1.0?pageNo=1&type=1&OPType=03&pageSize=50&resourceType=2021";
        let (my_val, col_val) = std::thread::scope(|s| {
            let h1 = s.spawn(|| self.get_json(my_url));
            let h2 = s.spawn(|| self.get_json(col_url));
            (
                h1.join()
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("线程异常"))),
                h2.join().ok().and_then(Result::ok),
            )
        });
        let my_val = my_val?;
        ensure_migu_ok(&my_val, "读取咪咕我的歌单失败")?;

        let mut playlists = extract_playlists_from_value(&my_val);
        let mut seen: std::collections::HashSet<String> =
            playlists.iter().map(|p| p.id.clone()).collect();
        if let Some(col_val) = col_val {
            for pl in extract_playlists_from_value(&col_val) {
                if seen.insert(pl.id.clone()) {
                    playlists.push(pl);
                }
            }
        }
        Ok(playlists)
    }

    /// 「我喜欢的音乐」。
    pub fn liked_songs(&self) -> Result<Vec<TrackItem>> {
        if !self.is_logged_in() {
            anyhow::bail!("尚未登录咪咕音乐");
        }
        let my_url =
            "https://app.c.nf.migu.cn/user/h5/my-music-list/v1.0?queryType=0&pageNo=1&pageSize=50";
        let my_val = self.get_json(my_url)?;
        ensure_migu_ok(&my_val, "读取咪咕歌单失败")?;
        let playlists = extract_playlists_from_value(&my_val);
        let liked = playlists
            .iter()
            .find(|p| {
                p.title.contains("喜欢") || p.title.contains("默认") || p.title.contains("收藏")
            })
            .or_else(|| playlists.first())
            .ok_or_else(|| anyhow::anyhow!("未找到咪咕「我喜欢的音乐」列表"))?;

        let raw_id = decode_entity_id(&liked.id);
        let url = format!(
            "https://app.c.nf.migu.cn/resource/h5/my/playlist/song/v1.0?pageNo=1&pageSize=200&playlistId={raw_id}"
        );
        let songs_val = self.get_json(&url)?;
        ensure_migu_ok(&songs_val, "读取咪咕喜欢歌曲失败")?;
        Ok(extract_tracks_from_list_value(&songs_val))
    }

    pub fn set_track_liked(&self, track_id: &str, liked: bool) -> Result<()> {
        if !self.is_logged_in() {
            anyhow::bail!("请先登录咪咕音乐账号后再操作收藏");
        }
        let (content_id, _) = parse_track_id(track_id)
            .ok_or_else(|| anyhow::anyhow!("非法的咪咕曲目 ID：{track_id}"))?;
        if liked {
            let value = self.post_json(
                "https://app.c.nf.migu.cn/pc/user/api/add-music-list-song/v1.0",
                &serde_json::json!({ "contentIds": [content_id] }),
            )?;
            ensure_migu_ok(&value, "咪咕收藏失败")
        } else {
            let value = self.post_json(
                "https://app.c.nf.migu.cn/pc/user/h5-import-musiclist/v1.0",
                &serde_json::json!({
                    "channel": "23",
                    "contentId": content_id,
                    "songflag": "2",
                }),
            )?;
            ensure_migu_ok(&value, "咪咕取消收藏失败")
        }
    }

    /// 发起中国移动 SIM 一键认证请求，返回 `sim_sessionid`。
    pub fn sim_login_send(&self, phone: &str) -> Result<String> {
        let enc_phone = rsa_encrypt_passport_field(phone.trim())?;
        let fp = random_hex_32();
        let form = form_encode(&[
            ("isAsync", "true"),
            ("sim", "1"),
            ("encMsisdn", &enc_phone),
            ("appType", "0"),
            ("relayState", ""),
            ("sourceID", "220029"),
            ("fingerPrint", &fp),
            ("fingerPrintDetail", &fp),
        ]);
        let resp = ureq::post("https://passport.migu.cn/api/simauth/send")
            .timeout(Duration::from_secs(15))
            .set("Content-Type", "application/x-www-form-urlencoded")
            .set("Referer", "https://passport.migu.cn/login")
            .set("Origin", "https://passport.migu.cn")
            .set("User-Agent", USER_AGENT)
            .send_string(&form)
            .map_err(|err| anyhow::anyhow!("发起 SIM 认证失败：{err}"))?;
        let val: Value =
            serde_json::from_reader(resp.into_reader()).context("解析 SIM 响应失败")?;
        if str_field(&val, &["status"]) != "2000" {
            let msg = str_field(&val, &["msg", "message", "desc"]);
            anyhow::bail!("SIM 一键认证下发失败：{msg}");
        }
        let sid = val
            .pointer("/result/sim_sessionid")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        anyhow::ensure!(!sid.is_empty(), "服务端未返回 sim_sessionid");
        Ok(sid)
    }

    /// 轮询移动 SIM 认证状态；确认后自动兑换 `pacmtoken`。
    pub fn sim_login_poll(&self, sim_session_id: &str) -> Result<MiguSimPoll> {
        let form = form_encode(&[
            ("isAsync", "true"),
            ("sim_sessionid", sim_session_id),
            ("sourceID", "220029"),
        ]);
        let resp = ureq::post("https://passport.migu.cn/api/simauth/query")
            .timeout(Duration::from_secs(15))
            .set("Content-Type", "application/x-www-form-urlencoded")
            .set("Referer", "https://passport.migu.cn/login")
            .set("Origin", "https://passport.migu.cn")
            .set("User-Agent", USER_AGENT)
            .send_string(&form)
            .map_err(|err| anyhow::anyhow!("查询 SIM 认证失败：{err}"))?;
        let val: Value =
            serde_json::from_reader(resp.into_reader()).context("解析 SIM 状态失败")?;
        match str_field(&val, &["status"]).as_str() {
            "4071" => Ok(MiguSimPoll::Waiting),
            "2000" => {
                let token = val
                    .pointer("/result/token")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim();
                if token.is_empty() {
                    return Ok(MiguSimPoll::Failed("登录凭证为空".to_string()));
                }
                let pacm = exchange_pacm_token(token)?;
                Ok(MiguSimPoll::Success { cookie: pacm })
            }
            status => {
                let msg = str_field(&val, &["msg", "message", "desc"]);
                Ok(MiguSimPoll::Failed(if msg.is_empty() {
                    format!("认证已中止（状态码 {status}）")
                } else {
                    msg
                }))
            }
        }
    }

    /// 发送短信动态码。若遇到风控图形验证码要求，不绕过，明确提示用户改用 Cookie 导入或 SIM 认证。
    pub fn sms_send_code(&self, phone: &str) -> Result<()> {
        let enc_phone = rsa_encrypt_passport_field(phone.trim())?;
        let fp = random_hex_32();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis().to_string())
            .unwrap_or_default();
        let query = form_encode(&[
            ("isAsync", "true"),
            ("msisdn", &enc_phone),
            ("captcha", ""),
            ("sourceID", "220029"),
            ("imgcodeType", "2"),
            ("fingerPrint", &fp),
            ("fingerPrintDetail", &fp),
            ("_", &ts),
        ]);
        let url = format!("https://passport.migu.cn/login/dynamicpassword?{query}");
        let resp = ureq::get(&url)
            .timeout(Duration::from_secs(15))
            .set("Referer", "https://passport.migu.cn/login")
            .set("Origin", "https://passport.migu.cn")
            .set("User-Agent", USER_AGENT)
            .call()
            .map_err(|err| anyhow::anyhow!("发送短信验证码失败：{err}"))?;
        let val: Value = serde_json::from_reader(resp.into_reader()).context("解析短信响应失败")?;
        let status = str_field(&val, &["status"]);
        if status == "2000" || val.get("code").and_then(Value::as_str) == Some("000000") {
            return Ok(());
        }
        let msg = str_field(&val, &["msg", "message", "desc", "info"]);
        if msg.contains("验证") || msg.contains("图片") || status == "4001" {
            anyhow::bail!(
                "当前网络环境触发图形验证码保护，请改用「Cookie 导入」或「移动 SIM 快捷认证」"
            );
        }
        anyhow::bail!(
            "短信验证码发送失败：{}",
            if msg.is_empty() { status } else { msg }
        )
    }

    /// 验证短信动态码并返回可落盘的 `pacmtoken=...` 字符串。
    pub fn sms_verify_login(&self, phone: &str, code: &str) -> Result<String> {
        let enc_phone = rsa_encrypt_passport_field(phone.trim())?;
        let enc_code = rsa_encrypt_passport_field(code.trim())?;
        let fp = random_hex_32();
        let form = form_encode(&[
            ("isAsync", "true"),
            ("msisdn", &enc_phone),
            ("dynamicPassword", &enc_code),
            ("captcha", ""),
            ("appType", ""),
            ("securityCode", "7619"),
            ("sourceID", "220029"),
            ("imgcodeType", "2"),
            ("fingerPrint", &fp),
            ("fingerPrintDetail", &fp),
        ]);
        let resp = ureq::post("https://passport.migu.cn/authn/dynamicpassword")
            .timeout(Duration::from_secs(15))
            .set("Content-Type", "application/x-www-form-urlencoded")
            .set("Referer", "https://passport.migu.cn/login")
            .set("Origin", "https://passport.migu.cn")
            .set("User-Agent", USER_AGENT)
            .send_string(&form)
            .map_err(|err| anyhow::anyhow!("短信验证失败：{err}"))?;
        let val: Value =
            serde_json::from_reader(resp.into_reader()).context("解析短信登录响应失败")?;
        if str_field(&val, &["status"]) != "2000" {
            let msg = str_field(&val, &["msg", "message", "desc"]);
            anyhow::bail!("短信登录失败：{msg}");
        }
        let token = val
            .pointer("/result/token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        anyhow::ensure!(!token.is_empty(), "服务端未返回登录 token");
        exchange_pacm_token(token)
    }
}

fn exchange_pacm_token(passport_token: &str) -> Result<String> {
    let url = format!(
        "https://c.musicapp.migu.cn/user/h5/token-validate/v3.0?token={}&type=2&sourceId=220029&activityId=MUSIC-WWW",
        url_encode(passport_token)
    );
    let resp = ureq::get(&url)
        .timeout(Duration::from_secs(15))
        .set("Cookie", "idmpauth=true@passport.migu.cn")
        .set("Origin", "https://music.migu.cn")
        .set("Referer", "https://music.migu.cn/")
        .set("Pragma", "no-cache")
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| anyhow::anyhow!("兑换咪咕音乐令牌失败：{err}"))?;

    for header_val in resp.all("set-cookie") {
        for part in header_val.split(';') {
            let part = part.trim();
            if let Some((k, v)) = part.split_once('=') {
                if k.trim().eq_ignore_ascii_case("pacmtoken") {
                    let val = v.trim();
                    if !val.is_empty() {
                        return Ok(format!("pacmtoken={val}"));
                    }
                }
            }
        }
    }

    let val: Value = serde_json::from_reader(resp.into_reader()).unwrap_or(Value::Null);
    let body_token = val
        .pointer("/data/pacmtoken")
        .or_else(|| val.pointer("/data/token"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if !body_token.is_empty() {
        return Ok(format!("pacmtoken={body_token}"));
    }
    anyhow::bail!("登录成功但未拿到 pacmtoken，请尝试直接复制浏览器 Cookie")
}

fn rsa_encrypt_passport_field(plaintext: &str) -> Result<String> {
    let resp = ureq::post("https://passport.migu.cn/password/publickey")
        .timeout(Duration::from_secs(12))
        .set("Referer", "https://passport.migu.cn/login")
        .set("Origin", "https://passport.migu.cn")
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| anyhow::anyhow!("获取咪咕登录公钥失败：{err}"))?;
    let val: Value = serde_json::from_reader(resp.into_reader()).context("解析公钥失败")?;
    let modulus = val
        .pointer("/result/modulus")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let exponent = val
        .pointer("/result/publicExponent")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if modulus.is_empty() || exponent.is_empty() {
        anyhow::bail!("咪咕登录公钥为空");
    }
    rsa_encrypt_pkcs1_v15_hex(plaintext.as_bytes(), modulus, exponent)
}

/// 纯 Rust PKCS#1 v1.5 Type-2 公钥加密，零额外依赖。
pub fn rsa_encrypt_pkcs1_v15_hex(
    plaintext: &[u8],
    modulus_hex: &str,
    exponent_hex: &str,
) -> Result<String> {
    let n_bytes = decode_hex_bytes(modulus_hex)?;
    let trimmed_n = strip_leading_zero_bytes(&n_bytes);
    anyhow::ensure!(!trimmed_n.is_empty(), "RSA modulus 为空");
    let k = trimmed_n.len();
    anyhow::ensure!(
        plaintext.len() + 11 <= k,
        "RSA 明文过长（最多 {} 字节）",
        k.saturating_sub(11)
    );

    let mut block = vec![0u8; k];
    block[0] = 0x00;
    block[1] = 0x02;
    let ps_len = k - plaintext.len() - 3;
    let mut rng_state = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9e37_79b9_7f4a_7c15)
        ^ (std::process::id() as u64);
    if rng_state == 0 {
        rng_state = 0x1234_5678_9abc_def0;
    }
    for byte in &mut block[2..2 + ps_len] {
        // xorshift64 生成非零随机填充字节
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        *byte = ((rng_state % 255) as u8) + 1;
    }
    block[2 + ps_len] = 0x00;
    block[3 + ps_len..].copy_from_slice(plaintext);

    let e_bytes = decode_hex_bytes(exponent_hex)?;
    let n = BigIntLimbs::from_be_bytes(trimmed_n);
    let m = BigIntLimbs::from_be_bytes(&block);
    let c = BigIntLimbs::mod_pow(&m, &e_bytes, &n);
    let out_bytes = c.to_be_bytes();
    let mut hex = String::with_capacity(out_bytes.len() * 2);
    for b in strip_leading_zero_bytes(&out_bytes) {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{b:02x}");
    }
    if hex.is_empty() {
        hex.push_str("00");
    }
    Ok(hex)
}

#[derive(Clone, PartialEq, Eq)]
struct BigIntLimbs {
    // 小端 u32 肢体（limbs[0] 为最低 32 位）
    limbs: Vec<u32>,
}

impl BigIntLimbs {
    fn zero() -> Self {
        Self { limbs: Vec::new() }
    }

    fn one() -> Self {
        Self { limbs: vec![1] }
    }

    fn trim(&mut self) {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
    }

    fn from_be_bytes(bytes: &[u8]) -> Self {
        let bytes = strip_leading_zero_bytes(bytes);
        let mut limbs = Vec::with_capacity(bytes.len().div_ceil(4));
        let mut i = bytes.len();
        while i > 0 {
            let start = i.saturating_sub(4);
            let mut buf = [0u8; 4];
            buf[4 - (i - start)..].copy_from_slice(&bytes[start..i]);
            limbs.push(u32::from_be_bytes(buf));
            i = start;
        }
        let mut out = Self { limbs };
        out.trim();
        out
    }

    fn to_be_bytes(&self) -> Vec<u8> {
        if self.limbs.is_empty() {
            return vec![0];
        }
        let mut bytes = Vec::with_capacity(self.limbs.len() * 4);
        for &limb in self.limbs.iter().rev() {
            bytes.extend_from_slice(&limb.to_be_bytes());
        }
        let stripped = strip_leading_zero_bytes(&bytes);
        if stripped.is_empty() {
            vec![0]
        } else {
            stripped.to_vec()
        }
    }

    fn cmp_abs(&self, other: &Self) -> std::cmp::Ordering {
        let len_cmp = self.limbs.len().cmp(&other.limbs.len());
        if len_cmp != std::cmp::Ordering::Equal {
            return len_cmp;
        }
        for i in (0..self.limbs.len()).rev() {
            let c = self.limbs[i].cmp(&other.limbs[i]);
            if c != std::cmp::Ordering::Equal {
                return c;
            }
        }
        std::cmp::Ordering::Equal
    }

    fn sub_assign(&mut self, other: &Self) {
        let mut borrow = 0i64;
        for i in 0..self.limbs.len() {
            let a = self.limbs[i] as i64;
            let b = other.limbs.get(i).copied().unwrap_or(0) as i64;
            let mut diff = a - b - borrow;
            if diff < 0 {
                diff += 1i64 << 32;
                borrow = 1;
            } else {
                borrow = 0;
            }
            self.limbs[i] = diff as u32;
        }
        self.trim();
    }

    fn add_mod(a: &Self, b: &Self, m: &Self) -> Self {
        let len = a.limbs.len().max(b.limbs.len());
        let mut res = Vec::with_capacity(len + 1);
        let mut carry = 0u64;
        for i in 0..len {
            let sum = a.limbs.get(i).copied().unwrap_or(0) as u64
                + b.limbs.get(i).copied().unwrap_or(0) as u64
                + carry;
            res.push(sum as u32);
            carry = sum >> 32;
        }
        if carry > 0 {
            res.push(carry as u32);
        }
        let mut out = Self { limbs: res };
        out.trim();
        while out.cmp_abs(m) != std::cmp::Ordering::Less {
            out.sub_assign(m);
        }
        out
    }

    fn double_mod(&mut self, m: &Self) {
        let mut carry = 0u32;
        for limb in &mut self.limbs {
            let next_carry = *limb >> 31;
            *limb = (*limb << 1) | carry;
            carry = next_carry;
        }
        if carry > 0 {
            self.limbs.push(carry);
        }
        self.trim();
        if self.cmp_abs(m) != std::cmp::Ordering::Less {
            self.sub_assign(m);
        }
    }

    fn mul_mod(a: &Self, b: &Self, m: &Self) -> Self {
        if a.limbs.is_empty() || b.limbs.is_empty() {
            return Self::zero();
        }
        let mut a_red = a.clone();
        while a_red.cmp_abs(m) != std::cmp::Ordering::Less {
            a_red.sub_assign(m);
        }
        let mut res = Self::zero();
        for &limb in b.limbs.iter().rev() {
            for bit in (0..32).rev() {
                res.double_mod(m);
                if (limb >> bit) & 1 == 1 {
                    res = Self::add_mod(&res, &a_red, m);
                }
            }
        }
        res
    }

    fn mod_pow(base: &Self, exp_be: &[u8], m: &Self) -> Self {
        let mut result = Self::one();
        let mut base_mod = base.clone();
        while base_mod.cmp_abs(m) != std::cmp::Ordering::Less {
            base_mod.sub_assign(m);
        }
        for &byte in exp_be {
            for bit in (0..8).rev() {
                result = Self::mul_mod(&result, &result, m);
                if (byte >> bit) & 1 == 1 {
                    result = Self::mul_mod(&result, &base_mod, m);
                }
            }
        }
        result
    }
}

fn decode_hex_bytes(hex: &str) -> Result<Vec<u8>> {
    let cleaned = hex.trim().trim_start_matches("0x");
    let normalized = if cleaned.len() % 2 == 1 {
        format!("0{cleaned}")
    } else {
        cleaned.to_string()
    };
    let mut out = Vec::with_capacity(normalized.len() / 2);
    for i in (0..normalized.len()).step_by(2) {
        let b = u8::from_str_radix(&normalized[i..i + 2], 16)
            .with_context(|| format!("非法十六进制字符串：{}", &normalized[i..i + 2]))?;
        out.push(b);
    }
    Ok(out)
}

fn strip_leading_zero_bytes(bytes: &[u8]) -> &[u8] {
    let first_non_zero = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
    &bytes[first_non_zero..]
}

fn random_hex_32() -> String {
    let mut s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(1) as u64
        ^ ((std::process::id() as u64) << 16);
    let mut out = String::with_capacity(32);
    for _ in 0..8 {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        use std::fmt::Write;
        let _ = write!(&mut out, "{:04x}", (s & 0xffff) as u16);
    }
    out
}

fn parse_listen_v2_response(value: &Value, requested_tone: &str) -> Option<MiguStreamResolved> {
    let data = value.get("data")?;
    let url = data
        .get("url")
        .or_else(|| data.pointer("/song/url"))
        .and_then(Value::as_str)?
        .trim()
        .to_string();
    if url.is_empty() {
        return None;
    }
    let format_type = data
        .get("audioFormatType")
        .and_then(Value::as_str)
        .unwrap_or(requested_tone)
        .trim()
        .to_ascii_uppercase();
    let (ext, quality_label) = match format_type.as_str() {
        "ZQ24" => ("flac", "臻品母带 24bit"),
        "SQ" => ("flac", "无损 FLAC"),
        "HQ" => ("mp3", "极高 320k"),
        "PQ" => ("mp3", "标准 128k"),
        "LQ" => ("mp3", "流畅 64k"),
        _ => {
            if url.to_ascii_lowercase().contains(".flac") {
                ("flac", "无损 FLAC")
            } else {
                ("mp3", "极高 320k")
            }
        }
    };
    let expected_size = data
        .get("audioSize")
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
        })
        .unwrap_or(0);
    let is_preview = data
        .get("isAudition")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || data
            .get("auditionUrl")
            .and_then(Value::as_str)
            .is_some_and(|aud| aud.trim() == url);
    Some(MiguStreamResolved {
        url,
        ext,
        quality_label: quality_label.to_string(),
        expected_size,
        is_preview,
    })
}

fn detect_audio_extension(bytes: &[u8], fallback: &'static str) -> &'static str {
    if bytes.len() >= 4 && &bytes[..4] == b"fLaC" {
        "flac"
    } else if bytes.len() >= 8 && &bytes[4..8] == b"ftyp" {
        "m4a"
    } else if bytes.len() >= 3
        && (&bytes[..3] == b"ID3" || (bytes[0] == 0xff && (bytes[1] & 0xe0) == 0xe0))
    {
        "mp3"
    } else {
        fallback
    }
}

fn unique_part_name(track_id: &str, tag: &str, ext: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static PART_SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = PART_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{track_id}-{tag}-{}-{seq}.{ext}.part", std::process::id())
}

fn ensure_migu_ok(value: &Value, context_msg: &str) -> Result<()> {
    let code = value
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if code == "000000" || code.is_empty() {
        return Ok(());
    }
    let info = str_field(value, &["info", "msg", "message"]);
    anyhow::bail!("{context_msg}（{code}: {info}）")
}

fn parse_migu_song(raw: &Value) -> Option<TrackItem> {
    let song = raw
        .get("song")
        .or_else(|| raw.get("songItem"))
        .unwrap_or(raw);
    let content_id = str_field(song, &["contentId", "resId", "songId"]);
    if content_id.is_empty() {
        return None;
    }
    let copyright_id = str_field(song, &["copyrightId", "ringCopyrightId"]);
    let title = str_field(song, &["songName", "name", "title", "txt"]);
    if title.is_empty() {
        return None;
    }

    let (artist, artist_id) = extract_artists(song);
    let album = str_field(song, &["album", "albumName", "txt3"]).if_empty_then(|| {
        song.pointer("/albums/0/name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    });
    let raw_album_id = str_field(song, &["albumId"]).if_empty_then(|| {
        song.pointer("/albums/0/id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    });
    let cover = extract_song_cover(song);
    let duration_seconds = parse_duration_seconds(song);
    let vip = is_song_vip(song);

    Some(TrackItem {
        id: encode_track_id(&content_id, &copyright_id),
        title,
        artist,
        album,
        artist_id: encode_entity_id(&artist_id),
        album_id: encode_entity_id(&raw_album_id),
        cover,
        duration_seconds,
        vip,
    })
}

fn extract_artists(song: &Value) -> (String, String) {
    for key in ["singerList", "singers", "artists"] {
        if let Some(arr) = song.get(key).and_then(Value::as_array) {
            let names: Vec<String> = arr
                .iter()
                .filter_map(|item| {
                    let s = str_field(item, &["name", "singer", "singerName"]);
                    (!s.is_empty()).then_some(s)
                })
                .collect();
            let first_id = arr
                .first()
                .map(|item| str_field(item, &["id", "singerId"]))
                .unwrap_or_default();
            if !names.is_empty() {
                return (names.join(" / "), first_id);
            }
        }
    }
    let fallback_name = str_field(song, &["singer", "singerName", "txt2"]);
    let fallback_id = str_field(song, &["singerId"]);
    (fallback_name, fallback_id)
}

fn extract_song_cover(song: &Value) -> String {
    let direct = str_field(song, &["img3", "img2", "img1", "img", "imgUrl", "imageUrl"]);
    if !direct.is_empty() {
        return normalize_img_url(&direct);
    }
    if let Some(s) = song
        .pointer("/songPic/lPic")
        .or_else(|| song.pointer("/songPic/mPic"))
        .or_else(|| song.pointer("/songPic/sPic"))
        .and_then(Value::as_str)
    {
        if !s.trim().is_empty() {
            return normalize_img_url(s);
        }
    }
    if let Some(img) = extract_img_from_array(song.get("imgItems"))
        .or_else(|| extract_img_from_array(song.get("albumImgs")))
    {
        return normalize_img_url(&img);
    }
    String::new()
}

fn extract_img_from_array(val: Option<&Value>) -> Option<String> {
    let arr = val?.as_array()?;
    for preferred in ["02", "03", "01"] {
        for item in arr {
            if item.get("imgSizeType").and_then(Value::as_str) == Some(preferred) {
                let s = str_field(item, &["img", "imgUrl", "url"]);
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }
    for item in arr {
        let s = str_field(item, &["img", "imgUrl", "url"]);
        if !s.is_empty() {
            return Some(s);
        }
    }
    None
}

fn parse_duration_seconds(song: &Value) -> i64 {
    if let Some(val) = song.get("duration").or_else(|| song.get("length")) {
        if let Some(n) = val.as_i64() {
            return n.max(0);
        }
        if let Some(f) = val.as_f64() {
            return f.round().max(0.0) as i64;
        }
        if let Some(s) = val.as_str() {
            let trimmed = s.trim();
            if let Ok(n) = trimmed.parse::<i64>() {
                return n.max(0);
            }
            if let Some((mm, ss)) = trimmed.split_once(':') {
                if let (Ok(m), Ok(sec)) = (mm.trim().parse::<i64>(), ss.trim().parse::<i64>()) {
                    return (m * 60 + sec).max(0);
                }
            }
        }
    }
    0
}

fn is_song_vip(song: &Value) -> bool {
    if matches!(
        str_field(song, &["vipFlag", "vip"])
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "vip"
    ) || int_field(song, &["vipType", "restrictType"]) > 0
    {
        return true;
    }
    if let Some(tags) = song
        .get("showTags")
        .or_else(|| song.get("showTag"))
        .and_then(Value::as_array)
    {
        if tags.iter().any(|t| {
            t.as_str()
                .is_some_and(|s| s.eq_ignore_ascii_case("vip") || s.eq_ignore_ascii_case("tl-vip"))
        }) {
            return true;
        }
    }
    if let Some(formats) = song.get("audioFormats").and_then(Value::as_array) {
        return formats.iter().any(|fmt| {
            fmt.get("showTagsNew")
                .or_else(|| fmt.get("showTags"))
                .and_then(Value::as_array)
                .is_some_and(|tags| {
                    tags.iter()
                        .any(|t| t.as_str().is_some_and(|s| s.contains("vip")))
                })
        });
    }
    false
}

fn parse_migu_playlist(pl: &Value) -> Option<PlaylistItem> {
    let raw_id = str_field(pl, &["musicListId", "playListId", "id", "resId"]);
    if raw_id.is_empty() {
        return None;
    }
    let title = str_field(pl, &["title", "musicListName", "name", "txt"]);
    let cover = pl
        .pointer("/imgItem/img")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| str_field(pl, &["originalImgUrl", "img", "imageUrl", "cover"]));
    Some(PlaylistItem {
        id: encode_entity_id(&raw_id),
        title: if title.is_empty() {
            "咪咕歌单".to_string()
        } else {
            title
        },
        cover: normalize_img_url(&cover),
        track_count: int_field(pl, &["musicNum", "songCount", "totalCount"]),
        creator: str_field(pl, &["ownerName", "userName", "creator"]),
    })
}

fn extract_playlists_from_value(value: &Value) -> Vec<PlaylistItem> {
    let mut out = Vec::new();
    for pointer in [
        "/data/list",
        "/data/playLists",
        "/data/items",
        "/data/collectionList",
        "/data",
    ] {
        if let Some(arr) = value.pointer(pointer).and_then(Value::as_array) {
            for item in arr {
                let inner = item
                    .get("musicList")
                    .or_else(|| item.get("playlist"))
                    .unwrap_or(item);
                if let Some(pl) = parse_migu_playlist(inner) {
                    out.push(pl);
                }
            }
            if !out.is_empty() {
                break;
            }
        }
    }
    out
}

fn extract_tracks_from_list_value(value: &Value) -> Vec<TrackItem> {
    let mut out = Vec::new();
    for pointer in [
        "/data/songItemList",
        "/data/songList",
        "/data/songItems",
        "/data/list",
        "/data/items",
        "/songList",
        "/data",
    ] {
        if let Some(arr) = value.pointer(pointer).and_then(Value::as_array) {
            for item in arr {
                if let Some(track) = parse_migu_song(item) {
                    out.push(track);
                }
            }
            if !out.is_empty() {
                break;
            }
        }
    }
    out
}

fn extract_singer_page_tracks(
    value: &Value,
    raw_artist_id: &str,
) -> (Vec<TrackItem>, String, String) {
    let mut tracks = Vec::new();
    let mut artist_name = String::new();
    let mut artist_avatar = String::new();

    for block in value
        .pointer("/data/contents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for item in block
            .get("contents")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(song_item) = item.get("songItem") {
                if artist_avatar.is_empty() {
                    if let Some(singers) = song_item.get("singerList").and_then(Value::as_array) {
                        for s in singers {
                            if str_field(s, &["id"]) == raw_artist_id || artist_name.is_empty() {
                                let name = str_field(s, &["name"]);
                                let img = normalize_img_url(&str_field(s, &["img"]));
                                if !name.is_empty() {
                                    artist_name = name;
                                }
                                if !img.is_empty() {
                                    artist_avatar = img;
                                }
                            }
                        }
                    }
                }
                if let Some(track) = parse_migu_song(song_item) {
                    tracks.push(track);
                }
            } else if item.get("resType").and_then(Value::as_str) == Some("4001")
                || item.get("songId").is_some()
            {
                if let Some(track) = parse_migu_song(item) {
                    tracks.push(track);
                }
            }
        }
    }

    (tracks, artist_name, artist_avatar)
}

fn build_scoped_search_results(
    keyword: String,
    tracks: Vec<TrackItem>,
    artists: Vec<ArtistItem>,
    albums: Vec<AlbumItem>,
    playlists: Vec<PlaylistItem>,
) -> SearchResults {
    let mut results = SearchResults {
        keyword,
        tracks: tracks.clone(),
        artists: artists.clone(),
        albums: albums.clone(),
        playlists: playlists.clone(),
        ..Default::default()
    };
    let mut groups = Vec::new();
    if !tracks.is_empty() {
        groups.push(SearchGroup {
            id: "tracks".to_string(),
            title: "歌曲".to_string(),
            can_view_all: false,
            entries: tracks.into_iter().map(SearchEntry::Track).collect(),
        });
    }
    if !artists.is_empty() {
        groups.push(SearchGroup {
            id: "artists".to_string(),
            title: "音乐人".to_string(),
            can_view_all: false,
            entries: artists.into_iter().map(SearchEntry::Artist).collect(),
        });
    }
    if !albums.is_empty() {
        groups.push(SearchGroup {
            id: "albums".to_string(),
            title: "专辑".to_string(),
            can_view_all: false,
            entries: albums.into_iter().map(SearchEntry::Album).collect(),
        });
    }
    if !playlists.is_empty() {
        groups.push(SearchGroup {
            id: "playlists".to_string(),
            title: "歌单".to_string(),
            can_view_all: false,
            entries: playlists.into_iter().map(SearchEntry::Playlist).collect(),
        });
    }
    results.groups = groups;
    results
}

fn str_field(value: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(v) = value.get(*key) {
            if let Some(s) = v.as_str() {
                let t = s.trim();
                if !t.is_empty() {
                    return t.to_string();
                }
            } else if let Some(n) = v.as_i64() {
                return n.to_string();
            }
        }
    }
    String::new()
}

fn int_field(value: &Value, keys: &[&str]) -> i64 {
    for key in keys {
        if let Some(v) = value.get(*key) {
            if let Some(n) = v.as_i64() {
                return n;
            }
            if let Some(s) = v.as_str() {
                if let Ok(n) = s.trim().parse::<i64>() {
                    return n;
                }
            }
        }
    }
    0
}

fn parse_ymd_timestamp(date_str: &str) -> i64 {
    let clean = date_str.trim().split(' ').next().unwrap_or_default();
    let mut parts = clean.split('-');
    let Ok(year) = parts.next().unwrap_or_default().parse::<i64>() else {
        return 0;
    };
    let Ok(month) = parts.next().unwrap_or_default().parse::<i64>() else {
        return 0;
    };
    let Ok(day) = parts.next().unwrap_or_default().parse::<i64>() else {
        return 0;
    };
    if year < 1970 || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return 0;
    }
    // 简化格里高利历天数转 UNIX 秒数（仅用于年份/日期展示）
    let (y, m) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    days * 86400
}

trait StringExt {
    fn if_empty_then<F: FnOnce() -> String>(self, fallback: F) -> String;
}

impl StringExt for String {
    fn if_empty_then<F: FnOnce() -> String>(self, fallback: F) -> String {
        if self.trim().is_empty() {
            fallback()
        } else {
            self
        }
    }
}

pub fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(*byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(&mut out, "%{byte:02X}");
        }
    }
    out
}

fn form_encode(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MusicProvider;

    #[test]
    fn decode_migu_bytes_decrypts_binary_frame() {
        let plain = br#"{"code":"000000","data":{"url":"https://example.com/song.flac"}}"#;
        let offset = 73u8;
        let mut frame = vec![171u8, 205u8, 1u8, offset];
        for (idx, &b) in plain.iter().enumerate() {
            frame.push(
                b.wrapping_add(MIGU_SECURE_KEY[idx % MIGU_SECURE_KEY.len()])
                    .wrapping_sub(offset),
            );
        }
        let decoded = decode_migu_bytes(&frame).expect("decode binary frame");
        assert_eq!(decoded["code"], "000000");
        assert_eq!(decoded["data"]["url"], "https://example.com/song.flac");
    }

    #[test]
    fn decrypt_mrc_and_convert_to_enhanced_lrc() {
        // 加密构造一段合法 UTF-16LE MRC 明文验证 TEA 解密环
        let sample =
            "[0,0]起(0,0)风(0,0)了\n[9272,850]吉他(9272,200)：Kouichi (Ikekubo)(9472,200)\n";
        let mut utf16_bytes: Vec<u8> = sample
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        while utf16_bytes.len() < MRC_MIN_LENGTH || !utf16_bytes.len().is_multiple_of(8) {
            utf16_bytes.push(b'0');
        }
        let mut data: Vec<i64> = utf16_bytes
            .chunks_exact(8)
            .map(|c| i64::from_le_bytes(c.try_into().unwrap()))
            .collect();

        let mut padded_key = MRC_KEY_RAW.to_vec();
        while padded_key.len() < MRC_MIN_LENGTH {
            padded_key.push(b'0');
        }
        let mut key_bytes = vec![0u8; padded_key.len() * 2];
        for (i, &b) in padded_key.iter().enumerate() {
            key_bytes[i * 2] = b;
        }
        let key: Vec<i64> = (0..key_bytes.len().div_ceil(8))
            .map(|i| {
                let mut buf = [0u8; 8];
                let start = i * 8;
                let end = (start + 8).min(key_bytes.len());
                buf[..end - start].copy_from_slice(&key_bytes[start..end]);
                i64::from_le_bytes(buf)
            })
            .collect();

        let n = data.len();
        let rounds = 6 + 52 / n;
        let mut z = data[n - 1];
        let mut sum: i64 = 0;
        for _ in 0..rounds {
            sum = sum.wrapping_add(MRC_TEA_DELTA);
            let e = 3 & (sum >> 2);
            for p in 0..n - 1 {
                let y = data[p + 1];
                let key_idx = ((p as i64 & 3) ^ e) as usize % key.len();
                let lhs = (y ^ sum).wrapping_add(z ^ key[key_idx]);
                let rhs = ((z >> 5) ^ y.wrapping_shl(2)).wrapping_add((y >> 3) ^ z.wrapping_shl(4));
                z = data[p].wrapping_add(lhs ^ rhs);
                data[p] = z;
            }
            let y = data[0];
            let key_idx = ((((n - 1) as i64) & 3) ^ e) as usize % key.len();
            let lhs = (key[key_idx] ^ z).wrapping_add(y ^ sum);
            let rhs = ((z >> 5) ^ y.wrapping_shl(2)).wrapping_add((y >> 3) ^ z.wrapping_shl(4));
            z = data[n - 1].wrapping_add(lhs ^ rhs);
            data[n - 1] = z;
        }
        let hex: String = data.iter().map(|w| format!("{:016x}", *w as u64)).collect();
        let decrypted = decrypt_mrc(&hex).expect("decrypt");
        assert!(decrypted.contains("Kouichi (Ikekubo)"), "{decrypted}");

        let enhanced = mrc_to_enhanced_lrc(&decrypted);
        let lines = parse_lrc(&enhanced);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "起风了");
        assert_eq!(lines[1].text, "吉他：Kouichi (Ikekubo)");
        assert_eq!(lines[1].words.len(), 2);
        assert_eq!(lines[1].words[1].text, "：Kouichi (Ikekubo)");
    }

    #[test]
    fn migu_track_id_round_trip_and_cache_compatibility() {
        let id = encode_track_id("600913000002601091", "63372200381");
        assert_eq!(id, "migu_600913000002601091_63372200381");
        assert_eq!(MusicProvider::from_track_id(&id), MusicProvider::Migu);
        assert_eq!(
            parse_track_id(&id),
            Some(("600913000002601091".into(), "63372200381".into()))
        );
        assert!(!id.contains(':'));
        assert!(!id.contains('-'));
    }

    #[test]
    fn parse_migu_credentials_handles_curl_cookie_and_raw_token() {
        let curl = r#"curl 'https://app.c.nf.migu.cn/pc/user/h5/queryUserInfo/v1.0' -H 'cookie: foo=1; pacmtoken=ABC_123; bar=2'"#;
        let (token, cookie) = parse_migu_credentials(curl);
        assert_eq!(token, "ABC_123");
        assert_eq!(cookie, "foo=1; pacmtoken=ABC_123; bar=2");

        let (token2, cookie2) = parse_migu_credentials("Cookie: pacmtoken=XYZ789; a=b");
        assert_eq!(token2, "XYZ789");
        assert_eq!(cookie2, "pacmtoken=XYZ789; a=b");

        let (token3, cookie3) = parse_migu_credentials("PLAIN_TOKEN_ONLY");
        assert_eq!(token3, "PLAIN_TOKEN_ONLY");
        assert_eq!(cookie3, "pacmtoken=PLAIN_TOKEN_ONLY");
    }

    #[test]
    fn rsa_pkcs1_v15_encrypts_and_matches_small_known_key() {
        // 160-bit 仅用于纯算法正确性校验的玩具 RSA 参数（非任何业务密钥）：
        // p = 1152921504606847009, q = 1152921504606847089 (primes)
        // n = p*q = 1329227995784915964804646179565134801 -> hex: 010000000000000600000000000006d1? Let's compute with small 128-bit n.
        // Using known p=0x10000000000000003? Let's verify e=0x010001 on p=4294967311, q=4294967357 => n=18446744348573223427 (64 bits, k=8? Too short for 11+1).
        // Let's use 128-bit primes p = 18446744073709551629, q = 18446744073709551667 => n = 0x1000000000000003700000000000002d3?
        // Actually exact product: (2^64 + 13) * (2^64 + 51) = 2^128 + 64 * 2^64 + 663
        // In hex (33 digits? 2^128 is bit 128 -> 33 hex digits = 17 bytes, wait k=17 allows msg len <= 6!).
        // n_hex = "0100000000000000400000000000000297"
        let n_hex = "0100000000000000400000000000000297";
        let e_hex = "010001";
        let cipher_hex = rsa_encrypt_pkcs1_v15_hex(b"13800", n_hex, e_hex).expect("rsa encrypt");
        let n = BigIntLimbs::from_be_bytes(&decode_hex_bytes(n_hex).unwrap());
        let c = BigIntLimbs::from_be_bytes(&decode_hex_bytes(&cipher_hex).unwrap());
        assert_eq!(c.cmp_abs(&n), std::cmp::Ordering::Less);
        assert!(cipher_hex.len() >= 30);
    }
}
