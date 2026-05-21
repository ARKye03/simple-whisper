import { load, type Store } from "@tauri-apps/plugin-store";
import { writable, get, type Writable } from "svelte/store";
import type { HistoryEntry } from "$lib/types";

const STORE_FILE = "history.json";
const ENTRIES_KEY = "entries";

let cachedStore: Store | null = null;

async function getStore(): Promise<Store> {
  if (!cachedStore) {
    cachedStore = await load(STORE_FILE, { autoSave: true, defaults: {} });
  }
  return cachedStore;
}

export const historyStore: Writable<HistoryEntry[]> = writable([]);
export const historySelectionStore: Writable<string | null> = writable(null);

let initialized = false;

export async function initHistory(): Promise<void> {
  if (initialized) return;
  const store = await getStore();
  const entries = (await store.get<HistoryEntry[]>(ENTRIES_KEY)) ?? [];
  historyStore.set(entries);
  initialized = true;
}

function makeId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

export async function addHistoryEntry(
  input: Omit<HistoryEntry, "id" | "createdAt">,
): Promise<HistoryEntry> {
  const entry: HistoryEntry = {
    id: makeId(),
    createdAt: Date.now(),
    ...input,
  };
  const next = [entry, ...get(historyStore)];
  historyStore.set(next);
  const store = await getStore();
  await store.set(ENTRIES_KEY, next);
  return entry;
}

export async function deleteHistoryEntry(id: string): Promise<void> {
  const next = get(historyStore).filter((e) => e.id !== id);
  historyStore.set(next);
  if (get(historySelectionStore) === id) historySelectionStore.set(null);
  const store = await getStore();
  await store.set(ENTRIES_KEY, next);
}

export async function clearHistory(): Promise<void> {
  historyStore.set([]);
  historySelectionStore.set(null);
  const store = await getStore();
  await store.set(ENTRIES_KEY, []);
}

export function getHistoryEntry(id: string): HistoryEntry | undefined {
  return get(historyStore).find((e) => e.id === id);
}
