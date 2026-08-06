export type FileStatus = "queued" | "downloading" | "processing" | "completed" | "error";

export type FileSource = "file" | "url";

export type TranscriptSegment = { speaker: string; text: string };

export type Transcript =
  | { kind: "plain"; text: string }
  | { kind: "diarized"; segments: TranscriptSegment[]; text: string };

export type TranscribeErrorKind =
  | "AuthInvalid"
  | "AuthForbidden"
  | "RateLimited"
  | "QuotaExceeded"
  | "PayloadTooLarge"
  | "BadRequest"
  | "ModelUnavailable"
  | "ServerBusy"
  | "ServerError"
  | "Network"
  | "Timeout"
  | "MalformedResponse"
  | "FfmpegMissing"
  | "FfmpegFailed"
  | "YtdlpMissing"
  | "DownloadFailed"
  | "UnsupportedUrl"
  | "DownloadAuthRequired"
  | "LiveUnsupported"
  | "ApiKeyMissing"
  | "Unknown";

export type TranscribeError = {
  kind: TranscribeErrorKind;
  provider: "groq" | "gemini" | null;
  message: string;
  retry_after_secs: number | null;
  raw: string | null;
};

export function isTranscribeError(v: unknown): v is TranscribeError {
  if (!v || typeof v !== "object") return false;
  const o = v as Record<string, unknown>;
  return typeof o.kind === "string" && typeof o.message === "string";
}

export function toTranscribeError(e: unknown): TranscribeError {
  if (isTranscribeError(e)) return e;
  return {
    kind: "Unknown",
    provider: null,
    message: typeof e === "string" ? e : String(e),
    retry_after_secs: null,
    raw: typeof e === "string" ? e : null,
  };
}

export function localTranscribeError(
  kind: TranscribeErrorKind,
  message: string,
  provider: "groq" | "gemini" | null = null,
): TranscribeError {
  return { kind, provider, message, retry_after_secs: null, raw: null };
}

/** Wire shape of `probe_url` — snake_case, mirrors the Rust struct exactly. */
export type UrlEntry = {
  url: string;
  title: string | null;
  duration_secs: number | null;
};

export type UrlProbe = {
  is_playlist: boolean;
  title: string;
  duration_secs: number | null;
  uploader: string | null;
  extractor: string | null;
  is_live: boolean;
  thumbnail: string | null;
  webpage_url: string;
  entry_count: number | null;
  entries: UrlEntry[];
};

/** Channel payload streamed by `transcribe_url`. */
export type DownloadProgress = {
  stage: "download" | "extract" | "transcribe";
  percent: number;
  downloaded_bytes: number | null;
  total_bytes: number | null;
  speed_bps: number | null;
  eta_secs: number | null;
  done: boolean;
};

/** In-app camelCase projection of UrlProbe, so wire naming stays out of markup. */
export type UrlMeta = {
  title: string | null;
  durationSecs: number | null;
  uploader: string | null;
  extractor: string | null;
};

export function urlMetaFrom(p: UrlProbe): UrlMeta {
  return {
    title: p.title?.trim() || null,
    durationSecs: p.duration_secs,
    uploader: p.uploader?.trim() || null,
    extractor: p.extractor,
  };
}

export type FileItem = {
  id: number;
  name: string;
  /** fs path for files, normalized URL for links — the dedup/identity key. */
  path: string;
  size: number;
  status: FileStatus;
  progress: number;
  transcript: Transcript | null;
  error: TranscribeError | null;
  index: number;
  source: FileSource;
  /** Original URL as entered — what yt-dlp actually receives. */
  url?: string;
  meta?: UrlMeta | null;
  probing?: boolean;
  download?: DownloadProgress | null;
};

export type HistoryProvider = "groq" | "gemini";

export type HistoryEntry = {
  id: string;
  createdAt: number;
  filename: string;
  sourcePath: string | null;
  sourceUrl: string | null;
  provider: HistoryProvider;
  model: string;
  language: string;
  diarize: boolean;
  transcript: Transcript;
};

