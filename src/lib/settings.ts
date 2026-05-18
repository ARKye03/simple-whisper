import { invoke } from "@tauri-apps/api/core";
import { load, type Store } from "@tauri-apps/plugin-store";
import { writable, type Writable } from "svelte/store";

export type Provider = "groq" | "gemini";
export type GroqModel = "whisper-large-v3-turbo" | "whisper-large-v3";
export type GeminiModel = "gemini-3.1-flash-lite" | "gemini-3-flash-preview";
export type Model = GroqModel | GeminiModel;
export type Language =
  | "auto" | "es" | "en" | "fr" | "de" | "pt" | "it" | "ja" | "zh";
export type ApiKeyBackend = "keychain" | "store" | "none";

export type AppSettings = {
  provider: Provider;
  groqApiKey: string;
  geminiApiKey: string;
  groqModel: GroqModel;
  geminiModel: GeminiModel;
  language: Language;
  diarize: boolean;
};

const STORE_FILE = "settings.json";
const API_KEY_LEGACY = "groq_api_key";
const PROVIDER_KEY = "provider";
const GROQ_MODEL_KEY = "groq_model";
const GEMINI_MODEL_KEY = "gemini_model";
const LANG_KEY = "language";
const DIARIZE_KEY = "diarize";

const DEFAULTS: AppSettings = {
  provider: "groq",
  groqApiKey: "",
  geminiApiKey: "",
  groqModel: "whisper-large-v3-turbo",
  geminiModel: "gemini-3.1-flash-lite",
  language: "auto",
  diarize: true,
};

let cachedStore: Store | null = null;

async function getStore(): Promise<Store> {
  if (!cachedStore) {
    cachedStore = await load(STORE_FILE, { autoSave: true, defaults: {} });
  }
  return cachedStore;
}

function fallbackKeyFor(provider: Provider): string {
  return `${provider}_api_key_fallback`;
}

async function loadApiKey(
  provider: Provider,
): Promise<{ key: string; backend: ApiKeyBackend }> {
  try {
    const k = await invoke<string>("keychain_get_api_key", { provider });
    if (k) return { key: k, backend: "keychain" };
  } catch {
    // keychain unavailable; fall through to store
  }
  const s = await getStore();
  const fallback = (await s.get<string>(fallbackKeyFor(provider))) ?? "";
  return { key: fallback, backend: fallback ? "store" : "none" };
}

async function saveApiKey(
  provider: Provider,
  key: string,
): Promise<ApiKeyBackend> {
  const trimmed = key.trim();
  const fbKey = fallbackKeyFor(provider);
  try {
    await invoke("keychain_set_api_key", { provider, key: trimmed });
    const s = await getStore();
    if ((await s.get<string>(fbKey)) !== undefined) {
      await s.delete(fbKey);
    }
    return trimmed ? "keychain" : "none";
  } catch {
    const s = await getStore();
    if (trimmed) {
      await s.set(fbKey, trimmed);
      return "store";
    } else {
      if ((await s.get<string>(fbKey)) !== undefined) {
        await s.delete(fbKey);
      }
      return "none";
    }
  }
}

export function activeApiKey(s: AppSettings): string {
  return s.provider === "groq" ? s.groqApiKey : s.geminiApiKey;
}

export function activeModel(s: AppSettings): Model {
  return s.provider === "groq" ? s.groqModel : s.geminiModel;
}

export function activeIsValidKey(s: AppSettings): boolean {
  const key = activeApiKey(s);
  if (!key) return false;
  return s.provider === "groq" ? isValidGroqKey(key) : isValidGeminiKey(key);
}

export async function getApiKey(provider: Provider): Promise<string> {
  return (await loadApiKey(provider)).key;
}

export async function setApiKey(
  provider: Provider,
  key: string,
): Promise<ApiKeyBackend> {
  const backend = await saveApiKey(provider, key);
  apiKeyBackend.set(backend);
  return backend;
}

export async function getSettings(): Promise<AppSettings> {
  const store = await getStore();
  const provider = (await store.get<Provider>(PROVIDER_KEY)) ?? DEFAULTS.provider;
  const groq = await loadApiKey("groq");
  const gemini = await loadApiKey("gemini");
  const activeBackend = provider === "groq" ? groq.backend : gemini.backend;
  apiKeyBackend.set(activeBackend);
  return {
    provider,
    groqApiKey: groq.key,
    geminiApiKey: gemini.key,
    groqModel:
      (await store.get<GroqModel>(GROQ_MODEL_KEY)) ?? DEFAULTS.groqModel,
    geminiModel:
      (await store.get<GeminiModel>(GEMINI_MODEL_KEY)) ?? DEFAULTS.geminiModel,
    language: (await store.get<Language>(LANG_KEY)) ?? DEFAULTS.language,
    diarize: (await store.get<boolean>(DIARIZE_KEY)) ?? DEFAULTS.diarize,
  };
}

async function updateSettings(patch: Partial<AppSettings>): Promise<void> {
  const store = await getStore();
  if (patch.provider !== undefined) await store.set(PROVIDER_KEY, patch.provider);
  if (patch.groqApiKey !== undefined) await setApiKey("groq", patch.groqApiKey);
  if (patch.geminiApiKey !== undefined) await setApiKey("gemini", patch.geminiApiKey);
  if (patch.groqModel !== undefined) await store.set(GROQ_MODEL_KEY, patch.groqModel);
  if (patch.geminiModel !== undefined) await store.set(GEMINI_MODEL_KEY, patch.geminiModel);
  if (patch.language !== undefined) await store.set(LANG_KEY, patch.language);
  if (patch.diarize !== undefined) await store.set(DIARIZE_KEY, patch.diarize);
}

async function migrateLegacyApiKey(): Promise<void> {
  const store = await getStore();
  const legacy = await store.get<string>(API_KEY_LEGACY);
  if (typeof legacy === "string" && legacy.trim()) {
    const current = await loadApiKey("groq").catch(
      () => ({ key: "", backend: "none" as const }),
    );
    if (!current.key) {
      await saveApiKey("groq", legacy);
    }
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
  const groqTrim =
    p.groqApiKey !== undefined ? p.groqApiKey.trim() : undefined;
  const geminiTrim =
    p.geminiApiKey !== undefined ? p.geminiApiKey.trim() : undefined;
  settingsStore.update((s) => ({
    ...s,
    ...p,
    ...(groqTrim !== undefined ? { groqApiKey: groqTrim } : {}),
    ...(geminiTrim !== undefined ? { geminiApiKey: geminiTrim } : {}),
  }));
  // When provider changes, surface that provider's key-backend status in the UI.
  if (p.provider !== undefined) {
    const k = await loadApiKey(p.provider);
    apiKeyBackend.set(k.backend);
  }
  await updateSettings(p);
}

export function isValidGroqKey(key: string): boolean {
  return /^gsk_[A-Za-z0-9]{20,}$/.test(key.trim());
}

export function isValidGeminiKey(key: string): boolean {
  return /^AIza[\w-]{20,}$/.test(key.trim());
}
