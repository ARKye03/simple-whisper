import { invoke } from "@tauri-apps/api/core";
import { writable, get, type Writable } from "svelte/store";

export type YtdlpState = "unknown" | "checking" | "ok" | "missing";
export type YtdlpStatus = { state: YtdlpState; version: string | null };

export const ytdlpStore: Writable<YtdlpStatus> = writable({
  state: "unknown",
  version: null,
});

let inFlight: Promise<YtdlpStatus> | null = null;

/** Always re-probes — for the "Comprobar" button, or after installing mid-session. */
export async function refreshYtdlp(): Promise<YtdlpStatus> {
  if (inFlight) return inFlight;
  ytdlpStore.update((s) => ({ ...s, state: "checking" }));
  // check_ytdlp returns Result<String, String>, so any rejection just means "not
  // usable" — there is nothing to classify.
  const p = (async (): Promise<YtdlpStatus> => {
    try {
      const version = await invoke<string>("check_ytdlp");
      return { state: "ok", version: version.trim() || null };
    } catch {
      return { state: "missing", version: null };
    }
  })();
  inFlight = p;
  try {
    const next = await p;
    ytdlpStore.set(next);
    return next;
  } finally {
    inFlight = null;
  }
}

/**
 * Probes at most once per session. yt-dlp is optional, so this is called the first
 * time a URL is queued rather than at boot.
 */
export async function ensureYtdlp(): Promise<YtdlpStatus> {
  const cur = get(ytdlpStore);
  if (cur.state === "ok" || cur.state === "missing") return cur;
  return refreshYtdlp();
}