export const MEDIA_EXT = [
  "mp4", "mov", "mkv", "webm", "avi", "m4v", "wmv", "flv",
  "mp3", "wav", "m4a", "flac", "ogg",
];

export function fileNameOf(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] ?? path;
}

export function extOf(path: string): string {
  const m = path.match(/\.([^.\\/]+)$/);
  return m ? m[1].toLowerCase() : "";
}

export function fmtSize(b: number): string {
  if (!b || b <= 0) return "—";
  if (b > 1e9) return (b / 1e9).toFixed(1) + " GB";
  if (b > 1e6) return (b / 1e6).toFixed(1) + " MB";
  if (b > 1e3) return (b / 1e3).toFixed(0) + " KB";
  return b + " B";
}

export function transcriptToPlainText(t: Transcript): string {
  if (t.kind === "plain") return t.text;
  return t.text;
}

/** Split a pasted blob into candidate tokens (newline- or space-separated lists). */
export function parseUrlCandidates(text: string): string[] {
  // JS \s already covers non-breaking space and friends.
  return text
    .split(/\s+/)
    .map((s) => s.trim().replace(/[),.;:'"\]]+$/, ""))
    .filter(Boolean);
}

/** http/https only — rejects javascript:, data:, file: and bare hostnames. */
export function isSupportedMediaUrl(raw: string): boolean {
  let u: URL;
  try {
    u = new URL(raw);
  } catch {
    return false;
  }
  if (u.protocol !== "http:" && u.protocol !== "https:") return false;
  return u.hostname.includes(".") && !u.hostname.endsWith(".");
}

/**
 * Identity key for dedup only. Never sent to the backend — normalization must not
 * be able to break the URL that actually gets fetched.
 */
export function urlDedupKey(raw: string): string {
  try {
    const u = new URL(raw);
    u.hash = "";
    u.hostname = u.hostname.toLowerCase().replace(/^www\./, "");
    for (const p of [
      "utm_source", "utm_medium", "utm_campaign", "utm_term", "utm_content",
      "si", "feature", "pp",
    ]) {
      u.searchParams.delete(p);
    }
    return u.toString().replace(/\/$/, "");
  } catch {
    return raw;
  }
}

/** Optimistic display name until probe_url lands. URLs must not go through fileNameOf. */
export function hostLabelOf(raw: string): string {
  try {
    return new URL(raw).hostname.replace(/^www\./, "");
  } catch {
    return raw;
  }
}

export function fmtDuration(secs: number | null): string {
  if (!secs || secs <= 0) return "—";
  const s = Math.round(secs);
  const pad = (n: number) => String(n).padStart(2, "0");
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = s % 60;
  return h > 0 ? `${h}:${pad(m)}:${pad(r)}` : `${m}:${pad(r)}`;
}

export function fmtSpeed(bps: number | null): string | null {
  if (!bps || bps <= 0) return null;
  if (bps > 1e6) return (bps / 1e6).toFixed(1) + " MB/s";
  if (bps > 1e3) return (bps / 1e3).toFixed(0) + " KB/s";
  return Math.round(bps) + " B/s";
}

export function fmtEta(secs: number | null): string | null {
  if (secs === null || secs < 0) return null;
  const s = Math.round(secs);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ${String(s % 60).padStart(2, "0")}s`;
  return `${Math.floor(m / 60)}h ${String(m % 60).padStart(2, "0")}m`;
}

/** yt-dlp titles contain / : ? * — sanitize before using as a save-dialog path. */
export function safeFileName(name: string): string {
  // Allowlist rather than a control-char range: keeps unicode letters/digits and a
  // few separators, collapses everything else (/ : ? * " < > |, controls) to "_".
  return (
    name
      .replace(/[^\p{L}\p{N} ._-]+/gu, "_")
      .replace(/\s+/g, " ")
      .trim()
      .slice(0, 120) || "transcript"
  );
}
