//! libmssdk HTTP 签名适配层：保留当前 libresoda 版本，兼容 v0.2.0 的错误契约。
//!
//! 只重试明确可重试的签名请求（429 / 503）；不重试汽水接口或音频下载。

use libresoda::soda::signature::{HttpSignature, SignRequest, SignResponse, SignatureProvider};
use libresoda::soda::types::DownloadInfo;
use libresoda::{Result, SodaError};
use std::cell::RefCell;
use std::io::Read;
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(6);
const MAX_RETRIES: usize = 2;
const MAX_TOTAL_WAIT: Duration = Duration::from_secs(5);
const MAX_RESPONSE_BYTES: u64 = 64 * 1024;

// 旧版 libresoda 的 apply_stream_signature 会吞掉提供者错误，再尝试未签名请求。
// 同步取流调用使用线程局部记录，避免并发预取之间串错，也不记录请求凭据。
thread_local! {
    static LAST_SIGNER_FAILURE: RefCell<Option<SodaError>> = const { RefCell::new(None) };
}

/// 若签名失败导致取流失败/回退试听，不让临时故障变成长期命中的试听缓存。
/// 仍允许成功获得的完整 Web 流，也保留无签名失败时正常的试听回退。
pub(crate) fn resolve_download_info(
    operation: impl FnOnce() -> Result<DownloadInfo>,
) -> Result<DownloadInfo> {
    let previous = LAST_SIGNER_FAILURE.with(|slot| slot.replace(None));
    let result = operation();
    let failure = LAST_SIGNER_FAILURE.with(|slot| slot.replace(previous));
    match (result, failure) {
        (Ok(info), Some(error)) if info.is_preview => Err(error),
        (Err(_), Some(error)) => Err(error),
        (result, _) => result,
    }
}

pub(crate) struct CompatHttpSignature {
    url: String,
    token: String,
    agent: ureq::Agent,
}

impl CompatHttpSignature {
    pub(crate) fn new(url: &str, token: &str) -> Self {
        Self {
            url: url.to_owned(),
            token: token.to_owned(),
            // 签名请求带有敏感输入；要求明确配置最终 /sign 地址，不向重定向目标转发。
            agent: ureq::AgentBuilder::new().redirects(0).build(),
        }
    }

    fn sign_inner(&self, request: &SignRequest) -> Result<SignResponse> {
        let payload = serde_json::to_vec(request)
            .map_err(|_| SodaError::json("signer error: 请求编码失败"))?;
        let mut waited = Duration::ZERO;
        for attempt in 0..=MAX_RETRIES {
            let mut http = self
                .agent
                .post(&self.url)
                .timeout(REQUEST_TIMEOUT)
                .set("Content-Type", "application/json; charset=utf-8")
                .set("Accept", "application/json");
            if !self.token.is_empty() {
                http = http.set("Authorization", &format!("Bearer {}", self.token));
            }

            let response = match http.send_bytes(&payload) {
                Ok(response) | Err(ureq::Error::Status(_, response)) => response,
                // 不输出 URL、请求体、响应体或原始传输错误，避免泄漏 Cookie/Token。
                Err(_) => return Err(SodaError::http("signer error: 签名服务连接失败或超时")),
            };
            let status = response.status();
            let retry_header = response.header("Retry-After").map(str::to_owned);
            let mut body = Vec::new();
            let read_ok = response
                .into_reader()
                .take(MAX_RESPONSE_BYTES + 1)
                .read_to_end(&mut body)
                .is_ok();
            let complete = read_ok && body.len() as u64 <= MAX_RESPONSE_BYTES;

            if status == 429 || status == 503 {
                let delay = retry_delay(
                    retry_header.as_deref(),
                    if complete { &body } else { &[] },
                );
                // 服务要求的等待超过剩余预算时，直接返回；绝不截短等待后提前重试。
                if attempt == MAX_RETRIES || delay > MAX_TOTAL_WAIT.saturating_sub(waited) {
                    return Err(status_error(status, Some(delay)));
                }
                std::thread::sleep(delay);
                waited += delay;
                continue;
            }

            if !(200..300).contains(&status) {
                return Err(status_error(status, None));
            }
            if !complete {
                return Err(SodaError::http("signer error: 签名响应读取失败或超过大小限制"));
            }

            // 兼容旧签名器的 HTTP 200 + ok:false，且不回显服务端自由文本。
            let value: serde_json::Value = serde_json::from_slice(&body)
                .map_err(|_| SodaError::json("signer error: 签名响应不是有效 JSON"))?;
            if value.get("ok").and_then(|item| item.as_bool()) == Some(false) {
                let code = value.get("code").and_then(|item| item.as_str()).unwrap_or("");
                return Err(if code.eq_ignore_ascii_case("UNAUTHORIZED") {
                    status_error(401, None)
                } else {
                    SodaError::http("signer error: 签名服务返回失败")
                });
            }
            // 复用固定版本已有的签名回包解析，兼容结构体及旧的扁平形式。
            let signed = HttpSignature::parse_response(&body)
                .map_err(|_| SodaError::json("signer error: 签名响应格式无效"))?;
            if signed.is_empty() {
                return Err(SodaError::http("signer error: 签名服务返回空签名"));
            }
            return Ok(signed);
        }
        // 循环必然在最后一次请求结束时返回。
        Err(SodaError::http("signer error: 签名重试次数已耗尽"))
    }
}

