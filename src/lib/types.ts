export type FileStatus = "queued" | "processing" | "completed" | "error";

export type Provider = "groq" | "gemini" | "local";

export const PROVIDERS = ["groq", "gemini", "local"] as const satisfies readonly Provider[];

/** Providers that require an API key. "local" runs on the user's machine. */
export const KEYED_PROVIDERS = ["groq", "gemini"] as const satisfies readonly Provider[];

export type LocalStage =
  | "starting"
  | "loading_model"
  | "downloading_model"
  | "cuda_fallback"
  | "transcribing"
  | "finalizing";

export type LocalProgressEvent = {
  job_id: string;
  stage: LocalStage;
  progress: number | null;
  detail: string | null;
};

export type LocalInstallEvent = { stream: string; line: string };

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
  | "ApiKeyMissing"
  | "PythonMissing"
  | "PythonTooOld"
  | "LocalRuntimeMissing"
  | "ModelDownloadFailed"
  | "LocalRuntimeFailed"
  | "Unknown";

export type TranscribeError = {
  kind: TranscribeErrorKind;
  provider: Provider | null;
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

export type FileItem = {
  id: number;
  name: string;
  path: string;
  size: number;
  status: FileStatus;
  progress: number;
  transcript: Transcript | null;
  error: TranscribeError | null;
  index: number;
  stage?: LocalStage | null;
  stageDetail?: string | null;
};

export type HistoryProvider = Provider;

export type HistoryEntry = {
  id: string;
  createdAt: number;
  filename: string;
  sourcePath: string | null;
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
