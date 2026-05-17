<script lang="ts">
  import { t } from "$lib/i18n/es";
  import { getSettings } from "$lib/settings";
  import { onMount } from "svelte";

  let model = $state("whisper-large-v3-turbo");

  onMount(async () => {
    const s = await getSettings();
    model = s.model;
  });
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
  <span>Groq · {model}</span>
</footer>
