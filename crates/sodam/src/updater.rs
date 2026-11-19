//! GitHub Release updater for the user's fork, not upstream SodaM.
//! Downloads signed-transport release assets, verifies SHA256SUMS and Debian
//! package metadata, then delegates installation to polkit + apt-get.

use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const REPOSITORY: &str = "lbg2002/sodam";
const LATEST_API: &str = "https://api.github.com/repos/lbg2002/sodam/releases/latest";
const MAX_INFO_SIZE: u64 = 1024 * 1024;
const MAX_PACKAGE_SIZE: u64 = 350 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct UpdateRelease {
    pub tag: String,
    pub filename: String,
    package_url: String,
    checksum_url: String,
    arch: &'static str,
}

fn version_number(text: &str) -> Option<[u64; 3]> {
    let mut items = text.strip_prefix('v').unwrap_or(text).split('.');
    let version = [
        items.next()?.parse().ok()?,
        items.next()?.parse().ok()?,
        items.next()?.parse().ok()?,
    ];
    items.next().is_none().then_some(version)
}

fn deb_arch() -> Result<&'static str> {
    if !cfg!(target_os = "linux") {
        bail!("该平台暂不支持应用内安装，请手动访问 Releases");
    }
    match std::env::consts::ARCH {
        "x86_64" => Ok("amd64"),
        "aarch64" => Ok("arm64"),
        _ => bail!("当前 CPU 架构没有对应安装包"),
    }
}

fn get_bytes(url: &str, maximum: u64, timeout: Duration) -> Result<Vec<u8>> {
    if !url.starts_with("https://") {
        bail!("更新必须使用 HTTPS");
    }
    let reply = ureq::get(url)
        .set("User-Agent", "SodaM-Update-Checker")
        .timeout(timeout)
        .call()
        .map_err(|_| anyhow!("GitHub 更新服务不可用"))?;
    let mut bytes = Vec::new();
    reply.into_reader()
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .context("读取 GitHub 更新失败")?;
    if bytes.len() as u64 > maximum {
        bail!("更新内容超过最大限制");
    }
    Ok(bytes)
}

fn release_asset_url(tag: &str, name: &str) -> String {
    format!("https://github.com/{REPOSITORY}/releases/download/{tag}/{name}")
}

pub(crate) fn check_latest() -> Result<Option<UpdateRelease>> {
    let arch = deb_arch()?;
    let data = get_bytes(LATEST_API, MAX_INFO_SIZE, Duration::from_secs(20))?;
    let release: serde_json::Value = serde_json::from_slice(&data)
        .context("Release 信息不是合法 JSON")?;
    let tag = release["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow!("没有可用的 Release 版本号"))?;
    let latest = version_number(tag).ok_or_else(|| anyhow!("不支持的 Release 版本号"))?;
    let local = version_number(env!("CARGO_PKG_VERSION"))
        .ok_or_else(|| anyhow!("当前软件版本号无效"))?;
    if latest <= local {
        return Ok(None);
    }
    let version = tag.strip_prefix('v').unwrap_or(tag);
    let filename = format!("sodam_{version}_{arch}.deb");
    let assets = release["assets"]
        .as_array()
        .ok_or_else(|| anyhow!("Release 缺少安装包列表"))?;
    let asset = |name: &str| -> Result<String> {
        let url = release_asset_url(tag, name);
        if !assets.iter().any(|entry| {
            entry["name"].as_str() == Some(name)
                && entry["browser_download_url"].as_str() == Some(url.as_str())
        }) {
            bail!("Release 缺少可信安装包或校验文件：{name}");
        }
        Ok(url)
    };
    Ok(Some(UpdateRelease {
        package_url: asset(&filename)?,
        checksum_url: asset("SHA256SUMS")?,
        filename,
        tag: tag.into(),
        arch,
    }))
}

fn expected_sha256(text: &str, filename: &str) -> Result<String> {
    let mut lines = text.lines().filter_map(|line| {
        let mut split = line.split_whitespace();
        let hash = split.next()?;
        let name = split.next()?;
        (name.trim_start_matches('*') == filename && split.next().is_none()).then_some(hash)
    });
    let sha = lines.next().ok_or_else(|| anyhow!("SHA256SUMS 缺少安装包记录"))?;
    if lines.next().is_some()
        || sha.len() != 64
        || !sha.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("SHA256SUMS 格式错误或重复");
    }
    Ok(sha.to_ascii_lowercase())
}

