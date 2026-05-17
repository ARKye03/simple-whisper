<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { save } from "@tauri-apps/plugin-dialog";
  import Icon from "./Icons.svelte";
  import { t } from "$lib/i18n/es";

  type Props = { text: string; fileName: string };
  let { text, fileName }: Props = $props();

  let copied = $state(false);
  let downloadError = $state<string | null>(null);
  let downloadErrorTimer: ReturnType<typeof setTimeout> | null = null;

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {}
  }

  function baseName(name: string): string {
    return name.replace(/\.[^./\\]+$/, "");
  }

  function showDownloadError(msg: string) {
    downloadError = msg;
    if (downloadErrorTimer) clearTimeout(downloadErrorTimer);
    downloadErrorTimer = setTimeout(() => (downloadError = null), 5000);
  }

  async function downloadAs(format: "txt" | "md" | "docx" | "pdf") {
    const path = await save({
      defaultPath: `${baseName(fileName)}.${format}`,
      filters: [{ name: format.toUpperCase(), extensions: [format] }],
    });
    if (!path) return;
    try {
      await invoke("save_transcript", { text, path, format });
      downloadError = null;
    } catch (e) {
      showDownloadError(String(e));
    }
  }
</script>

<div
  style="
    padding:10px 14px 14px;
    border-top:1px solid var(--border-1);
    background: var(--bg-2);
    animation: fadeIn .2s ease;
  "
>
  <div style="display:flex; justify-content:flex-end; gap:6px; margin-bottom:10px; flex-wrap:wrap;">
    <button class="btn-ghost" onclick={handleCopy}>
      {#if copied}<Icon name="check" size={11} />{:else}<Icon name="copy" size={11} />{/if}
      {copied ? t.copied : t.copy}
    </button>
    <button class="btn-ghost" onclick={() => downloadAs("txt")}>
      <Icon name="download" size={11} /> .txt
    </button>
    <button class="btn-ghost" onclick={() => downloadAs("md")}>
      <Icon name="download" size={11} /> .md
    </button>
    <button class="btn-ghost" onclick={() => downloadAs("docx")}>
      <Icon name="download" size={11} /> .docx
    </button>
    <button class="btn-ghost" onclick={() => downloadAs("pdf")}>
      <Icon name="download" size={11} /> .pdf
    </button>
  </div>
  {#if downloadError}
    <div
      style="
        font-size:11px; color: var(--error);
        background: var(--error-muted); border: 1px solid var(--error);
        border-radius: var(--r-sm); padding: 6px 8px; margin-bottom: 10px;
        white-space: pre-wrap;
      "
      role="alert"
    >{downloadError}</div>
  {/if}
  <div
    style="
      font-size:12.5px; line-height:1.75; color:var(--text-2);
      max-height:180px; overflow-y:auto; padding-right:4px;
      white-space:pre-wrap;
    "
  >{text}</div>
</div>
