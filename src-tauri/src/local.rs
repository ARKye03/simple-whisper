//! Local transcription via faster-whisper running in a managed Python venv.
//!
//! Nothing is bundled: we discover the user's Python, create a venv under
//! `app_local_data_dir()`, and pip-install faster-whisper on demand. The driver
//! script is embedded as source text and re-materialized on every run.
//!
//! The NDJSON protocol is shared with `python/transcribe.py` — change both together.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

use crate::transcribe_error::{TranscribeError, TranscribeErrorKind};

const DRIVER: &str = include_str!("../python/transcribe.py");

pub const DEFAULT_MODEL: &str = "large-v3-turbo";

const MIN_PY: (u32, u32) = (3, 9);
const VERSION_TIMEOUT: Duration = Duration::from_secs(5);
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
const TAIL_LINES: usize = 40;
const LOG_EVENT: &str = "local:install";
/// Internal channel marker for a pipe read failure — never emitted to the frontend.
const READ_ERR: &str = "\0read-error";
const PROGRESS_EVENT: &str = "local:progress";

// ── discovery ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct PythonCmd {
    program: String,
    args: Vec<String>,
}

impl PythonCmd {
    fn bare(program: &str) -> Self {
        Self {
            program: program.to_string(),
            args: Vec::new(),
        }
    }

    fn label(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        cmd.kill_on_drop(true);
        no_window(&mut cmd);
        cmd
    }
}

// macOS GUI launches strip PATH, so absolute Homebrew/MacPorts paths come first.
// /usr/bin/python3 goes last: without Command Line Tools it is a stub that can
// pop a system dialog and hang.
#[cfg(target_os = "macos")]
fn python_candidates() -> Vec<PythonCmd> {
    vec![
        PythonCmd::bare("/opt/homebrew/bin/python3"),
        PythonCmd::bare("/usr/local/bin/python3"),
        PythonCmd::bare("/opt/local/bin/python3"),
        PythonCmd::bare("python3"),
        PythonCmd::bare("/usr/bin/python3"),
    ]
}

// On Linux /usr/bin/python3 is the real interpreter, and PATH first picks up
// pyenv/asdf/uv shims.
#[cfg(all(unix, not(target_os = "macos")))]
fn python_candidates() -> Vec<PythonCmd> {
    vec![
        PythonCmd::bare("python3"),
        PythonCmd::bare("/usr/bin/python3"),
        PythonCmd::bare("/usr/local/bin/python3"),
    ]
}

// `py -3` first: a bare `python` with no install is an App Execution Alias stub
// that opens the Microsoft Store.
#[cfg(windows)]
fn python_candidates() -> Vec<PythonCmd> {
    vec![
        PythonCmd {
            program: "py".to_string(),
            args: vec!["-3".to_string()],
        },
        PythonCmd::bare("python"),
        PythonCmd::bare("python3"),
    ]
}

fn parse_version(out: &str) -> Option<(u32, u32, String)> {
    let raw = out.split_whitespace().last()?;
    let mut parts = raw.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor, raw.to_string()))
}

async fn python_version(cmd: &PythonCmd) -> Option<(u32, u32, String)> {
    let mut c = cmd.command();
    c.arg("--version");
    let out = timeout(VERSION_TIMEOUT, c.output()).await.ok()?.ok()?;
    if !out.status.success() {
        return None;
    }
    let text = if out.stdout.is_empty() {
        String::from_utf8_lossy(&out.stderr).to_string()
    } else {
        String::from_utf8_lossy(&out.stdout).to_string()
    };
    parse_version(text.trim())
}

