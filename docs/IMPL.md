# IMPL — `simple-whisper`

Implementation guide for a desktop app that turns video files into Word documents using local FFmpeg + the Groq Whisper API.

**Stack:** Rust + Tauri v2 · SvelteKit · Tailwind CSS · Groq Cloud (`whisper-large-v3-turbo`).
**Heavy work split:** local CPU does cheap audio extraction; Groq LPU does the AI transcription.

---

## 1. Architecture

```
┌─────────────────────────┐
│  SvelteKit UI (ES, TW)  │  ← Spanish copy, light/dark/system theme
└───────────┬─────────────┘
            │ invoke()
┌───────────▼─────────────┐
│  Rust commands (Tauri)  │
│  · check_ffmpeg         │
│  · extract_audio        │
│  · chunk_audio          │
│  · transcribe_audio     │
│  · transcribe_video     │
│  · save_to_docx         │
│  · get/set_api_key      │
└──┬──────────────┬───────┘
   │              │
   ▼              ▼
┌──────────┐  ┌────────────────────────────┐
│ ffmpeg   │  │ POST api.groq.com/.../audio │
│ (system) │  │ /transcriptions             │
└──────────┘  └────────────────────────────┘
```

Three steps:

1. **System FFmpeg** — invoked via `tauri-plugin-shell`. Not bundled. App probes for it at startup and warns if missing.
2. **Groq Cloud (Whisper)** — fast, high-quality transcription. Free tier is fine for typical use.
3. **`docx-rs`** — pure-Rust `.docx` generation. No Office install needed.

---

## 2. System FFmpeg (no sidecar)

FFmpeg must already be on the user's `PATH`. We do **not** bundle it.

| OS              | Install                          |
| --------------- | -------------------------------- |
| macOS           | `brew install ffmpeg`            |
| Debian / Ubuntu | `sudo apt install ffmpeg`        |
| Windows         | `winget install ffmpeg` or `choco install ffmpeg` |

### Startup probe

Run `ffmpeg -version` once at app boot. If it fails, surface a persistent banner in the UI and disable the transcribe button.

```rust
use tauri_plugin_shell::ShellExt;

#[tauri::command]
async fn check_ffmpeg(app: tauri::AppHandle) -> Result<String, String> {
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
    Ok(String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("").to_string())
}
```

UI behavior: if the probe fails, render a banner with install hints and disable the file picker / transcribe action.

---

## 3. Setup (CLI-first — never hand-edit manifests)

```sh
# Rust deps + Tauri plugins
cd src-tauri
cargo tauri add shell
cargo tauri add store
cargo tauri add dialog
cargo add reqwest --features multipart,rustls-tls,json
cargo add docx-rs
cargo add tokio --features full
cargo add thiserror

# Frontend deps (from repo root)
cd ..
pnpm add @tauri-apps/plugin-shell @tauri-apps/plugin-store @tauri-apps/plugin-dialog
pnpm add -D tailwindcss @tailwindcss/vite
```

### `vite.config.js`

Add the Tailwind v4 plugin:

```js
import tailwindcss from "@tailwindcss/vite";
// ...
plugins: [tailwindcss(), sveltekit()],
```

### `src/app.css`

```css
@import "tailwindcss";
@custom-variant dark (&:where(.dark, .dark *));
```

Import it from `src/routes/+layout.svelte`.

---

## 4. `tauri.conf.json`

No `externalBin` block — FFmpeg is not bundled.

```json
"bundle": {
  "active": true,
  "targets": "all"
}
```

### Capabilities (`src-tauri/capabilities/default.json`)

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "dialog:default",
    "store:default",
    {
      "identifier": "shell:allow-execute",
      "allow": [
        { "name": "ffmpeg", "cmd": "ffmpeg", "args": true },
        { "name": "ffprobe", "cmd": "ffprobe", "args": true }
      ]
    }
  ]
}
```

`shell:allow-execute` scoped to program name `ffmpeg` / `ffprobe` — PATH lookup, no absolute path required.

---

## 5. Rust commands (`src-tauri/src/lib.rs`)

### A. Extract audio

Converts video to a 32 kbps mono 16 kHz MP3 — small enough to stay well under the Groq 25 MB cap.

```rust
use tauri_plugin_shell::ShellExt;

