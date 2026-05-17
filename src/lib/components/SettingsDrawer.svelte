<script lang="ts">
  import { onMount } from "svelte";
  import { settingsOpen } from "$lib/drawer";
  import { setTheme, loadTheme, type Theme } from "$lib/theme";
  import {
    getSettings,
    updateSettings,
    type AppSettings,
    type Model,
    type Language,
    type OutputFormat,
  } from "$lib/settings";
  import { t } from "$lib/i18n/es";
  import Icon from "./Icons.svelte";

  let open = $state(false);
  settingsOpen.subscribe((v) => (open = v));

  let settings = $state<AppSettings>({
    apiKey: "",
    model: "whisper-large-v3-turbo",
    language: "auto",
    format: "docx",
  });
  let theme = $state<Theme>("system");
  let showKey = $state(false);

  onMount(async () => {
    settings = await getSettings();
    theme = await loadTheme();
  });

  async function patch(p: Partial<AppSettings>) {
    settings = { ...settings, ...p };
    await updateSettings(p);
  }

  async function chooseTheme(next: Theme) {
    theme = next;
    await setTheme(next);
  }

  function close() {
    settingsOpen.set(false);
  }

  const themes: { v: Theme; l: string }[] = [
    { v: "light", l: t.themeLight },
    { v: "dark", l: t.themeDark },
    { v: "system", l: t.themeSystem },
  ];

  const languages: { v: Language; l: string }[] = [
    { v: "auto", l: t.languageAuto },
    { v: "es", l: "Español" },
    { v: "en", l: "English" },
    { v: "fr", l: "Français" },
    { v: "de", l: "Deutsch" },
    { v: "pt", l: "Português" },
    { v: "it", l: "Italiano" },
    { v: "ja", l: "日本語" },
    { v: "zh", l: "中文" },
  ];

  const FORMAT_ROW_1: { v: OutputFormat; l: string; disabled?: boolean }[] = [
    { v: "txt", l: "TXT" },
    { v: "md", l: "MD" },
  ];
  // SRT/VTT placeholders disabled
  const FORMAT_ROW_2: { v: OutputFormat; l: string; disabled?: boolean }[] = [
    { v: "docx", l: "DOCX" },
    { v: "pdf", l: "PDF" },
  ];
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

{#if open}
  <div
    onclick={close}
    role="presentation"
    style="position:fixed; inset:0; background: rgba(0,0,0,.45); z-index:50; animation: fadeIn .15s ease;"
  ></div>
  <div
    style="
      position:fixed; top:0; right:0; bottom:0; width:330px;
      background: var(--bg-1); border-left: 1px solid var(--border-1);
      z-index:51; display:flex; flex-direction:column;
      animation: slideIn .22s var(--t-slow);
    "
  >
    <div
      style="
        display:flex; align-items:center; justify-content:space-between;
        padding:14px 18px; border-bottom:1px solid var(--border-1);
      "
    >
      <span style="font-size:14px; font-weight:600;">{t.settings}</span>
      <button class="icon-btn" style="width:32px; height:32px;" onclick={close} aria-label="Cerrar">
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
        <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{t.model}</div>
        <select
          value={settings.model}
          onchange={(e) => patch({ model: (e.currentTarget as HTMLSelectElement).value as Model })}
          style="width:100%; padding:9px 12px; border-radius:var(--r-md); background:var(--bg-3); border:1px solid var(--border-2); color:var(--text-1); font-family:var(--font); font-size:13px; outline:none;"
        >
          <option value="whisper-large-v3-turbo">whisper-large-v3-turbo</option>
          <option value="whisper-large-v3">whisper-large-v3</option>
        </select>
      </div>

      <div>
        <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{t.language}</div>
        <select
          value={settings.language}
          onchange={(e) => patch({ language: (e.currentTarget as HTMLSelectElement).value as Language })}
          style="width:100%; padding:9px 12px; border-radius:var(--r-md); background:var(--bg-3); border:1px solid var(--border-2); color:var(--text-1); font-family:var(--font); font-size:13px; outline:none;"
        >
          {#each languages as l (l.v)}
            <option value={l.v}>{l.l}</option>
          {/each}
        </select>
      </div>

      <div>
        <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{t.outputFormat}</div>
        <div style="display:flex; gap:2px; background:var(--bg-3); border-radius:var(--r-md); padding:3px;">
          {#each FORMAT_ROW_1 as f (f.v)}
            {@render segBtn(settings.format === f.v, f.l, () => patch({ format: f.v }), f.disabled)}
          {/each}
        </div>
        <div style="display:flex; gap:2px; background:var(--bg-3); border-radius:var(--r-md); padding:3px; margin-top:4px;">
          {#each FORMAT_ROW_2 as f (f.v)}
            {@render segBtn(settings.format === f.v, f.l, () => patch({ format: f.v }), f.disabled)}
          {/each}
        </div>
      </div>

      <div style="height:1px; background:var(--border-1); margin:2px 0;"></div>

      <div>
        <div style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em; margin-bottom:8px;">{t.apiKey}</div>
        <div style="position:relative;">
          <input
            type={showKey ? "text" : "password"}
            value={settings.apiKey}
            oninput={(e) => patch({ apiKey: (e.currentTarget as HTMLInputElement).value })}
            placeholder={t.apiKeyPlaceholder}
            spellcheck="false"
            autocomplete="off"
            style="
              width:100%; padding:9px 12px; padding-right:40px;
              border-radius:var(--r-md); background:var(--bg-3);
              border:1px solid var(--border-2); color:var(--text-1);
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
        <p style="font-size:10px; color:var(--text-4); margin-top:6px; line-height:1.5;">
          {t.apiKeyHint}
        </p>
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
      <span>Groq API</span>
      <span>{settings.model}</span>
    </div>
  </div>
{/if}
