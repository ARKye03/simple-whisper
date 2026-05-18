import { locale } from "@tauri-apps/plugin-os";
import { es, type Dict } from "./es";
import { en } from "./en";

export type Lang = "es" | "en";
export type UiLanguagePref = "system" | Lang;

const dicts: Record<Lang, Dict> = { es, en };

function detectBrowserLang(): Lang {
  if (typeof navigator === "undefined") return "en";
  return navigator.language?.toLowerCase().startsWith("es") ? "es" : "en";
}

const state = $state<{ lang: Lang }>({ lang: detectBrowserLang() });

export async function resolveSystemLanguage(): Promise<Lang> {
  try {
    const l = await locale();
    if (l) return l.toLowerCase().startsWith("es") ? "es" : "en";
  } catch {
    // fall through to browser
  }
  return detectBrowserLang();
}

export async function applyUiLanguage(pref: UiLanguagePref): Promise<void> {
  state.lang = pref === "system" ? await resolveSystemLanguage() : pref;
}

export const t = new Proxy({} as Dict, {
  get(_, key: string | symbol) {
    return (dicts[state.lang] as Record<string | symbol, unknown>)[key];
  },
});
