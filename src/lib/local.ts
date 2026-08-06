import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { get, writable, type Writable } from "svelte/store";
import type { LocalInstallEvent, LocalProgressEvent } from "$lib/types";
import type { LocalModel } from "$lib/settings";

export type LocalStatus = {
  python: { path: string; version: string; major: number; minor: number } | null;
  python_too_old: boolean;
  venv_ready: boolean;
  venv_broken: boolean;
  faster_whisper: string | null;
  models: string[];
  cuda: boolean | null;
  venv_dir: string;
  models_dir: string;
  venv_bytes: number;
  models_bytes: number;
  platform: string;
  distro: string | null;
  sandboxed: boolean;
};

export type LocalState =
  | "unknown"
  | "sandboxed"
  | "python_missing"
  | "python_too_old"
  | "venv_broken"
  | "not_installed"
  | "model_missing"
  | "ready";

export type LocalBusy = "idle" | "installing" | "downloading" | "uninstalling";

export const LOCAL_MODEL_META: Record<
  LocalModel,
  { size: string; englishOnly?: boolean }
> = {
  tiny: { size: "75 MB" },
  base: { size: "145 MB" },
  small: { size: "480 MB" },
  medium: { size: "1.5 GB" },
  "large-v2": { size: "3.1 GB" },
  "large-v3": { size: "3.1 GB" },
  "large-v3-turbo": { size: "1.6 GB" },
  "distil-large-v3": { size: "1.5 GB", englishOnly: true },
};

const LOG_LIMIT = 200;

export const localStatusStore: Writable<LocalStatus | null> = writable(null);
export const localInstallLog: Writable<string[]> = writable([]);
export const localBusy: Writable<LocalBusy> = writable("idle");
export const localDetail: Writable<string | null> = writable(null);
export const localError: Writable<string | null> = writable(null);

export function localState(s: LocalStatus | null, model?: string): LocalState {
  if (!s) return "unknown";
  // A working venv wins over a since-removed system python: transcription only
  // ever uses the venv interpreter.
  if (s.venv_ready && s.faster_whisper) {
    if (model && !s.models.includes(model)) return "model_missing";
    return "ready";
  }
  if (s.sandboxed) return "sandboxed";
  if (s.venv_broken) return "venv_broken";
  if (!s.python) return "python_missing";
  if (s.python_too_old) return "python_too_old";
  return "not_installed";
}

export function localReady(s: LocalStatus | null, model: string): boolean {
  return localState(s, model) === "ready";
}

export async function refreshLocalStatus(
  withCuda = false,
): Promise<LocalStatus | null> {
  try {
    const s = await invoke<LocalStatus>("local_probe", { withCuda });
    localStatusStore.set(s);
    return s;
  } catch (e) {
    console.error("local_probe failed", e);
    return get(localStatusStore);
  }
}

function appendLog(line: string): void {
  localInstallLog.update((lines) => {
    const next = [...lines, line];
    return next.length > LOG_LIMIT ? next.slice(next.length - LOG_LIMIT) : next;
  });
}

/** Registers listeners before invoking, otherwise early events are lost. */
async function withRuntimeEvents<T>(
  busy: LocalBusy,
  run: () => Promise<T>,
): Promise<T | null> {
  localBusy.set(busy);
  localError.set(null);
  localDetail.set(null);
  const unlisten: UnlistenFn[] = [
    await listen<LocalInstallEvent>("local:install", (e) =>
      appendLog(e.payload.line),
    ),
    await listen<LocalProgressEvent>("local:progress", (e) =>
      localDetail.set(e.payload.detail),
    ),
  ];
  try {
    return await run();
  } catch (e) {
    localError.set(typeof e === "string" ? e : String(e));
    appendLog(typeof e === "string" ? e : String(e));
    return null;
  } finally {
    for (const un of unlisten) un();
    localBusy.set("idle");
    localDetail.set(null);
  }
}

export async function installLocalRuntime(): Promise<void> {
  localInstallLog.set([]);
  const s = await withRuntimeEvents("installing", () =>
    invoke<LocalStatus>("local_install"),
  );
  if (s) localStatusStore.set(s);
  else await refreshLocalStatus();
}

export async function downloadLocalModel(model: string): Promise<void> {
  const s = await withRuntimeEvents("downloading", () =>
    invoke<LocalStatus>("local_download_model", {
      model,
      jobId: crypto.randomUUID(),
    }),
  );
  if (s) localStatusStore.set(s);
  else await refreshLocalStatus();
}

export async function uninstallLocalRuntime(purgeModels: boolean): Promise<void> {
  const s = await withRuntimeEvents("uninstalling", () =>
    invoke<LocalStatus>("local_uninstall", { purgeModels }),
  );
  if (s) localStatusStore.set(s);
  else await refreshLocalStatus();
  localInstallLog.set([]);
}

export async function repairLocalRuntime(): Promise<void> {
  await uninstallLocalRuntime(false);
  await installLocalRuntime();
}

/** One install command for the user's platform / distro family. */
export function pythonInstallHint(s: LocalStatus | null): string {
  if (!s) return "";
  if (s.platform === "macos") return "brew install python";
  if (s.platform === "windows") return "winget install Python.Python.3.12";
  switch (s.distro) {
    case "debian":
      return "sudo apt install python3 python3-venv python3-pip";
    case "fedora":
      return "sudo dnf install python3 python3-pip";
    case "arch":
      return "sudo pacman -S python python-pip";
    case "suse":
      return "sudo zypper install python3 python3-pip";
    default:
      return "python3 >= 3.9";
  }
}
