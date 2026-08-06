<script lang="ts">
  import Icon from "./Icons.svelte";
  import { t } from "$lib/i18n/state.svelte";
  import {
    historyStore,
    historySelectionStore,
    deleteHistoryEntry,
    clearHistory,
  } from "$lib/history";

  type Props = {
    collapsed: boolean;
    onToggle: () => void;
  };
  let { collapsed, onToggle }: Props = $props();

  let hoveredId = $state<string | null>(null);
  let confirmingClear = $state(false);

  function relativeTime(ts: number): string {
    const diff = Math.max(0, Date.now() - ts);
    const min = Math.floor(diff / 60_000);
    if (min < 1) return t.historyJustNow;
    if (min < 60) return t.historyMinutesAgo(min);
    const hr = Math.floor(min / 60);
    if (hr < 24) return t.historyHoursAgo(hr);
    const day = Math.floor(hr / 24);
    return t.historyDaysAgo(day);
  }

  function selectEntry(id: string) {
    historySelectionStore.set(id);
  }

  async function onDelete(e: MouseEvent, id: string) {
    e.stopPropagation();
    await deleteHistoryEntry(id);
  }

  async function onClearAll() {
    if (!confirmingClear) {
      confirmingClear = true;
      setTimeout(() => (confirmingClear = false), 3500);
      return;
    }
    confirmingClear = false;
    await clearHistory();
  }
</script>

<aside
  class="history-sidebar"
  class:collapsed
  aria-label={t.historyTitle}
>
  <div class="header">
    {#if !collapsed}
      <span class="title">
        <Icon name="history" size={14} />
        {t.historyTitle}
      </span>
    {/if}
    <button
      class="icon-btn toggle"
      onclick={onToggle}
      aria-label={collapsed ? t.historyExpandAria : t.historyCollapseAria}
      title={collapsed ? t.historyExpandAria : t.historyCollapseAria}
    >
      <Icon name={collapsed ? "chevron-right" : "chevron-left"} size={16} />
    </button>
  </div>

  {#if !collapsed}
    <div class="list" role="list">
      {#if $historyStore.length === 0}
        <p class="empty">{t.historyEmpty}</p>
      {:else}
        {#each $historyStore as entry (entry.id)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="row"
            class:selected={$historySelectionStore === entry.id}
            role="listitem"
            onmouseenter={() => (hoveredId = entry.id)}
            onmouseleave={() => (hoveredId = null)}
          >
            <button
              class="row-main"
              onclick={() => selectEntry(entry.id)}
              aria-label={t.historyEntryAria(entry.filename)}
            >
              <div class="row-top">
                <span class="filename" title={entry.filename}>{entry.filename}</span>
              </div>
              <div class="row-meta">
                <span class="time">{relativeTime(entry.createdAt)}</span>
                <span class="dot">·</span>
                <span class="badge">{t.providerName(entry.provider)}</span>
                {#if entry.diarize}
                  <span class="dot">·</span>
                  <span class="badge subtle">{t.diarize}</span>
                {/if}
              </div>
            </button>
            <button
              class="del"
              class:visible={hoveredId === entry.id ||
                $historySelectionStore === entry.id}
              onclick={(e) => onDelete(e, entry.id)}
              aria-label={t.historyDeleteEntry}
              title={t.historyDeleteEntry}
            >
              <Icon name="trash" size={13} />
            </button>
          </div>
        {/each}
      {/if}
    </div>

    {#if $historyStore.length > 0}
      <div class="footer">
        <button
          class="clear-btn"
          class:confirming={confirmingClear}
          onclick={onClearAll}
        >
          <Icon name="trash" size={12} />
          {confirmingClear ? t.historyClearConfirm : t.historyClearAll}
        </button>
      </div>
    {/if}
  {/if}
</aside>

<style>
  .history-sidebar {
    width: 260px;
    background: var(--bg-1);
    border-right: 1px solid var(--border-1);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    min-height: 0;
    transition: width var(--t-slow);
  }
  .history-sidebar.collapsed {
    width: 44px;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 10px 10px 14px;
    border-bottom: 1px solid var(--border-1);
    flex-shrink: 0;
    min-height: 48px;
  }
  .history-sidebar.collapsed .header {
    justify-content: center;
    padding: 10px 4px;
  }
  .title {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-2);
  }
  .toggle {
    width: 28px;
    height: 28px;
  }

  .list {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 0;
  }
  .empty {
    margin: 14px 8px;
    font-size: 12px;
    color: var(--text-4);
    line-height: 1.5;
    text-align: center;
  }

  .row {
    position: relative;
    display: flex;
    align-items: stretch;
    border-radius: var(--r-md);
    transition: background var(--t);
  }
  .row:hover {
    background: var(--bg-2);
  }
  .row.selected {
    background: var(--accent-subtle);
  }
  .row.selected::before {
    content: "";
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 2px;
    background: var(--accent);
    border-radius: 0 2px 2px 0;
  }

  .row-main {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 8px 10px 9px;
    text-align: left;
    color: var(--text-1);
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-family: var(--font);
  }

  .row-top {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .filename {
    font-size: 12.5px;
    font-weight: 500;
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    flex: 1;
  }

  .row-meta {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 10.5px;
    color: var(--text-3);
    overflow: hidden;
  }
  .time {
    color: var(--text-3);
  }
  .dot {
    color: var(--text-4);
  }
  .badge {
    padding: 1px 5px;
    border-radius: 3px;
    background: var(--bg-3);
    color: var(--text-2);
    font-size: 9.5px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 500;
  }
  .badge.subtle {
    background: var(--accent-subtle);
    color: var(--accent);
  }

  .del {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--r-sm);
    color: var(--text-3);
    cursor: pointer;
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--t), color var(--t), background var(--t);
  }
  .del.visible {
    opacity: 1;
    pointer-events: auto;
  }
  .del:hover {
    color: var(--error);
    background: var(--error-muted);
  }

  .footer {
    border-top: 1px solid var(--border-1);
    padding: 8px 10px;
    flex-shrink: 0;
  }
  .clear-btn {
    width: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 7px 10px;
    border-radius: var(--r-sm);
    background: transparent;
    border: 1px solid var(--border-1);
    color: var(--text-3);
    font-family: var(--font);
    font-size: 11px;
    cursor: pointer;
    transition: all var(--t);
  }
  .clear-btn:hover {
    color: var(--error);
    border-color: var(--error-muted);
    background: var(--error-muted);
  }
  .clear-btn.confirming {
    color: var(--error);
    border-color: var(--error);
    background: var(--error-muted);
    font-weight: 500;
  }
</style>