#[tauri::command]
async fn extract_audio(app: tauri::AppHandle, video_path: String) -> Result<String, String> {
    let audio_path = std::env::temp_dir().join("simple_whisper_audio.mp3");
    let audio_str = audio_path.to_string_lossy().to_string();

    let output = app
        .shell()
        .command("ffmpeg")
        .args([
            "-i", &video_path,
            "-vn", "-ar", "16000", "-ac", "1", "-b:a", "32k",
            &audio_str, "-y",
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
```

### B. Chunk audio (for long inputs)

Groq caps file uploads at **25 MB**. At 32 kbps mono, that's ~104 min — we split at **90 min** to leave headroom.

```rust
#[tauri::command]
async fn chunk_audio(app: tauri::AppHandle, audio_path: String) -> Result<Vec<String>, String> {
    const MAX_BYTES: u64 = 24 * 1024 * 1024;
    let size = std::fs::metadata(&audio_path).map_err(|e| e.to_string())?.len();
    if size <= MAX_BYTES {
        return Ok(vec![audio_path]);
    }

    let dir = std::env::temp_dir().join("simple_whisper_chunks");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let pattern = dir.join("chunk_%03d.mp3").to_string_lossy().to_string();

    let output = app
        .shell()
        .command("ffmpeg")
        .args([
            "-i", &audio_path,
            "-f", "segment",
            "-segment_time", "5400",   // 90 min
            "-c", "copy",
            &pattern, "-y",
        ])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("FFmpeg falló al fragmentar".into());
    }

    let mut chunks: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|d| d.path().to_string_lossy().to_string()))
        .collect();
    chunks.sort();
    Ok(chunks)
}
```

### C. Transcribe via Groq

```rust
use reqwest::multipart;

#[tauri::command]
async fn transcribe_audio(audio_path: String, api_key: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let bytes = std::fs::read(&audio_path).map_err(|e| e.to_string())?;
    let part = multipart::Part::bytes(bytes)
        .file_name("audio.mp3")
        .mime_str("audio/mpeg")
        .map_err(|e| e.to_string())?;

    let form = multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-large-v3-turbo")
        .text("response_format", "text");

    let res = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {api_key}"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let code = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Groq {code}: {body}"));
    }
    res.text().await.map_err(|e| e.to_string())
}
```

### D. Orchestrator

```rust
#[tauri::command]
async fn transcribe_video(app: tauri::AppHandle, video_path: String, api_key: String)
    -> Result<String, String>
{
    let audio = extract_audio(app.clone(), video_path).await?;
    let chunks = chunk_audio(app, audio).await?;
    let mut out = String::new();
    for chunk in chunks {
        let text = transcribe_audio(chunk, api_key.clone()).await?;
        if !out.is_empty() { out.push_str("\n\n"); }
        out.push_str(text.trim());
    }
    Ok(out)
}
```

### E. Save as `.docx`

```rust
use docx_rs::*;

#[tauri::command]
fn save_to_docx(text: String, path: String) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut doc = Docx::new();
    for line in text.split("\n\n") {
        doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(line)));
    }
    doc.build().pack(file).map_err(|e| e.to_string())?;
    Ok(())
}
```

### Register all handlers

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_store::Builder::new().build())
    .plugin(tauri_plugin_dialog::init())
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
```

---

## 6. API key handling

Store the Groq API key with `tauri-plugin-store` (lives in the app config dir, scoped per OS).

From the UI:

```ts
import { load } from "@tauri-apps/plugin-store";

const store = await load("settings.json", { autoSave: true });
await store.set("groq_api_key", key);
const key = (await store.get<string>("groq_api_key")) ?? "";
```

The settings page exposes a text input and save button. The transcribe flow reads from the store; if empty, it shows a modal pointing back to **Ajustes**.

