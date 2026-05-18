import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { ask, message } from "@tauri-apps/plugin-dialog";
import { t } from "$lib/i18n/state.svelte";

export async function checkForUpdates({ silent }: { silent: boolean }): Promise<void> {
  try {
    const update = await check();
    if (!update) {
      if (!silent) {
        await message(t.updateUpToDate, { title: t.updates, kind: "info" });
      }
      return;
    }

    const confirmed = await ask(t.updateAvailableBody(update.version, update.body ?? ""), {
      title: t.updateAvailableTitle,
      kind: "info",
      okLabel: t.updateInstall,
      cancelLabel: t.updateLater,
    });
    if (!confirmed) return;

    await update.downloadAndInstall();
    await relaunch();
  } catch (err) {
    console.error("Update check failed:", err);
    if (!silent) {
      await message(t.updateCheckFailed, { title: t.updates, kind: "error" });
    }
  }
}
