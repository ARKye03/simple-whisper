import { invoke } from "@tauri-apps/api/core";
import { load, type Store } from "@tauri-apps/plugin-store";
import { writable, type Writable } from "svelte/store";
import { applyUiLanguage, type UiLanguagePref } from "$lib/i18n/state.svelte";
import { KEYED_PROVIDERS, PROVIDERS, type Provider } from "$lib/types";

export type { Provider };
export type GroqModel = "whisper-large-v3-turbo" | "whisper-large-v3";
export type GeminiModel = "gemini-3.1-flash-lite" | "gemini-3-flash-preview";
export type LocalModel =
  | "tiny" | "base" | "small" | "medium"
  | "large-v2" | "large-v3" | "large-v3-turbo" | "distil-large-v3";
export type Model = GroqModel | GeminiModel | LocalModel;
export type Language =
  | "auto" | "es" | "en" | "fr" | "de" | "pt" | "it" | "ja" | "zh";
export type ApiKeyBackend = "encrypted" | "none";

export type AppSettings = {
  provider: Provider;
  groqApiKey: string;
  geminiApiKey: string;
  groqModel: GroqModel;
  geminiModel: GeminiModel;
  localModel: LocalModel;
  language: Language;
  diarize: boolean;
  uiLanguage: UiLanguagePref;
  historySidebarCollapsed: boolean;
};

export type KeyedProvider = (typeof KEYED_PROVIDERS)[number];

const STORE_FILE = "settings.json";
const API_KEY_LEGACY = "groq_api_key";
const PROVIDER_KEY = "provider";
const GROQ_MODEL_KEY = "groq_model";
const GEMINI_MODEL_KEY = "gemini_model";
const LOCAL_MODEL_KEY = "local_model";
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
  localModel: "large-v3-turbo",
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

function fallbackKeyFor(provider: KeyedProvider): string {
  return `${provider}_api_key_fallback`;
}

// Narrowed to KeyedProvider on purpose: secrets.rs `account_for` rejects "local".
async function loadApiKey(provider: KeyedProvider): Promise<string> {
  try {
    return await invoke<string>("secret_get", { provider });
  } catch {
    return "";
  }
}

async function saveApiKey(provider: KeyedProvider, key: string): Promise<void> {
  await invoke("secret_set", { provider, value: key.trim() });
}

export function providerNeedsKey(p: Provider): p is KeyedProvider {
  return (KEYED_PROVIDERS as readonly Provider[]).includes(p);
}

export function keyFieldFor(p: Provider): "groqApiKey" | "geminiApiKey" | null {
  switch (p) {
    case "groq":
      return "groqApiKey";
    case "gemini":
      return "geminiApiKey";
    case "local":
      return null;
  }
}

export function apiKeyFor(s: AppSettings, p: Provider): string {
  const field = keyFieldFor(p);
  return field ? s[field] : "";
}

export function activeApiKey(s: AppSettings): string {
  return apiKeyFor(s, s.provider);
}

export function modelFor(s: AppSettings, p: Provider): Model {
  switch (p) {
    case "groq":
      return s.groqModel;
    case "gemini":
      return s.geminiModel;
    case "local":
      return s.localModel;
  }
}

export function activeModel(s: AppSettings): Model {
  return modelFor(s, s.provider);
}

export function isValidKeyFor(p: Provider, key: string): boolean {
  switch (p) {
    case "groq":
      return isValidGroqKey(key);
    case "gemini":
      return isValidGeminiKey(key);
    case "local":
      return true;
  }
}

export function activeIsValidKey(s: AppSettings): boolean {
  if (!providerNeedsKey(s.provider)) return true;
  const key = activeApiKey(s);
  if (!key) return false;
  return isValidKeyFor(s.provider, key);
}

/**
 * Every provider other than the current one that could actually run now — a
 * cloud provider with a key, plus "local" once its runtime is ready. Replaces
 * the old single "the other one" slot, which silently dropped an alternative.
 */
export function retryProviders(s: AppSettings, localReady: boolean): Provider[] {
  return PROVIDERS.filter((p) => {
    if (p === s.provider) return false;
    if (p === "local") return localReady;
    return !!apiKeyFor(s, p);
  });
}

export async function getApiKey(provider: KeyedProvider): Promise<string> {
  return loadApiKey(provider);
}

export async function setApiKey(provider: KeyedProvider, key: string): Promise<ApiKeyBackend> {
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
  // A keyless provider must not inherit the other provider's "stored encrypted" badge.
  const activeKey = providerNeedsKey(provider)
    ? provider === "groq"
      ? groqKey
      : geminiKey
    : "";
  apiKeyBackend.set(activeKey ? "encrypted" : "none");
  return {
    provider,
    groqApiKey: groqKey,
    geminiApiKey: geminiKey,
    groqModel:
      (await store.get<GroqModel>(GROQ_MODEL_KEY)) ?? DEFAULTS.groqModel,
    geminiModel:
      (await store.get<GeminiModel>(GEMINI_MODEL_KEY)) ?? DEFAULTS.geminiModel,
    localModel:
      (await store.get<LocalModel>(LOCAL_MODEL_KEY)) ?? DEFAULTS.localModel,
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
  if (patch.localModel !== undefined) await store.set(LOCAL_MODEL_KEY, patch.localModel);
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

  for (const provider of KEYED_PROVIDERS) {
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
    for (const provider of KEYED_PROVIDERS) {
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
    const key = providerNeedsKey(p.provider) ? await loadApiKey(p.provider) : "";
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
