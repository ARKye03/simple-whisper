import { load, type Store } from "@tauri-apps/plugin-store";

const STORE_FILE = "settings.json";
const API_KEY = "groq_api_key";

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
