<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { loadTheme } from "$lib/theme";
  import { initSettings, settingsStore, patchSettings } from "$lib/settings";
  import { initHistory } from "$lib/history";
  import { checkForUpdates } from "$lib/updater";
  import { t } from "$lib/i18n/state.svelte";
  import Header from "$lib/components/Header.svelte";
  import Footer from "$lib/components/Footer.svelte";
  import SettingsDrawer from "$lib/components/SettingsDrawer.svelte";
  import HistorySidebar from "$lib/components/HistorySidebar.svelte";

  let { children } = $props();

  let ffmpegStatus = $state<"checking" | "ok" | "missing">("checking");

  onMount(async () => {
    await loadTheme();
    await initSettings();
    await initHistory();
    try {
      await invoke<string>("check_ffmpeg");
      ffmpegStatus = "ok";
    } catch {
      ffmpegStatus = "missing";
    }
    checkForUpdates({ silent: true }).catch((err) =>
      console.error("Silent update check failed:", err),
    );
  });

  async function toggleSidebar() {
    await patchSettings({
      historySidebarCollapsed: !$settingsStore.historySidebarCollapsed,
    });
  }
</script>

<div style="display:flex; flex-direction:column; height:100vh; background: var(--bg-0);">
  <Header />

  {#if ffmpegStatus === "missing"}
    <div
      role="alert"
      style="
        background: var(--accent-muted);
        border-bottom: 1px solid var(--accent);
        padding: 10px 28px;
        display: flex;
        align-items: flex-start;
        gap: 10px;
        font-size: 12.5px;
        line-height: 1.5;
        color: var(--text-1);
        flex-shrink: 0;
      "
    >
      <span
        style="color: var(--accent); font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; font-size: 10px; margin-top: 2px;"
      >FFmpeg</span>
      <p style="flex:1; margin:0;">{t.ffmpegMissing}</p>
    </div>
  {/if}

  <div style="flex:1; display:flex; min-height:0;">
    <HistorySidebar
      collapsed={$settingsStore.historySidebarCollapsed}
      onToggle={toggleSidebar}
    />
    <div style="flex:1; min-width:0; display:flex; flex-direction:column;">
      {@render children()}
    </div>
  </div>

  <Footer />
</div>

<SettingsDrawer />
