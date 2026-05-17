<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import Icon from "./Icons.svelte";
  import { t } from "$lib/i18n/es";
  import { MEDIA_EXT, extOf } from "$lib/types";

  type Props = {
    compact?: boolean;
    busy?: boolean;
    onFilesAdded: (paths: string[]) => void;
  };
  let { compact = false, busy = false, onFilesAdded }: Props = $props();

  let dragging = $state(false);
  let unlisten: (() => void) | null = null;

  function filterValid(paths: string[]): string[] {
    return paths.filter((p) => MEDIA_EXT.includes(extOf(p)));
  }

  onMount(async () => {
    try {
      const webview = getCurrentWebview();
      const u = await webview.onDragDropEvent((event) => {
        const payload = event.payload as
          | { type: "enter" | "over" | "drop" | "leave"; paths?: string[] };
        if (busy) return;
        if (payload.type === "enter" || payload.type === "over") {
          dragging = true;
        } else if (payload.type === "leave") {
          dragging = false;
        } else if (payload.type === "drop") {
          dragging = false;
          const valid = filterValid(payload.paths ?? []);
          if (valid.length) onFilesAdded(valid);
        }
      });
      unlisten = u;
    } catch {
      // running outside Tauri (SSR/preview); no-op
    }
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  async function pick() {
    if (busy) return;
    const selected = await open({
      multiple: true,
      filters: [{ name: t.videoFilters, extensions: MEDIA_EXT }],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    if (paths.length) onFilesAdded(paths);
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
  }
  function onDrop(e: DragEvent) {
    // Tauri drives the actual file paths; just suppress browser nav.
    e.preventDefault();
  }
</script>

{#if compact}
  <div
    class="drop-zone {dragging ? 'dragging' : ''}"
    style="display:flex; align-items:center; gap:10px; padding:11px 18px; border-radius:var(--r-lg);"
    onclick={pick}
    ondragover={onDragOver}
    ondrop={onDrop}
    role="button"
    tabindex="0"
    onkeydown={(e) => (e.key === "Enter" || e.key === " ") && pick()}
  >
    <span style="color: var(--text-4); display:flex;">
      <Icon name="plus" size={16} />
    </span>
    <span style="font-size:12px; color:var(--text-4);">{t.dropMore}</span>
  </div>
{:else}
  <div
    class="drop-zone {dragging ? 'dragging' : ''}"
    style="display:flex; flex-direction:column; align-items:center; justify-content:center; text-align:center; padding:56px 40px; min-height:260px;"
    onclick={pick}
    ondragover={onDragOver}
    ondrop={onDrop}
    role="button"
    tabindex="0"
    onkeydown={(e) => (e.key === "Enter" || e.key === " ") && pick()}
  >
    <div
      style="
        width:52px; height:52px; border-radius:var(--r-lg);
        background: var(--accent-muted); display:flex;
        align-items:center; justify-content:center;
        color: var(--accent); margin-bottom:18px;
      "
    >
      <Icon name="upload" size={24} />
    </div>
    <p style="font-size:15px; font-weight:600; margin-bottom:6px; letter-spacing:-0.02em;">
      {dragging ? t.dropHere : t.dropHero}
    </p>
    <p style="font-size:13px; color:var(--text-3); margin-bottom:18px;">
      {t.dropHeroOr}
      <span style="color: var(--accent); text-decoration: underline; text-underline-offset: 3px; cursor: pointer;">
        {t.dropHeroPick}
      </span>
      {t.dropHeroOrPick}
    </p>
    <p style="font-size:11px; color:var(--text-4); letter-spacing:0.04em;">
      {t.dropHeroFormats}
    </p>
  </div>
{/if}