/// Returns the first candidate that satisfies `MIN_PY`. A too-old interpreter is
/// reported only when the whole list has no suitable one — otherwise a stale 3.8
/// early in the list (old Homebrew prefix, distro python) would mask a newer
/// interpreter further down and refuse the install outright.
async fn resolve_system_python() -> Option<(PythonCmd, u32, u32, String)> {
    let mut too_old: Option<(PythonCmd, u32, u32, String)> = None;
    for cand in python_candidates() {
        let is_path = cand.program.contains('/') || cand.program.contains('\\');
        if is_path && !Path::new(&cand.program).is_file() {
            continue;
        }
        if let Some((major, minor, version)) = python_version(&cand).await {
            if (major, minor) >= MIN_PY {
                return Some((cand, major, minor, version));
            }
            if too_old.is_none() {
                too_old = Some((cand, major, minor, version));
            }
        }
    }
    too_old
}

fn platform_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(windows) {
        "windows"
    } else {
        "linux"
    }
}

/// Flatpak and Snap sandbox `/usr` and cannot spawn host processes, so local
/// mode is simply unavailable there — reported separately from "no Python".
fn is_sandboxed() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    Path::new("/.flatpak-info").exists() || std::env::var_os("SNAP").is_some()
}

/// Distro family, used to pick the right `apt`/`dnf`/`pacman` install command.
fn distro_family() -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut ids: Vec<String> = Vec::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key == "ID" || key == "ID_LIKE" {
            let value = value.trim().trim_matches('"');
            ids.extend(value.split_whitespace().map(|s| s.to_lowercase()));
        }
    }
    let family = if ids.iter().any(|i| matches!(i.as_str(), "debian" | "ubuntu")) {
        "debian"
    } else if ids
        .iter()
        .any(|i| matches!(i.as_str(), "fedora" | "rhel" | "centos"))
    {
        "fedora"
    } else if ids.iter().any(|i| i == "arch") {
        "arch"
    } else if ids.iter().any(|i| i.starts_with("suse") || i == "opensuse") {
        "suse"
    } else {
        "unknown"
    };
    Some(family.to_string())
}

fn venv_package_hint() -> String {
    match distro_family().as_deref() {
        Some("debian") => "Instálalo con `sudo apt install python3-venv python3-pip`.".to_string(),
        Some("fedora") => "Instálalo con `sudo dnf install python3-pip`.".to_string(),
        Some("arch") => "Instálalo con `sudo pacman -S python-pip`.".to_string(),
        Some("suse") => "Instálalo con `sudo zypper install python3-pip`.".to_string(),
        _ => "Instala el paquete venv/pip de tu distribución.".to_string(),
    }
}

// ── paths ───────────────────────────────────────────────────────────────────

fn app_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("No se pudo resolver el directorio de datos: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn venv_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_dir(app)?.join("pyenv"))
}

fn venv_python(app: &AppHandle) -> Result<PathBuf, String> {
    let base = venv_dir(app)?;
    if cfg!(windows) {
        Ok(base.join("Scripts").join("python.exe"))
    } else {
        Ok(base.join("bin").join("python"))
    }
}

fn models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_dir(app)?.join("models");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Rewritten on every call so the script can never drift from the app version.
fn script_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_dir(app)?.join("python");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("transcribe.py");
    std::fs::write(&path, DRIVER).map_err(|e| e.to_string())?;
    Ok(path)
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                total += meta.len();
            }
        }
    }
    total
}

// ── process helpers ─────────────────────────────────────────────────────────

