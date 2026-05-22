mod secrets;
mod transcribe_error;

use std::path::{Path, PathBuf};

use base64::Engine;
use docx_rs::{Docx, Paragraph, Run};
use printpdf::{BuiltinFont, Mm, PdfDocument};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::process::Command;

use transcribe_error::{
    classify_gemini, classify_groq, classify_reqwest, TranscribeError, TranscribeErrorKind,
};

const GROQ_CHUNK_BYTES: u64 = 24 * 1024 * 1024;
const GROQ_SEGMENT_SECONDS: &str = "5400";
// Gemini accepts 20MB inline; base64 inflates ~33%, so source chunk must be ≤ ~14MB.
const GEMINI_CHUNK_BYTES: u64 = 14 * 1024 * 1024;
const GEMINI_SEGMENT_SECONDS: &str = "1800";

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

fn temp_dir() -> PathBuf {
    std::env::temp_dir().join("simple-whisper")
}

fn ensure_temp_dir() -> Result<PathBuf, String> {
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
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

async fn extract_audio_inner(video_path: &str) -> Result<String, TranscribeError> {
    let dir = ensure_temp_dir().map_err(TranscribeError::unknown)?;
    let audio_path = dir.join("audio.mp3");
    let audio_str = audio_path.to_string_lossy().to_string();

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

fn default_model(provider: &str) -> &'static str {
    match provider {
        "gemini" => "gemini-3.1-flash-lite",
        _ => "whisper-large-v3-turbo",
    }
}

async fn transcribe_pipeline(
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
        other => Err(TranscribeError::new(
            TranscribeErrorKind::BadRequest,
            format!("Proveedor desconocido: {other}"),
        )),
    }
}

#[tauri::command]
async fn transcribe_audio(
    audio_path: String,
    provider: String,
    api_key: String,
    model: Option<String>,
    language: Option<String>,
    diarize: Option<bool>,
) -> Result<Transcript, TranscribeError> {
    if api_key.trim().is_empty() {
        return Err(TranscribeError::api_key_missing(provider));
    }
    let model = model
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| default_model(&provider).to_string());
    let language = language.unwrap_or_else(|| "auto".to_string());
    let diarize = diarize.unwrap_or(false);
    transcribe_pipeline(audio_path, &provider, &api_key, &model, &language, diarize).await
}

#[tauri::command]
async fn transcribe_video(
    video_path: String,
    provider: String,
    api_key: String,
    model: Option<String>,
    language: Option<String>,
    diarize: Option<bool>,
) -> Result<Transcript, TranscribeError> {
    if api_key.trim().is_empty() {
        return Err(TranscribeError::api_key_missing(provider));
    }
    let model = model
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| default_model(&provider).to_string());
    let language = language.unwrap_or_else(|| "auto".to_string());
    let diarize = diarize.unwrap_or(false);

    let audio = extract_audio_inner(&video_path).await?;
    transcribe_pipeline(audio, &provider, &api_key, &model, &language, diarize).await
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
            transcribe_audio,
            transcribe_video,
            save_transcript,
            secrets::secret_get,
            secrets::secret_set,
            secrets::secret_migrate_from_keychain,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
