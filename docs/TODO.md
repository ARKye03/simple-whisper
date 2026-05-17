# TODO

Tracked follow-ups. Tick items when done.

## Fonts — self-host `.woff2`, drop Google Fonts CDN

**Why:** Google Fonts CDN is unreliable in a desktop-only Tauri app (no guaranteed internet, privacy leak, FOUT on slow connections, latency on first paint). Self-host the exact subsets we use.

**Current state:** `src/app.html` links to `fonts.googleapis.com` for Fraunces, Newsreader, IBM Plex Mono.

**Action items**

- [ ] Download `.woff2` files for the exact axes/weights we need:
  - **Fraunces** (variable) — axes `opsz 9..144`, `wght 300..900`, `SOFT 0..100`. Subset: `latin`, `latin-ext`.
  - **Newsreader** (variable) — axes `opsz 6..72`, `wght 300..600`. Subset: `latin`, `latin-ext`.
  - **IBM Plex Mono** — static weights 300 / 400 / 500 / 600. Subset: `latin`, `latin-ext`.
- [ ] Place under `static/fonts/` (SvelteKit serves `static/*` at root).
- [ ] Remove the `<link href="https://fonts.googleapis.com/..." …>` block from `src/app.html`. Also drop the two `preconnect` lines.
- [ ] Add `@font-face` declarations at the top of `src/app.css` pointing to `/fonts/*.woff2`. Use `font-display: swap` and the correct `unicode-range` for each subset to keep payload small.
- [ ] Sanity-check in `pnpm tauri build` that the bundled app renders the right faces with the network cut.
- [ ] Verify Spanish glyphs (á é í ó ú ñ ¿ ¡ ü) ship in the `latin` or `latin-ext` subset chosen.

**Optional**

- [ ] Use `glyphhanger` or `subset-font` to strip to ASCII + Spanish glyphs only, shrinking each file further.
- [ ] Add a fallback CSS stack with `size-adjust` / `ascent-override` to avoid layout shift while the variable font loads.

## Drag & drop video onto the window

**Why:** Faster than clicking through the file dialog. Standard UX for desktop transcription tools.

**Current state:** Only the `Seleccionar video` button (file dialog via `@tauri-apps/plugin-dialog`) is wired up. `src/routes/+page.svelte` has no drop handlers.

**Action items**

- [ ] Enable file-drop on the Tauri window — already on by default in v2, but confirm `app.windows[0].dragDropEnabled` is not set to `false` in `src-tauri/tauri.conf.json`.
- [ ] In `+page.svelte`, listen via `getCurrentWebview().onDragDropEvent(...)` from `@tauri-apps/api/webview`. Handle the `drag-drop` event payload (`{ paths: string[], position }`).
- [ ] Filter dropped paths by extension (`mp4`, `mov`, `mkv`, `webm`, `avi`, `m4v`, `wmv`, `flv`) — first valid path wins; reject silently otherwise (toast in Spanish: "Formato no soportado").
- [ ] Add visual drop affordance: full-window overlay on `drag-enter` (dashed vermilion border + centered eyebrow `Soltar para transcribir`), dismissed on `drag-leave` / `drag-drop`.
- [ ] Keep button-based picker as fallback (a11y + keyboard users).
- [ ] Block drops while `busy` (in-flight transcription) so we don't clobber state.
- [ ] Add Spanish copy entries to `src/lib/i18n/es.ts`: `dropHere`, `unsupportedFormat`.
