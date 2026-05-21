import type { Dict } from "./es";

export const en: Dict = {
  appTitle: "Simple Whisper",

  // Header / drawer
  settings: "Settings",

  // Provider
  provider: "Provider",
  providerGroq: "Groq",
  providerGemini: "Gemini",

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