fn base_env(cmd: &mut Command, models: &Path) {
    cmd.env("PYTHONUNBUFFERED", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONNOUSERSITE", "1")
        .env("HF_HUB_DISABLE_PROGRESS_BARS", "1")
        .env("HF_HUB_DISABLE_TELEMETRY", "1")
        .env("TOKENIZERS_PARALLELISM", "false")
        .env("HF_HOME", models)
        .env_remove("PYTHONPATH")
        .env_remove("PYTHONHOME");
}

#[cfg(windows)]
fn no_window(cmd: &mut Command) {
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
}

#[cfg(not(windows))]
fn no_window(_cmd: &mut Command) {}

/// Best-effort stderr tail. A read error ends the drain, which in principle could
/// block the child on a full stderr buffer — but the caller kills the child on any
/// stdout read failure, and a pipe error here implies the write end is gone too.
async fn collect_tail<R: AsyncRead + Unpin>(reader: R, max_lines: usize) -> String {
    let mut lines = BufReader::new(reader).lines();
    let mut tail: VecDeque<String> = VecDeque::with_capacity(max_lines);
    while let Ok(Some(line)) = lines.next_line().await {
        if tail.len() == max_lines {
            tail.pop_front();
        }
        tail.push_back(line);
    }
    tail.into_iter().collect::<Vec<_>>().join("\n")
}

#[derive(Serialize, Clone)]
struct LocalInstallEvent {
    stream: String,
    line: String,
}

#[derive(Serialize, Clone)]
struct LocalProgressEvent {
    job_id: String,
    stage: String,
    progress: Option<f64>,
    detail: Option<String>,
}

fn emit_progress(app: &AppHandle, job_id: &str, stage: &str, progress: Option<f64>, detail: Option<String>) {
    let _ = app.emit(
        PROGRESS_EVENT,
        LocalProgressEvent {
            job_id: job_id.to_string(),
            stage: stage.to_string(),
            progress,
            detail,
        },
    );
}

/// Streams both pipes of a child to the frontend and returns the output tail.
///
/// Draining stdout and stderr concurrently is mandatory: pip and huggingface_hub
/// write megabytes to stderr, and a single-stream reader deadlocks the child once
/// the OS pipe buffer fills.
async fn run_logged(app: &AppHandle, mut cmd: Command, phase: &str) -> Result<(), String> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("No se pudo iniciar {phase}: {e}"))?;

    let stdout = child.stdout.take().ok_or("stdout no disponible")?;
    let stderr = child.stderr.take().ok_or("stderr no disponible")?;
    let (tx, mut rx) = mpsc::unbounded_channel::<(&'static str, String)>();

    for (name, pipe) in [("stdout", Box::new(stdout) as Box<dyn AsyncRead + Unpin + Send>), ("stderr", Box::new(stderr))] {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(pipe).lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if tx.send((name, line)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        let _ = tx.send((READ_ERR, e.to_string()));
                        break;
                    }
                }
            }
        });
    }
    drop(tx);

    let mut tail: VecDeque<String> = VecDeque::with_capacity(TAIL_LINES);
    let mut read_err: Option<String> = None;
    while let Some((stream, line)) = rx.recv().await {
        // A pipe we've stopped draining will block the child on a full buffer and
        // hang `child.wait()` — pip writes megabytes here, so kill instead.
        if stream == READ_ERR {
            read_err = Some(line);
            let _ = child.start_kill();
            continue;
        }
        if tail.len() == TAIL_LINES {
            tail.pop_front();
        }
        tail.push_back(line.clone());
        let _ = app.emit(
            LOG_EVENT,
            LocalInstallEvent {
                stream: stream.to_string(),
                line,
            },
        );
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    if let Some(e) = read_err {
        let tail = tail.into_iter().collect::<Vec<_>>().join("\n");
        return Err(format!(
            "{phase} falló: error leyendo la salida del proceso: {e}\n\n{tail}"
        ));
    }
    if status.success() {
        Ok(())
    } else {
        let tail = tail.into_iter().collect::<Vec<_>>().join("\n");
        Err(format!("{phase} falló.\n\n{tail}"))
    }
}

// ── status ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct PythonInfo {
    path: String,
    version: String,
    major: u32,
    minor: u32,
}

#[derive(Serialize, Clone)]
pub struct LocalStatus {
    python: Option<PythonInfo>,
    python_too_old: bool,
    venv_ready: bool,
    venv_broken: bool,
    faster_whisper: Option<String>,
    models: Vec<String>,
    cuda: Option<bool>,
    venv_dir: String,
    models_dir: String,
    venv_bytes: u64,
    models_bytes: u64,
    platform: String,
    distro: Option<String>,
    sandboxed: bool,
}

