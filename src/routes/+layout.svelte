<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { loadTheme } from "$lib/theme";
  import { t } from "$lib/i18n/es";

  let { children } = $props();

  let ffmpegStatus = $state<"checking" | "ok" | "missing">("checking");

  onMount(async () => {
    await loadTheme();
    try {
      await invoke<string>("check_ffmpeg");
      ffmpegStatus = "ok";
    } catch {
      ffmpegStatus = "missing";
    }
  });
</script>

<div class="grain"></div>

<div class="relative z-10 min-h-screen flex flex-col">
  {#if ffmpegStatus === "missing"}
    <div
      class="border-b border-[var(--color-accent)]/40"
      style:background="var(--color-accent-quiet)"
      role="alert"
    >
      <div class="max-w-4xl mx-auto px-5 sm:px-8 py-3 flex items-start gap-3 sm:gap-4">
        <span class="eyebrow" style:color="var(--color-accent)">FFmpeg</span>
        <p class="text-[0.86rem] leading-relaxed flex-1" style:color="var(--color-ink)">
          {t.ffmpegMissing}
        </p>
      </div>
    </div>
  {/if}

  {@render children()}

  <footer class="mt-auto border-t border-[var(--color-rule)]">
    <div class="max-w-4xl mx-auto px-5 sm:px-8 py-3 sm:py-4 flex flex-col sm:flex-row sm:justify-between sm:items-center gap-1 sm:gap-0">
      <span class="eyebrow">Simple Whisper · 2026</span>
      <span class="eyebrow">Groq · Whisper-v3-turbo</span>
    </div>
  </footer>
</div>