---

## 7. Frontend (SvelteKit + Tailwind, Spanish UI)

**Design pass:** before writing components, run the `frontend-design` skill to define layout, typography, motion, and color tokens. The skill output drives the visual system.

### Theme (light / dark / system)

- Selection stored in `tauri-plugin-store` as `"light" | "dark" | "system"`.
- On app boot, read the preference. When `system`, listen to `matchMedia('(prefers-color-scheme: dark)')` and toggle the `.dark` class on `<html>`.
- Tailwind v4 dark variant is registered globally via `@custom-variant dark (&:where(.dark, .dark *));`.

```ts
// src/lib/theme.ts
import { load } from "@tauri-apps/plugin-store";

export type Theme = "light" | "dark" | "system";

export async function applyTheme(theme: Theme) {
  const store = await load("settings.json", { autoSave: true });
  await store.set("theme", theme);
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const isDark = theme === "dark" || (theme === "system" && prefersDark);
  document.documentElement.classList.toggle("dark", isDark);
}
```

In `+layout.svelte`, subscribe to `matchMedia` changes so `system` mode updates live.

### Spanish copy

All visible strings live in `src/lib/i18n/es.ts`:

```ts
export const t = {
  appTitle: "Simple Whisper",
  selectVideo: "Seleccionar video",
  transcribe: "Transcribir",
  transcribing: "Transcribiendo…",
  extractingAudio: "Procesando audio…",
  chunking: "Dividiendo audio en fragmentos…",
  ready: "Listo",
  saveDocx: "Guardar como .docx",
  settings: "Ajustes",
  apiKeyLabel: "Clave API de Groq",
  theme: "Tema",
  themeLight: "Claro",
  themeDark: "Oscuro",
  themeSystem: "Sistema",
  ffmpegMissing:
    "FFmpeg no detectado en el sistema. Instálalo con `brew install ffmpeg` (macOS), `sudo apt install ffmpeg` (Debian/Ubuntu) o `winget install ffmpeg` (Windows).",
  apiKeyMissing: "Configura tu clave API de Groq en Ajustes antes de transcribir.",
  groqError: "Error de Groq",
};
```

### Routes

| Route                | Purpose                                                        |
| -------------------- | -------------------------------------------------------------- |
| `+layout.svelte`     | Imports `app.css`, mounts theme + FFmpeg probe.                |
| `+page.svelte`       | File picker, pipeline progress, ffmpeg-missing banner.         |
| `settings/+page.svelte` | API key input, theme selector (3 buttons).                  |

State uses Svelte 5 runes (`$state`, `$derived`, `$effect`).

---

## 8. Why it runs fine on a weak PC

- **Cheap local work:** converting a 10-min video to a 32 kbps mono MP3 is seconds even on old hardware.
- **No GPU needed:** Whisper runs on Groq's LPUs, not the user's machine.
- **Tiny binary:** Tauri output is ~10 MB. No Chromium duplication.

---

## 9. Failure modes

| Condition                                | Behavior                                                                 |
| ---------------------------------------- | ------------------------------------------------------------------------ |
| FFmpeg missing                           | Persistent Spanish banner with install commands; transcribe disabled.    |
| API key missing                          | Modal pointing to **Ajustes**.                                           |
| Groq 4xx / 5xx                           | Surface server response verbatim (status code + body).                   |
| Audio still >25 MB after chunking        | Hard error — user receives the failing file path.                        |
| Network failure                          | `reqwest` error propagated to the UI.                                    |

---

## 10. Verification

- `cd src-tauri && cargo check`
- `pnpm tauri dev` → pick a short MP4 → `.docx` produced and opens in Word/LibreOffice.
- Rename / remove ffmpeg from `PATH` → banner appears, transcribe disabled.
- Process a 2+ hour audio → chunking runs, transcript is joined cleanly.
- Toggle theme **Claro / Oscuro / Sistema**; reload app — choice persists.
- Switch OS theme while app is set to **Sistema** → UI flips immediately.
- Confirm no English copy leaks into the UI.
