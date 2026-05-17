<div align="center">
  <img src="src/assets/sw-logo.png" alt="Simple Whisper" width="140" />

  # Simple Whisper

  Batch transcription for video and audio. Drag, drop, transcribe to DOCX, MD, TXT, or PDF.

  <sub>Tauri 2 · SvelteKit · Groq Whisper</sub>
</div>

---

## Overview

Simple Whisper is a desktop app that turns local media files into clean text. Drop a queue of videos onto the window, pick a model and language, hit transcribe. Each file streams through Groq's hosted Whisper, then exports to your chosen format. Everything stays on your machine except the audio sent to Groq for inference.

- **Multi-file queue.** Sequential processing with per-file status, progress, and inline preview.
- **Four output formats.** DOCX, Markdown, TXT, PDF (Latin-1 only).
- **Auto-chunking.** Files larger than Groq's 24 MB limit are split with FFmpeg, then stitched back together.
- **Native drag-and-drop.** OS-level paths via Tauri's webview event, not browser-only.
- **Settings drawer.** Theme, model, language, format, API key, all live.
- **Secure key storage.** macOS Keychain primary, encrypted plugin-store fallback for unsigned dev builds.

## Screenshot

<div align="center">
  <img src=".github/sw.avif" alt="Simple Whisper UI" width="720" />
</div>

> [!NOTE]
> Dark UI by default. Coral `#e8634a` accent. Self-hosted Lora.

## Prerequisites

> [!IMPORTANT]
> FFmpeg must be installed and reachable. On macOS GUI launches the app probes `/opt/homebrew/bin`, `/usr/local/bin`, `/opt/local/bin`, `/usr/bin` before falling back to `PATH`.

| Tool        | Why                                     | Install                           |
|-------------|-----------------------------------------|-----------------------------------|
| Node ≥ 20   | Frontend toolchain                      | <https://nodejs.org>              |
| pnpm        | Package manager                         | `npm i -g pnpm`                   |
| Rust stable | Tauri backend                           | <https://rustup.rs>               |
| FFmpeg      | Audio extraction + chunking             | `brew install ffmpeg` (macOS)     |
| Groq API key| Whisper inference (`gsk_*` format)      | <https://console.groq.com/keys>   |

## Run

```sh
pnpm install
pnpm tauri dev
```

## Build

```sh
pnpm tauri build
```

Bundles land in `src-tauri/target/release/bundle/`:

- `macos/simple-whisper.app`
- `dmg/simple-whisper_<version>_<arch>.dmg`

## Install from a release

Grab the latest DMG from [Releases](https://github.com/ARKye03/simple-whisper/releases):

- `simple-whisper_<version>_aarch64.dmg` — Apple Silicon (M1/M2/M3/M4)
- `simple-whisper_<version>_x64.dmg` — Intel Mac
- `simple-whisper_<version>_x64-setup.exe` / `.msi` — Windows

> [!WARNING]
> macOS builds are unsigned. Gatekeeper marks the app as "damaged" on first launch. Strip the quarantine xattr after dragging the app to `/Applications`:
>
> ```sh
> xattr -cr /Applications/simple-whisper.app
> ```
>
> Then open normally. One-time fix per install.

## Configure

Open the gear icon (top-right) and paste your Groq key. Saved automatically on input (350 ms debounce). Validation: `gsk_` prefix plus 20+ alphanumeric chars.

| Setting    | Options                                                   |
|------------|-----------------------------------------------------------|
| Theme      | Light, Dark, System                                       |
| Model      | `whisper-large-v3-turbo`, `whisper-large-v3`              |
| Language   | Auto, ES, EN, FR, DE, PT, IT, JA, ZH                      |
| Format     | TXT, MD, DOCX, PDF (PDF disabled for JA/ZH)               |

> [!WARNING]
> PDF export uses Helvetica WinAnsi: Latin-1 only. Use DOCX, MD, or TXT for CJK or other non-Latin scripts.

## How it works

```
video/audio  ->  ffmpeg -ar 16000 -ac 1 -b:a 32k -> audio.mp3
audio.mp3    ->  size > 24 MB?  ->  yes: ffmpeg -f segment -segment_time 5400
                                    no:  use as-is
chunks[]     ->  POST groq.com/openai/v1/audio/transcriptions  (per chunk)
texts[]      ->  joined with "\n\n"  ->  user picks export format
```

All FFmpeg calls go through `tokio::process::Command` directly. No `tauri-plugin-shell` dependency.

## Privacy

| Data            | Location                                                                       |
|-----------------|--------------------------------------------------------------------------------|
| API key         | macOS Keychain (service `com.arkye03.simple-whisper`), or plugin-store fallback |
| Settings        | `~/Library/Application Support/com.arkye03.simple-whisper/settings.json`       |
| Audio chunks    | `$TMPDIR/simple-whisper/` (system-managed cleanup)                              |
| Transcriptions  | Saved where you choose, nowhere else                                            |
| Telemetry       | None                                                                            |

Audio is sent to Groq for inference. Groq's data policy applies; transcripts are not stored remotely.

## Tech stack

- **Frontend.** SvelteKit + Svelte 5 runes, static adapter, self-hosted Lora.
- **Desktop shell.** Tauri 2 (`tauri`, `tauri-plugin-dialog`, `tauri-plugin-store`, `tauri-plugin-opener`).
- **Transcription.** Groq hosted Whisper (`whisper-large-v3-turbo` / `whisper-large-v3`).
- **Export.** `docx-rs`, `printpdf` (0.7), plain `std::fs` for TXT/MD.
- **Secrets.** `keyring` 3.x (macOS Security framework backend).

## Acknowledgments

- [Groq](https://groq.com) for the Whisper inference API.
- [OpenAI Whisper](https://github.com/openai/whisper), the underlying model.
- [Tauri](https://tauri.app) for letting a Rust + web stack feel native.
- [Lora](https://fonts.google.com/specimen/Lora) by Cyreal, typography.
