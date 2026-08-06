<script lang="ts">
  import { onMount } from "svelte";
  import { settingsOpen } from "$lib/drawer";
  import { setTheme, loadTheme, type Theme } from "$lib/theme";
  import {
    settingsStore,
    apiKeyBackend,
    patchSettings,
    isValidGroqKey,
    isValidGeminiKey,
    activeApiKey,
    activeModel,
    type AppSettings,
    type GroqModel,
    type GeminiModel,
    COOKIE_BROWSERS,
    COOKIE_BROWSER_LABELS,
    type CookieBrowser,
    type Provider,
    type Language,
  } from "$lib/settings";
  import { t, type UiLanguagePref } from "$lib/i18n/state.svelte";
  import { checkForUpdates } from "$lib/updater";
  import { ytdlpStore, refreshYtdlp } from "$lib/ytdlp";
  import Icon from "./Icons.svelte";

  let theme = $state<Theme>("system");
  let showKey = $state(false);
  let updateChecking = $state(false);
  let apiKeySaveState = $state<"idle" | "saving" | "saved" | "error">("idle");
  let apiKeySaveTimer: ReturnType<typeof setTimeout> | null = null;
  let apiKeyDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  // yt-dlp is optional, so it is probed the first time the drawer is opened rather
  // than at boot. Plain let, not $state, so the effect cannot re-trigger itself.
  let ytdlpProbed = false;
  $effect(() => {
    if (!$settingsOpen || ytdlpProbed) return;
    ytdlpProbed = true;
    void refreshYtdlp();
  });

  onMount(async () => {
    theme = await loadTheme();
  });

  const isGemini = $derived($settingsStore.provider === "gemini");
  const currentKey = $derived(activeApiKey($settingsStore));
  const apiKeyValid = $derived(
    currentKey.length === 0 ||
      (isGemini ? isValidGeminiKey(currentKey) : isValidGroqKey(currentKey)),
  );

  const apiKeyLabel = $derived(isGemini ? t.apiKeyLabelGemini : t.apiKeyLabelGroq);
  const apiKeyPlaceholder = $derived(
    isGemini ? t.apiKeyPlaceholderGemini : t.apiKeyPlaceholderGroq,
  );
  const apiKeyInvalidMsg = $derived(
    isGemini ? t.apiKeyInvalidGemini : t.apiKeyInvalidGroq,
  );

  async function patch(p: Partial<AppSettings>) {
    await patchSettings(p);
  }

  function onApiKeyInput(value: string) {
    const providerAtInput = $settingsStore.provider;
    settingsStore.update((s) =>
      providerAtInput === "groq"
        ? { ...s, groqApiKey: value }
        : { ...s, geminiApiKey: value },
    );
    if (apiKeyDebounceTimer) clearTimeout(apiKeyDebounceTimer);
    apiKeyDebounceTimer = setTimeout(async () => {
      // If user switched providers during the debounce window, drop this save —
      // the typed value belongs to the previously-selected provider and writing
      // it to the new one would corrupt that key.
      if ($settingsStore.provider !== providerAtInput) return;
      apiKeySaveState = "saving";
      try {
        await patchSettings(
          providerAtInput === "groq"
            ? { groqApiKey: value }
            : { geminiApiKey: value },
        );
        apiKeySaveState = "saved";
        if (apiKeySaveTimer) clearTimeout(apiKeySaveTimer);
        apiKeySaveTimer = setTimeout(() => (apiKeySaveState = "idle"), 2000);
      } catch {
        apiKeySaveState = "error";
        if (apiKeySaveTimer) clearTimeout(apiKeySaveTimer);
        apiKeySaveTimer = setTimeout(() => (apiKeySaveState = "idle"), 3000);
      }
    }, 350);
  }

  async function chooseTheme(next: Theme) {
    theme = next;
    await setTheme(next);
  }

  function close() {
    settingsOpen.set(false);
  }

  async function onCheckForUpdates() {
    if (updateChecking) return;
    updateChecking = true;
    try {
      await checkForUpdates({ silent: false });
    } finally {
      updateChecking = false;
    }
  }

  const themes: { v: Theme; l: string }[] = $derived([
    { v: "light", l: t.themeLight },
    { v: "dark", l: t.themeDark },
    { v: "system", l: t.themeSystem },
  ]);

  const uiLanguages: { v: UiLanguagePref; l: string }[] = $derived([
    { v: "system", l: t.uiLanguageSystem },
    { v: "es", l: t.uiLanguageEs },
    { v: "en", l: t.uiLanguageEn },
  ]);

  const providers: { v: Provider; l: string }[] = $derived([
    { v: "groq", l: t.providerGroq },
    { v: "gemini", l: t.providerGemini },
  ]);

  const languages: { v: Language; l: string }[] = $derived([
    { v: "auto", l: t.languageAuto },
    { v: "es", l: "Español" },
    { v: "en", l: "English" },
    { v: "fr", l: "Français" },
    { v: "de", l: "Deutsch" },
    { v: "pt", l: "Português" },
    { v: "it", l: "Italiano" },
    { v: "ja", l: "日本語" },
    { v: "zh", l: "中文" },
  ]);

  const GROQ_MODELS: GroqModel[] = ["whisper-large-v3-turbo", "whisper-large-v3"];
  const GEMINI_MODELS: GeminiModel[] = ["gemini-3.1-flash-lite", "gemini-3-flash-preview"];
