import type { Dict } from "./es";

export const en: Dict = {
  appTitle: "Simple Whisper",

  // Header / drawer
  settings: "Settings",

  // Provider
  provider: "Provider",
  providerGroq: "Groq",
  providerGemini: "Gemini",
  providerLocal: "Local",

  // API key (shared scaffold; provider-specific labels below)
  apiKey: "API Key",
  apiKeyLabel: "API Key",
  apiKeyLabelGroq: "Groq API Key",
  apiKeyLabelGemini: "Google AI Studio API Key",
  apiKeyPlaceholder: "",
  apiKeyPlaceholderGroq: "gsk_xxxxxxxxxxxxxxxx",
  apiKeyPlaceholderGemini: "AIzaxxxxxxxxxxxxxxxx",
  apiKeyHint: "Your key is stored locally and never leaves your device.",
  apiKeySaved: "Key saved",
  apiKeySaving: "Saving…",
  apiKeyInvalid: "Invalid format",
  apiKeyInvalidGroq: "Invalid format (must start with gsk_)",
  apiKeyInvalidGemini: "Invalid format (must start with AIza)",
  apiKeyValid: "Valid format",
  apiKeyBackendEncrypted: "Stored encrypted on this device",
  apiKeyBackendNone: "No key saved",
  apiKeyMissing: "Set your API key in Settings before transcribing.",
  show: "Show",
  hide: "Hide",
  closeAria: "Close",

  // Theme
  theme: "Theme",
  mode: "Mode",
  themeLight: "Light",
  themeDark: "Dark",
  themeSystem: "System",

  // UI language
  uiLanguage: "Interface language",
  uiLanguageSystem: "System",
  uiLanguageEs: "Español",
  uiLanguageEn: "English",

  // Settings sections
  model: "Model",
  language: "Audio language",
  languageAuto: "Auto-detect",
  diarize: "Diarization",
  diarizeHint: "Separate and label speakers (Gemini only).",

  // Local runtime (faster-whisper)
  localRuntime: "Local runtime",
  localNoKeyNeeded: "Local mode needs no API key — everything runs on your machine.",
  localRuntimeReady: (v: string) => `faster-whisper ${v} installed`,
  localRuntimeDevice: (d: string) =>
    d === "cuda" ? "GPU accelerated (CUDA)" : "Running on CPU",
  localSandboxed:
    "Local mode is unavailable in Flatpak/Snap builds, which cannot use the system Python. Use the AppImage, .deb or .rpm.",
  localPythonMissing: (cmd: string) =>
    `Python 3.9 or newer was not found. Install it with \`${cmd}\` and reopen Settings.`,
  localPythonTooOld: (v: string) =>
    `Found Python ${v}, but 3.9 or newer is required.`,
  localPythonFound: (v: string, path: string) => `Python ${v} · ${path}`,
  localNotInstalled:
    "An isolated Python environment will be created and faster-whisper installed (~250 MB). One time only.",
  localInstall: "Install local runtime",
  localInstalling: "Installing…",
  localProbing: "Checking the local runtime…",
  localInstallLog: "Install log",
  localVenvBroken:
    "The local runtime stopped working (usually after a Python upgrade). Reinstall it to fix.",
  localRepair: "Reinstall runtime",
  localModelMissing: (m: string, size: string) =>
    `Model ${m} (${size}) isn't downloaded yet. It downloads on first transcription, or you can fetch it now.`,
  localModelDownload: "Download model",
  localModelDownloading: "Downloading model…",
  localModelReady: (m: string) => `Model ${m} ready`,
  localModelEnglishOnly: "English only",
  localUninstall: "Remove local runtime",
  localUninstallModels: "Also delete downloaded models",
  localUninstallConfirm: "Remove the local runtime? You can reinstall it any time.",
  localDiskUsage: (venv: string, models: string) =>
    `Runtime: ${venv} · Models: ${models}`,

  // Local transcription stages
  localStageStarting: "Starting…",
  localStageLoadingModel: "Loading model…",
  localStageDownloadingModel: "Downloading model…",
  localStageCudaFallback: "GPU unavailable, using CPU…",
  localStageTranscribing: "Transcribing…",
  localStageFinalizing: "Finishing…",

  // Drop zone
  dropHero: "Drop your videos here",
  dropHeroOr: "or",
  dropHeroPick: "pick files",
  dropHeroOrPick: "from your computer",
  dropHeroFormats: "MP4 · MOV · AVI · MKV · WEBM · MP3 · WAV",
  dropMore: "Drop more files or click",
  dropHere: "Release to transcribe",
  unsupportedFormat: "Unsupported format",

  // Queue
  statusQueued: "Queued",
  statusProcessing: "Processing",
  statusCompleted: "Completed",
  statusError: "Error",
  transcribeBtn: "Transcribe",
  processing: "Processing files…",
  clearAll: "Clear all",

  // Transcript panel
  copy: "Copy",
  copied: "Copied",
  download: "Download",
  speakerLabel: "Speaker",

  // FFmpeg
  ffmpegMissing:
    "FFmpeg not detected on your system. Install it with `brew install ffmpeg` (macOS), `sudo apt install ffmpeg` (Debian/Ubuntu) or `winget install ffmpeg` (Windows).",

  // Filters / save dialog
  videoFilters: "Media",
  saveAs: "Save as",

  // Footer
  brand: "Simple Whisper · 2026",
  poweredBy: "Groq · Whisper",
  poweredByGemini: "Google · Gemini 3.1 Flash-Lite",

  // History
  historyTitle: "History",
  historyEmpty: "No transcriptions yet. Completed ones will appear here.",
  historyDeleteEntry: "Delete",
  historyClearAll: "Clear history",
  historyClearConfirm: "Clear all history? This cannot be undone.",
  historyCollapseAria: "Collapse history",
  historyExpandAria: "Expand history",
  historyEntryAria: (filename: string) => `View ${filename}`,
  historyJustNow: "now",
  historyMinutesAgo: (n: number) => `${n} min ago`,
  historyHoursAgo: (n: number) => `${n} h ago`,
  historyDaysAgo: (n: number) => `${n} d ago`,

  // Provider display names
  providerName: (p: string) =>
    p === "groq" ? "Groq" : p === "gemini" ? "Gemini" : p === "local" ? "Local" : p,
  providerVendor: (p: string) =>
    p === "gemini" ? "Google" : p === "local" ? "On your machine" : "Groq",

  // Transcription errors — titles
  errAuthInvalidTitle: "Invalid API key",
  errAuthForbiddenTitle: "Access denied",
  errRateLimitedTitle: "Rate limit reached",
  errQuotaExceededTitle: "Quota exhausted",
  errPayloadTooLargeTitle: "File too large",
  errBadRequestTitle: "Invalid request",
  errModelUnavailableTitle: "Model unavailable",
  errServerBusyTitle: "Service busy",
  errServerErrorTitle: "Server error",
  errNetworkTitle: "No connection",
  errTimeoutTitle: "Request timed out",
  errMalformedTitle: "Unexpected response",
  errFfmpegMissingTitle: "FFmpeg not found",
  errFfmpegFailedTitle: "FFmpeg failed",
  errApiKeyMissingTitle: "Missing API key",
  errPythonMissingTitle: "Python not found",
  errPythonTooOldTitle: "Python too old",
  errLocalRuntimeMissingTitle: "Local runtime not installed",
  errModelDownloadFailedTitle: "Model download failed",
  errLocalRuntimeFailedTitle: "Local runtime failed",
  errUnknownTitle: "Unknown error",

  // Transcription errors — bodies
  errAuthInvalidBody: (p: string) =>
    `The configured key is not accepted by ${p}. Review it in Settings.`,
  errAuthForbiddenBody: (p: string) =>
    `Your key isn't allowed to use this model on ${p}.`,
  errRateLimitedBody: (p: string, secs: number | null) =>
    secs
      ? `Too many requests to ${p}. Wait ~${secs}s and try again.`
      : `Too many requests to ${p}. Wait a few seconds and try again.`,
  errQuotaExceededBody: (p: string) =>
    `You've exhausted your ${p} quota. Switch provider or wait for the next cycle.`,
  errPayloadTooLargeBody: (p: string) =>
    `An audio chunk exceeds the ${p} size limit.`,
  errBadRequestBody: (p: string) =>
    `${p} rejected the request. Check language and model in Settings.`,
  errModelUnavailableBody: (p: string) =>
    `The selected model no longer exists on ${p}. Pick another in Settings.`,
  errServerBusyBody: (p: string) =>
    `${p} is overloaded. Retry in a few minutes or switch provider.`,
  errServerErrorBody: (p: string) =>
    `${p} returned an error. Retry or switch provider.`,
  errNetworkBody: (p: string) =>
    `Could not reach ${p}. Check your internet connection.`,
  errTimeoutBody: (p: string) => `${p} took too long to respond. Retry.`,
  errMalformedBody: (p: string) =>
    `${p} returned a response that couldn't be parsed.`,
  errFfmpegMissingBody: "Install FFmpeg and restart the app.",
  errFfmpegFailedBody:
    "FFmpeg could not process the file. Check that the file isn't corrupted.",
  errApiKeyMissingBody: (p: string) => `Configure a ${p} key in Settings.`,
  errPythonMissingBody:
    "Local mode requires Python 3.9 or newer. Install it and try again from Settings.",
  errPythonTooOldBody: "Upgrade Python to 3.9 or newer to use local mode.",
  errLocalRuntimeMissingBody:
    "Install the local runtime from Settings before transcribing offline.",
  errModelDownloadFailedBody:
    "Check your internet connection and try again. The download resumes where it stopped.",
  errLocalRuntimeFailedBody:
    "The local process exited with an error. Check the technical details.",
  errUnknownBody: "An unexpected error occurred.",

  // Error actions
  errRetry: "Retry",
  errRetryWith: (p: string) => `Retry with ${p}`,
  errShowDetails: "Technical details",
  errHideDetails: "Hide details",

  // Updates
  updates: "Updates",
  checkForUpdates: "Check for updates",
  checkingForUpdates: "Checking…",
  updateUpToDate: "You're on the latest version.",
  updateAvailableTitle: "Update available",
  updateInstall: "Install",
  updateLater: "Later",
  updateAvailableBody: (version: string, notes: string) =>
    `Version ${version} available.${notes ? `\n\n${notes}` : ""}\n\nInstall now?`,
  updateCheckFailed: "Could not check for updates.",
};
