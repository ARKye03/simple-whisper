<script lang="ts">
  import { settingsOpen } from "$lib/drawer";
  import Icon from "./Icons.svelte";
  import { t } from "$lib/i18n/es";
</script>

<header
  style="
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 28px;
    height: 54px;
    border-bottom: 1px solid var(--border-1);
    flex-shrink: 0;
  "
>
  <div style="display:flex; align-items:center; gap:10px;">
    <span style="color: var(--accent); display: flex;">
      <Icon name="waveform" size={20} />
    </span>
    <span style="font-size:15px; font-weight:600; letter-spacing:-0.025em;">
      {t.appTitle}
    </span>
  </div>
  <button
    class="icon-btn"
    onclick={() => settingsOpen.set(true)}
    title={t.settings}
    aria-label={t.settings}
  >
    <Icon name="settings" size={20} />
  </button>
</header>
