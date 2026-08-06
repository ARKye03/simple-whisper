<script lang="ts">
  import { t } from "$lib/i18n/state.svelte";
  import { settingsStore, activeModel } from "$lib/settings";
</script>

<footer
  style="
    display:flex;
    align-items:center;
    justify-content:space-between;
    padding:0 28px;
    height:40px;
    border-top:1px solid var(--border-1);
    font-size:10px;
    color:var(--text-4);
    flex-shrink:0;
    letter-spacing:0.06em;
    text-transform:uppercase;
  "
>
  <span>{t.brand}</span>
  <span>{t.providerVendor($settingsStore.provider)} · {activeModel($settingsStore)}</span>
</footer>
