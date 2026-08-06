export const es = {
  appTitle: "Simple Whisper",

  // Header / drawer
  settings: "Ajustes",

  // Provider
  provider: "Proveedor",
  providerGroq: "Groq",
  providerGemini: "Gemini",
  providerLocal: "Local",

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

  // Local runtime (faster-whisper)
  localRuntime: "Entorno local",
  localNoKeyNeeded: "El modo local no necesita clave API: todo se procesa en tu equipo.",
  localRuntimeReady: (v: string) => `faster-whisper ${v} instalado`,
  localRuntimeDevice: (d: string): string =>
    d === "cuda" ? "Acelerado por GPU (CUDA)" : "Procesando en CPU",
  localSandboxed:
    "El modo local no está disponible en compilaciones Flatpak/Snap, porque no pueden usar el Python del sistema. Usa el AppImage, .deb o .rpm.",
  localPythonMissing: (cmd: string) =>
    `No se encontró Python 3.9 o superior. Instálalo con \`${cmd}\` y vuelve a abrir Ajustes.`,
  localPythonTooOld: (v: string) =>
    `Se encontró Python ${v}, pero se necesita 3.9 o superior.`,
  localPythonFound: (v: string, path: string) => `Python ${v} · ${path}`,
  localNotInstalled:
    "Se creará un entorno de Python aislado y se instalará faster-whisper (~250 MB). Solo se hace una vez.",
  localInstall: "Instalar entorno local",
  localInstalling: "Instalando…",
  localProbing: "Comprobando el entorno local…",
  localInstallLog: "Registro de instalación",
  localVenvBroken:
    "El entorno local dejó de funcionar (suele pasar al actualizar Python). Reinstálalo para arreglarlo.",
  localRepair: "Reinstalar entorno",
  localModelMissing: (m: string, size: string) =>
    `El modelo ${m} (${size}) aún no está descargado. Se descargará al transcribir, o puedes hacerlo ahora.`,
  localModelDownload: "Descargar modelo",
  localModelDownloading: "Descargando modelo…",
  localModelReady: (m: string) => `Modelo ${m} listo`,
  localModelEnglishOnly: "solo inglés",
  localUninstall: "Eliminar entorno local",
  localUninstallModels: "Eliminar también los modelos descargados",
  localUninstallConfirm:
    "¿Eliminar el entorno local? Podrás reinstalarlo cuando quieras.",
  localDiskUsage: (venv: string, models: string) =>
    `Entorno: ${venv} · Modelos: ${models}`,

  // Local transcription stages
  localStageStarting: "Iniciando…",
  localStageLoadingModel: "Cargando modelo…",
  localStageDownloadingModel: "Descargando modelo…",
  localStageCudaFallback: "GPU no disponible, usando CPU…",
  localStageTranscribing: "Transcribiendo…",
  localStageFinalizing: "Finalizando…",

  // Drop zone
  dropHero: "Arrastra tus videos aquí",
  dropHeroOr: "o",
  dropHeroPick: "selecciona archivos",
  dropHeroOrPick: "desde tu equipo",
  dropHeroFormats: "MP4 · MOV · AVI · MKV · WEBM · MP3 · WAV",
  dropMore: "Arrastra más archivos o haz clic",
  dropHere: "Soltar para transcribir",
  unsupportedFormat: "Formato no soportado",

  // URL ingestion
  orPasteLink: "o pega un enlace",
  urlInputLabel: "Enlace de video",
  urlInputPlaceholder: "https://www.youtube.com/watch?v=…",
  urlInputSubmit: "Añadir",
  urlInputSubmitAria: "Añadir enlace a la cola",
  urlInputHint:
    "Pega uno o varios enlaces y pulsa Enter. También puedes pegar en cualquier momento.",
  urlInvalid: "Enlace no válido. Usa una dirección http o https.",
  urlResolving: "Obteniendo información…",
  urlAddedOne: "Enlace añadido a la cola",
  urlAddedCount: (n: number) => `${n} enlaces añadidos a la cola`,

  // Playlist expansion
  playlistConfirmTitle: "Lista de reproducción",
  playlistConfirmBody: (shown: number, total: number) =>
    shown < total
      ? `Esta lista tiene ${total} videos. Se añadirán los primeros ${shown} a la cola y se transcribirán uno por uno. ¿Continuar?`
      : `Esta lista tiene ${shown} videos. Se añadirán todos a la cola y se transcribirán uno por uno. ¿Continuar?`,
  playlistConfirmOk: "Añadir",
  playlistCapNotice: (shown: number, total: number) =>
    `Se añadieron ${shown} de ${total} videos`,
  playlistExpanded: (n: number) => `${n} videos añadidos a la cola`,

  // Queue
  statusQueued: "En cola",
  statusDownloading: "Descargando",
  statusProcessing: "Procesando",
  statusCompleted: "Completado",
  statusError: "Error",
  transcribeBtn: "Transcribir",
  processing: "Procesando…",
  clearAll: "Limpiar todo",
  remove: "Quitar",
  downloadEta: (eta: string) => `faltan ${eta}`,
  downloadProgressAria: (pct: number) => `Descargando audio, ${pct} por ciento`,

  // Transcript panel
  copy: "Copiar",
  copied: "Copiado",
  download: "Descargar",
  speakerLabel: "Hablante",

  // FFmpeg
  ffmpegMissing:
    "FFmpeg no detectado en el sistema. Instálalo con `brew install ffmpeg` (macOS), `sudo apt install ffmpeg` (Debian/Ubuntu) o `winget install ffmpeg` (Windows).",

  // Downloads (yt-dlp)
  downloads: "Descargas",
  ytdlpChecking: "Comprobando yt-dlp…",
  ytdlpFound: (v: string) => `yt-dlp ${v} detectado`,
  ytdlpNotFound: "yt-dlp no encontrado",
  ytdlpUnknown: "yt-dlp sin comprobar",
  ytdlpRecheck: "Comprobar",
  ytdlpOptional: "Opcional. Solo se necesita para transcribir enlaces.",
  ytdlpInstallHint:
    "Instálalo con `brew install yt-dlp` (macOS), `pipx install yt-dlp` (Linux) o `winget install yt-dlp` (Windows).",
  cookiesBrowser: "Cookies del navegador",
  cookiesBrowserNone: "No usar cookies",
  cookiesBrowserHint:
    "Algunos videos exigen sesión. yt-dlp puede leer las cookies del navegador que elijas; se usan solo en tu equipo.",
  cookiesBrowserSafariHint:
    "En macOS, Safari exige conceder Acceso total al disco a la app.",

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
  historySourceUrlBadge: "Enlace",
  historyOpenSource: "Abrir enlace original",

  // Provider display names
  providerName: (p: string) =>
    p === "groq" ? "Groq" : p === "gemini" ? "Gemini" : p === "local" ? "Local" : p,
  providerVendor: (p: string): string =>
    p === "gemini" ? "Google" : p === "local" ? "En tu equipo" : "Groq",

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
  errYtdlpMissingTitle: "yt-dlp no encontrado",
  errDownloadFailedTitle: "No se pudo descargar el audio",
  errUnsupportedUrlTitle: "Enlace no compatible",
  errDownloadAuthRequiredTitle: "El enlace requiere iniciar sesión",
  errLiveUnsupportedTitle: "Transmisión en directo",
  errApiKeyMissingTitle: "Falta clave API",
  errPythonMissingTitle: "Python no encontrado",
  errPythonTooOldTitle: "Python demasiado antiguo",
  errLocalRuntimeMissingTitle: "Entorno local no instalado",
  errModelDownloadFailedTitle: "No se pudo descargar el modelo",
  errLocalRuntimeFailedTitle: "El entorno local falló",
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
  errYtdlpMissingBody:
    "Para transcribir enlaces necesitas yt-dlp. Instálalo con `brew install yt-dlp` (macOS), `pipx install yt-dlp` (Linux) o `winget install yt-dlp` (Windows).",
  errDownloadFailedBody:
    "yt-dlp no pudo descargar este enlace. Puede que el video sea privado, se haya eliminado o que yt-dlp esté desactualizado.",
  errUnsupportedUrlBody:
    "yt-dlp no reconoce este enlace. Prueba con la dirección directa del video.",
  errDownloadAuthRequiredBody:
    "Este video pide sesión o tiene restricción de edad. Elige tu navegador en Ajustes › Descargas para usar sus cookies.",
  errLiveUnsupportedBody:
    "No se pueden transcribir transmisiones en directo. Espera a que termine y usa el enlace de la grabación.",
  errApiKeyMissingBody: (p: string) =>
    `Configura una clave de ${p} en Ajustes.`,
  errPythonMissingBody:
    "El modo local necesita Python 3.9 o superior. Instálalo y vuelve a intentarlo desde Ajustes.",
  errPythonTooOldBody:
    "Actualiza Python a la versión 3.9 o superior para usar el modo local.",
  errLocalRuntimeMissingBody:
    "Instala el entorno local desde Ajustes antes de transcribir sin conexión.",
  errModelDownloadFailedBody:
    "Comprueba tu conexión a internet y vuelve a intentarlo. La descarga se reanuda donde quedó.",
  errLocalRuntimeFailedBody:
    "El proceso local terminó con error. Revisa los detalles técnicos.",
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
