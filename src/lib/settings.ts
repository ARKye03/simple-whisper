import { invoke } from "@tauri-apps/api/core";
import { load, type Store } from "@tauri-apps/plugin-store";
import { writable, type Writable } from "svelte/store";
import { applyUiLanguage, type UiLanguagePref } from "$lib/i18n/state.svelte";

export type Provider = "groq" | "gemini";
export type GroqModel = "whisper-large-v3-turbo" | "whisper-large-v3";
export type GeminiModel = "gemini-3.1-flash-lite" | "gemini-3-flash-preview";
export type Model = GroqModel | GeminiModel;
export type Language =
  | "auto" | "es" | "en" | "fr" | "de" | "pt" | "it" | "ja" | "zh";
export type ApiKeyBackend = "encrypted" | "none";

export type AppSettings = {
  provider: Provider;
  groqApiKey: string;
  geminiApiKey: string;
  groqModel: GroqModel;
  geminiModel: GeminiModel;
  language: Language;
  diarize: boolean;
  uiLanguage: UiLanguagePref;
  historySidebarCollapsed: boolean;
};

const STORE_FILE = "settings.json";
const API_KEY_LEGACY = "groq_api_key";
const PROVIDER_KEY = "provider";
const GROQ_MODEL_KEY = "groq_model";
const GEMINI_MODEL_KEY = "gemini_model";
const LANG_KEY = "language";
const DIARIZE_KEY = "diarize";
const UI_LANG_KEY = "ui_language";
const HISTORY_COLLAPSED_KEY = "history_sidebar_collapsed";
const MIGRATION_FLAG = "secrets_migrated_v2";