#[derive(Deserialize)]
struct ProbeLine {
    #[serde(default)]
    faster_whisper: Option<String>,
    #[serde(default)]
    models: Vec<String>,
    #[serde(default)]
    cuda: Option<bool>,
}

async fn probe_status(app: &AppHandle, with_cuda: bool) -> Result<LocalStatus, String> {
    let venv = venv_dir(app)?;
    let models = models_dir(app)?;

    let mut status = LocalStatus {
        python: None,
        python_too_old: false,
        venv_ready: false,
        venv_broken: false,
        faster_whisper: None,
        models: Vec::new(),
        cuda: None,
        venv_dir: venv.to_string_lossy().to_string(),
        models_dir: models.to_string_lossy().to_string(),
        venv_bytes: dir_size(&venv),
        models_bytes: dir_size(&models),
        platform: platform_name().to_string(),
        distro: distro_family(),
        sandboxed: is_sandboxed(),
    };

    if let Some((cmd, major, minor, version)) = resolve_system_python().await {
        status.python_too_old = (major, minor) < MIN_PY;
        status.python = Some(PythonInfo {
            path: cmd.label(),
            version,
            major,
            minor,
        });
    }

    let py = venv_python(app)?;
    if !py.is_file() {
        return Ok(status);
    }

    // Execute, don't just stat: a Homebrew python upgrade silently breaks the
    // venv, and without this it would present as an opaque transcription failure.
    let venv_cmd = PythonCmd::bare(&py.to_string_lossy());
    if python_version(&venv_cmd).await.is_none() {
        status.venv_broken = true;
        return Ok(status);
    }
    status.venv_ready = true;

    let script = script_path(app)?;
    let mut cmd = venv_cmd.command();
    cmd.arg("-u")
        .arg(&script)
        .arg("probe")
        .arg("--download-root")
        .arg(&models);
    if with_cuda {
        cmd.arg("--with-cuda");
    }
    base_env(&mut cmd, &models);

    let Ok(Ok(out)) = timeout(PROBE_TIMEOUT, cmd.output()).await else {
        return Ok(status);
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("probe") {
            continue;
        }
        if let Ok(probe) = serde_json::from_value::<ProbeLine>(value) {
            status.faster_whisper = probe.faster_whisper;
            status.models = probe.models;
            status.cuda = probe.cuda;
        }
    }
    Ok(status)
}

// ── commands ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn local_probe(app: AppHandle, with_cuda: Option<bool>) -> Result<LocalStatus, String> {
    probe_status(&app, with_cuda.unwrap_or(false)).await
}

#[tauri::command]
pub async fn local_install(app: AppHandle) -> Result<LocalStatus, String> {
    if is_sandboxed() {
        return Err(
            "El modo local no está disponible en compilaciones Flatpak/Snap: no pueden usar el Python del sistema."
                .to_string(),
        );
    }

    let Some((python, major, minor, version)) = resolve_system_python().await else {
        return Err("No se encontró Python 3.9 o superior en el sistema.".to_string());
    };
    if (major, minor) < MIN_PY {
        return Err(format!(
            "Se encontró Python {version}; se necesita 3.9 o superior."
        ));
    }

    let venv = venv_dir(&app)?;
    let py = venv_python(&app)?;
    let models = models_dir(&app)?;

    if python_version(&PythonCmd::bare(&py.to_string_lossy()))
        .await
        .is_none()
    {
        // A venv whose interpreter no longer runs can't be repaired in place
        // (`python -m venv` would reuse the stale prefix), so start clean.
        if venv.exists() {
            std::fs::remove_dir_all(&venv).map_err(|e| e.to_string())?;
        }
        let mut cmd = python.command();
        cmd.arg("-m").arg("venv").arg(&venv);
        base_env(&mut cmd, &models);
        run_logged(&app, cmd, "La creación del entorno").await.map_err(|e| {
            if e.contains("ensurepip") || e.contains("No module named venv") {
                format!(
                    "Falta el módulo venv de Python. {}\n\n{e}",
                    venv_package_hint()
                )
            } else {
                e
            }
        })?;
    }

    let venv_cmd = PythonCmd::bare(&py.to_string_lossy());
    for args in [
        vec!["-m", "pip", "install", "--upgrade", "pip"],
        vec!["-m", "pip", "install", "faster-whisper"],
    ] {
        let mut cmd = venv_cmd.command();
        cmd.args(&args)
            .arg("--disable-pip-version-check")
            .arg("--no-input");
        base_env(&mut cmd, &models);
        run_logged(&app, cmd, "La instalación").await?;
    }

    probe_status(&app, false).await
}

