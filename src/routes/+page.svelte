<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy } from "svelte";
  import {
    getSettings,
    type AppSettings,
    type Provider,
  } from "$lib/settings";
  import {
    addHistoryEntry,
    historySelectionStore,
    historyStore,
  } from "$lib/history";
  import { t } from "$lib/i18n/state.svelte";
  import DropZone from "$lib/components/DropZone.svelte";
  import FileCard from "$lib/components/FileCard.svelte";
  import Icon from "$lib/components/Icons.svelte";
  import {
    fileNameOf,
    toTranscribeError,
    type FileItem,
    type Transcript,
  } from "$lib/types";

  let files = $state<FileItem[]>([]);
  let expandedId = $state<number | null>(null);
  let isProcessing = $state(false);
  let nextId = 0;
  let lastSettings = $state<AppSettings | null>(null);

  const otherProvider = $derived.by<Provider | null>(() => {
    if (!lastSettings) return null;
    const other: Provider = lastSettings.provider === "groq" ? "gemini" : "groq";
    const otherKey =
      other === "groq" ? lastSettings.groqApiKey : lastSettings.geminiApiKey;
    return otherKey ? other : null;
  });

  getSettings().then((s) => (lastSettings = s));

  let viewingHistoryId = $state<string | null>(null);
  const unsubHistorySel = historySelectionStore.subscribe((id) => {
    viewingHistoryId = id;
    if (id) expandedId = null;
  });
  onDestroy(unsubHistorySel);

  const historyEntry = $derived(
    viewingHistoryId
      ? $historyStore.find((e) => e.id === viewingHistoryId) ?? null
      : null,
  );
  const historyFileItem = $derived<FileItem | null>(
    historyEntry
      ? {
          id: -1,
          name: historyEntry.filename,
          path: historyEntry.sourcePath ?? "",
          size: 0,
          status: "completed",
          progress: 100,
          transcript: historyEntry.transcript,
          error: null,
          index: 0,
        }
      : null,
  );

  function clearHistoryView() {
    historySelectionStore.set(null);
  }

  const hasFiles = $derived(files.length > 0);
  const queuedCount = $derived(files.filter((f) => f.status === "queued").length);
  const allDone = $derived(hasFiles && files.every((f) => f.status === "completed"));

  async function statSize(_path: string): Promise<number> {
    return 0; // size unknown without fs read permission — leave blank ("—")
  }

  async function onFilesAdded(paths: string[]) {
    if (viewingHistoryId) clearHistoryView();
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

  function effectiveProvider(s: AppSettings, override: Provider | null): Provider {
    return override ?? s.provider;
  }

  function effectiveKey(s: AppSettings, override: Provider | null): string {
    const p = effectiveProvider(s, override);
    return p === "groq" ? s.groqApiKey : s.geminiApiKey;
  }

  function effectiveModel(s: AppSettings, override: Provider | null): string {
    const p = effectiveProvider(s, override);
    return p === "groq" ? s.groqModel : s.geminiModel;
  }

  async function runFile(fid: number, override: Provider | null = null) {
    const s = await getSettings();
    lastSettings = s;
    const provider = effectiveProvider(s, override);
    const apiKey = effectiveKey(s, override);

    if (!apiKey) {
      files = files.map((f) =>
        f.id === fid
          ? {
              ...f,
              status: "error",
              error: {
                kind: "ApiKeyMissing",
                provider,
                message: "Missing API key",
                retry_after_secs: null,
                raw: null,
              },
            }
          : f,
      );
      expandedId = fid;
      return;
    }

    files = files.map((f) =>
      f.id === fid
        ? { ...f, status: "processing", progress: 5, error: null }
        : f,
    );

    // Indeterminate-ish animation while invoke runs (real progress would need event plumbing).
    let progress = 5;
    const ticker = setInterval(() => {
      progress = Math.min(92, progress + Math.random() * 6);
      files = files.map((f) => (f.id === fid ? { ...f, progress } : f));
    }, 250);

    try {
      const file = files.find((f) => f.id === fid)!;
      const model = effectiveModel(s, override);
      const transcript = await invoke<Transcript>("transcribe_video", {
        videoPath: file.path,
        provider,
        apiKey,
        model,
        language: s.language,
        diarize: provider === "gemini" ? s.diarize : false,
      });
      clearInterval(ticker);
      files = files.map((f) =>
        f.id === fid
          ? { ...f, status: "completed", progress: 100, transcript, error: null }
          : f,
      );
      expandedId = fid;
      try {
        await addHistoryEntry({
          filename: file.name,
          sourcePath: file.path,
          provider,
          model,
          language: s.language,
          diarize: provider === "gemini" ? s.diarize : false,
          transcript,
        });
      } catch (err) {
        console.error("Failed to persist history entry:", err);
      }
    } catch (e) {
      clearInterval(ticker);
      files = files.map((f) =>
        f.id === fid
          ? { ...f, status: "error", error: toTranscribeError(e) }
          : f,
      );
      expandedId = fid;
    }
  }

  async function startProcessing() {
    if (isProcessing) return;
    isProcessing = true;
    const queuedIds = files.filter((f) => f.status === "queued").map((f) => f.id);
    for (const fid of queuedIds) {
      await runFile(fid, null);
    }
    isProcessing = false;
  }

  async function retryFile(fid: number) {
    if (isProcessing) return;
    isProcessing = true;
    await runFile(fid, null);
    isProcessing = false;
  }

  async function retryFileWith(fid: number, provider: "groq" | "gemini") {
    if (isProcessing) return;
    isProcessing = true;
    await runFile(fid, provider);
    isProcessing = false;
  }
</script>

<main style="flex:1; overflow-y:auto; padding: 28px;">
  <div style="max-width: 660px; margin: 0 auto;">
    {#if historyFileItem}
      <div style="display:flex; flex-direction:column; gap:14px;">
        <div style="display:flex; align-items:center; justify-content:space-between;">
          <button
            class="btn-ghost"
            onclick={clearHistoryView}
            style="font-size:11px;"
          >
            <Icon name="chevron-left" size={12} /> {t.closeAria}
          </button>
          <span style="font-size:11px; color: var(--text-4); letter-spacing:0.08em; text-transform:uppercase;">
            {t.historyTitle}
          </span>
        </div>
        <FileCard
          file={historyFileItem}
          expanded={true}
          onRemove={() => {}}
          onToggle={() => {}}
        />
      </div>
    {:else if !hasFiles}
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
              onRetry={retryFile}
              onRetryWith={retryFileWith}
              {otherProvider}
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
