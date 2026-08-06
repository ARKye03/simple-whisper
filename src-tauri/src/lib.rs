mod local;
mod secrets;
mod transcribe_error;

use std::path::{Path, PathBuf};
use std::process::Stdio;

use base64::Engine;
use docx_rs::{Docx, Paragraph, Run};
use printpdf::{BuiltinFont, Mm, PdfDocument};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::ipc::Channel;
use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

use transcribe_error::{
    classify_gemini, classify_groq, classify_reqwest, classify_ytdlp, TranscribeError,
    TranscribeErrorKind,
};

const GROQ_CHUNK_BYTES: u64 = 24 * 1024 * 1024;
const GROQ_SEGMENT_SECONDS: &str = "5400";
// Gemini accepts 20MB inline; base64 inflates ~33%, so source chunk must be ≤ ~14MB.
const GEMINI_CHUNK_BYTES: u64 = 14 * 1024 * 1024;
const GEMINI_SEGMENT_SECONDS: &str = "1800";

// Prefixes we make yt-dlp print on stdout so progress frames and the final file
// path can be told apart from anything else it emits.
const YTDLP_PROGRESS_SENTINEL: &str = "SWPROG|";
const YTDLP_FILE_SENTINEL: &str = "SWFILE|";
// Pipe-separated so parse_progress_line can split positionally. Unavailable fields
// render as the literal "NA"; speed and total_bytes_estimate are floats while
// downloaded_bytes and eta are ints, so everything is parsed as f64.
const YTDLP_PROGRESS_FIELDS: &str = "%(progress.status)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s|%(progress.fragment_index)s|%(progress.fragment_count)s";
const PROBE_PLAYLIST_LIMIT: &str = "100";
const STALE_JOB_SECS: u64 = 6 * 60 * 60;

// Identical to yt-dlp's own supported list (yt_dlp/cookies.py).
const COOKIE_BROWSERS: &[&str] = &[
    "brave", "chrome", "chromium", "edge", "firefox", "opera", "safari", "vivaldi", "whale",
];

// macOS GUI apps launched from Finder inherit only a minimal PATH
// (/usr/bin:/bin:/usr/sbin:/sbin), so Homebrew/MacPorts binaries are invisible
// to a plain `Command::new("ffmpeg")`. Probe common install locations first.
fn resolve_ffmpeg() -> String {
    const CANDIDATES: &[&str] = &[
        "/opt/homebrew/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "/opt/local/bin/ffmpeg",
        "/usr/bin/ffmpeg",
    ];
    for c in CANDIDATES {
        if Path::new(c).is_file() {
            return (*c).to_string();
        }
    }
    "ffmpeg".to_string()
}

// Same stripped-PATH problem as resolve_ffmpeg, plus one extra location: unlike
// ffmpeg, yt-dlp is very commonly installed via pipx or `pip install --user`, which
// is why the candidate list is built at runtime instead of being a const.
fn resolve_ytdlp() -> String {
    let mut candidates: Vec<PathBuf> = [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/opt/local/bin",
        "/usr/bin",
    ]
    .iter()
    .map(|d| Path::new(d).join("yt-dlp"))
    .collect();
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(Path::new(&home).join(".local/bin/yt-dlp"));
    }
    for c in &candidates {
        if c.is_file() {
            return c.to_string_lossy().to_string();
        }
    }
    "yt-dlp".to_string()
}

fn temp_dir() -> PathBuf {
    std::env::temp_dir().join("simple-whisper")
}

