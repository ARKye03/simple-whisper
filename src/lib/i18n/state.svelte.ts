import { locale } from "@tauri-apps/plugin-os";
import { es, type Dict } from "./es";
import { en } from "./en";

export type Lang = "es" | "en";
export type UiLanguagePref = "system" | Lang;

const dicts: Record<Lang, Dict> = { es, en };

const state = $state<{ lang: Lang }>({ lang: "en" });

export async function resolveSystemLanguage(): Promise<Lang> {
  try {
    const l = await locale();
    return l?.toLowerCase().startsWith("es") ? "es" : "en";
  } catch {
    return "en";
  }
}

export async function applyUiLanguage(pref: UiLanguagePref): Promise<void> {
  state.lang = pref === "system" ? await resolveSystemLanguage() : pref;
}

export const t = new Proxy({} as Dict, {
  get(_, key: string | symbol) {
    return (dicts[state.lang] as Record<string | symbol, unknown>)[key];
  },
});