const DEFAULTS: AppSettings = {
  provider: "groq",
  groqApiKey: "",
  geminiApiKey: "",
  groqModel: "whisper-large-v3-turbo",
  geminiModel: "gemini-3.1-flash-lite",
  language: "auto",
  diarize: true,
  uiLanguage: "system",
  historySidebarCollapsed: false,
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

async function loadApiKey(provider: Provider): Promise<string> {
  try {
    return await invoke<string>("secret_get", { provider });
  } catch {
    return "";
  }
}

async function saveApiKey(provider: Provider, key: string): Promise<void> {
  await invoke("secret_set", { provider, value: key.trim() });
}

export function activeApiKey(s: AppSettings): string {
  return s.provider === "groq" ? s.groqApiKey : s.geminiApiKey;
}

export function activeModel(s: AppSettings): Model {
  return s.provider === "groq" ? s.groqModel : s.geminiModel;
}

export function otherProviderWithKey(s: AppSettings): Provider | null {
  const other: Provider = s.provider === "groq" ? "gemini" : "groq";
  const key = other === "groq" ? s.groqApiKey : s.geminiApiKey;
  return key ? other : null;
}

export function activeIsValidKey(s: AppSettings): boolean {
  const key = activeApiKey(s);
  if (!key) return false;
  return s.provider === "groq" ? isValidGroqKey(key) : isValidGeminiKey(key);
}

export async function getApiKey(provider: Provider): Promise<string> {
  return loadApiKey(provider);
}

export async function setApiKey(provider: Provider, key: string): Promise<ApiKeyBackend> {
  await saveApiKey(provider, key);
  const backend: ApiKeyBackend = key.trim() ? "encrypted" : "none";
  apiKeyBackend.set(backend);
  return backend;
}

export async function getSettings(): Promise<AppSettings> {
  const store = await getStore();
  const provider = (await store.get<Provider>(PROVIDER_KEY)) ?? DEFAULTS.provider;
  const groqKey = await loadApiKey("groq");
  const geminiKey = await loadApiKey("gemini");
  const activeKey = provider === "groq" ? groqKey : geminiKey;
  apiKeyBackend.set(activeKey ? "encrypted" : "none");
  return {
    provider,
    groqApiKey: groqKey,
    geminiApiKey: geminiKey,
    groqModel:
      (await store.get<GroqModel>(GROQ_MODEL_KEY)) ?? DEFAULTS.groqModel,
    geminiModel:
      (await store.get<GeminiModel>(GEMINI_MODEL_KEY)) ?? DEFAULTS.geminiModel,
    language: (await store.get<Language>(LANG_KEY)) ?? DEFAULTS.language,
    diarize: (await store.get<boolean>(DIARIZE_KEY)) ?? DEFAULTS.diarize,
    uiLanguage:
      (await store.get<UiLanguagePref>(UI_LANG_KEY)) ?? DEFAULTS.uiLanguage,
    historySidebarCollapsed:
      (await store.get<boolean>(HISTORY_COLLAPSED_KEY)) ??
      DEFAULTS.historySidebarCollapsed,
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
  if (patch.uiLanguage !== undefined) await store.set(UI_LANG_KEY, patch.uiLanguage);
  if (patch.historySidebarCollapsed !== undefined)
    await store.set(HISTORY_COLLAPSED_KEY, patch.historySidebarCollapsed);
}

async function migrateSecrets(): Promise<void> {
  const store = await getStore();

  // Legacy plaintext entries: pull into encrypted store. Cheap to retry every
  // launch (just store reads) — so we don't gate on MIGRATION_FLAG. Delete only
  // after a successful re-encrypt; a failing secret_set must not destroy the
  // user's only copy.
  const legacyGroq = await store.get<string>(API_KEY_LEGACY);
  if (typeof legacyGroq === "string" && legacyGroq.trim()) {
    try {
      await saveApiKey("groq", legacyGroq);
      await store.delete(API_KEY_LEGACY);
    } catch {
      // leave legacy entry in place; retry next launch
    }
  } else if (legacyGroq !== undefined) {
    await store.delete(API_KEY_LEGACY);
  }

  for (const provider of ["groq", "gemini"] as const) {
    const fbKey = fallbackKeyFor(provider);
    const fallback = await store.get<string>(fbKey);
    if (typeof fallback === "string" && fallback.trim()) {
      const existing = await loadApiKey(provider);
      if (existing) {
        await store.delete(fbKey);
        continue;
      }
      try {
        await saveApiKey(provider, fallback);
        await store.delete(fbKey);
      } catch {
        // leave fallback entry in place; retry next launch
      }
    } else if (fallback !== undefined) {
      await store.delete(fbKey);
    }
  }

  // Keychain pull is the only step the user can perceive (password prompt on
  // macOS), so it gets gated by MIGRATION_FLAG and runs at most once. Flag is
  // set unconditionally so a Deny doesn't re-prompt every launch.
  if ((await store.get<boolean>(MIGRATION_FLAG)) !== true) {
    for (const provider of ["groq", "gemini"] as const) {
      try {
        await invoke<boolean>("secret_migrate_from_keychain", { provider });
      } catch {
        // ignore — user denied / Keychain unavailable / no entry
      }
    }
    await store.set(MIGRATION_FLAG, true);
  }
}

export const settingsStore: Writable<AppSettings> = writable(DEFAULTS);
export const apiKeyBackend: Writable<ApiKeyBackend> = writable("none");

let initialized = false;
export async function initSettings(): Promise<void> {
  if (initialized) return;
  await migrateSecrets();
  const s = await getSettings();
  settingsStore.set(s);
  await applyUiLanguage(s.uiLanguage);
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
  if (p.provider !== undefined) {
    const key = await loadApiKey(p.provider);
    apiKeyBackend.set(key ? "encrypted" : "none");
  }
  if (p.uiLanguage !== undefined) {
    await applyUiLanguage(p.uiLanguage);
  }
  await updateSettings(p);
}

export function isValidGroqKey(key: string): boolean {
  return /^gsk_[A-Za-z0-9]{20,}$/.test(key.trim());
}

export function isValidGeminiKey(key: string): boolean {
  return /^AIza[\w-]{20,}$/.test(key.trim());
}
