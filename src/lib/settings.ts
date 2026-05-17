import { load, type Store } from "@tauri-apps/plugin-store";

export type Model = "whisper-large-v3-turbo" | "whisper-large-v3";
export type Language =
  | "auto" | "es" | "en" | "fr" | "de" | "pt" | "it" | "ja" | "zh";
export type OutputFormat = "txt" | "md" | "docx" | "pdf";

export type AppSettings = {
  apiKey: string;
  model: Model;
  language: Language;
  format: OutputFormat;
};

const STORE_FILE = "settings.json";
const API_KEY = "groq_api_key";
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

export async function getApiKey(): Promise<string> {
  const store = await getStore();
  return (await store.get<string>(API_KEY)) ?? "";
}

export async function setApiKey(key: string): Promise<void> {
  const store = await getStore();
  await store.set(API_KEY, key.trim());
}

export async function getSettings(): Promise<AppSettings> {
  const store = await getStore();
  return {
    apiKey: (await store.get<string>(API_KEY)) ?? DEFAULTS.apiKey,
    model: (await store.get<Model>(MODEL_KEY)) ?? DEFAULTS.model,
    language: (await store.get<Language>(LANG_KEY)) ?? DEFAULTS.language,
    format: (await store.get<OutputFormat>(FORMAT_KEY)) ?? DEFAULTS.format,
  };
}

export async function updateSettings(patch: Partial<AppSettings>): Promise<void> {
  const store = await getStore();
  if (patch.apiKey !== undefined) await store.set(API_KEY, patch.apiKey.trim());
  if (patch.model !== undefined) await store.set(MODEL_KEY, patch.model);
  if (patch.language !== undefined) await store.set(LANG_KEY, patch.language);
  if (patch.format !== undefined) await store.set(FORMAT_KEY, patch.format);
}
