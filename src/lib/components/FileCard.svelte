<script lang="ts">
  import Icon from "./Icons.svelte";
  import TranscriptPanel from "./TranscriptPanel.svelte";
  import ErrorPanel from "./ErrorPanel.svelte";
  import { t } from "$lib/i18n/state.svelte";
  import type { Provider } from "$lib/settings";
  import {
    fmtDuration,
    fmtEta,
    fmtSize,
    fmtSpeed,
    hostLabelOf,
    type FileItem,
    type LocalStage,
  } from "$lib/types";

  type Props = {
    file: FileItem;
    expanded: boolean;
    /** Omitted for read-only cards (history detail), which hides the trash button. */
    onRemove?: (id: number) => void;
    onToggle: (id: number) => void;
    onRetry?: (id: number) => void;
    onRetryWith?: (id: number, provider: Provider) => void;
    retryOptions?: Provider[];
  };
  let {
    file,
    expanded,
    onRemove,
    onToggle,
    onRetry,
    onRetryWith,
    retryOptions = [],
  }: Props = $props();

  const STATUS = $derived({
    queued: { bg: "var(--bg-4)", color: "var(--text-3)", label: t.statusQueued },
    downloading: { bg: "var(--accent-subtle)", color: "var(--accent)", label: t.statusDownloading },
    processing: { bg: "var(--accent-muted)", color: "var(--accent)", label: t.statusProcessing },
    completed: { bg: "var(--success-muted)", color: "var(--success)", label: t.statusCompleted },
    error: { bg: "var(--error-muted)", color: "var(--error)", label: t.statusError },
  });

  const st = $derived(STATUS[file.status]);
  const isClickable = $derived(file.status === "completed" || file.status === "error");

  function stageLabel(stage: LocalStage): string {
    switch (stage) {
      case "starting": return t.localStageStarting;
      case "loading_model": return t.localStageLoadingModel;
      case "downloading_model": return t.localStageDownloadingModel;
      case "cuda_fallback": return t.localStageCudaFallback;
      case "transcribing": return t.localStageTranscribing;
      case "finalizing": return t.localStageFinalizing;
    }
  }

  const isUrl = $derived(file.source === "url");
  const inFlight = $derived(file.status === "downloading" || file.status === "processing");
  const removable = $derived(!inFlight);
  // A rejected live stream can never succeed on retry.
  const retryable = $derived(file.error?.kind !== "LiveUnsupported");

  const subtitle = $derived.by(() => {
    if (!isUrl) return fmtSize(file.size);
    if (file.probing) return t.urlResolving;
    const bits: string[] = [];
    if (file.meta?.uploader) bits.push(file.meta.uploader);
    if (file.meta?.durationSecs) bits.push(fmtDuration(file.meta.durationSecs));
    if (!bits.length) bits.push(hostLabelOf(file.url ?? file.path));
    return bits.join(" · ");
  });

  const dlSpeed = $derived(fmtSpeed(file.download?.speed_bps ?? null));
  const dlEta = $derived(fmtEta(file.download?.eta_secs ?? null));

  // Quantized to 25% steps so a screen reader isn't flooded at 4 frames/second.
  const announcement = $derived(
    file.status === "downloading"
      ? t.downloadProgressAria(Math.floor((file.download?.percent ?? 0) / 25) * 25)
      : file.status === "processing"
        ? t.statusProcessing
        : file.status === "completed"
          ? t.statusCompleted
          : file.status === "error"
            ? t.statusError
            : "",
  );
</script>

<div
  style="
    background: var(--bg-1);
    border: 1px solid var(--border-1);
    border-radius: var(--r-lg);
    overflow: hidden;
    position: relative;
    animation: fadeInUp .3s ease both;
    animation-delay: {(file.index || 0) * 40}ms;
    transition: border-color var(--t);
  "
  onmouseenter={(e) => ((e.currentTarget as HTMLElement).style.borderColor = "var(--border-2)")}
  onmouseleave={(e) => ((e.currentTarget as HTMLElement).style.borderColor = "var(--border-1)")}
  role="group"
