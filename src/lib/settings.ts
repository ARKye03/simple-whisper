import { invoke } from "@tauri-apps/api/core";
import { load, type Store } from "@tauri-apps/plugin-store";
import { writable, type Writable } from "svelte/store";

export type Model = "whisper-large-v3-turbo" | "whisper-large-v3";
export type Language =
  | "auto" | "es" | "en" | "fr" | "de" | "pt" | "it" | "ja" | "zh";
export type OutputFormat = "txt" | "md" | "docx" | "pdf";
export type ApiKeyBackend = "keychain" | "store" | "none";

export type AppSettings = {
  apiKey: string;
  model: Model;
  language: Language;
  format: OutputFormat;
};

const STORE_FILE = "settings.json";
const API_KEY_LEGACY = "groq_api_key";
const API_KEY_FALLBACK = "groq_api_key_fallback";
const MODEL_KEY = "model";
const LANG_KEY = "language";
const FORMAT_KEY = "format";

const DEFAULTS: AppSettings = {
  apiKey: "",
  model: "whisper-large-v3-turbo",
  language: "auto",
  format: "docx",
};

let cachedStore: Store | null = null;

async function getStore(): Promise<Store> {
  if (!cachedStore) {
    cachedStore = await load(STORE_FILE, { autoSave: true, defaults: {} });
  }
  return cachedStore;
}

async function loadApiKey(): Promise<{ key: string; backend: ApiKeyBackend }> {
  try {
    const k = await invoke<string>("keychain_get_api_key");
    if (k) return { key: k, backend: "keychain" };
  } catch {
    // keychain unavailable; fall through to store
  }
  const s = await getStore();
  const fallback = (await s.get<string>(API_KEY_FALLBACK)) ?? "";
  return { key: fallback, backend: fallback ? "store" : "none" };
}

async function saveApiKey(key: string): Promise<ApiKeyBackend> {
  const trimmed = key.trim();
  try {
    await invoke("keychain_set_api_key", { key: trimmed });
    const s = await getStore();
    if ((await s.get<string>(API_KEY_FALLBACK)) !== undefined) {
      await s.delete(API_KEY_FALLBACK);
    }
    return trimmed ? "keychain" : "none";
  } catch {
    const s = await getStore();
    if (trimmed) {
      await s.set(API_KEY_FALLBACK, trimmed);
      return "store";
    } else {
      if ((await s.get<string>(API_KEY_FALLBACK)) !== undefined) {
        await s.delete(API_KEY_FALLBACK);
      }
      return "none";
    }
  }
}

export async function getApiKey(): Promise<string> {
  return (await loadApiKey()).key;
}

export async function setApiKey(key: string): Promise<ApiKeyBackend> {
  const backend = await saveApiKey(key);
  apiKeyBackend.set(backend);
  return backend;
}

export async function getSettings(): Promise<AppSettings> {
  const store = await getStore();
  const k = await loadApiKey();
  apiKeyBackend.set(k.backend);
  return {
    apiKey: k.key,
    model: (await store.get<Model>(MODEL_KEY)) ?? DEFAULTS.model,
    language: (await store.get<Language>(LANG_KEY)) ?? DEFAULTS.language,
    format: (await store.get<OutputFormat>(FORMAT_KEY)) ?? DEFAULTS.format,
  };
}

async function updateSettings(patch: Partial<AppSettings>): Promise<void> {
  const store = await getStore();
  if (patch.apiKey !== undefined) await setApiKey(patch.apiKey);
  if (patch.model !== undefined) await store.set(MODEL_KEY, patch.model);
  if (patch.language !== undefined) await store.set(LANG_KEY, patch.language);
  if (patch.format !== undefined) await store.set(FORMAT_KEY, patch.format);
}

async function migrateLegacyApiKey(): Promise<void> {
  const store = await getStore();
  const legacy = await store.get<string>(API_KEY_LEGACY);
  if (typeof legacy === "string" && legacy.trim()) {
    await saveApiKey(legacy);
  }
  if (legacy !== undefined) {
    await store.delete(API_KEY_LEGACY);
  }
}

export const settingsStore: Writable<AppSettings> = writable(DEFAULTS);
export const apiKeyBackend: Writable<ApiKeyBackend> = writable("none");

let initialized = false;
export async function initSettings(): Promise<void> {
  if (initialized) return;
  await migrateLegacyApiKey();
  const s = await getSettings();
  settingsStore.set(s);
  initialized = true;
}

export async function patchSettings(p: Partial<AppSettings>): Promise<void> {
  const apiKey = p.apiKey !== undefined ? p.apiKey.trim() : undefined;
  settingsStore.update((s) => ({ ...s, ...p, ...(apiKey !== undefined ? { apiKey } : {}) }));
  await updateSettings(p);
}

export function isValidGroqKey(key: string): boolean {
  return /^gsk_[A-Za-z0-9]{20,}$/.test(key.trim());
}
