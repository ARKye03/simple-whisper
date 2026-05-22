<script lang="ts">
  import Icon from "./Icons.svelte";
  import TranscriptPanel from "./TranscriptPanel.svelte";
  import ErrorPanel from "./ErrorPanel.svelte";
  import { t } from "$lib/i18n/state.svelte";
  import { fmtSize, type FileItem } from "$lib/types";

  type Props = {
    file: FileItem;
    expanded: boolean;
    onRemove: (id: number) => void;
    onToggle: (id: number) => void;
    onRetry?: (id: number) => void;
    onRetryWith?: (id: number, provider: "groq" | "gemini") => void;
    otherProvider?: "groq" | "gemini" | null;
  };
  let {
    file,
    expanded,
    onRemove,
    onToggle,
    onRetry,
    onRetryWith,
    otherProvider = null,
  }: Props = $props();

  const STATUS = $derived({
    queued: { bg: "var(--bg-4)", color: "var(--text-3)", label: t.statusQueued },
    processing: { bg: "var(--accent-muted)", color: "var(--accent)", label: t.statusProcessing },
    completed: { bg: "var(--success-muted)", color: "var(--success)", label: t.statusCompleted },
    error: { bg: "var(--error-muted)", color: "var(--error)", label: t.statusError },
  });

  const st = $derived(STATUS[file.status]);
  const isClickable = $derived(file.status === "completed" || file.status === "error");
</script>

<div
  style="
    background: var(--bg-1);
    border: 1px solid var(--border-1);
    border-radius: var(--r-lg);
    overflow: hidden;
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
      <Icon name="film" size={18} />
    </div>

    <div style="flex:1; min-width:0;">
      <div
        style="font-size:13px; font-weight:500; margin-bottom:1px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;"
        title={file.name}
      >{file.name}</div>
      <div style="font-size:11px; color: var(--text-4);">{fmtSize(file.size)}</div>
    </div>

    <div
      style="
        display:flex; align-items:center; gap:5px;
        padding:3px 9px; border-radius:var(--r-sm);
        background: {st.bg}; font-size:11px; font-weight:500;
        color: {st.color}; white-space:nowrap;
      "
    >
      {#if file.status === "processing"}<Icon name="spinner" size={11} />{/if}
      {#if file.status === "completed"}<Icon name="check" size={11} />{/if}
      {st.label}
    </div>

    {#if file.status === "queued"}
      <button
        class="icon-btn"
        style="width:30px; height:30px;"
        onclick={(e) => {
          e.stopPropagation();
          onRemove(file.id);
        }}
        aria-label="Quitar"
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

  {#if file.status === "processing"}
    <div style="padding:0 14px 10px;">
      <div class="progress-track">
        <div class="progress-fill" style="width: {file.progress || 0}%;"></div>
      </div>
    </div>
  {/if}

  {#if expanded && file.status === "completed" && file.transcript}
    <TranscriptPanel transcript={file.transcript} fileName={file.name} />
  {/if}

  {#if expanded && file.status === "error" && file.error}
    <ErrorPanel
      error={file.error}
      {otherProvider}
      onRetry={onRetry ? () => onRetry(file.id) : undefined}
      onRetryWith={onRetryWith ? (p) => onRetryWith(file.id, p) : undefined}
    />
  {/if}
</div>
