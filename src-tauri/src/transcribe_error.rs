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
    YtdlpMissing,
    DownloadFailed,
    UnsupportedUrl,
    DownloadAuthRequired,
    LiveUnsupported,
    ApiKeyMissing,
    PythonMissing,
    PythonTooOld,
    LocalRuntimeMissing,
    ModelDownloadFailed,
    LocalRuntimeFailed,
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

    pub fn ytdlp_missing() -> Self {
        Self::new(
            TranscribeErrorKind::YtdlpMissing,
            "yt-dlp no encontrado en el sistema",
        )
    }

    pub fn api_key_missing(provider: impl Into<String>) -> Self {
        Self::new(TranscribeErrorKind::ApiKeyMissing, "Falta clave API").with_provider(provider)
    }

    pub fn python_missing() -> Self {
        Self::new(
            TranscribeErrorKind::PythonMissing,
            "No se encontró Python 3.9 o superior",
        )
        .with_provider("local")
    }

    pub fn python_too_old(found: &str) -> Self {
        Self::new(
            TranscribeErrorKind::PythonTooOld,
            format!("Python {found} es demasiado antiguo; se necesita 3.9 o superior"),
        )
        .with_provider("local")
    }

    pub fn local_runtime_missing() -> Self {
        Self::new(
            TranscribeErrorKind::LocalRuntimeMissing,
            "El entorno local no está instalado",
        )
        .with_provider("local")
    }

    pub fn model_download_failed(raw: impl Into<String>) -> Self {
        Self::new(
            TranscribeErrorKind::ModelDownloadFailed,
            "No se pudo descargar el modelo",
        )
        .with_provider("local")
        .with_raw(raw)
    }

    pub fn local_runtime_failed(raw: impl Into<String>) -> Self {
        Self::new(TranscribeErrorKind::LocalRuntimeFailed, "El entorno local falló")
            .with_provider("local")
            .with_raw(raw)
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

const RAW_BODY_LIMIT: usize = 4096;

fn truncate_at_char(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

fn raw_summary(status: StatusCode, body: &str) -> String {
    if body.len() > RAW_BODY_LIMIT {
        format!("HTTP {}\n\n{}…", status, truncate_at_char(body, RAW_BODY_LIMIT))
    } else {
        format!("HTTP {}\n\n{}", status, body)
    }
}

// yt-dlp runs with `--print`, which implies `--quiet` and pushes routine extractor
// chatter ("[youtube] abc: Downloading webpage") onto stderr. Keep only the
// diagnostic lines so `raw` stays readable and substring matching in
// classify_ytdlp cannot trip over progress noise.
fn ytdlp_stderr_summary(stderr: &str) -> String {
    let picked = stderr
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("ERROR:") || t.starts_with("WARNING:") || t.starts_with("yt-dlp: error:")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let text = if picked.is_empty() {
        stderr.trim()
    } else {
        picked.as_str()
    };
    truncate_at_char(text, RAW_BODY_LIMIT).to_string()
}

fn parse_retry_after(header: Option<&str>) -> Option<u64> {
    header.and_then(|h| h.trim().parse::<u64>().ok())
}

fn classify_by_status(status: StatusCode) -> TranscribeErrorKind {
    match status.as_u16() {
        401 => TranscribeErrorKind::AuthInvalid,
        403 => TranscribeErrorKind::AuthForbidden,
        404 => TranscribeErrorKind::ModelUnavailable,
        413 => TranscribeErrorKind::PayloadTooLarge,
        429 => TranscribeErrorKind::RateLimited,
        400 | 422 => TranscribeErrorKind::BadRequest,
        498 | 502 | 503 => TranscribeErrorKind::ServerBusy,
        s if s >= 500 => TranscribeErrorKind::ServerError,
        _ => TranscribeErrorKind::Unknown,
    }
}

pub fn classify_groq(
    status: StatusCode,
    body: &str,
    retry_after_header: Option<&str>,
) -> TranscribeError {
    let kind = if status.as_u16() == 429 {
        let sniff = truncate_at_char(body, RAW_BODY_LIMIT).to_lowercase();
        if sniff.contains("quota") || sniff.contains("daily") {
            TranscribeErrorKind::QuotaExceeded
        } else {
            TranscribeErrorKind::RateLimited
        }
    } else {
        classify_by_status(status)
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
        _ => classify_by_status(status),
    };

    let mut err = TranscribeError::new(kind, format!("Gemini HTTP {}", status.as_u16()))
        .with_provider("gemini")
        .with_raw(raw_summary(status, body));

    if let Some(secs) = retry_after_secs {
        err = err.with_retry_after(secs);
    }
    err
}

// Order is load-bearing: most specific needle first. Cookie-database failures come
// before the generic "sign in" needle because a stale-cookie run emits both and the
// cookie hint is the actionable one; 404 comes before the network bucket because
// yt-dlp wraps it in "Unable to download webpage" even though it means dead link,
// not lost connectivity.
pub fn classify_ytdlp(stderr: &str) -> TranscribeError {
    let raw = ytdlp_stderr_summary(stderr);
    let low = raw.to_lowercase();
    let has = |needle: &str| low.contains(needle);

    // An old yt-dlp rejects flags download_audio_to depends on (--progress-delta
    // needs 2024.07+, --color 2023.03+) and exits before downloading anything.
    // check_ytdlp deliberately accepts any version, so this is where a too-old
    // install gets named instead of surfacing as a bare "La descarga falló".
    let (kind, message) = if has("no such option") || has("unrecognized arguments") {
        (
            TranscribeErrorKind::DownloadFailed,
            "Tu versión de yt-dlp es demasiado antigua. Actualízala con `brew upgrade yt-dlp` o `pipx upgrade yt-dlp`",
        )
    } else if has("cookies database") || has("could not find local state file") {
        (
            TranscribeErrorKind::DownloadAuthRequired,
            "No se encontró la base de cookies de ese navegador. Ábrelo al menos una vez o elige otro",
        )
    } else if has("unsupported browser specified for cookies") {
        (
            TranscribeErrorKind::BadRequest,
            "Navegador no soportado para cookies",
        )
    } else if has("operation not permitted") || has("full disk access") {
        (
            TranscribeErrorKind::DownloadAuthRequired,
            "macOS bloqueó el acceso a las cookies. Concede Acceso total al disco a la app en Ajustes › Privacidad y seguridad",
        )
    } else if has("failed to decrypt") || has("could not decrypt") || has("keyring") {
        (
            TranscribeErrorKind::DownloadAuthRequired,
            "No se pudieron descifrar las cookies. Acepta el permiso del llavero de macOS e inténtalo de nuevo",
        )
    } else if has("members-only") || has("members only") || has("join this channel") {
        (
            TranscribeErrorKind::DownloadAuthRequired,
            "Video exclusivo para miembros del canal",
        )
    } else if has("private video") {
        (
            TranscribeErrorKind::DownloadAuthRequired,
            "Video privado. Usa las cookies del navegador donde tienes acceso",
        )
    } else if has("not a bot")
        || has("sign in")
        || has("login required")
        || has("only available for registered users")
    {
        (
            TranscribeErrorKind::DownloadAuthRequired,
            "El sitio pide iniciar sesión. Elige en Ajustes el navegador donde ya estás conectado",
        )
    } else if has("is not a valid url") {
        (
            TranscribeErrorKind::UnsupportedUrl,
            "El enlace no es una URL válida",
        )
    } else if has("unsupported url") {
        (TranscribeErrorKind::UnsupportedUrl, "Enlace no soportado")
    } else if has("http error 404") || has("http error 410") {
        (
            TranscribeErrorKind::UnsupportedUrl,
            "El enlace no existe o fue eliminado",
        )
    } else if has("video unavailable")
        || has("has been removed")
        || has("has been terminated")
        || has("no longer available")
    {
        (
            TranscribeErrorKind::UnsupportedUrl,
            "El video no está disponible",
        )
    } else if has("geo restriction")
        || has("available in your country")
        || has("available from your location")
    {
        (
            TranscribeErrorKind::UnsupportedUrl,
            "El video no está disponible en tu región",
        )
    } else if has("drm protected") {
        (
            TranscribeErrorKind::UnsupportedUrl,
            "El video está protegido con DRM",
        )
    } else if has("requested format is not available")
        || has("no video formats found")
        || has("only images are available")
    {
        (
            TranscribeErrorKind::UnsupportedUrl,
            "Ese enlace no ofrece una pista de audio descargable",
        )
    } else if has("live event will begin") || has("premieres in") || has("is not yet available") {
        (
            TranscribeErrorKind::UnsupportedUrl,
            "El video aún no está disponible (estreno o evento programado)",
        )
    } else if has("http error 429") || has("too many requests") {
        (
            TranscribeErrorKind::RateLimited,
            "El sitio limitó las descargas. Espera unos minutos",
        )
    } else if has("http error 403") || has("forbidden") {
        (
            TranscribeErrorKind::AuthForbidden,
            "El sitio rechazó la descarga (403)",
        )
    } else if has("timed out") || has("timeout") {
        (TranscribeErrorKind::Timeout, "La descarga tardó demasiado")
    } else if has("unable to download webpage")
        || has("unable to download api page")
        || has("connection reset")
        || has("connection refused")
        || has("nodename nor servname")
        || has("failed to resolve")
        || has("name or service not known")
        || has("network is unreachable")
    {
        (
            TranscribeErrorKind::Network,
            "No se pudo conectar con el sitio",
        )
    } else if has("no space left on device") {
        (
            TranscribeErrorKind::DownloadFailed,
            "No hay espacio en disco para la descarga",
        )
    } else if has("ffmpeg") {
        (
            TranscribeErrorKind::FfmpegFailed,
            "FFmpeg falló al procesar la descarga",
        )
    } else {
        (TranscribeErrorKind::DownloadFailed, "La descarga falló")
    };

    TranscribeError::new(kind, message).with_raw(raw)
}