>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    onclick={() => isClickable && onToggle(file.id)}
    onkeydown={(e) => isClickable && (e.key === "Enter" || e.key === " ") && onToggle(file.id)}
    role={isClickable ? "button" : "presentation"}
    tabindex={isClickable ? 0 : undefined}
    style="
      display:flex; align-items:center; gap:12px;
      padding:12px 14px;
      cursor: {isClickable ? 'pointer' : 'default'};
    "
  >
    <div
      style="
        width:38px; height:38px; border-radius:var(--r-md);
        background: var(--bg-3); display:flex;
        align-items:center; justify-content:center;
        color: var(--text-4); flex-shrink:0;
      "
    >
      <Icon name={isUrl ? "link" : "film"} size={18} />
    </div>

    <div style="flex:1; min-width:0;">
      <div
        style="font-size:13px; font-weight:500; margin-bottom:1px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;"
        title={file.name}
      >{file.name}</div>
      <div
        style="font-size:11px; color: var(--text-4); overflow:hidden; text-overflow:ellipsis; white-space:nowrap;"
        title={isUrl ? file.url : undefined}
      >{subtitle}</div>
    </div>

    <div
      style="
        display:flex; align-items:center; gap:5px;
        padding:3px 9px; border-radius:var(--r-sm);
        background: {st.bg}; font-size:11px; font-weight:500;
        color: {st.color}; white-space:nowrap;
      "
    >
      {#if inFlight}<Icon name="spinner" size={11} />{/if}
      {#if file.status === "completed"}<Icon name="check" size={11} />{/if}
      {st.label}
    </div>

    {#if removable && onRemove}
      <button
        class="icon-btn"
        style="width:30px; height:30px;"
        onclick={(e) => {
          e.stopPropagation();
          onRemove?.(file.id);
        }}
        aria-label={t.remove}
      >
        <Icon name="trash" size={14} />
      </button>
    {/if}
    {#if isClickable}
      <span
        style="color: var(--text-4); display:flex; transform: {expanded ? 'rotate(180deg)' : 'rotate(0deg)'}; transition: transform var(--t);"
      >
        <Icon name="chevron-down" size={15} />
      </span>
    {/if}
  </div>

  {#if inFlight}
    <div style="padding:0 14px 10px; display:flex; flex-direction:column; gap:5px;">
      <div
        class="progress-track"
        role="progressbar"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={Math.round(file.progress || 0)}
        aria-label={st.label}
      >
        <div class="progress-fill" style="width: {file.progress || 0}%;"></div>
      </div>
      {#if file.status === "downloading" && file.download}
        <div
          aria-hidden="true"
          style="font-size:10.5px; color:var(--text-4); display:flex; gap:5px;"
        >
          <span>{Math.round(file.download.percent)}%</span>
          {#if dlSpeed}<span>·</span><span>{dlSpeed}</span>{/if}
          {#if dlEta}<span>·</span><span>{t.downloadEta(dlEta)}</span>{/if}
        </div>
      {:else if file.stage}
        <div style="font-size:10px; color:var(--text-4); overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
          {stageLabel(file.stage)}{file.stageDetail ? ` · ${file.stageDetail}` : ""}
        </div>
      {/if}
    </div>
  {/if}

  {#if expanded && file.status === "completed" && file.transcript}
    <TranscriptPanel transcript={file.transcript} fileName={file.name} />
  {/if}

  {#if expanded && file.status === "error" && file.error}
    <ErrorPanel
      error={file.error}
      {retryOptions}
      onRetry={retryable && onRetry ? () => onRetry(file.id) : undefined}
      onRetryWith={retryable && onRetryWith ? (p) => onRetryWith(file.id, p) : undefined}
    />
  {/if}

  <span
    role="status"
    aria-live="polite"
    style="position:absolute; width:1px; height:1px; margin:-1px; padding:0; overflow:hidden; clip-path:inset(50%); white-space:nowrap; border:0;"
  >{announcement}</span>
</div>
