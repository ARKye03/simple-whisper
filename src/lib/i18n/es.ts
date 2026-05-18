export const t = {
  appTitle: "Simple Whisper",

  // Header / drawer
  settings: "Ajustes",

  // Provider
  provider: "Proveedor",
  providerGroq: "Groq",
  providerGemini: "Gemini",

  // API key (shared scaffold; provider-specific labels below)
  apiKey: "API Key",
  apiKeyLabel: "Clave API",
  apiKeyLabelGroq: "Clave API de Groq",
  apiKeyLabelGemini: "Clave API de Google AI Studio",
  apiKeyPlaceholder: "",
  apiKeyPlaceholderGroq: "gsk_xxxxxxxxxxxxxxxx",
  apiKeyPlaceholderGemini: "AIzaxxxxxxxxxxxxxxxx",
  apiKeyHint: "Tu clave se almacena localmente y nunca sale de tu dispositivo.",
  apiKeySaved: "Clave guardada",
  apiKeySaving: "Guardando…",
  apiKeyInvalid: "Formato no válido",
  apiKeyInvalidGroq: "Formato no válido (debe empezar con gsk_)",
  apiKeyInvalidGemini: "Formato no válido (debe empezar con AIza)",
  apiKeyValid: "Formato válido",
  apiKeyBackendKeychain: "Almacenada en el Llavero del sistema",
  apiKeyBackendStore: "Almacenada en archivo local (Llavero no disponible)",
  apiKeyBackendNone: "Sin clave guardada",
  apiKeyMissing: "Configura tu clave API en Ajustes antes de transcribir.",
  show: "Mostrar",
  hide: "Ocultar",

  // Theme
  theme: "Tema",
  mode: "Modo",
  themeLight: "Claro",
  themeDark: "Oscuro",
  themeSystem: "Sistema",

  // Settings sections
  model: "Modelo",
  language: "Idioma del audio",
  languageAuto: "Auto-detectar",
  outputFormat: "Formato de salida",
  pdfUnavailableForLang: "PDF no disponible para japonés/chino (la fuente integrada no incluye esos glifos).",
  diarize: "Diarización",
  diarizeHint: "Separar e identificar hablantes (solo Gemini).",

  // Drop zone
  dropHero: "Arrastra tus videos aquí",
  dropHeroOr: "o",
  dropHeroPick: "selecciona archivos",
  dropHeroOrPick: "desde tu equipo",
  dropHeroFormats: "MP4 · MOV · AVI · MKV · WEBM · MP3 · WAV",
  dropMore: "Arrastra más archivos o haz clic",
  dropHere: "Soltar para transcribir",
  unsupportedFormat: "Formato no soportado",

  // Queue
  statusQueued: "En cola",
  statusProcessing: "Procesando",
  statusCompleted: "Completado",
  statusError: "Error",
  transcribeBtn: "Transcribir",
  processing: "Procesando archivos…",
  clearAll: "Limpiar todo",

  // Transcript panel
  copy: "Copiar",
  copied: "Copiado",
  download: "Descargar",

  // FFmpeg
  ffmpegMissing:
    "FFmpeg no detectado en el sistema. Instálalo con `brew install ffmpeg` (macOS), `sudo apt install ffmpeg` (Debian/Ubuntu) o `winget install ffmpeg` (Windows).",

  // Filters / save dialog
  videoFilters: "Media",
  saveAs: "Guardar como",

  // Footer
  brand: "Simple Whisper · 2026",
  poweredBy: "Groq · Whisper",
  poweredByGemini: "Google · Gemini 3.1 Flash-Lite",
};
