<script lang="ts">
  import { Channel, invoke } from "@tauri-apps/api/core";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onDestroy } from "svelte";
  import { settingsOpen } from "$lib/drawer";
  import {
    cookiesArg,
    getSettings,
    otherProviderWithKey,
    settingsStore,
    type AppSettings,
    type Provider,
  } from "$lib/settings";
  import {
    addHistoryEntry,
    historySelectionStore,
    historyStore,
  } from "$lib/history";
  import { t } from "$lib/i18n/state.svelte";
  import { ensureYtdlp } from "$lib/ytdlp";
  import DropZone from "$lib/components/DropZone.svelte";
  import UrlInput from "$lib/components/UrlInput.svelte";
  import FileCard from "$lib/components/FileCard.svelte";
  import Icon from "$lib/components/Icons.svelte";
  import {
    fileNameOf,
    hostLabelOf,
    isSupportedMediaUrl,
    localTranscribeError,
    parseUrlCandidates,
    toTranscribeError,
    urlDedupKey,
    urlMetaFrom,
    type DownloadProgress,
    type FileItem,
    type TranscribeErrorKind,
    type Transcript,
    type UrlProbe,
  } from "$lib/types";

  let files = $state<FileItem[]>([]);
  let expandedId = $state<number | null>(null);
  let isProcessing = $state(false);
  let nextId = 0;
  let lastSettings = $state<AppSettings | null>(null);

  const unsubSettings = settingsStore.subscribe((s) => (lastSettings = s));
  onDestroy(unsubSettings);

  const otherProvider = $derived<Provider | null>(
    lastSettings ? otherProviderWithKey(lastSettings) : null,
  );

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
          path: historyEntry.sourcePath ?? historyEntry.sourceUrl ?? "",
          size: 0,
          status: "completed",
          progress: 100,
          transcript: historyEntry.transcript,
          error: null,
          index: 0,
          source: historyEntry.sourceUrl ? "url" : "file",
          url: historyEntry.sourceUrl ?? undefined,
          meta: null,
          // Must be false, or the card reads "Obteniendo información…" forever.
          probing: false,
          download: null,
        }
      : null,
  );

  function clearHistoryView() {
    historySelectionStore.set(null);
  }

  /** Keeps the never-mutate-in-place invariant in one place. */
  function patchItem(fid: number, patch: Partial<FileItem>) {
    files = files.map((f) => (f.id === fid ? { ...f, ...patch } : f));
  }

  // Pasting a link is otherwise silent feedback for screen readers.
  let liveMessage = $state("");
  let liveTimer: ReturnType<typeof setTimeout> | null = null;
  function announce(msg: string) {
    liveMessage = msg;
    if (liveTimer) clearTimeout(liveTimer);
    liveTimer = setTimeout(() => (liveMessage = ""), 4000);
  }
  onDestroy(() => {
    if (liveTimer) clearTimeout(liveTimer);
  });

  const hasFiles = $derived(files.length > 0);
  const queuedCount = $derived(files.filter((f) => f.status === "queued").length);
  const inFlightCount = $derived(
    files.filter((f) => f.status === "downloading" || f.status === "processing").length,
  );
  // Was `every(f => f.status === "completed")`, which let a single permanent error
  // (rejected live link, missing yt-dlp) hide "Limpiar todo" forever.
  const canClear = $derived(
    hasFiles && !isProcessing && queuedCount === 0 && inFlightCount === 0,
  );

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
        source: "file",
        meta: null,
        download: null,
      });
    }
    files = [...files, ...additions];
  }

  // probe_url failures that can never resolve themselves: park the item in `error`
  // so startProcessing() skips it (it snapshots status === "queued"). Everything
  // else (Network/Timeout/Unknown) stays queued — the real run will surface it and
  // we only lose the resolved title.
  const BLOCKING_PROBE_KINDS: TranscribeErrorKind[] = [
    "YtdlpMissing",
    "UnsupportedUrl",
    "DownloadAuthRequired",
  ];

  async function onUrlsAdded(raw: string[]) {
    if (isProcessing) return; // parity with DropZone's busy guard
    if (viewingHistoryId) clearHistoryView();

    const existing = new Set(files.map((f) => f.path));
    const seen = new Set<string>();
    const additions: FileItem[] = [];

    for (const candidate of raw) {
      if (!isSupportedMediaUrl(candidate)) continue;
      const key = urlDedupKey(candidate);
      if (existing.has(key) || seen.has(key)) continue;
      seen.add(key);
      additions.push({
        id: ++nextId,
        name: hostLabelOf(candidate), // optimistic; probe_url swaps in the title
        path: key,
        size: 0,
        status: "queued",
        progress: 0,
        transcript: null,
        error: null,
        index: files.length + additions.length,
        source: "url",
        url: candidate,
        meta: null,
        probing: true,
        download: null,
      });
    }
    if (!additions.length) return;
    files = [...files, ...additions];
    announce(
      additions.length === 1 ? t.urlAddedOne : t.urlAddedCount(additions.length),
    );

    // A missing yt-dlp fails every item identically, so say so once instead of
    // firing N probes that all fail the same way.
    const yt = await ensureYtdlp();
    if (yt.state === "missing") {
      for (const a of additions) {
        patchItem(a.id, {
          probing: false,
          status: "error",
          error: localTranscribeError("YtdlpMissing", "yt-dlp no encontrado"),
        });
      }
      expandedId = additions[0].id;
      return;
    }

    await probeAll(additions);
  }

  // Bounded, because every probe is a yt-dlp process and, with cookiesBrowser set,
  // one macOS Keychain prompt. Pasting a 30-link blob must not spawn 30 of each.
  const PROBE_CONCURRENCY = 3;
  async function probeAll(items: FileItem[]) {
    let next = 0;
    const worker = async () => {
      while (next < items.length) {
        const item = items[next++];
        await probeItem(item.id, item.url!);
      }
    };
    await Promise.all(
      Array.from({ length: Math.min(PROBE_CONCURRENCY, items.length) }, worker),
    );
  }

  async function probeItem(fid: number, url: string) {
    patchItem(fid, { probing: true });
    try {
      const info = await invoke<UrlProbe>("probe_url", {
        url,
        cookiesBrowser: lastSettings ? cookiesArg(lastSettings) : null,
      });

      if (info.is_playlist) {
        await expandPlaylist(fid, info);
        return;
      }

      const meta = urlMetaFrom(info);
      if (info.is_live) {
        patchItem(fid, {
          probing: false,
          meta,
          status: "error",
          error: localTranscribeError(
            "LiveUnsupported",
            "Las transmisiones en vivo no se pueden transcribir",
          ),
        });
        expandedId = fid;
        return;
      }

      patchItem(fid, { probing: false, meta, name: meta.title ?? hostLabelOf(url) });
    } catch (e) {
      const err = toTranscribeError(e);
      if (BLOCKING_PROBE_KINDS.includes(err.kind)) {
        patchItem(fid, { probing: false, status: "error", error: err });
        expandedId = fid;
      } else {
        patchItem(fid, { probing: false }); // stays queued, keeps the host as its name
      }
    }
  }

  /**
   * Replaces the single placeholder card with one queued item per playlist entry.
   * Entries are not probed individually — --flat-playlist already supplies title and
   * duration, and transcribe_url probes each one at download time anyway.
   */
  async function expandPlaylist(fid: number, info: UrlProbe) {
    const entries = info.entries.filter((e) => isSupportedMediaUrl(e.url));
    const total = info.entry_count ?? entries.length;

    if (!entries.length) {
      patchItem(fid, {
        probing: false,
        status: "error",
        error: localTranscribeError(
          "UnsupportedUrl",
          "La lista no contiene videos accesibles",
        ),
      });
      expandedId = fid;
      return;
    }

    const ok = await ask(t.playlistConfirmBody(entries.length, total), {
      title: t.playlistConfirmTitle,
      kind: "info",
      okLabel: t.playlistConfirmOk,
    });
    if (!ok) {
      removeFile(fid);
      return;
    }

    const existing = new Set(files.filter((f) => f.id !== fid).map((f) => f.path));
    const items: FileItem[] = [];
    for (const e of entries) {
      const key = urlDedupKey(e.url);
      if (existing.has(key)) continue;
      existing.add(key);
      items.push({
        id: ++nextId,
        name: e.title ?? hostLabelOf(e.url),
        path: key,
        size: 0,
        status: "queued",
        progress: 0,
        transcript: null,
        error: null,
        index: 0, // reassigned below so the entry animation stays staggered
        source: "url",
        url: e.url,
        meta: {
          title: e.title,
          durationSecs: e.duration_secs,
          uploader: info.uploader,
          extractor: info.extractor,
        },
        probing: false,
        download: null,
      });
    }

    // `at === -1` would make the two slices overlap and duplicate every existing
    // item, which blows up the keyed {#each} on duplicate ids. Append instead.
    const at = files.findIndex((f) => f.id === fid);
    const next =
      at === -1
        ? [...files, ...items]
        : [...files.slice(0, at), ...items, ...files.slice(at + 1)];
    files = next.map((f, i) => ({ ...f, index: i }));

    announce(
      entries.length < total
        ? t.playlistCapNotice(entries.length, total)
        : t.playlistExpanded(items.length),
    );
  }

  function isTextEntry(target: EventTarget | null): boolean {
    const el = target as HTMLElement | null;
    if (!el?.tagName) return false;
    return (
      el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable === true
    );
  }

  function onWindowPaste(e: ClipboardEvent) {
    if (isProcessing) return;
    if ($settingsOpen) return; // never race the API-key field in the drawer
    if (isTextEntry(e.target)) return; // UrlInput handles its own paste
    const urls = parseUrlCandidates(e.clipboardData?.getData("text") ?? "").filter(
      isSupportedMediaUrl,
    );
    if (!urls.length) return; // not a link: leave the paste alone
    e.preventDefault();
    // onUrlsAdded announces the count it actually queued (duplicates are dropped).
    void onUrlsAdded(urls);
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

  const DL_SHARE = 50; // download owns 0-50% of the bar, transcription 50-100%
  const STALL_MS = 3000; // no progress frame for this long => download is done
  const STALL_ARM_PCT = 90; // ...but only once the download is nearly finished

  /** Monotonic: a real download frame must never visually rewind the pre-roll. */
  function progressWriter(fid: number) {
    let shown = 0;
    return (p: number) => {
      shown = Math.max(shown, Math.min(100, p));
      patchItem(fid, { progress: shown });
    };
  }

  /** The pre-existing indeterminate ticker, extracted and parameterized. */
  function creep(write: (p: number) => void, from: number, to: number, step: number) {
    let p = from;
    return setInterval(() => {
      p = Math.min(to, p + Math.random() * step);
      write(p);
    }, 250);
  }

  type RunArgs = {
    provider: Provider;
    apiKey: string;
    model: string;
    language: AppSettings["language"];
    diarize: boolean;
    cookiesBrowser: string | null;
  };

  async function runLocalFile(fid: number, path: string, a: RunArgs): Promise<Transcript> {
    patchItem(fid, { status: "processing", progress: 5, error: null });
    const write = progressWriter(fid);
    write(5);
    const ticker = creep(write, 5, 92, 6);
    try {
      return await invoke<Transcript>("transcribe_video", {
        videoPath: path,
        provider: a.provider,
        apiKey: a.apiKey,
        model: a.model,
        language: a.language,
        diarize: a.diarize,
      });
    } finally {
      clearInterval(ticker);
    }
  }

  async function runUrlItem(fid: number, url: string, a: RunArgs): Promise<Transcript> {
    patchItem(fid, {
      status: "downloading",
      progress: 1,
      error: null,
      download: null,
    });

    const write = progressWriter(fid);
    let phase: "download" | "transcribe" = "download";
    // Keeps the bar alive while yt-dlp resolves the URL and picks formats, before
    // it emits its first frame. Killed by that first real frame.
    let preroll: ReturnType<typeof setInterval> | null = creep(write, 1, 20, 1);
    let ticker: ReturnType<typeof setInterval> | null = null;
    let stall: ReturnType<typeof setTimeout> | null = null;

    function stopPreroll() {
      if (preroll) {
        clearInterval(preroll);
        preroll = null;
      }
    }

    function toTranscribePhase() {
      if (phase === "transcribe") return;
      phase = "transcribe";
      stopPreroll();
      if (stall) {
        clearTimeout(stall);
        stall = null;
      }
      patchItem(fid, { status: "processing", download: null });
      write(DL_SHARE);
      ticker = creep(write, DL_SHARE, 96, 4);
    }

    const onProgress = new Channel<DownloadProgress>();
    onProgress.onmessage = (msg) => {
      // The backend also sends "extract"/"transcribe" stage markers.
      if (msg.stage !== "download") {
        toTranscribePhase();
        return;
      }
      if (phase === "transcribe") return; // trailing frames after the flip
      stopPreroll();
      const pct = Math.max(0, Math.min(100, msg.percent ?? 0));
      patchItem(fid, { download: msg });
      write((pct * DL_SHARE) / 100);
      if (pct >= 99.5) {
        toTranscribePhase();
        return;
      }
      // Watchdog: yt-dlp's last frame is often 99.x and post-processing emits
      // nothing. Armed only near the end — mid-download gaps are normal (a stalled
      // socket waits up to --socket-timeout 30s) and the flip is irreversible, so
      // arming it early would strand the card in a fake "transcribing" state.
      if (stall) clearTimeout(stall);
      if (pct >= STALL_ARM_PCT) stall = setTimeout(toTranscribePhase, STALL_MS);
    };

    try {
      return await invoke<Transcript>("transcribe_url", {
        url,
        provider: a.provider,
        apiKey: a.apiKey,
        model: a.model,
        language: a.language,
        diarize: a.diarize,
        cookiesBrowser: a.cookiesBrowser,
        onProgress,
      });
    } finally {
      phase = "transcribe"; // silence any frame racing the resolve/reject
      stopPreroll();
      if (stall) clearTimeout(stall);
      if (ticker) clearInterval(ticker);
    }
  }

  async function runFile(fid: number, override: Provider | null = null) {
    const s = await getSettings();
    const provider = effectiveProvider(s, override);
    const apiKey = effectiveKey(s, override);

    if (!apiKey) {
      patchItem(fid, {
        status: "error",
        error: localTranscribeError("ApiKeyMissing", "Missing API key", provider),
      });
      expandedId = fid;
      return;
    }

    // Was `files.find(...)!` inside the try, so an item removed mid-run threw and
    // then wrote an error onto a nonexistent id.
    const file = files.find((f) => f.id === fid);
    if (!file) return;

    // Retry path: a URL item with no meta either failed a blocking probe or hit a
    // transient one. Re-probe first, and if it parks in `error` again, don't
    // start a download.
    if (file.source === "url" && !file.meta) {
      await probeItem(fid, file.url!);
      // The item may also be gone entirely: a playlist re-probe replaces it with its
      // expanded entries, or removes it if the confirmation was cancelled.
      const after = files.find((f) => f.id === fid);
      if (!after || after.status === "error") return;
    }

    const args: RunArgs = {
      provider,
      apiKey,
      model: effectiveModel(s, override),
      language: s.language,
      diarize: provider === "gemini" ? s.diarize : false,
      cookiesBrowser: cookiesArg(s),
    };

    try {
      const transcript =
        file.source === "url"
          ? await runUrlItem(fid, file.url!, args)
          : await runLocalFile(fid, file.path, args);

      patchItem(fid, {
        status: "completed",
        progress: 100,
        transcript,
        error: null,
        download: null,
      });
      expandedId = fid;

      // Re-read: probe_url may have swapped `name` from host to title while the
      // download was running, and history should record the resolved title.
      const done = files.find((f) => f.id === fid);
      try {
        await addHistoryEntry({
          filename: done?.name ?? file.name,
          sourcePath: file.source === "url" ? null : file.path,
          sourceUrl: file.source === "url" ? file.url! : null,
          provider,
          model: args.model,
          language: args.language,
          diarize: args.diarize,
          transcript,
        });
      } catch (err) {
        console.error("Failed to persist history entry:", err);
      }
    } catch (e) {
      patchItem(fid, {
        status: "error",
        error: toTranscribeError(e),
        download: null,
      });
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

  async function retryFileWith(fid: number, provider: Provider) {
    if (isProcessing) return;
    isProcessing = true;
    await runFile(fid, provider);
    isProcessing = false;
  }
</script>

<svelte:window onpaste={onWindowPaste} />

<!-- The preventDefault pair is a safety net: a link dropped outside the DropZone
     must not navigate the webview and white-screen the app. -->
<main
  style="flex:1; overflow-y:auto; padding: 28px;"
  ondragover={(e) => e.preventDefault()}
  ondrop={(e) => e.preventDefault()}
>
  <div style="max-width: 660px; margin: 0 auto;">
    <span
      role="status"
      aria-live="polite"
      style="position:absolute; width:1px; height:1px; margin:-1px; overflow:hidden; clip-path:inset(50%); white-space:nowrap;"
    >{liveMessage}</span>

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
          <div style="display:flex; align-items:center; gap:10px;">
            {#if historyEntry?.sourceUrl}
              <button
                class="btn-ghost"
                style="font-size:11px;"
                onclick={() => openUrl(historyEntry!.sourceUrl!)}
                title={historyEntry.sourceUrl}
              >
                <Icon name="link" size={12} /> {t.historyOpenSource}
              </button>
            {/if}
            <span style="font-size:11px; color: var(--text-4); letter-spacing:0.08em; text-transform:uppercase;">
              {t.historyTitle}
            </span>
          </div>
        </div>
        <FileCard file={historyFileItem} expanded={true} onToggle={() => {}} />
      </div>
    {:else if !hasFiles}
      <div
        style="padding-top: 48px; padding-bottom: 32px; display:flex; flex-direction:column; gap:18px;"
      >
        <DropZone compact={false} busy={isProcessing} {onFilesAdded} {onUrlsAdded} />
        <div style="display:flex; align-items:center; gap:12px;">
          <span style="flex:1; height:1px; background:var(--border-1);"></span>
          <span style="font-size:10.5px; color:var(--text-4); letter-spacing:0.08em; text-transform:uppercase;">
            {t.orPasteLink}
          </span>
          <span style="flex:1; height:1px; background:var(--border-1);"></span>
        </div>
        <UrlInput busy={isProcessing} {onUrlsAdded} />
      </div>
    {:else}
      <div style="display:flex; flex-direction:column; gap:14px;">
        <div style="display:flex; gap:8px; align-items:flex-start;">
          <div style="flex:0 0 auto;">
            <DropZone compact={true} busy={isProcessing} {onFilesAdded} {onUrlsAdded} />
          </div>
          <div style="flex:1; min-width:0;">
            <UrlInput compact={true} busy={isProcessing} {onUrlsAdded} />
          </div>
        </div>

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
          {#if canClear}
            <button class="btn-ghost" onclick={clearAll} style="font-size:12px; padding:6px 14px;">
              {t.clearAll}
            </button>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</main>
