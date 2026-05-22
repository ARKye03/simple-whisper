use reqwest::StatusCode;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub enum TranscribeErrorKind {
    AuthInvalid,
    AuthForbidden,
    RateLimited,
    QuotaExceeded,
    PayloadTooLarge,
    BadRequest,
    ModelUnavailable,
    ServerBusy,
    ServerError,
    Network,
    Timeout,
    MalformedResponse,
    FfmpegMissing,
    FfmpegFailed,
    ApiKeyMissing,
    Unknown,
}

#[derive(Serialize, Debug, Clone)]
pub struct TranscribeError {
    pub kind: TranscribeErrorKind,
    pub provider: Option<String>,
    pub message: String,
    pub retry_after_secs: Option<u64>,
    pub raw: Option<String>,
}

impl TranscribeError {
    pub fn new(kind: TranscribeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            provider: None,
            message: message.into(),
            retry_after_secs: None,
            raw: None,
        }
    }

    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn with_raw(mut self, raw: impl Into<String>) -> Self {
        self.raw = Some(raw.into());
        self
    }

    pub fn with_retry_after(mut self, secs: u64) -> Self {
        self.retry_after_secs = Some(secs);
        self
    }

    pub fn unknown(msg: impl Into<String>) -> Self {
        Self::new(TranscribeErrorKind::Unknown, msg)
    }

    pub fn ffmpeg_missing() -> Self {
        Self::new(
            TranscribeErrorKind::FfmpegMissing,
            "FFmpeg no encontrado en el sistema",
        )
    }

    pub fn ffmpeg_failed(stderr: impl Into<String>) -> Self {
        let raw = stderr.into();
        Self::new(TranscribeErrorKind::FfmpegFailed, "FFmpeg falló").with_raw(raw)
    }

    pub fn api_key_missing(provider: impl Into<String>) -> Self {
        Self::new(TranscribeErrorKind::ApiKeyMissing, "Falta clave API").with_provider(provider)
    }
}

impl std::fmt::Display for TranscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub fn classify_reqwest(err: reqwest::Error, provider: &str) -> TranscribeError {
    let kind = if err.is_timeout() {
        TranscribeErrorKind::Timeout
    } else if err.is_connect() || err.is_request() {
        TranscribeErrorKind::Network
    } else {
        TranscribeErrorKind::Unknown
    };
    let msg = err.to_string();
    TranscribeError::new(kind, msg.clone())
        .with_provider(provider)
        .with_raw(msg)
}

fn raw_summary(status: StatusCode, body: &str) -> String {
    let trimmed = if body.len() > 4096 {
        format!("{}…", &body[..4096])
    } else {
        body.to_string()
    };
    format!("HTTP {}\n\n{}", status, trimmed)
}

fn parse_retry_after(header: Option<&str>) -> Option<u64> {
    header.and_then(|h| h.trim().parse::<u64>().ok())
}

pub fn classify_groq(
    status: StatusCode,
    body: &str,
    retry_after_header: Option<&str>,
) -> TranscribeError {
    let body_low = body.to_lowercase();
    let kind = match status.as_u16() {
        401 => TranscribeErrorKind::AuthInvalid,
        403 => TranscribeErrorKind::AuthForbidden,
        404 => TranscribeErrorKind::ModelUnavailable,
        413 => TranscribeErrorKind::PayloadTooLarge,
        429 => {
            if body_low.contains("quota") || body_low.contains("daily") {
                TranscribeErrorKind::QuotaExceeded
            } else {
                TranscribeErrorKind::RateLimited
            }
        }
        400 | 422 => TranscribeErrorKind::BadRequest,
        498 | 502 | 503 => TranscribeErrorKind::ServerBusy,
        s if s >= 500 => TranscribeErrorKind::ServerError,
        _ => TranscribeErrorKind::Unknown,
    };

    let mut err = TranscribeError::new(kind, format!("Groq HTTP {}", status.as_u16()))
        .with_provider("groq")
        .with_raw(raw_summary(status, body));

    if let Some(secs) = parse_retry_after(retry_after_header) {
        err = err.with_retry_after(secs);
    }
    err
}

pub fn classify_gemini(status: StatusCode, body: &str) -> TranscribeError {
    let parsed: Option<serde_json::Value> = serde_json::from_str(body).ok();

    let status_str = parsed
        .as_ref()
        .and_then(|v| v.pointer("/error/status"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let message_str = parsed
        .as_ref()
        .and_then(|v| v.pointer("/error/message"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    let retry_after_secs = parsed
        .as_ref()
        .and_then(|v| v.pointer("/error/details"))
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter().find_map(|d| {
                let is_retry_info = d
                    .get("@type")
                    .and_then(|t| t.as_str())
                    .map(|s| s.ends_with("RetryInfo"))
                    .unwrap_or(false);
                if !is_retry_info {
                    return None;
                }
                d.get("retryDelay")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.trim_end_matches('s').parse::<f64>().ok())
                    .map(|f| f.ceil() as u64)
            })
        });

    let kind = match status_str {
        "UNAUTHENTICATED" => TranscribeErrorKind::AuthInvalid,
        "PERMISSION_DENIED" => TranscribeErrorKind::AuthForbidden,
        "NOT_FOUND" => TranscribeErrorKind::ModelUnavailable,
        "RESOURCE_EXHAUSTED" => {
            if message_str.contains("quota") && !message_str.contains("per minute") {
                TranscribeErrorKind::QuotaExceeded
            } else {
                TranscribeErrorKind::RateLimited
            }
        }
        "INVALID_ARGUMENT" | "FAILED_PRECONDITION" => TranscribeErrorKind::BadRequest,
        "UNAVAILABLE" => TranscribeErrorKind::ServerBusy,
        "INTERNAL" | "UNKNOWN" => TranscribeErrorKind::ServerError,
        "DEADLINE_EXCEEDED" => TranscribeErrorKind::Timeout,
        _ => match status.as_u16() {
            401 => TranscribeErrorKind::AuthInvalid,
            403 => TranscribeErrorKind::AuthForbidden,
            404 => TranscribeErrorKind::ModelUnavailable,
            413 => TranscribeErrorKind::PayloadTooLarge,
            429 => TranscribeErrorKind::RateLimited,
            400 | 422 => TranscribeErrorKind::BadRequest,
            498 | 502 | 503 => TranscribeErrorKind::ServerBusy,
            s if s >= 500 => TranscribeErrorKind::ServerError,
            _ => TranscribeErrorKind::Unknown,
        },
    };

    let mut err = TranscribeError::new(kind, format!("Gemini HTTP {}", status.as_u16()))
        .with_provider("gemini")
        .with_raw(raw_summary(status, body));

    if let Some(secs) = retry_after_secs {
        err = err.with_retry_after(secs);
    }
    err
}
