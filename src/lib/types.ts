export type FileStatus = "queued" | "processing" | "completed" | "error";

export type TranscriptSegment = { speaker: string; text: string };

export type Transcript =
  | { kind: "plain"; text: string }
  | { kind: "diarized"; segments: TranscriptSegment[]; text: string };

export type FileItem = {
  id: number;
  name: string;
  path: string;
  size: number;
  status: FileStatus;
  progress: number;
  transcript: Transcript | null;
  error: string | null;
  index: number;
};

export type HistoryProvider = "groq" | "gemini";

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