impl SignatureProvider for CompatHttpSignature {
    fn sign(&self, request: &SignRequest) -> Result<SignResponse> {
        let result = self.sign_inner(request);
        LAST_SIGNER_FAILURE.with(|slot| {
            slot.replace(result.as_ref().err().cloned());
        });
        result
    }

    fn name(&self) -> &'static str {
        "http"
    }
}

/// v0.2.0 同时在头和 JSON 中发送整数秒；取较大者避免提前请求。
/// 缺失或非法值保守回退为 1 秒（旧版/代理也可能只提供状态码）。
fn retry_delay(header: Option<&str>, body: &[u8]) -> Duration {
    let header_seconds = header.and_then(|value| value.trim().parse::<u64>().ok());
    let body_seconds = serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            let field = value.get("retry_after_seconds")?;
            field.as_u64().or_else(|| field.as_str()?.trim().parse().ok())
        });
    Duration::from_secs(header_seconds.into_iter().chain(body_seconds).max().unwrap_or(1).max(1))
}

fn status_error(status: u16, delay: Option<Duration>) -> SodaError {
    let reason = match status {
        400 => "INVALID_REQUEST：签名请求格式错误",
        401 | 403 => "UNAUTHORIZED：请检查签名服务 Bearer Token",
        404 | 410 => "签名端点不可用，请确认配置的是最终 /sign 地址",
        429 => "RATE_LIMITED：签名请求过于频繁",
        503 => "BUSY：签名服务暂时繁忙或尚未就绪",
        500 => "签名服务内部故障或配置错误，请检查服务日志",
        502 => "签名服务或网关故障（旧版服务可能使用此状态码）",
        _ => "签名服务返回非成功状态",
    };
    let wait = delay
        .map(|duration| format!("；服务要求等待 {} 秒后再试", duration.as_secs()))
        .unwrap_or_default();
    SodaError::http(format!("signer error: HTTP {status} {reason}{wait}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    // 本机模拟 HTTP，不请求真实签名器/汽水服务，也不使用真实凭据。
    fn mock_server(
        replies: Vec<(u16, Option<&'static str>, &'static str)>,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/sign", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            for (status, retry, body) in replies {
                let deadline = std::time::Instant::now() + Duration::from_secs(8);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(std::time::Instant::now() < deadline, "missing signer request");
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(err) => panic!("mock accept: {err}"),
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                stream.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut reader = BufReader::new(&mut stream);
                let mut length = 0;
                let mut authorization = false;
                loop {
                    let mut line = String::new();
                    assert!(reader.read_line(&mut line).unwrap() > 0);
                    if line == "\r\n" {
                        break;
                    }
                    if let Some((name, value)) = line.split_once(':') {
                        if name.eq_ignore_ascii_case("content-length") {
                            length = value.trim().parse::<usize>().unwrap();
                        }
                        if name.eq_ignore_ascii_case("authorization") {
                            authorization = value.trim() == "Bearer test-token";
                        }
                    }
                }
                assert!(authorization);
                let mut payload = vec![0; length];
                reader.read_exact(&mut payload).unwrap();
                let _: SignRequest = serde_json::from_slice(&payload).unwrap();
                drop(reader);
                let retry = retry.map(|value| format!("Retry-After: {value}\r\n")).unwrap_or_default();
                write!(
                    stream,
                    "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{retry}\r\n{body}",
                    body.len()
                ).unwrap();
            }
        });
        (url, worker)
    }

    fn request() -> SignRequest {
        SignRequest {
            url: "https://example.invalid/track".into(),
            method: "POST".into(),
            ..Default::default()
        }
    }

    #[test]
    fn retry_after_uses_header_and_body_and_never_zero() {
        assert_eq!(retry_delay(Some("2"), br#"{"retry_after_seconds":3}"#), Duration::from_secs(3));
        assert_eq!(retry_delay(None, br#"{"retry_after_seconds":"4"}"#), Duration::from_secs(4));
        assert_eq!(retry_delay(Some("0"), b""), Duration::from_secs(1));
        assert_eq!(retry_delay(Some("invalid"), b"not json"), Duration::from_secs(1));
    }

    #[test]
    fn retries_rate_limit_and_busy_then_accepts_signature() {
        let (url, worker) = mock_server(vec![
            (429, Some("1"), r#"{"ok":false,"code":"RATE_LIMITED","retry_after_seconds":1}"#),
            (503, None, r#"{"ok":false,"code":"BUSY","retry_after_seconds":1}"#),
            (200, None, r#"{"ok":true,"headers":{"X-Helios":"h","X-Medusa":"m"}}"#),
        ]);
        let started = std::time::Instant::now();
        let signed = CompatHttpSignature::new(&url, "test-token").sign(&request()).unwrap();
        assert!(!signed.is_empty());
        assert!(started.elapsed() >= Duration::from_secs(2));
        worker.join().unwrap();
    }

    #[test]
    fn permanent_errors_and_long_wait_are_not_retried() {
        for (status, retry, expected) in [
            (401, None, "UNAUTHORIZED"),
            (500, None, "内部故障"),
            (502, None, "网关故障"),
            (429, Some("60"), "60 秒"),
        ] {
            let (url, worker) = mock_server(vec![(status, retry, r#"{"ok":false}"#)]);
            let err = CompatHttpSignature::new(&url, "test-token").sign(&request()).unwrap_err();
            assert!(err.to_string().contains(expected), "{err}");
            worker.join().unwrap();
        }
    }

    #[test]
    fn empty_signature_fails_and_preview_is_not_cached_after_signer_failure() {
        let (url, worker) = mock_server(vec![(200, None, r#"{"ok":true}"#)]);
        let provider = CompatHttpSignature::new(&url, "test-token");
        let result = resolve_download_info(|| {
            assert!(provider.sign(&request()).is_err());
            Ok(DownloadInfo { is_preview: true, ..Default::default() })
        });
        assert!(result.unwrap_err().to_string().contains("空签名"));
        worker.join().unwrap();

        // 正常试听或整曲不能被之前的失败污染。
        let preview = resolve_download_info(|| Ok(DownloadInfo { is_preview: true, ..Default::default() })).unwrap();
        assert!(preview.is_preview);
        let full = resolve_download_info(|| {
            LAST_SIGNER_FAILURE.with(|slot| slot.replace(Some(SodaError::http("test signer error"))));
            Ok(DownloadInfo::default())
        }).unwrap();
        assert!(!full.is_preview);
    }
}
