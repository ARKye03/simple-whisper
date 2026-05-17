<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { t } from "$lib/i18n/es";
  import { getApiKey } from "$lib/settings";

  type Stage = "idle" | "extracting" | "chunking" | "uploading" | "ready" | "saved" | "error";

  let videoPath = $state<string | null>(null);
  let stage = $state<Stage>("idle");
  let transcript = $state("");
  let errorMsg = $state("");

  const stageLabel = $derived.by(() => {
    switch (stage) {
      case "extracting": return t.extractingAudio;
      case "chunking": return t.chunking;
      case "uploading": return t.uploadingToGroq;
      case "ready": return t.ready;
      case "saved": return t.saved;
      case "error": return t.errorTitle;
      default: return "";
    }
  });

  const busy = $derived(
    stage === "extracting" || stage === "chunking" || stage === "uploading"
  );

  async function pickVideo() {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: t.videoFilters,
          extensions: ["mp4", "mov", "mkv", "webm", "avi", "m4v", "wmv", "flv"],
        },
      ],
    });
    if (typeof selected === "string") {
      videoPath = selected;
      transcript = "";
      stage = "idle";
      errorMsg = "";
    }
  }

  async function runTranscribe() {
    if (!videoPath) return;
    const apiKey = await getApiKey();
    if (!apiKey) {
      errorMsg = t.apiKeyMissing;
      stage = "error";
      return;
    }

    try {
      transcript = "";
      errorMsg = "";
      stage = "extracting";
      const text = await invoke<string>("transcribe_video", {
        videoPath,
        apiKey,
      });
      transcript = text;
      stage = "ready";
    } catch (e) {
      errorMsg = String(e);
      stage = "error";
    }
  }

  async function saveDocx() {
    if (!transcript) return;
    const path = await save({
      defaultPath: "transcripcion.docx",
      filters: [{ name: t.docxFilters, extensions: ["docx"] }],
    });
    if (!path) return;
    try {
      await invoke("save_to_docx", { text: transcript, path });
      stage = "saved";
    } catch (e) {
      errorMsg = String(e);
      stage = "error";
    }
  }

  function fileName(p: string): string {
    const parts = p.split(/[\\/]/);
    return parts[parts.length - 1] ?? p;
  }

  const wordCount = $derived(
    transcript.trim() ? transcript.trim().split(/\s+/).length : 0
  );
</script>

<main class="flex-1">
  <div class="max-w-4xl mx-auto px-5 sm:px-8 pt-8 sm:pt-12 pb-12 sm:pb-16">

    <header class="reveal reveal-1 flex flex-col sm:flex-row sm:items-end sm:justify-between gap-4 sm:gap-8 mb-8 sm:mb-12">
      <div class="flex-1">
        <div class="eyebrow mb-3">Transcripción de video</div>
        <h1 class="display text-[clamp(2.4rem,9vw,5.4rem)] font-light">
          Simple<br/>
          <em style:color="var(--color-accent)" style:font-style="italic" style:font-variation-settings="'opsz' 144, 'wght' 400, 'SOFT' 80">Whisper</em>
        </h1>
      </div>
      <a href="/settings" class="btn-ghost" style:white-space="nowrap">
        {t.settings}
      </a>
    </header>

    <div class="rule mb-12"></div>

    <!-- 01 — Source -->
    <section class="reveal reveal-2 grid grid-cols-1 sm:grid-cols-[4.5rem_1fr] gap-4 sm:gap-8 mb-10">
      <div class="numeral">01</div>
      <div class="flex flex-col gap-4">
        <div class="eyebrow">Origen</div>
        <div
          class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 sm:gap-6 border-b pb-5"
          style:border-color="var(--color-rule)"
        >
          <div class="min-w-0 flex-1">
            <div
              class="font-mono text-[0.95rem] truncate"
              style:color={videoPath ? "var(--color-ink)" : "var(--color-ink-soft)"}
            >
              {videoPath ? fileName(videoPath) : t.noFileSelected}
            </div>
            {#if videoPath}
              <div class="font-mono text-[0.7rem] truncate mt-1" style:color="var(--color-ink-soft)">
                {videoPath}
              </div>
            {/if}
          </div>
          <button type="button" onclick={pickVideo} disabled={busy} class="btn-ghost shrink-0">
            {t.selectVideo}
          </button>
        </div>
      </div>
    </section>

    <!-- 02 — Transcribe -->
    <section class="reveal reveal-3 grid grid-cols-1 sm:grid-cols-[4.5rem_1fr] gap-4 sm:gap-8 mb-10">
      <div class="numeral">02</div>
      <div class="flex flex-col gap-4">
        <div class="eyebrow">Acción</div>
        <div class="flex flex-wrap items-center gap-x-5 gap-y-3">
          <button
            type="button"
            onclick={runTranscribe}
            disabled={!videoPath || busy}
            class="btn-primary"
          >
            {busy ? t.transcribing : t.transcribe}
          </button>

          {#if busy}
            <div class="waveform" aria-hidden="true">
              <span></span><span></span><span></span><span></span><span></span>
            </div>
          {/if}

          {#if stage !== "idle"}
            <span class="eyebrow" style:color={stage === "error" ? "var(--color-accent)" : "var(--color-ink-soft)"}>
              · {stageLabel}
            </span>
          {/if}
        </div>

        {#if stage === "error"}
          <pre
            class="font-mono text-[0.78rem] whitespace-pre-wrap border-l-2 pl-3 py-1 leading-relaxed"
            style:color="var(--color-accent)"
            style:border-color="var(--color-accent)"
          >{errorMsg}</pre>
        {/if}
      </div>
    </section>

    <!-- 03 — Transcript -->
    {#if transcript}
      <section class="reveal reveal-4 grid grid-cols-1 sm:grid-cols-[4.5rem_1fr] gap-4 sm:gap-8">
        <div class="numeral">03</div>
        <div class="flex flex-col gap-4">
          <div class="flex items-baseline justify-between">
            <div class="eyebrow">Transcripción</div>
            <div class="eyebrow">{wordCount} palabras</div>
          </div>

          <textarea
            bind:value={transcript}
            rows="18"
            class="transcript-area"
            spellcheck="false"
          ></textarea>

          <div class="flex items-center gap-4">
            <button type="button" onclick={saveDocx} class="btn-primary">
              {t.saveDocx}
            </button>
            {#if stage === "saved"}
              <span class="eyebrow" style:color="var(--color-accent)">✓ {t.saved}</span>
            {/if}
          </div>
        </div>
      </section>
    {/if}

  </div>
</main>