#[tauri::command]
pub async fn local_download_model(
    app: AppHandle,
    model: String,
    job_id: String,
) -> Result<LocalStatus, String> {
    let py = venv_python(&app)?;
    if !py.is_file() {
        return Err("El entorno local no está instalado.".to_string());
    }
    let models = models_dir(&app)?;
    let script = script_path(&app)?;

    let mut cmd = PythonCmd::bare(&py.to_string_lossy()).command();
    cmd.arg("-u")
        .arg(&script)
        .arg("download")
        .arg("--model")
        .arg(&model)
        .arg("--download-root")
        .arg(&models);
    base_env(&mut cmd, &models);

    match stream_driver(&app, cmd, &job_id).await {
        Ok(_) => probe_status(&app, false).await,
        // Keep the raw tail: the download path emits no install-log lines, so
        // dropping it would leave the panel with a message and no diagnostics.
        Err(err) => Err(match err.raw {
            Some(raw) if !raw.trim().is_empty() => format!("{}\n\n{raw}", err.message),
            _ => err.message,
        }),
    }
}

#[tauri::command]
pub async fn local_uninstall(app: AppHandle, purge_models: bool) -> Result<LocalStatus, String> {
    let venv = venv_dir(&app)?;
    if venv.exists() {
        std::fs::remove_dir_all(&venv).map_err(|e| e.to_string())?;
    }
    if purge_models {
        let models = models_dir(&app)?;
        if models.exists() {
            std::fs::remove_dir_all(&models).map_err(|e| e.to_string())?;
        }
    }
    probe_status(&app, false).await
}

// ── transcription ───────────────────────────────────────────────────────────

pub async fn transcribe(
    app: &AppHandle,
    job_id: &str,
    audio_path: &str,
    model: &str,
    language: &str,
) -> Result<String, TranscribeError> {
    let py = venv_python(app).map_err(TranscribeError::local_runtime_failed)?;
    if !py.is_file() {
        // Distinguish "no Python at all" from "Python fine, runtime not installed"
        // so the error card can give the right instruction.
        return Err(match resolve_system_python().await {
            None => TranscribeError::python_missing(),
            Some((_, major, minor, version)) if (major, minor) < MIN_PY => {
                TranscribeError::python_too_old(&version)
            }
            Some(_) => TranscribeError::local_runtime_missing(),
        });
    }

    let models = models_dir(app).map_err(TranscribeError::local_runtime_failed)?;
    let script = script_path(app).map_err(TranscribeError::local_runtime_failed)?;

    let mut cmd = PythonCmd::bare(&py.to_string_lossy()).command();
    cmd.arg("-u")
        .arg(&script)
        .arg("transcribe")
        .arg("--audio")
        .arg(audio_path)
        .arg("--model")
        .arg(model)
        .arg("--download-root")
        .arg(&models)
        .arg("--language")
        .arg(language)
        .arg("--device")
        .arg("auto");
    base_env(&mut cmd, &models);

    emit_progress(app, job_id, "starting", None, None);
    stream_driver(app, cmd, job_id).await
}

