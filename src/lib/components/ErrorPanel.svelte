<script lang="ts">
  import Icon from "./Icons.svelte";
  import { t } from "$lib/i18n/state.svelte";
  import type { TranscribeError, TranscribeErrorKind } from "$lib/types";

  type Props = {
    error: TranscribeError;
    otherProvider?: "groq" | "gemini" | null;
    onRetry?: () => void;
    onRetryWith?: (provider: "groq" | "gemini") => void;
  };

  let { error, otherProvider = null, onRetry, onRetryWith }: Props = $props();

  let showDetails = $state(false);

  const providerLabel = $derived(
    error.provider ? t.providerName(error.provider) : "",
  );

  const title = $derived(titleFor(error.kind));
  const body = $derived(bodyFor(error));

  function titleFor(kind: TranscribeErrorKind): string {
    switch (kind) {
      case "AuthInvalid": return t.errAuthInvalidTitle;
      case "AuthForbidden": return t.errAuthForbiddenTitle;
      case "RateLimited": return t.errRateLimitedTitle;
      case "QuotaExceeded": return t.errQuotaExceededTitle;
      case "PayloadTooLarge": return t.errPayloadTooLargeTitle;
      case "BadRequest": return t.errBadRequestTitle;
      case "ModelUnavailable": return t.errModelUnavailableTitle;
      case "ServerBusy": return t.errServerBusyTitle;
      case "ServerError": return t.errServerErrorTitle;
      case "Network": return t.errNetworkTitle;
      case "Timeout": return t.errTimeoutTitle;
      case "MalformedResponse": return t.errMalformedTitle;
      case "FfmpegMissing": return t.errFfmpegMissingTitle;
      case "FfmpegFailed": return t.errFfmpegFailedTitle;
      case "ApiKeyMissing": return t.errApiKeyMissingTitle;
      default: return t.errUnknownTitle;
    }
  }

  function bodyFor(e: TranscribeError): string {
    const p = e.provider ? t.providerName(e.provider) : "";
    switch (e.kind) {
      case "AuthInvalid": return t.errAuthInvalidBody(p);
      case "AuthForbidden": return t.errAuthForbiddenBody(p);
      case "RateLimited": return t.errRateLimitedBody(p, e.retry_after_secs);
      case "QuotaExceeded": return t.errQuotaExceededBody(p);
      case "PayloadTooLarge": return t.errPayloadTooLargeBody(p);
      case "BadRequest": return t.errBadRequestBody(p);
      case "ModelUnavailable": return t.errModelUnavailableBody(p);
      case "ServerBusy": return t.errServerBusyBody(p);
      case "ServerError": return t.errServerErrorBody(p);
      case "Network": return t.errNetworkBody(p);
      case "Timeout": return t.errTimeoutBody(p);
      case "MalformedResponse": return t.errMalformedBody(p);
      case "FfmpegMissing": return t.errFfmpegMissingBody;
      case "FfmpegFailed": return t.errFfmpegFailedBody;
      case "ApiKeyMissing": return t.errApiKeyMissingBody(p);
      default: return e.message || t.errUnknownBody;
    }
  }
</script>

<div
  style="
    padding:12px 14px 14px;
    border-top:1px solid var(--border-1);
    background: var(--bg-2);
    display:flex; flex-direction:column; gap:10px;
  "
>
  <div style="display:flex; align-items:flex-start; gap:10px;">
    <span style="color: var(--error); flex-shrink:0; margin-top:1px;">
      <Icon name="x" size={16} />
    </span>
    <div style="flex:1; min-width:0;">
      <div style="font-size:13px; font-weight:600; color: var(--text-1); margin-bottom:2px;">
        {title}{providerLabel ? ` · ${providerLabel}` : ""}
      </div>
      <div style="font-size:12px; color: var(--text-3); line-height:1.5;">
        {body}
      </div>
    </div>
  </div>

  {#if onRetry || (otherProvider && onRetryWith)}
    <div style="display:flex; gap:8px; flex-wrap:wrap; padding-left:26px;">
      {#if onRetry}
        <button class="btn-primary" style="font-size:12px; padding:6px 14px;" onclick={onRetry}>
          {t.errRetry}
        </button>
      {/if}
      {#if otherProvider && onRetryWith}
        <button
          class="btn-ghost"
          style="font-size:12px; padding:6px 14px;"
          onclick={() => onRetryWith?.(otherProvider!)}
        >
          {t.errRetryWith(t.providerName(otherProvider))}
        </button>
      {/if}
    </div>
  {/if}

  {#if error.raw}
    <div style="padding-left:26px;">
      <button
        class="btn-ghost"
        style="font-size:11px; padding:4px 10px; color: var(--text-4);"
        onclick={() => (showDetails = !showDetails)}
      >
        {showDetails ? t.errHideDetails : t.errShowDetails}
      </button>
      {#if showDetails}
        <pre
          style="
            margin:8px 0 0;
            padding:10px;
            background: var(--bg-3);
            border-radius: var(--r-sm);
            font-size:11px;
            color: var(--text-3);
            white-space: pre-wrap;
            word-break: break-word;
            font-family: ui-monospace, monospace;
            max-height: 200px;
            overflow:auto;
          ">{error.raw}</pre>
      {/if}
    </div>
  {/if}
</div>
