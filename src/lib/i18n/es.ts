export const t = {
  appTitle: "Simple Whisper",

  // Header / drawer
  settings: "Ajustes",
  apiKey: "Groq API Key",
  apiKeyLabel: "Clave API de Groq",
  apiKeyPlaceholder: "gsk_xxxxxxxxxxxxxxxx",
  apiKeyHint: "Tu clave se almacena localmente y nunca sale de tu dispositivo.",
  apiKeyMissing: "Configura tu clave API de Groq en Ajustes antes de transcribir.",
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
};
