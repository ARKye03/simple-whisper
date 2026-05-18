<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { save } from "@tauri-apps/plugin-dialog";
  import Icon from "./Icons.svelte";
  import { t } from "$lib/i18n/es";
  import { transcriptToPlainText, type Transcript } from "$lib/types";

  type Props = { transcript: Transcript; fileName: string };
  let { transcript, fileName }: Props = $props();

  const plainText = $derived(transcriptToPlainText(transcript));
  const isDiarized = $derived(transcript.kind === "diarized");

  let copied = $state(false);
  let downloadError = $state<string | null>(null);
  let downloadErrorTimer: ReturnType<typeof setTimeout> | null = null;

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(plainText);
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
      await invoke("save_transcript", { transcript, path, format });
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
  <div class="actions-bar">
    <button class="action-primary" class:is-copied={copied} onclick={handleCopy}>
      <Icon name={copied ? "check" : "copy"} size={13} />
      <span>{copied ? t.copied : t.copy}</span>
    </button>

    <div class="download-group" role="group" aria-label={t.saveAs}>
      <span class="group-label" aria-hidden="true">{t.saveAs}</span>
      <div class="chip-strip">
        <button class="chip" onclick={() => downloadAs("txt")} aria-label="{t.saveAs} TXT">
          <Icon name="download" size={11} /> <span>TXT</span>
        </button>
        <span class="chip-divider" aria-hidden="true"></span>
        <button class="chip" onclick={() => downloadAs("md")} aria-label="{t.saveAs} MD">
          <Icon name="download" size={11} /> <span>MD</span>
        </button>
        <span class="chip-divider" aria-hidden="true"></span>
        <button class="chip" onclick={() => downloadAs("docx")} aria-label="{t.saveAs} DOCX">
          <Icon name="download" size={11} /> <span>DOCX</span>
        </button>
        <span class="chip-divider" aria-hidden="true"></span>
        <button class="chip" onclick={() => downloadAs("pdf")} aria-label="{t.saveAs} PDF">
          <Icon name="download" size={11} /> <span>PDF</span>
        </button>
      </div>
    </div>
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
  >
    {#if isDiarized && transcript.kind === "diarized"}
      {#each transcript.segments as seg, i (i)}
        <div style="margin-bottom:8px;">
          <span style="font-weight:600; color:var(--text-1);">Hablante {seg.speaker}:</span>
          <span> {seg.text}</span>
        </div>
      {/each}
    {:else}
      {plainText}
    {/if}
  </div>
</div>

<style>
  .actions-bar {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 18px;
    margin-bottom: 14px;
    flex-wrap: wrap;
  }

  .action-primary {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    padding: 0 14px;
    border-radius: var(--r-md);
    background: var(--accent);
    color: #fff;
    border: none;
    font-family: var(--font);
    font-size: 12.5px;
    font-weight: 600;
    letter-spacing: 0.01em;
    cursor: pointer;
    transition: background var(--t), transform var(--t), box-shadow var(--t);
    box-shadow:
      0 1px 0 rgba(0, 0, 0, 0.18),
      0 6px 16px -10px var(--accent);
  }
  .action-primary:hover {
    background: var(--accent-hover);
    transform: translateY(-1px);
    box-shadow:
      0 1px 0 rgba(0, 0, 0, 0.22),
      0 10px 22px -10px var(--accent);
  }
  .action-primary:active { transform: translateY(0); }
  .action-primary.is-copied {
    background: var(--success);
    color: #062b1d;
    box-shadow:
      0 1px 0 rgba(0, 0, 0, 0.18),
      0 6px 16px -10px var(--success);
  }
  .action-primary:focus-visible {
    outline: 2px solid var(--accent-hover);
    outline-offset: 2px;
  }

  .download-group {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }

  .group-label {
    font-family: var(--font);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-3);
    font-feature-settings: "tnum" 1;
  }

  .chip-strip {
    display: inline-flex;
    align-items: stretch;
    background: var(--bg-3);
    border: 1px solid var(--border-2);
    border-radius: var(--r-md);
    padding: 3px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text-2);
    border: none;
    font-family: var(--font);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    cursor: pointer;
    transition: background var(--t), color var(--t);
  }
  .chip:hover {
    background: var(--bg-1);
    color: var(--text-1);
  }
  .chip:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
    color: var(--text-1);
  }

  .chip-divider {
    width: 1px;
    background: var(--border-2);
    margin: 4px 0;
    align-self: stretch;
  }
</style>