fn human_bytes(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    if (bytes as f64) < MB {
        format!("{} KB", bytes / 1024)
    } else if (bytes as f64) < MB * 1024.0 {
        format!("{:.0} MB", bytes as f64 / MB)
    } else {
        format!("{:.1} GB", bytes as f64 / (MB * 1024.0))
    }
}

/// Runs the driver, forwards NDJSON events, and returns the transcript text.
///
/// Unparseable stdout lines are ignored on purpose (a dependency may print a
/// stray warning), but a missing `done` line is a hard failure — a protocol gap
/// must never surface as a silently-empty transcript.
async fn stream_driver(
    app: &AppHandle,
    mut cmd: Command,
    job_id: &str,
) -> Result<String, TranscribeError> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| {
        TranscribeError::local_runtime_failed(format!("No se pudo iniciar Python: {e}"))
    })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| TranscribeError::local_runtime_failed("stdout no disponible"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| TranscribeError::local_runtime_failed("stderr no disponible"))?;
    let tail_task = tokio::spawn(collect_tail(stderr, TAIL_LINES));

    let mut lines = BufReader::new(stdout).lines();
    let mut text: Option<String> = None;
    let mut script_err: Option<(String, String, Option<String>)> = None;
    let mut read_err: Option<String> = None;

    loop {
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            // Stop reading and kill the child: leaving stdout undrained would
            // block Python on a full pipe buffer and hang `child.wait()`.
            Err(e) => {
                read_err = Some(e.to_string());
                let _ = child.start_kill();
                break;
            }
        };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        match value.get("type").and_then(Value::as_str) {
            Some("stage") => {
                let stage = value.get("stage").and_then(Value::as_str).unwrap_or("starting");
                let detail = value
                    .get("detail")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                emit_progress(app, job_id, stage, None, detail);
            }
            Some("download") => {
                let bytes = value.get("bytes").and_then(Value::as_u64).unwrap_or(0);
                emit_progress(
                    app,
                    job_id,
                    "downloading_model",
                    None,
                    Some(human_bytes(bytes)),
                );
            }
            Some("info") => {
                let device = value
                    .get("device")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                emit_progress(app, job_id, "transcribing", None, device);
            }
            Some("segment") => {
                let ratio = value.get("ratio").and_then(Value::as_f64).unwrap_or(0.0);
                let detail = value
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                emit_progress(
                    app,
                    job_id,
                    "transcribing",
                    Some((ratio * 100.0).clamp(1.0, 99.0)),
                    detail,
                );
            }
            Some("done") => {
                text = Some(
                    value
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                );
            }
            Some("error") => {
                script_err = Some((
                    value
                        .get("code")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    value
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    value
                        .get("traceback")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                ));
            }
            _ => {}
        }
    }

    let status = child.wait().await;
    let tail = tail_task.await.unwrap_or_default();

    if let Some((code, message, tb)) = script_err {
        let raw = [message.as_str(), tb.as_deref().unwrap_or(""), tail.as_str()]
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n");
        return Err(match code.as_str() {
            "import_failed" => TranscribeError::local_runtime_missing().with_raw(raw),
            "unknown_model" => TranscribeError::new(TranscribeErrorKind::BadRequest, message)
                .with_provider("local")
                .with_raw(raw),
            "model_download_failed" => TranscribeError::model_download_failed(raw),
            _ => TranscribeError::local_runtime_failed(raw),
        });
    }

    match text {
        Some(t) => Ok(t),
        None => {
            let code = status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
            let reason = read_err
                .map(|e| format!("Error leyendo la salida del proceso: {e}\n\n"))
                .unwrap_or_default();
            Err(TranscribeError::local_runtime_failed(format!(
                "{reason}El proceso terminó con código {code} sin devolver transcripción.\n\n{tail}"
            )))
        }
    }
}
