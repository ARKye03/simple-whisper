export const es = {
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
  apiKeyBackendEncrypted: "Almacenada cifrada en este equipo",
  apiKeyBackendNone: "Sin clave guardada",
  apiKeyMissing: "Configura tu clave API en Ajustes antes de transcribir.",
  show: "Mostrar",
  hide: "Ocultar",
  closeAria: "Cerrar",

  // Theme
  theme: "Tema",
  mode: "Modo",
  themeLight: "Claro",
  themeDark: "Oscuro",
  themeSystem: "Sistema",

  // UI language
  uiLanguage: "Idioma de la interfaz",
  uiLanguageSystem: "Sistema",
  uiLanguageEs: "Español",
  uiLanguageEn: "English",

  // Settings sections
  model: "Modelo",
  language: "Idioma del audio",
  languageAuto: "Auto-detectar",
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
  speakerLabel: "Hablante",

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

  // History
  historyTitle: "Historial",
  historyEmpty: "Aún no hay transcripciones. Las que completes aparecerán aquí.",
  historyDeleteEntry: "Eliminar",
  historyClearAll: "Borrar historial",
  historyClearConfirm: "¿Borrar todo el historial? Esta acción no se puede deshacer.",
  historyCollapseAria: "Contraer historial",
  historyExpandAria: "Expandir historial",
  historyEntryAria: (filename: string) => `Ver ${filename}`,
  historyJustNow: "ahora",
  historyMinutesAgo: (n: number) => `hace ${n} min`,
  historyHoursAgo: (n: number) => `hace ${n} h`,
  historyDaysAgo: (n: number) => `hace ${n} d`,

  // Provider display names
  providerName: (p: string) => (p === "groq" ? "Groq" : p === "gemini" ? "Gemini" : p),

  // Transcription errors — titles
  errAuthInvalidTitle: "Clave API inválida",
  errAuthForbiddenTitle: "Acceso denegado",
  errRateLimitedTitle: "Límite de uso alcanzado",
  errQuotaExceededTitle: "Cuota agotada",
  errPayloadTooLargeTitle: "Archivo demasiado grande",
  errBadRequestTitle: "Solicitud inválida",
  errModelUnavailableTitle: "Modelo no disponible",
  errServerBusyTitle: "Servicio ocupado",
  errServerErrorTitle: "Error del servidor",
  errNetworkTitle: "Sin conexión",
  errTimeoutTitle: "Tiempo de espera agotado",
  errMalformedTitle: "Respuesta inesperada",
  errFfmpegMissingTitle: "FFmpeg no encontrado",
  errFfmpegFailedTitle: "FFmpeg falló",
  errApiKeyMissingTitle: "Falta clave API",
  errUnknownTitle: "Error desconocido",

  // Transcription errors — bodies (provider name interpolated)
  errAuthInvalidBody: (p: string) =>
    `La clave configurada no es aceptada por ${p}. Revísala en Ajustes.`,
  errAuthForbiddenBody: (p: string) =>
    `Tu clave no tiene permiso para usar este modelo en ${p}.`,
  errRateLimitedBody: (p: string, secs: number | null) =>
    secs
      ? `Demasiadas solicitudes a ${p}. Espera ~${secs}s e inténtalo de nuevo.`
      : `Demasiadas solicitudes a ${p}. Espera unos segundos e inténtalo de nuevo.`,
  errQuotaExceededBody: (p: string) =>
    `Has agotado tu cuota de ${p}. Cambia de proveedor o espera al próximo ciclo.`,
  errPayloadTooLargeBody: (p: string) =>
    `Un fragmento del audio supera el límite de ${p}.`,
  errBadRequestBody: (p: string) =>
    `${p} rechazó la solicitud. Revisa idioma y modelo en Ajustes.`,
  errModelUnavailableBody: (p: string) =>
    `El modelo seleccionado ya no existe en ${p}. Elige otro en Ajustes.`,
  errServerBusyBody: (p: string) =>
    `${p} está saturado. Reintenta en unos minutos o cambia de proveedor.`,
  errServerErrorBody: (p: string) =>
    `${p} devolvió un error. Reintenta o cambia de proveedor.`,
  errNetworkBody: (p: string) =>
    `No se pudo contactar a ${p}. Comprueba tu conexión a internet.`,
  errTimeoutBody: (p: string) =>
    `${p} tardó demasiado en responder. Reintenta.`,
  errMalformedBody: (p: string) =>
    `${p} devolvió una respuesta que no se pudo interpretar.`,
  errFfmpegMissingBody: "Instala FFmpeg y reinicia la app.",
  errFfmpegFailedBody:
    "FFmpeg no pudo procesar el archivo. Revisa que el archivo no esté dañado.",
  errApiKeyMissingBody: (p: string) =>
    `Configura una clave de ${p} en Ajustes.`,
  errUnknownBody: "Ocurrió un error inesperado.",

  // Error actions
  errRetry: "Reintentar",
  errRetryWith: (p: string) => `Reintentar con ${p}`,
  errShowDetails: "Detalles técnicos",
  errHideDetails: "Ocultar detalles",

  // Updates
  updates: "Actualizaciones",
  checkForUpdates: "Buscar actualizaciones",
  checkingForUpdates: "Buscando…",
  updateUpToDate: "Ya tienes la última versión.",
  updateAvailableTitle: "Actualización disponible",
  updateInstall: "Instalar",
  updateLater: "Después",
  updateAvailableBody: (version: string, notes: string) =>
    `Versión ${version} disponible.${notes ? `\n\n${notes}` : ""}\n\n¿Instalar ahora?`,
  updateCheckFailed: "No se pudo comprobar actualizaciones.",
};

export type Dict = typeof es;
