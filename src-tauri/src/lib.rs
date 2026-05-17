use std::path::PathBuf;

use docx_rs::{Docx, Paragraph, Run};
use reqwest::multipart;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

const MAX_CHUNK_BYTES: u64 = 24 * 1024 * 1024;
const SEGMENT_SECONDS: &str = "5400";

fn temp_dir() -> PathBuf {
    std::env::temp_dir().join("simple-whisper")
}

fn ensure_temp_dir() -> Result<PathBuf, String> {
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

#[tauri::command]
async fn check_ffmpeg(app: AppHandle) -> Result<String, String> {
    let output = app
        .shell()
        .command("ffmpeg")
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
async fn extract_audio(app: AppHandle, video_path: String) -> Result<String, String> {
    let dir = ensure_temp_dir()?;
    let audio_path = dir.join("audio.mp3");
    let audio_str = audio_path.to_string_lossy().to_string();

    let output = app
        .shell()
        .command("ffmpeg")
        .args([
            "-i",
            &video_path,
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
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(format!(
            "FFmpeg falló: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(audio_str)
}

#[tauri::command]
async fn chunk_audio(app: AppHandle, audio_path: String) -> Result<Vec<String>, String> {
    let size = std::fs::metadata(&audio_path)
        .map_err(|e| e.to_string())?
        .len();

    if size <= MAX_CHUNK_BYTES {
        return Ok(vec![audio_path]);
    }

    let dir = ensure_temp_dir()?.join("chunks");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let pattern = dir.join("chunk_%03d.mp3").to_string_lossy().to_string();

    let output = app
        .shell()
        .command("ffmpeg")
        .args([
            "-i",
            &audio_path,
            "-f",
            "segment",
            "-segment_time",
            SEGMENT_SECONDS,
            "-c",
            "copy",
            &pattern,
            "-y",
        ])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(format!(
            "FFmpeg falló al fragmentar: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let mut chunks: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok().map(|e| e.path().to_string_lossy().to_string()))
        .filter(|p| p.ends_with(".mp3"))
        .collect();

    chunks.sort();

    if chunks.is_empty() {
        return Err("No se generaron fragmentos".into());
    }

    Ok(chunks)
}

#[tauri::command]
async fn transcribe_audio(audio_path: String, api_key: String) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("Falta clave API de Groq".into());
    }

    let bytes = std::fs::read(&audio_path).map_err(|e| e.to_string())?;
    let part = multipart::Part::bytes(bytes)
        .file_name("audio.mp3")
        .mime_str("audio/mpeg")
        .map_err(|e| e.to_string())?;

    let form = multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-large-v3-turbo")
        .text("response_format", "text");

    let client = reqwest::Client::new();
    let res = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {api_key}"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Groq {status}: {body}"));
    }

    res.text().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn transcribe_video(
    app: AppHandle,
    video_path: String,
    api_key: String,
) -> Result<String, String> {
    let audio = extract_audio(app.clone(), video_path).await?;
    let chunks = chunk_audio(app, audio).await?;

    let mut out = String::new();
    for chunk in chunks {
        let text = transcribe_audio(chunk, api_key.clone()).await?;
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(text.trim());
    }
    Ok(out)
}

#[tauri::command]
fn save_to_docx(text: String, path: String) -> Result<(), String> {
    let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut doc = Docx::new();
    for paragraph in text.split("\n\n") {
        doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(paragraph)));
    }
    doc.build().pack(file).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            check_ffmpeg,
            extract_audio,
            chunk_audio,
            transcribe_audio,
            transcribe_video,
            save_to_docx,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
