<script lang="ts">
  import Icon from "./Icons.svelte";
  import { t } from "$lib/i18n/state.svelte";
  import { isSupportedMediaUrl, parseUrlCandidates } from "$lib/types";

  type Props = {
    compact?: boolean;
    busy?: boolean;
    onUrlsAdded: (urls: string[]) => void;
  };
  let { compact = false, busy = false, onUrlsAdded }: Props = $props();

  const uid = $props.id();
  const inputId = `url-input-${uid}`;
  const msgId = `url-msg-${uid}`;

  let value = $state("");
  let error = $state<string | null>(null);
  let focused = $state(false);

  const canSubmit = $derived(!busy && value.trim().length > 0);

  function submit() {
    if (busy) return;
    const urls = parseUrlCandidates(value).filter(isSupportedMediaUrl);
    if (!urls.length) {
      error = value.trim() ? t.urlInvalid : null;
      return;
    }
    error = null;
    value = "";
    onUrlsAdded(urls);
  }

  function onKeydown(e: KeyboardEvent) {
    // Never let Enter/Space bubble to an ancestor DropZone, whose key handler opens
    // the file dialog.
    e.stopPropagation();
    if (e.key === "Enter") {
      e.preventDefault();
      submit();
    } else if (e.key === "Escape") {
      value = "";
      error = null;
    }
  }
</script>

<div style="display:flex; flex-direction:column; gap:{compact ? '4px' : '7px'}; width:100%;">
  {#if !compact}
    <label
      for={inputId}
      style="font-size:11px; font-weight:500; color:var(--text-3); text-transform:uppercase; letter-spacing:0.08em;"
    >{t.urlInputLabel}</label>
  {/if}

  <div style="display:flex; gap:8px; align-items:stretch;">
    <input
      id={inputId}
      type="url"
      inputmode="url"
      spellcheck="false"
      autocomplete="off"
      autocapitalize="off"
      disabled={busy}
      bind:value
      onkeydown={onKeydown}
      onfocus={() => (focused = true)}
      onblur={() => (focused = false)}
      oninput={() => (error = null)}
      onclick={(e) => e.stopPropagation()}
      placeholder={t.urlInputPlaceholder}
      aria-label={compact ? t.urlInputLabel : undefined}
      aria-invalid={error ? "true" : undefined}
      aria-describedby={msgId}
      style="
        flex:1; min-width:0;
        padding:{compact ? '9px 12px' : '11px 14px'};
        border-radius:var(--r-md);
        background:var(--bg-3);
        border:1px solid {error ? 'var(--error)' : focused ? 'var(--border-3)' : 'var(--border-2)'};
        color:var(--text-1);
        font-family:var(--font);
        font-size:{compact ? '12px' : '13px'};
        outline:none;
        opacity:{busy ? 0.5 : 1};
        transition:border-color var(--t);
      "
    />
    <button
      onclick={submit}
      disabled={!canSubmit}
      aria-label={compact ? t.urlInputSubmitAria : undefined}
      title={t.urlInputSubmitAria}
      style="
        flex:0 0 auto;
        display:inline-flex; align-items:center; justify-content:center; gap:6px;
        {compact ? 'width:36px;' : 'padding:0 20px;'}
        border-radius:var(--r-md);
        background:{canSubmit ? 'var(--accent)' : 'var(--bg-3)'};
        color:{canSubmit ? '#fff' : 'var(--text-4)'};
        border:1px solid {canSubmit ? 'var(--accent)' : 'var(--border-2)'};
        font-family:var(--font); font-size:{compact ? '12px' : '13px'}; font-weight:600;
        cursor:{canSubmit ? 'pointer' : 'not-allowed'};
        transition:all var(--t);
      "
    >
      {#if compact}<Icon name="link" size={15} />{:else}{t.urlInputSubmit}{/if}
    </button>
  </div>

  <!-- Doubles as the aria-describedby target and a polite live region: live regions
       only announce mutations, so the static hint stays silent but an error speaks. -->
  <p
    id={msgId}
    aria-live="polite"
    style="
      margin:0; min-height:{compact ? '0' : '15px'};
      font-size:10.5px; line-height:1.45;
      color:{error ? 'var(--error)' : 'var(--text-4)'};
    "
  >{error ?? (compact ? "" : t.urlInputHint)}</p>
</div>
