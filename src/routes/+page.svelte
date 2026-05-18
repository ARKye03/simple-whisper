<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { getSettings, activeApiKey, activeModel } from "$lib/settings";
  import { t } from "$lib/i18n/es";
  import DropZone from "$lib/components/DropZone.svelte";
  import FileCard from "$lib/components/FileCard.svelte";
  import Icon from "$lib/components/Icons.svelte";
  import { fileNameOf, type FileItem, type Transcript } from "$lib/types";

  let files = $state<FileItem[]>([]);
  let expandedId = $state<number | null>(null);
  let isProcessing = $state(false);
  let nextId = 0;

  const hasFiles = $derived(files.length > 0);
  const queuedCount = $derived(files.filter((f) => f.status === "queued").length);
  const allDone = $derived(hasFiles && files.every((f) => f.status === "completed"));

  async function statSize(_path: string): Promise<number> {
    return 0; // size unknown without fs read permission — leave blank ("—")
  }

  async function onFilesAdded(paths: string[]) {
    const existing = new Set(files.map((f) => f.path));
    const fresh = paths.filter((p) => !existing.has(p));
    if (!fresh.length) return;

    const additions: FileItem[] = [];
    for (const p of fresh) {
      additions.push({
        id: ++nextId,
        name: fileNameOf(p),
        path: p,
        size: await statSize(p),
        status: "queued",
        progress: 0,
        transcript: null,
        error: null,
        index: files.length + additions.length,
      });
    }
    files = [...files, ...additions];
  }

  function removeFile(id: number) {
    files = files.filter((f) => f.id !== id);
    if (expandedId === id) expandedId = null;
  }

  function toggleExpand(id: number) {
    expandedId = expandedId === id ? null : id;
  }

  function clearAll() {
    files = [];
    expandedId = null;
  }

  async function startProcessing() {
    if (isProcessing) return;
    const s = await getSettings();
    if (!activeApiKey(s)) {
      const queued = files.filter((f) => f.status === "queued");
      files = files.map((f) =>
        queued.find((q) => q.id === f.id)
          ? { ...f, status: "error", error: t.apiKeyMissing }
          : f,
      );
      return;
    }

    isProcessing = true;
    const queuedIds = files.filter((f) => f.status === "queued").map((f) => f.id);

    for (const fid of queuedIds) {
      files = files.map((f) =>
        f.id === fid ? { ...f, status: "processing", progress: 5 } : f,
      );

      // Indeterminate-ish animation while invoke runs (real progress would need event plumbing).
      let progress = 5;
      const ticker = setInterval(() => {
        progress = Math.min(92, progress + Math.random() * 6);
        files = files.map((f) => (f.id === fid ? { ...f, progress } : f));
      }, 250);

      try {
        const file = files.find((f) => f.id === fid)!;
        const transcript = await invoke<Transcript>("transcribe_video", {
          videoPath: file.path,
          provider: s.provider,
          apiKey: activeApiKey(s),
          model: activeModel(s),
          language: s.language,
          diarize: s.provider === "gemini" ? s.diarize : false,
        });
        clearInterval(ticker);
        files = files.map((f) =>
          f.id === fid
            ? { ...f, status: "completed", progress: 100, transcript }
            : f,
        );
        expandedId = fid;
      } catch (e) {
        clearInterval(ticker);
        files = files.map((f) =>
          f.id === fid ? { ...f, status: "error", error: String(e) } : f,
        );
        expandedId = fid;
      }
    }

    isProcessing = false;
  }
</script>

<main style="flex:1; overflow-y:auto; padding: 28px;">
  <div style="max-width: 660px; margin: 0 auto;">
    {#if !hasFiles}
      <div style="padding-top: 48px; padding-bottom: 32px;">
        <DropZone compact={false} busy={isProcessing} {onFilesAdded} />
      </div>
    {:else}
      <div style="display:flex; flex-direction:column; gap:14px;">
        <DropZone compact={true} busy={isProcessing} {onFilesAdded} />

        <div style="display:flex; flex-direction:column; gap:10px;">
          {#each files as file (file.id)}
            <FileCard
              {file}
              expanded={expandedId === file.id}
              onRemove={removeFile}
              onToggle={toggleExpand}
            />
          {/each}
        </div>

        <div style="display:flex; align-items:center; justify-content:center; gap:12px; padding-top:8px;">
          {#if queuedCount > 0 && !isProcessing}
            <button class="btn-primary" onclick={startProcessing}>
              {t.transcribeBtn}{queuedCount > 1 ? ` (${queuedCount})` : ""}
            </button>
          {/if}
          {#if isProcessing}
            <span style="font-size:12px; color: var(--text-3); display:flex; align-items:center; gap:8px;">
              <Icon name="spinner" size={14} /> {t.processing}
            </span>
          {/if}
          {#if allDone && !isProcessing}
            <button class="btn-ghost" onclick={clearAll} style="font-size:12px; padding:6px 14px;">
              {t.clearAll}
            </button>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</main>