fn deb_field(path: &Path, field: &str) -> Result<String> {
    let output = Command::new("dpkg-deb")
        .arg("--field")
        .arg(path)
        .arg(field)
        .output()
        .context("缺少 dpkg-deb，无法安装 Debian 软件包")?;
    if !output.status.success() {
        bail!("安装包字段校验失败");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn require_deb_install() -> Result<()> {
    let output = Command::new("dpkg-query")
        .args(["-W", "-f=${Status}", "sodam"])
        .output()
        .context("无法读取已安装软件包信息")?;
    if !output.status.success()
        || String::from_utf8_lossy(&output.stdout).trim() != "install ok installed"
    {
        bail!("软件不是通过 deb 安装，请使用 GitHub Releases 手动更新");
    }
    Ok(())
}

pub(crate) fn install(release: &UpdateRelease) -> Result<()> {
    require_deb_install()?;
    let manifest = get_bytes(&release.checksum_url, MAX_INFO_SIZE, Duration::from_secs(30))?;
    let manifest = String::from_utf8(manifest).context("SHA256SUMS 文本无效")?;
    let expected = expected_sha256(&manifest, &release.filename)?;
    let temp = tempfile::Builder::new().prefix("sodam-update-").tempdir()?;
    let path = temp.path().join(&release.filename);
    let mut output = File::create(&path)?;
    let reply = ureq::get(&release.package_url)
        .set("User-Agent", "SodaM-Update-Checker")
        .timeout(Duration::from_secs(300))
        .call()
        .map_err(|_| anyhow!("下载安装包失败"))?;
    if reply
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .is_some_and(|size| size > MAX_PACKAGE_SIZE)
    {
        bail!("安装包超过大小限制");
    }
    let mut input = reply.into_reader();
    let mut count = 0u64;
    let mut checksum = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let read = input.read(&mut buffer).context("下载过程中读取失败")?;
        if read == 0 {
            break;
        }
        count += read as u64;
        if count > MAX_PACKAGE_SIZE {
            bail!("安装包超过大小限制");
        }
        checksum.update(&buffer[..read]);
        output.write_all(&buffer[..read])?;
    }
    output.sync_all()?;
    drop(output);
    if count == 0 || format!("{:x}", checksum.finalize()) != expected {
        bail!("SHA-256 校验不通过，拒绝安装");
    }
    let version = release.tag.strip_prefix('v').unwrap_or(&release.tag);
    if deb_field(&path, "Package")? != "sodam"
        || deb_field(&path, "Version")? != version
        || deb_field(&path, "Architecture")? != release.arch
    {
        bail!("安装包的包名、版本或架构不符合要求");
    }
    // Explicit user click, then desktop polkit authorization; never run a shell.
    let exit = Command::new("pkexec")
        .arg("/usr/bin/apt-get")
        .arg("install")
        .arg("-y")
        .arg("--")
        .arg(&path)
        .status()
        .context("无法启动 polkit 授权安装，请检查 pkexec")?;
    if !exit.success() {
        bail!("更新未安装：授权取消、依赖冲突或 apt 安装失败");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_version_comparison() {
        assert_eq!(version_number("v0.1.23"), Some([0, 1, 23]));
        assert!(version_number("v0.4.0") > version_number("0.1.22"));
        assert_eq!(version_number("v0.4.0-rc"), None);
    }

    #[test]
    fn checksum_validation_is_exact_and_unique() {
        let hash = "a".repeat(64);
        let filename = "sodam_0.1.23_amd64.deb";
        let manifest = format!("{hash}  {filename}\n{hash}  other.deb\n");
        assert_eq!(expected_sha256(&manifest, filename).unwrap(), hash);
        assert!(expected_sha256(&manifest, "sodam_0.1.23_arm64.deb").is_err());
        assert!(expected_sha256(&format!("{manifest}{hash}  {filename}\n"), filename).is_err());
    }

    #[test]
    fn downloads_are_scoped_to_custom_fork() {
        assert_eq!(
            release_asset_url("v0.1.23", "SHA256SUMS"),
            "https://github.com/lbg2002/sodam/releases/download/v0.1.23/SHA256SUMS"
        );
    }
}
