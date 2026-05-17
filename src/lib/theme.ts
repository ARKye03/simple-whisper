import { load, type Store } from "@tauri-apps/plugin-store";

export type Theme = "light" | "dark" | "system";

const STORE_FILE = "settings.json";
const THEME_KEY = "theme";

let cachedStore: Store | null = null;
let mediaQuery: MediaQueryList | null = null;
let mediaListener: ((e: MediaQueryListEvent) => void) | null = null;
let currentTheme: Theme = "system";

async function getStore(): Promise<Store> {
  if (!cachedStore) {
    cachedStore = await load(STORE_FILE, { autoSave: true, defaults: {} });
  }
  return cachedStore;
}

function applyClass(theme: Theme) {
  const prefersDark =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches;
  const isDark = theme === "dark" || (theme === "system" && prefersDark);
  document.documentElement.classList.toggle("dark", isDark);
}

function subscribeSystem(theme: Theme) {
  if (typeof window === "undefined") return;

  if (mediaQuery && mediaListener) {
    mediaQuery.removeEventListener("change", mediaListener);
    mediaListener = null;
  }

  if (theme !== "system") return;

  mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
  mediaListener = () => applyClass("system");
  mediaQuery.addEventListener("change", mediaListener);
}

export async function loadTheme(): Promise<Theme> {
  const store = await getStore();
  const stored = (await store.get<Theme>(THEME_KEY)) ?? "system";
  currentTheme = stored;
  applyClass(stored);
  subscribeSystem(stored);
  return stored;
}

export async function setTheme(theme: Theme): Promise<void> {
  const store = await getStore();
  await store.set(THEME_KEY, theme);
  currentTheme = theme;
  applyClass(theme);
  subscribeSystem(theme);
}

export function getCurrentTheme(): Theme {
  return currentTheme;
}