</script>

{#snippet segBtn(active: boolean, label: string, onClick: () => void, disabled = false)}
  <button
    onclick={onClick}
    {disabled}
    style="
      flex:1; padding:7px 0; border-radius: var(--r-sm);
      background: {active ? 'var(--bg-1)' : 'transparent'};
      color: {disabled ? 'var(--text-4)' : active ? 'var(--text-1)' : 'var(--text-3)'};
      border: {active ? '1px solid var(--border-1)' : '1px solid transparent'};
      font-family: var(--font); font-size:12px;
      font-weight:500; cursor: {disabled ? 'not-allowed' : 'pointer'};
      opacity: {disabled ? 0.5 : 1};
      transition: all var(--t);
    "
  >{label}</button>
{/snippet}

<div
  class="drawer-backdrop"
  class:open={$settingsOpen}
  onclick={close}
  role="presentation"
></div>
<div
  class="drawer-panel"
  class:open={$settingsOpen}
  inert={!$settingsOpen}
>
    <div
      style="
        display:flex; align-items:center; justify-content:space-between;
        padding:14px 18px; border-bottom:1px solid var(--border-1);
      "
    >
      <span style="font-size:14px; font-weight:600;">{t.settings}</span>
      <button class="icon-btn" style="width:32px; height:32px;" onclick={close} aria-label={t.closeAria}>
        <Icon name="x" size={18} />
      </button>
    </div>

    <div
      style="flex:1; overflow-y:auto; padding:18px; display:flex; flex-direction:column; gap:22px;"
    >
      <div>
        <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{t.mode}</div>
        <div style="display:flex; gap:2px; background:var(--bg-3); border-radius:var(--r-md); padding:3px;">
          {#each themes as opt (opt.v)}
            {@render segBtn(theme === opt.v, opt.l, () => chooseTheme(opt.v))}
          {/each}
        </div>
      </div>

      <div>
        <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{t.uiLanguage}</div>
        <div style="display:flex; gap:2px; background:var(--bg-3); border-radius:var(--r-md); padding:3px;">
          {#each uiLanguages as opt (opt.v)}
            {@render segBtn($settingsStore.uiLanguage === opt.v, opt.l, () => patch({ uiLanguage: opt.v }))}
          {/each}
        </div>
      </div>

      <div>
        <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{t.provider}</div>
        <div style="display:flex; gap:2px; background:var(--bg-3); border-radius:var(--r-md); padding:3px;">
          {#each providers as opt (opt.v)}
            {@render segBtn($settingsStore.provider === opt.v, opt.l, () => patch({ provider: opt.v }))}
          {/each}
        </div>
      </div>

      <div>
        <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{t.model}</div>
        {#if isGemini}
          <select
            value={$settingsStore.geminiModel}
            onchange={(e) => patch({ geminiModel: (e.currentTarget as HTMLSelectElement).value as GeminiModel })}
            style="width:100%; padding:9px 12px; border-radius:var(--r-md); background:var(--bg-3); border:1px solid var(--border-2); color:var(--text-1); font-family:var(--font); font-size:13px; outline:none;"
          >
            {#each GEMINI_MODELS as m (m)}
              <option value={m}>{m}</option>
            {/each}
          </select>
        {:else}
          <select
            value={$settingsStore.groqModel}
            onchange={(e) => patch({ groqModel: (e.currentTarget as HTMLSelectElement).value as GroqModel })}
            style="width:100%; padding:9px 12px; border-radius:var(--r-md); background:var(--bg-3); border:1px solid var(--border-2); color:var(--text-1); font-family:var(--font); font-size:13px; outline:none;"
          >
            {#each GROQ_MODELS as m (m)}
              <option value={m}>{m}</option>
            {/each}
          </select>
        {/if}
      </div>

      <div>
        <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{t.language}</div>
        <select
          value={$settingsStore.language}
          onchange={(e) => patch({ language: (e.currentTarget as HTMLSelectElement).value as Language })}
          style="width:100%; padding:9px 12px; border-radius:var(--r-md); background:var(--bg-3); border:1px solid var(--border-2); color:var(--text-1); font-family:var(--font); font-size:13px; outline:none;"
        >
          {#each languages as l (l.v)}
            <option value={l.v}>{l.l}</option>
          {/each}
        </select>
      </div>

      {#if isGemini}
        <div>
          <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{t.diarize}</div>
          <label style="display:flex; align-items:center; gap:10px; cursor:pointer; padding:9px 12px; border-radius:var(--r-md); background:var(--bg-3); border:1px solid var(--border-2);">
            <input
              type="checkbox"
              checked={$settingsStore.diarize}
              onchange={(e) => patch({ diarize: (e.currentTarget as HTMLInputElement).checked })}
              style="width:16px; height:16px; accent-color:var(--accent);"
            />
            <span style="font-size:13px; color:var(--text-1);">{t.diarize}</span>
          </label>
          <p style="font-size:10px; color:var(--text-4); margin-top:6px; line-height:1.5;">{t.diarizeHint}</p>
        </div>
      {/if}

      <div style="height:1px; background:var(--border-1); margin:2px 0;"></div>

      <div>
        <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{apiKeyLabel}</div>
        <div style="position:relative;">
          <input
            type={showKey ? "text" : "password"}
            value={currentKey}
            oninput={(e) => onApiKeyInput((e.currentTarget as HTMLInputElement).value)}
            placeholder={apiKeyPlaceholder}
            spellcheck="false"
            autocomplete="off"
            style="
              width:100%; padding:9px 12px; padding-right:40px;
              border-radius:var(--r-md); background:var(--bg-3);
              border:1px solid {apiKeyValid ? 'var(--border-2)' : 'var(--error)'};
              color:var(--text-1);
              font-family: {showKey ? 'var(--font)' : 'ui-monospace, monospace'};
              font-size:12px;
              letter-spacing: {showKey ? '0' : '0.08em'};
              outline:none;
            "
          />
          <button
            onclick={() => (showKey = !showKey)}
            title={showKey ? t.hide : t.show}
            aria-label={showKey ? t.hide : t.show}
            style="
              position:absolute; right:4px; top:50%; transform:translateY(-50%);
              background:none; border:none; cursor:pointer;
              color:var(--text-4); display:flex; align-items:center;
              padding:6px; border-radius:var(--r-sm); transition: color var(--t);
            "
          >
            <Icon name={showKey ? "eye-off" : "eye"} size={16} />
          </button>
        </div>
        <div style="margin-top:6px; min-height:16px; font-size:10px; line-height:1.5; display:flex; align-items:center; gap:6px;">
          {#if apiKeySaveState === "saving"}
            <span style="color: var(--text-3); display:inline-flex; align-items:center; gap:5px;">
              <Icon name="spinner" size={11} /> {t.apiKeySaving}
            </span>
          {:else if apiKeySaveState === "saved"}
            <span style="color: var(--success); display:inline-flex; align-items:center; gap:5px;">
              <Icon name="check" size={11} /> {t.apiKeySaved}
            </span>
          {:else if apiKeySaveState === "error"}
            <span style="color: var(--error);">⚠ Error</span>
          {:else if currentKey && !apiKeyValid}
            <span style="color: var(--error);">{apiKeyInvalidMsg}</span>
          {:else if currentKey && apiKeyValid}
            <span style="color: var(--success); display:inline-flex; align-items:center; gap:5px;">
              <Icon name="check" size={11} /> {t.apiKeyValid}
            </span>
          {/if}
        </div>
        <p style="font-size:10px; color:var(--text-4); margin-top:6px; line-height:1.5;">
          {#if $apiKeyBackend === "encrypted"}
            🔒 {t.apiKeyBackendEncrypted}
          {:else}
            {t.apiKeyHint}
          {/if}
        </p>
      </div>

      <div style="height:1px; background:var(--border-1); margin:2px 0;"></div>

      <div>
        <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{t.downloads}</div>
        <div
          style="display:flex; align-items:center; justify-content:space-between; gap:10px; padding:9px 12px; border-radius:var(--r-md); background:var(--bg-3); border:1px solid var(--border-2);"
        >
          <span
            style="font-size:12.5px; color:var(--text-1); display:inline-flex; align-items:center; gap:7px; min-width:0;"
          >
            {#if $ytdlpStore.state === "checking"}
              <Icon name="spinner" size={13} /> {t.ytdlpChecking}
            {:else if $ytdlpStore.state === "ok"}
              <span style="color:var(--success); display:inline-flex;"><Icon name="check" size={13} /></span>
              {t.ytdlpFound($ytdlpStore.version ?? "")}
            {:else if $ytdlpStore.state === "missing"}
              <span style="color:var(--error); display:inline-flex;"><Icon name="x" size={13} /></span>
              {t.ytdlpNotFound}
            {:else}
              <span style="color:var(--text-4);">{t.ytdlpUnknown}</span>
            {/if}
          </span>
          <button
            class="btn-ghost"
            style="font-size:11px; flex-shrink:0;"
            onclick={() => refreshYtdlp()}
            disabled={$ytdlpStore.state === "checking"}
          >
            {t.ytdlpRecheck}
          </button>
        </div>
        <p style="font-size:10px; color:var(--text-4); margin-top:6px; line-height:1.5;">
          {$ytdlpStore.state === "missing" ? t.ytdlpInstallHint : t.ytdlpOptional}
        </p>
      </div>

      <div>
        <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{t.cookiesBrowser}</div>
        <select
          value={$settingsStore.cookiesBrowser}
          onchange={(e) =>
            patch({ cookiesBrowser: (e.currentTarget as HTMLSelectElement).value as CookieBrowser })}
          style="width:100%; padding:9px 12px; border-radius:var(--r-md); background:var(--bg-3); border:1px solid var(--border-2); color:var(--text-1); font-family:var(--font); font-size:13px; outline:none;"
        >
          {#each COOKIE_BROWSERS as b (b)}
            <option value={b}>{b === "none" ? t.cookiesBrowserNone : COOKIE_BROWSER_LABELS[b]}</option>
          {/each}
        </select>
        <p style="font-size:10px; color:var(--text-4); margin-top:6px; line-height:1.5;">
          {t.cookiesBrowserHint}{$settingsStore.cookiesBrowser === "safari"
            ? ` ${t.cookiesBrowserSafariHint}`
            : ""}
        </p>
      </div>

      <div style="height:1px; background:var(--border-1); margin:2px 0;"></div>

      <div>
        <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{t.updates}</div>
        <button
          onclick={onCheckForUpdates}
          disabled={updateChecking}
          style="
            width:100%; padding:9px 12px; border-radius:var(--r-md);
            background:var(--bg-3); border:1px solid var(--border-2);
            color:var(--text-1); font-family:var(--font); font-size:13px;
            cursor: {updateChecking ? 'wait' : 'pointer'};
            display:flex; align-items:center; justify-content:center; gap:8px;
            transition: background var(--t);
          "
        >
          {#if updateChecking}
            <Icon name="spinner" size={13} /> {t.checkingForUpdates}
          {:else}
            {t.checkForUpdates}
          {/if}
        </button>
      </div>
    </div>

    <div
      style="
        padding:14px 18px; border-top:1px solid var(--border-1);
        font-size:11px; color:var(--text-4);
        display:flex; justify-content:space-between;
        letter-spacing:0.03em;
      "
    >
      <span>{isGemini ? "Google AI" : "Groq API"}</span>
      <span>{activeModel($settingsStore)}</span>
    </div>
</div>

<style>
  .drawer-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    z-index: 50;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.15s ease;
  }
  .drawer-backdrop.open {
    opacity: 1;
    pointer-events: auto;
  }
  .drawer-panel {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: 330px;
    background: var(--bg-1);
    border-left: 1px solid var(--border-1);
    z-index: 51;
    display: flex;
    flex-direction: column;
    transform: translateX(100%);
    transition: transform 0.22s cubic-bezier(0.4, 0, 0.2, 1);
    will-change: transform;
  }
  .drawer-panel.open {
    transform: translateX(0);
  }
</style>