fn ensure_temp_dir() -> Result<PathBuf, String> {
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn downloads_dir() -> PathBuf {
    temp_dir().join("downloads")
}

// pid + nanos is enough to be unique inside one $TMPDIR, which avoids pulling in a
// uuid dependency for something no one ever reads.
fn new_job_dir() -> Result<PathBuf, TranscribeError> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = downloads_dir().join(format!("{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| TranscribeError::unknown(e.to_string()))?;
    Ok(dir)
}

// Best effort: a hard quit mid-download leaves a job dir holding a possibly huge
// media file. Every error is swallowed — this must never fail a transcription.
fn sweep_stale_downloads() {
    let Ok(entries) = std::fs::read_dir(downloads_dir()) else {
        return;
    };
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_dir() {
            continue;
        }
        let stale = meta
            .modified()
            .ok()
            .and_then(|m| now.duration_since(m).ok())
            .map(|age| age.as_secs() > STALE_JOB_SECS)
            .unwrap_or(false);
        if stale {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

// The value lands in an argv slot for `--cookies-from-browser`. There is no shell
// involved (tokio::process execs directly), so this is not about shell injection —
// it blocks the *argument shapes* yt-dlp itself accepts on that flag, notably
// `chrome:/some/other/profile` and `firefox::keyring`, which would point cookie
// extraction at an arbitrary path. It also turns yt-dlp's usage dump into a clean
// Spanish error.
fn validate_cookies_browser(value: Option<&str>) -> Result<Option<String>, TranscribeError> {
    let Some(raw) = value else { return Ok(None) };
    let browser = raw.trim().to_ascii_lowercase();
    if browser.is_empty() {
        return Ok(None);
    }
    if !COOKIE_BROWSERS.contains(&browser.as_str()) {
        return Err(TranscribeError::new(
            TranscribeErrorKind::BadRequest,
            format!("Navegador no soportado para cookies: {raw}"),
        ));
    }
    Ok(Some(browser))
}

// The frontend already filters to http/https, but the command is the trust boundary:
// keep file://, data:// and bare paths from ever reaching a yt-dlp argv slot.
fn validate_media_url(url: &str) -> Result<String, TranscribeError> {
    let url = url.trim();
    if url.is_empty() {
        return Err(TranscribeError::new(
            TranscribeErrorKind::BadRequest,
            "Falta el enlace",
        ));
    }
    let lower = url.to_ascii_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(TranscribeError::new(
            TranscribeErrorKind::UnsupportedUrl,
            "El enlace debe empezar por http:// o https://",
        ));
    }
    Ok(url.to_string())
}

fn ytdlp_base_args(cookies_browser: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = vec![
        // A user's ~/.config/yt-dlp/config can set -o, -x, --quiet or --paths and
        // silently break both the output-path contract and progress parsing.
        "--ignore-config".into(),
        "--no-warnings".into(),
        // Keeps ANSI escapes out of the stderr text classify_ytdlp matches on.
        "--color".into(),
        "never".into(),
        // GUI launches inherit a stripped PATH, so yt-dlp cannot find ffmpeg on its
        // own — same root cause as resolve_ffmpeg() itself.
        "--ffmpeg-location".into(),
        resolve_ffmpeg(),
    ];
    if let Some(b) = cookies_browser {
        args.push("--cookies-from-browser".into());
        args.push(b.to_string());
    }
    args
}

#[tauri::command]
async fn check_ffmpeg() -> Result<String, String> {
    let output = Command::new(resolve_ffmpeg())
        .args(["-version"])
        .output()
        .await
        .map_err(|_| "FFmpeg no encontrado en el sistema".to_string())?;

    if !output.status.success() {
        return Err("FFmpeg falló al ejecutarse".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().next().unwrap_or("ffmpeg").to_string())
}

#[tauri::command]
async fn check_ytdlp() -> Result<String, String> {
    let output = Command::new(resolve_ytdlp())
        .args(["--version"])
        .output()
        .await
        .map_err(|_| "yt-dlp no encontrado en el sistema".to_string())?;

    if !output.status.success() {
        return Err("yt-dlp falló al ejecutarse".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().next().unwrap_or("yt-dlp").trim().to_string())
}

async fn extract_audio_inner(video_path: &str) -> Result<String, TranscribeError> {
    let dir = ensure_temp_dir().map_err(TranscribeError::unknown)?;
    extract_audio_to(video_path, &dir.join("audio.mp3")).await
}

async fn extract_audio_to(video_path: &str, out_path: &Path) -> Result<String, TranscribeError> {
    let audio_str = out_path.to_string_lossy().to_string();

    let output = Command::new(resolve_ffmpeg())
        .args([
            "-i",
            video_path,
            "-vn",
            "-ar",
            "16000",
            "-ac",
            "1",
            "-b:a",
            "32k",
            &audio_str,
            "-y",
        ])
        .output()
        .await
        .map_err(|_| TranscribeError::ffmpeg_missing())?;

    if !output.status.success() {
        return Err(TranscribeError::ffmpeg_failed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }

    Ok(audio_str)
}

async fn chunk_audio_with(
    audio_path: &str,
    max_bytes: u64,
    segment_seconds: &str,
) -> Result<Vec<String>, TranscribeError> {
    let size = std::fs::metadata(audio_path)
        .map_err(|e| TranscribeError::unknown(e.to_string()))?
        .len();

    if size <= max_bytes {
        return Ok(vec![audio_path.to_string()]);
    }

    // Shared, wiped per call — safe only because the frontend queue awaits one job
    // at a time. This and extract_audio_inner's fixed audio.mp3 break together the
    // moment anyone processes the queue in parallel.
    let dir = ensure_temp_dir()
        .map_err(TranscribeError::unknown)?
        .join("chunks");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| TranscribeError::unknown(e.to_string()))?;
    let pattern = dir.join("chunk_%03d.mp3").to_string_lossy().to_string();

    let output = Command::new(resolve_ffmpeg())
        .args([
            "-i",
            audio_path,
            "-f",
            "segment",
            "-segment_time",
            segment_seconds,
            "-c",
            "copy",
            &pattern,
            "-y",
        ])
        .output()
        .await
        .map_err(|_| TranscribeError::ffmpeg_missing())?;

    if !output.status.success() {
        return Err(TranscribeError::ffmpeg_failed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }

    let mut chunks: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|e| TranscribeError::unknown(e.to_string()))?
        .filter_map(|entry| entry.ok().map(|e| e.path().to_string_lossy().to_string()))
        .filter(|p| p.ends_with(".mp3"))
        .collect();

    chunks.sort();

    if chunks.is_empty() {
        return Err(TranscribeError::new(
            TranscribeErrorKind::FfmpegFailed,
            "No se generaron fragmentos",
        ));
    }

    Ok(chunks)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TranscriptSegment {
    pub speaker: String,
    pub text: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Transcript {
    Plain { text: String },
    Diarized {
        segments: Vec<TranscriptSegment>,
        text: String,
    },
}

fn flatten_segments(segs: &[TranscriptSegment]) -> String {
    segs.iter()
        .map(|s| format!("Hablante {}: {}", s.speaker, s.text))
        .collect::<Vec<_>>()
        .join("\n\n")
}

async fn transcribe_one_groq(
    audio_path: &str,
    api_key: &str,
    model: &str,
    language: &str,
) -> Result<String, TranscribeError> {
    let bytes = std::fs::read(audio_path)
        .map_err(|e| TranscribeError::unknown(e.to_string()).with_provider("groq"))?;
    let part = multipart::Part::bytes(bytes)
        .file_name("audio.mp3")
        .mime_str("audio/mpeg")
        .map_err(|e| TranscribeError::unknown(e.to_string()).with_provider("groq"))?;

    let mut form = multipart::Form::new()
        .part("file", part)
        .text("model", model.to_string())
        .text("response_format", "text");

    if !language.is_empty() && language != "auto" {
        form = form.text("language", language.to_string());
    }

    let client = reqwest::Client::new();
    let res = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {api_key}"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| classify_reqwest(e, "groq"))?;

    if !res.status().is_success() {
        let status = res.status();
        let retry_after = res
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = res.text().await.unwrap_or_default();
        return Err(classify_groq(status, &body, retry_after.as_deref()));
    }

    res.text()
        .await
        .map_err(|e| classify_reqwest(e, "groq"))
}

fn gemini_diarize_schema() -> Value {
    json!({
        "type": "OBJECT",
        "properties": {
            "segments": {
                "type": "ARRAY",
                "items": {
                    "type": "OBJECT",
                    "properties": {
                        "speaker": { "type": "STRING" },
                        "text": { "type": "STRING" }
                    },
                    "required": ["speaker", "text"]
                }
            }
        },
        "required": ["segments"]
    })
}

fn gemini_prompt(diarize: bool, language: &str) -> String {
    let lang_hint = if language.is_empty() || language == "auto" {
        "Preserva el idioma original del audio.".to_string()
    } else {
        format!("Transcribe en el idioma del audio (probablemente {language}).")
    };
    if diarize {
        format!(
            "Transcribe este audio con diarización de hablantes.\n\
            - Etiqueta cada hablante con una letra (A, B, C, ...). Mantén consistencia dentro del audio.\n\
            - Devuelve únicamente segmentos en orden cronológico. Cada cambio de hablante o pausa larga = nuevo segmento.\n\
            - {lang_hint}\n\
            - No inventes contenido. Si no se entiende, escribe [inaudible].\n\
            - Sin timestamps, sin resumen, sin metadatos. Solo segmentos."
        )
    } else {
        format!(
            "Transcribe este audio palabra por palabra.\n\
            - {lang_hint}\n\
            - No inventes contenido. Si no se entiende, escribe [inaudible].\n\
            - Devuelve solo el texto, sin etiquetas ni timestamps."
        )
    }
}

fn audio_mime_for(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("wav") => "audio/wav",
        Some("m4a") | Some("mp4") | Some("aac") => "audio/mp4",
        Some("ogg") | Some("opus") => "audio/ogg",
        Some("flac") => "audio/flac",
        Some("aiff") | Some("aif") => "audio/aiff",
        _ => "audio/mpeg",
    }
}

async fn transcribe_one_gemini(
    audio_path: &str,
    api_key: &str,
    model: &str,
    language: &str,
    diarize: bool,
) -> Result<GeminiResult, TranscribeError> {
    let bytes = std::fs::read(audio_path)
        .map_err(|e| TranscribeError::unknown(e.to_string()).with_provider("gemini"))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let mime = audio_mime_for(audio_path);

    let prompt = gemini_prompt(diarize, language);

    let mut body = json!({
        "contents": [{
            "parts": [
                { "inline_data": { "mime_type": mime, "data": b64 } },
                { "text": prompt }
            ]
        }]
    });

    if diarize {
        body["generation_config"] = json!({
            "response_mime_type": "application/json",
            "response_schema": gemini_diarize_schema(),
        });
    }

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
    );

    let client = reqwest::Client::new();
    let res = client
        .post(&url)
        .header("x-goog-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| classify_reqwest(e, "gemini"))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(classify_gemini(status, &body));
    }

    let resp: Value = res.json().await.map_err(|e| {
        TranscribeError::new(TranscribeErrorKind::MalformedResponse, e.to_string())
            .with_provider("gemini")
            .with_raw(e.to_string())
    })?;
    let text = resp
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            TranscribeError::new(
                TranscribeErrorKind::MalformedResponse,
                "Gemini: respuesta sin texto",
            )
            .with_provider("gemini")
            .with_raw(resp.to_string())
        })?
        .to_string();

    if diarize {
        let parsed: Value = serde_json::from_str(&text).map_err(|e| {
            TranscribeError::new(
                TranscribeErrorKind::MalformedResponse,
                format!("Gemini: JSON inválido en respuesta ({e})"),
            )
            .with_provider("gemini")
            .with_raw(text.clone())
        })?;
        let segments: Vec<TranscriptSegment> = parsed
            .get("segments")
            .and_then(|s| s.as_array())
            .ok_or_else(|| {
                TranscribeError::new(
                    TranscribeErrorKind::MalformedResponse,
                    "Gemini: falta 'segments' en la respuesta",
                )
                .with_provider("gemini")
                .with_raw(parsed.to_string())
            })?
            .iter()
            .filter_map(|item| {
                let speaker = item.get("speaker")?.as_str()?.trim().to_string();
                let text = item.get("text")?.as_str()?.trim().to_string();
                if text.is_empty() {
                    return None;
                }
                Some(TranscriptSegment { speaker, text })
            })
            .collect();
        Ok(GeminiResult::Diarized(segments))
    } else {
        Ok(GeminiResult::Plain(text.trim().to_string()))
    }
}

enum GeminiResult {
    Plain(String),
    Diarized(Vec<TranscriptSegment>),
}

// Empty for unknown providers, so transcribe_pipeline's catch-all reports the
// real problem instead of the request going out with a Groq model name.
fn default_model(provider: &str) -> &'static str {
    match provider {
        "groq" => "whisper-large-v3-turbo",
        "gemini" => "gemini-3.1-flash-lite",
        "local" => local::DEFAULT_MODEL,
        _ => "",
    }
}

fn provider_needs_key(provider: &str) -> bool {
    provider != "local"
}

#[allow(clippy::too_many_arguments)]
async fn transcribe_pipeline(
    app: &AppHandle,
    job_id: &str,
    audio_path: String,
    provider: &str,
    api_key: &str,
    model: &str,
    language: &str,
    diarize: bool,
) -> Result<Transcript, TranscribeError> {
    match provider {
        "groq" => {
            let chunks =
                chunk_audio_with(&audio_path, GROQ_CHUNK_BYTES, GROQ_SEGMENT_SECONDS).await?;
            let mut out = String::new();
            for chunk in chunks {
                let text = transcribe_one_groq(&chunk, api_key, model, language).await?;
                if !out.is_empty() {
                    out.push_str("\n\n");
                }
                out.push_str(text.trim());
            }
            Ok(Transcript::Plain { text: out })
        }
        "gemini" => {
            let chunks =
                chunk_audio_with(&audio_path, GEMINI_CHUNK_BYTES, GEMINI_SEGMENT_SECONDS).await?;
            if diarize {
                // NOTE: speaker labels reset per chunk — Gemini doesn't see prior context.
                // Acceptable for v1; future work could thread previous chunk's last segments.
                let mut all_segments: Vec<TranscriptSegment> = Vec::new();
                for chunk in chunks {
                    match transcribe_one_gemini(&chunk, api_key, model, language, true).await? {
                        GeminiResult::Diarized(mut segs) => all_segments.append(&mut segs),
                        GeminiResult::Plain(t) => {
                            all_segments.push(TranscriptSegment {
                                speaker: "?".to_string(),
                                text: t,
                            });
                        }
                    }
                }
                let text = flatten_segments(&all_segments);
                Ok(Transcript::Diarized {
                    segments: all_segments,
                    text,
                })
            } else {
                let mut out = String::new();
                for chunk in chunks {
                    match transcribe_one_gemini(&chunk, api_key, model, language, false).await? {
                        GeminiResult::Plain(t) => {
                            if !out.is_empty() {
                                out.push_str("\n\n");
                            }
                            out.push_str(&t);
                        }
                        GeminiResult::Diarized(segs) => {
                            if !out.is_empty() {
                                out.push_str("\n\n");
                            }
                            out.push_str(&flatten_segments(&segs));
                        }
                    }
                }
                Ok(Transcript::Plain { text: out })
            }
        }
        // No chunking: the local runtime reads the file directly, so splitting
        // would only cost an extra ffmpeg pass and reset Whisper's context.
        "local" => {
            let text = local::transcribe(app, job_id, &audio_path, model, language).await?;
            Ok(Transcript::Plain { text })
        }
        other => Err(TranscribeError::new(
            TranscribeErrorKind::BadRequest,
            format!("Proveedor desconocido: {other}"),
        )),
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn transcribe_video(
    app: AppHandle,
    video_path: String,
    provider: String,
    api_key: String,
    model: Option<String>,
    language: Option<String>,
    diarize: Option<bool>,
    job_id: Option<String>,
) -> Result<Transcript, TranscribeError> {
    if provider_needs_key(&provider) && api_key.trim().is_empty() {
        return Err(TranscribeError::api_key_missing(provider));
    }
    let model = model
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| default_model(&provider).to_string());
    let language = language.unwrap_or_else(|| "auto".to_string());
    let diarize = diarize.unwrap_or(false);
    let job_id = job_id.unwrap_or_default();

    let audio = extract_audio_inner(&video_path).await?;
    transcribe_pipeline(
        &app, &job_id, audio, &provider, &api_key, &model, &language, diarize,
    )
    .await
}

#[derive(Serialize)]
pub struct UrlEntry {
    pub url: String,
    pub title: Option<String>,
    pub duration_secs: Option<f64>,
}

#[derive(Serialize)]
pub struct UrlProbe {
    pub is_playlist: bool,
    pub title: String,
    pub duration_secs: Option<f64>,
    pub uploader: Option<String>,
    pub extractor: Option<String>,
    pub is_live: bool,
    pub thumbnail: Option<String>,
    pub webpage_url: String,
    pub entry_count: Option<u64>,
    pub entries: Vec<UrlEntry>,
}

// yt-dlp's JSON is enormous, varies per extractor and changes between releases, so a
// #[derive(Deserialize)] struct would reject a whole payload over one unexpected
// type. Read through Value with helpers that cannot fail instead.
fn str_at(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

// `duration` is an int for videos (213) but a float for flat-playlist entries
// (235.0), so everything numeric goes through as_f64.
fn f64_at(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|x| x.as_f64())
}

fn parse_probe(v: &Value, input_url: &str) -> Result<UrlProbe, TranscribeError> {
    if v.is_null() {
        return Err(TranscribeError::new(
            TranscribeErrorKind::UnsupportedUrl,
            "No se pudo leer ese enlace",
        ));
    }

    let is_playlist = str_at(v, "_type").as_deref() == Some("playlist");
    let extractor = str_at(v, "extractor").or_else(|| str_at(v, "extractor_key"));

    // The generic extractor hands back *any* HTTP URL as a pseudo-video, so a plain
    // webpage would probe fine and only fail much later. It marks those with
    // ext=unknown_video, while a real direct .mp3 link gets ext=mp3 — so this is
    // precise. Deliberately not keyed on a missing duration: direct media links
    // legitimately have none.
    if !is_playlist
        && extractor.as_deref() == Some("generic")
        && str_at(v, "ext")
            .map(|e| e.starts_with("unknown"))
            .unwrap_or(false)
    {
        return Err(TranscribeError::new(
            TranscribeErrorKind::UnsupportedUrl,
            "Ese enlace no contiene audio ni video descargable",
        ));
    }

    let entries: Vec<UrlEntry> = v
        .get("entries")
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| {
                    let url = str_at(e, "url").or_else(|| str_at(e, "webpage_url"))?;
                    Some(UrlEntry {
                        url,
                        title: str_at(e, "title"),
                        duration_secs: f64_at(e, "duration"),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // playlist_count is the real total even when --playlist-end capped `entries`.
    let entry_count = v
        .get("playlist_count")
        .and_then(|c| c.as_u64())
        .or(if is_playlist {
            Some(entries.len() as u64)
        } else {
            None
        });

    // is_upcoming is grouped with live: a scheduled premiere has nothing to
    // transcribe yet. post_live is deliberately allowed — that's a finished stream
    // and downloads normally.
    let live_status = str_at(v, "live_status").unwrap_or_default();
    let is_live = v.get("is_live").and_then(|b| b.as_bool()).unwrap_or(false)
        || live_status == "is_live"
        || live_status == "is_upcoming";

    Ok(UrlProbe {
        is_playlist,
        title: str_at(v, "title")
            .or_else(|| str_at(v, "id"))
            .unwrap_or_else(|| "(sin título)".to_string()),
        duration_secs: f64_at(v, "duration"),
        uploader: str_at(v, "uploader")
            .or_else(|| str_at(v, "channel"))
            .or_else(|| str_at(v, "uploader_id")),
        extractor,
        is_live,
        thumbnail: str_at(v, "thumbnail"),
        webpage_url: str_at(v, "webpage_url").unwrap_or_else(|| input_url.to_string()),
        entry_count,
        entries,
    })
}

async fn probe_inner(url: &str, cookies_browser: Option<&str>) -> Result<UrlProbe, TranscribeError> {
    let mut args = ytdlp_base_args(cookies_browser);
    args.extend([
        "-J".into(),
        "--skip-download".into(),
        // Stops a watch?v=X&list=Y URL from expanding into the whole playlist. A
        // bare playlist or channel URL ignores this flag, and that asymmetry is
        // exactly how parse_probe tells the two cases apart.
        "--no-playlist".into(),
        // Entries as stubs: one page request instead of one per video.
        "--flat-playlist".into(),
        "--playlist-end".into(),
        PROBE_PLAYLIST_LIMIT.into(),
        "--socket-timeout".into(),
        "15".into(),
        "--retries".into(),
        "2".into(),
        // End of options, so a URL beginning with '-' is never read as a flag.
        "--".into(),
        url.to_string(),
    ]);

    let output = Command::new(resolve_ytdlp())
        .args(&args)
        .output()
        .await
        .map_err(|_| TranscribeError::ytdlp_missing())?;

    if !output.status.success() {
        return Err(classify_ytdlp(&String::from_utf8_lossy(&output.stderr)));
    }

    // `-J` prints a bare `null` on soft failures, so a zero exit alone is not proof
    // of success — parse_probe rejects Value::Null.
    let value: Value = serde_json::from_slice(&output.stdout).map_err(|e| {
        TranscribeError::new(
            TranscribeErrorKind::MalformedResponse,
            "yt-dlp devolvió metadatos ilegibles",
        )
        .with_raw(e.to_string())
    })?;

    parse_probe(&value, url)
}

#[tauri::command]
async fn probe_url(
    url: String,
    cookies_browser: Option<String>,
) -> Result<UrlProbe, TranscribeError> {
    let url = validate_media_url(&url)?;
    let browser = validate_cookies_browser(cookies_browser.as_deref())?;
    // Reports is_live / is_playlist rather than rejecting: the UI needs to know why
    // a link is unusable, and needs the entries to expand a playlist.
    probe_inner(&url, browser.as_deref()).await
}

#[derive(Serialize, Clone)]
pub struct DownloadProgress {
    pub stage: String,
    pub percent: f64,
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub speed_bps: Option<f64>,
    pub eta_secs: Option<u64>,
    pub done: bool,
}

impl DownloadProgress {
    fn stage(stage: &str, percent: f64) -> Self {
        Self {
            stage: stage.to_string(),
            percent,
            downloaded_bytes: None,
            total_bytes: None,
            speed_bps: None,
            eta_secs: None,
            done: false,
        }
    }
}

fn na_f64(field: Option<&str>) -> Option<f64> {
    let f = field?.trim();
    if f.is_empty() || f == "NA" {
        return None;
    }
    f.parse::<f64>().ok()
}

fn parse_progress_line(rest: &str) -> Option<DownloadProgress> {
    let mut f = rest.split('|');
    let status = f.next()?.trim().to_string();
    let downloaded = na_f64(f.next());
    let total = na_f64(f.next());
    let estimate = na_f64(f.next());
    let speed = na_f64(f.next());
    let eta = na_f64(f.next());
    let frag_index = na_f64(f.next());
    let frag_count = na_f64(f.next());

    let done = status == "finished";
    let total_any = total.or(estimate);

    // Byte counts first, then fragment counts: with -N on DASH/HLS downloads yt-dlp
    // frequently reports no total size at all, which would peg the bar at 0%.
    let percent = if done {
        100.0
    } else {
        match (downloaded, total_any) {
            (Some(d), Some(t)) if t > 0.0 => (d / t * 100.0).clamp(0.0, 100.0),
            _ => match (frag_index, frag_count) {
                (Some(i), Some(c)) if c > 0.0 => (i / c * 100.0).clamp(0.0, 100.0),
                _ => 0.0,
            },
        }
    };

    Some(DownloadProgress {
        stage: "download".to_string(),
        percent,
        downloaded_bytes: downloaded.map(|d| d as u64),
        total_bytes: total_any.map(|t| t as u64),
        speed_bps: speed,
        eta_secs: eta.map(|e| e.round() as u64),
        done,
    })
}

// No glob crate in the tree, and we own the "source." stem, so a directory scan is
// enough. Largest file wins so a sidecar can never be picked.
fn find_downloaded_source(job_dir: &Path) -> Option<String> {
    let mut best: Option<(u64, String)> = None;
    for entry in std::fs::read_dir(job_dir).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("source.") {
            continue;
        }
        if name.ends_with(".part") || name.ends_with(".ytdl") || name.ends_with(".temp") {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        if best.as_ref().map(|(s, _)| size > *s).unwrap_or(true) {
            best = Some((size, entry.path().to_string_lossy().to_string()));
        }
    }
    best.map(|(_, p)| p)
}

async fn download_audio_to(
    job_dir: &Path,
    url: &str,
    cookies_browser: Option<&str>,
    on_progress: &Channel<DownloadProgress>,
) -> Result<String, TranscribeError> {
    let out_tmpl = job_dir.join("source.%(ext)s").to_string_lossy().to_string();

    let mut args = ytdlp_base_args(cookies_browser);
    args.extend([
        // bestaudio* keeps us on an audio-only stream so yt-dlp never has to merge.
        // The /best fallback can yield a video container, which is harmless because
        // extract_audio_to passes -vn.
        "-f".into(),
        "bestaudio*/best".into(),
        "--no-playlist".into(),
        "--restrict-filenames".into(),
        "--retries".into(),
        "3".into(),
        "--fragment-retries".into(),
        "3".into(),
        "-N".into(),
        "4".into(),
        "--socket-timeout".into(),
        "30".into(),
        // --newline turns the \r-updated progress bar into one line per update,
        // which is what BufReader::lines() needs. --progress is NOT optional: the
        // --print below implies --quiet, and noprogress defaults to quiet, so
        // dropping it means no progress is ever emitted.
        "--newline".into(),
        "--progress".into(),
        "--progress-delta".into(),
        "0.4".into(),
        "--progress-template".into(),
        format!("download:{YTDLP_PROGRESS_SENTINEL}{YTDLP_PROGRESS_FIELDS}"),
        // Final path after any move, so the extension never has to be guessed.
        // after_move is a late stage, so this does not imply --simulate.
        "--print".into(),
        format!("after_move:{YTDLP_FILE_SENTINEL}%(filepath)s"),
        "--no-simulate".into(),
        "-o".into(),
        out_tmpl,
        "--".into(),
        url.to_string(),
    ]);

    let mut child = Command::new(resolve_ytdlp())
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // If this future is dropped (app quit, or an early `?` below) the child must
        // not survive as an orphan still writing hundreds of MB into $TMPDIR.
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| TranscribeError::ytdlp_missing())?;

    // .take() moves the handles out, which is required rather than stylistic:
    // child.wait() needs &mut child, so keeping a borrow of child.stdout alive
    // across the read loop below would not borrow-check.
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| TranscribeError::unknown("stdout no disponible"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| TranscribeError::unknown("stderr no disponible"))?;

    // Drain stderr on its own task. Child::wait() does not drain the pipes, and
    // --print's implied --quiet pushes all extractor chatter onto stderr — enough to
    // fill the pipe buffer and deadlock a stdout-only reader. tokio::join! is not an
    // option here: one branch would have to borrow `child` mutably.
    let stderr_task = tokio::spawn(async move {
        let mut buf = String::new();
        let mut reader = BufReader::new(stderr);
        let _ = reader.read_to_string(&mut buf).await;
        buf
    });

    let mut final_path: Option<String> = None;
    let mut lines = BufReader::new(stdout).lines();
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| TranscribeError::unknown(e.to_string()))?
    {
        if let Some(rest) = line.strip_prefix(YTDLP_PROGRESS_SENTINEL) {
            if let Some(p) = parse_progress_line(rest) {
                // Ignore send errors: a closed window is not a download failure.
                let _ = on_progress.send(p);
            }
        } else if let Some(path) = line.strip_prefix(YTDLP_FILE_SENTINEL) {
            final_path = Some(path.trim().to_string());
        }
    }

    // stdout is at EOF, so yt-dlp is exiting and this returns immediately.
    let status = child
        .wait()
        .await
        .map_err(|e| TranscribeError::unknown(e.to_string()))?;
    let stderr_text = stderr_task.await.unwrap_or_default();

    if !status.success() {
        return Err(classify_ytdlp(&stderr_text));
    }

    // yt-dlp exits 0 when a filter skips a video, so success-with-no-file is a
    // reachable state and deserves its own message rather than a confusing ffmpeg
    // failure downstream.
    final_path
        .filter(|p| Path::new(p).is_file())
        .or_else(|| find_downloaded_source(job_dir))
        .ok_or_else(|| {
            TranscribeError::new(
                TranscribeErrorKind::DownloadFailed,
                "yt-dlp terminó sin dejar un archivo de audio",
            )
            .with_raw(stderr_text)
        })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn transcribe_url(
    app: AppHandle,
    url: String,
    provider: String,
    api_key: String,
    model: Option<String>,
    language: Option<String>,
    diarize: Option<bool>,
    cookies_browser: Option<String>,
    job_id: Option<String>,
    on_progress: Channel<DownloadProgress>,
) -> Result<Transcript, TranscribeError> {
    if provider_needs_key(&provider) && api_key.trim().is_empty() {
        return Err(TranscribeError::api_key_missing(provider));
    }
    let url = validate_media_url(&url)?;
    let browser = validate_cookies_browser(cookies_browser.as_deref())?;
    let model = model
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| default_model(&provider).to_string());
    let language = language.unwrap_or_else(|| "auto".to_string());
    let diarize = diarize.unwrap_or(false);
    let job_id = job_id.unwrap_or_default();

    // Probing first is a correctness requirement, not a nicety: yt-dlp would happily
    // record a live stream forever, and a bare playlist URL would otherwise collapse
    // to whatever --no-playlist picks.
    let probe = probe_inner(&url, browser.as_deref()).await?;
    if probe.is_live {
        return Err(TranscribeError::new(
            TranscribeErrorKind::LiveUnsupported,
            "Las transmisiones en vivo no se pueden transcribir",
        ));
    }
    if probe.is_playlist {
        return Err(TranscribeError::new(
            TranscribeErrorKind::BadRequest,
            "Ese enlace es una lista de reproducción. Elige un video concreto",
        ));
    }

    sweep_stale_downloads();
    let job_dir = new_job_dir()?;

    let result = transcribe_url_inner(
        &app,
        &job_id,
        &job_dir,
        &url,
        browser.as_deref(),
        &provider,
        &api_key,
        &model,
        &language,
        diarize,
        &on_progress,
    )
    .await;

    // Single cleanup path for both outcomes: the job dir holds the download and the
    // extracted mp3, and nothing outside this call references either.
    let _ = std::fs::remove_dir_all(&job_dir);
    result
}

#[allow(clippy::too_many_arguments)]
async fn transcribe_url_inner(
    app: &AppHandle,
    job_id: &str,
    job_dir: &Path,
    url: &str,
    cookies_browser: Option<&str>,
    provider: &str,
    api_key: &str,
    model: &str,
    language: &str,
    diarize: bool,
    on_progress: &Channel<DownloadProgress>,
) -> Result<Transcript, TranscribeError> {
    let source = download_audio_to(job_dir, url, cookies_browser, on_progress).await?;

    let _ = on_progress.send(DownloadProgress::stage("extract", 0.0));
    let audio = extract_audio_to(&source, &job_dir.join("audio.mp3")).await?;

    // The source can be hundreds of MB; only the 32kbps mp3 matters from here on.
    let _ = std::fs::remove_file(&source);

    // The HTTP legs have no progress signal, so this is a stage marker, not a bar.
    let _ = on_progress.send(DownloadProgress::stage("transcribe", 0.0));
    let transcript = transcribe_pipeline(
        app, job_id, audio, provider, api_key, model, language, diarize,
    )
    .await?;

    let mut finished = DownloadProgress::stage("transcribe", 100.0);
    finished.done = true;
    let _ = on_progress.send(finished);

    Ok(transcript)
}

fn save_docx(transcript: &Transcript, path: &str) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut doc = Docx::new();
    match transcript {
        Transcript::Plain { text } => {
            for paragraph in text.split("\n\n") {
                doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(paragraph)));
            }
        }
        Transcript::Diarized { segments, .. } => {
            for seg in segments {
                let label = format!("Hablante {}: ", seg.speaker);
                doc = doc.add_paragraph(
                    Paragraph::new()
                        .add_run(Run::new().bold().add_text(&label))
                        .add_run(Run::new().add_text(&seg.text)),
                );
            }
        }
    }
    doc.build().pack(file).map_err(|e| e.to_string())?;
    Ok(())
}

fn save_txt(transcript: &Transcript, path: &str) -> Result<(), String> {
    let body = match transcript {
        Transcript::Plain { text } => text.clone(),
        Transcript::Diarized { segments, .. } => segments
            .iter()
            .map(|s| format!("Hablante {}: {}", s.speaker, s.text))
            .collect::<Vec<_>>()
            .join("\n\n"),
    };
    std::fs::write(path, body).map_err(|e| e.to_string())
}

fn save_md(transcript: &Transcript, path: &str) -> Result<(), String> {
    let body = match transcript {
        Transcript::Plain { text } => format!("# Transcripción\n\n{}\n", text.trim_end()),
        Transcript::Diarized { segments, .. } => {
            let lines: Vec<String> = segments
                .iter()
                .map(|s| format!("**Hablante {}:** {}", s.speaker, s.text))
                .collect();
            format!("# Transcripción\n\n{}\n", lines.join("\n\n"))
        }
    };
    std::fs::write(path, body).map_err(|e| e.to_string())
}

fn wrap_line(line: &str, max_chars: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            out.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn save_pdf(transcript: &Transcript, path: &str) -> Result<(), String> {
    let body = match transcript {
        Transcript::Plain { text } => text.clone(),
        Transcript::Diarized { segments, .. } => segments
            .iter()
            .map(|s| format!("Hablante {}: {}", s.speaker, s.text))
            .collect::<Vec<_>>()
            .join("\n\n"),
    };

    if body.chars().any(|c| c as u32 > 0xFF) {
        return Err(
            "El formato PDF solo soporta caracteres Latin-1. Usa DOCX, MD o TXT para este texto."
                .to_string(),
        );
    }
    let (doc, page1, layer1) =
        PdfDocument::new("Transcripción", Mm(210.0), Mm(297.0), "Layer 1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;

    let font_size: f32 = 11.0;
    let line_height: f32 = 5.2;
    let top_margin: f32 = 280.0;
    let bottom_margin: f32 = 18.0;
    let left_margin: f32 = 20.0;
    let max_chars: usize = 92;

    let mut current_layer = doc.get_page(page1).get_layer(layer1);
    let mut y: f32 = top_margin;

    for paragraph in body.split('\n') {
        let lines = wrap_line(paragraph, max_chars);
        for line in lines {
            if y < bottom_margin {
                let (new_page, new_layer) =
                    doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
                current_layer = doc.get_page(new_page).get_layer(new_layer);
                y = top_margin;
            }
            current_layer.use_text(&line, font_size, Mm(left_margin), Mm(y), &font);
            y -= line_height;
        }
    }

    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut buf = std::io::BufWriter::new(file);
    doc.save(&mut buf).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn save_transcript(transcript: Transcript, path: String, format: String) -> Result<(), String> {
    match format.as_str() {
        "docx" => save_docx(&transcript, &path),
        "txt" => save_txt(&transcript, &path),
        "md" => save_md(&transcript, &path),
        "pdf" => save_pdf(&transcript, &path),
        other => Err(format!("Formato no soportado: {other}")),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init());

    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    builder
        .invoke_handler(tauri::generate_handler![
            check_ffmpeg,
            check_ytdlp,
            probe_url,
            transcribe_video,
            transcribe_url,
            save_transcript,
            secrets::secret_get,
            secrets::secret_set,
            secrets::secret_migrate_from_keychain,
            local::local_probe,
            local::local_install,
            local::local_download_model,
            local::local_uninstall,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
