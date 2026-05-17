<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n/es";
  import { getApiKey, setApiKey } from "$lib/settings";
  import { loadTheme, setTheme, type Theme } from "$lib/theme";

  let apiKey = $state("");
  let theme = $state<Theme>("system");
  let savedFlash = $state(false);

  onMount(async () => {
    apiKey = await getApiKey();
    theme = await loadTheme();
  });

  async function saveKey() {
    await setApiKey(apiKey);
    savedFlash = true;
    setTimeout(() => (savedFlash = false), 1600);
  }

  async function chooseTheme(next: Theme) {
    theme = next;
    await setTheme(next);
  }

  const themes: { value: Theme; label: string; glyph: string }[] = [
    { value: "light", label: t.themeLight, glyph: "☼" },
    { value: "dark", label: t.themeDark, glyph: "☾" },
    { value: "system", label: t.themeSystem, glyph: "◐" },
  ];
</script>

<main class="flex-1">
  <div class="max-w-4xl mx-auto px-5 sm:px-8 pt-8 sm:pt-12 pb-12 sm:pb-16">

    <header class="reveal reveal-1 flex flex-col sm:flex-row sm:items-end sm:justify-between gap-4 sm:gap-8 mb-8 sm:mb-12">
      <div>
        <div class="eyebrow mb-3">Configuración</div>
        <h1 class="display text-[clamp(2.2rem,8vw,4.8rem)] font-light">
          {t.settings}
        </h1>
      </div>
      <a href="/" class="btn-ghost self-start sm:self-auto" style:white-space="nowrap">← {t.back}</a>
    </header>

    <div class="rule mb-12"></div>

    <!-- 01 — API key -->
    <section class="reveal reveal-2 grid grid-cols-1 sm:grid-cols-[4.5rem_1fr] gap-4 sm:gap-8 mb-14">
      <div class="numeral">01</div>
      <div class="flex flex-col gap-4">
        <div class="eyebrow">Credenciales</div>
        <label for="api-key" class="text-[1.05rem] font-medium" style:font-family="var(--font-display)">
          {t.apiKeyLabel}
        </label>
        <div class="flex flex-col sm:flex-row gap-3 sm:items-stretch">
          <input
            id="api-key"
            type="password"
            placeholder={t.apiKeyPlaceholder}
            bind:value={apiKey}
            class="input-field flex-1"
            autocomplete="off"
            spellcheck="false"
          />
          <button type="button" onclick={saveKey} class="btn-primary shrink-0">
            {t.apiKeySave}
          </button>
        </div>
        {#if savedFlash}
          <span class="eyebrow" style:color="var(--color-accent)">✓ {t.apiKeySaved}</span>
        {/if}
      </div>
    </section>

    <!-- 02 — Theme -->
    <section class="reveal reveal-3 grid grid-cols-1 sm:grid-cols-[4.5rem_1fr] gap-4 sm:gap-8">
      <div class="numeral">02</div>
      <div class="flex flex-col gap-4">
        <div class="eyebrow">Apariencia</div>
        <span class="text-[1.05rem] font-medium" style:font-family="var(--font-display)">
          {t.theme}
        </span>
        <div
          class="inline-flex self-start divide-x"
          style:border="1px solid var(--color-rule)"
          style:--tw-divide-color="var(--color-rule)"
        >
          {#each themes as opt (opt.value)}
            <button
              type="button"
              onclick={() => chooseTheme(opt.value)}
              class="px-3 sm:px-5 py-2.5 sm:py-3 transition flex items-center gap-2 sm:gap-2.5 font-mono text-[0.7rem] sm:text-[0.78rem] tracking-[0.12em] sm:tracking-[0.14em] uppercase"
              style:background={theme === opt.value ? "var(--color-ink)" : "transparent"}
              style:color={theme === opt.value ? "var(--color-paper)" : "var(--color-ink)"}
              style:border-color="var(--color-rule)"
            >
              <span class="text-base leading-none" aria-hidden="true">{opt.glyph}</span>
              <span>{opt.label}</span>
            </button>
          {/each}
        </div>
      </div>
    </section>

  </div>
</main>
