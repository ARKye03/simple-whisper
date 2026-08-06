"""faster-whisper driver for simple-whisper.

Speaks NDJSON on stdout, one JSON object per line, discriminated by "type".
The protocol is shared with src-tauri/src/local.rs — change both together.

Exit codes: 0 success, 2 handled failure (an "error" line was emitted),
1 unhandled crash (Rust falls back to the stderr tail).
"""

import argparse
import importlib.metadata
import json
import os
import platform
import sys
import threading
import traceback

# We own this map so Rust never has to parse the HuggingFace cache layout and we
# never touch faster_whisper's private _MODELS.
MODEL_REPOS = {
    "tiny": "Systran/faster-whisper-tiny",
    "base": "Systran/faster-whisper-base",
    "small": "Systran/faster-whisper-small",
    "medium": "Systran/faster-whisper-medium",
    "large-v2": "Systran/faster-whisper-large-v2",
    "large-v3": "Systran/faster-whisper-large-v3",
    "large-v3-turbo": "mobiuslabsgmbh/faster-whisper-large-v3-turbo",
    "distil-large-v3": "Systran/faster-distil-whisper-large-v3",
}

TICK_SECONDS = 0.75
TICK_MIN_DELTA = 1024 * 1024


# DownloadTicker emits from its own thread, so the whole line is serialized
# first and written under a lock: interleaved partial writes would corrupt the
# NDJSON and Rust would silently drop the line.
_EMIT_LOCK = threading.Lock()


def emit(obj):
    line = json.dumps(obj, ensure_ascii=False) + "\n"
    with _EMIT_LOCK:
        sys.stdout.write(line)
        sys.stdout.flush()


def stage(name, detail=None):
    emit({"type": "stage", "stage": name, "detail": detail})


def fail(code, message, tb=None):
    emit({"type": "error", "code": code, "message": str(message), "traceback": tb})
    sys.exit(2)


def repo_dir(root, repo):
    return os.path.join(root, "models--" + repo.replace("/", "--"))


def dir_bytes(path):
    total = 0
    stack = [path]
    while stack:
        try:
            entries = list(os.scandir(stack.pop()))
        except OSError:
            continue
        for e in entries:
            try:
                if e.is_dir(follow_symlinks=False):
                    stack.append(e.path)
                elif e.is_file(follow_symlinks=False):
                    total += e.stat(follow_symlinks=False).st_size
            except OSError:
                pass
    return total


def is_cached(model, root):
    """True only when model.bin is actually resolvable — a half-finished
    download leaves blobs behind and must not read as ready."""
    repo = MODEL_REPOS.get(model)
    if repo is None:
        return False
    try:
        from huggingface_hub import try_to_load_from_cache
    except Exception:
        return False
    try:
        hit = try_to_load_from_cache(repo, "model.bin", cache_dir=root)
    except Exception:
        return False
    return isinstance(hit, str) and os.path.isfile(hit)


class DownloadTicker(threading.Thread):
    def __init__(self, path):
        super().__init__(daemon=True)
        self.path = path
        self.done = threading.Event()

    def run(self):
        last = -1
        while not self.done.wait(TICK_SECONDS):
            total = dir_bytes(self.path)
            if last < 0 or total - last >= TICK_MIN_DELTA:
                last = total
                emit({"type": "download", "bytes": total})

    def stop(self):
        self.done.set()


def pick_device(pref):
    if pref == "cpu":
        return "cpu", "int8"
    if pref == "cuda":
        return "cuda", "float16"
    try:
        import ctranslate2

        if ctranslate2.get_cuda_device_count() > 0:
            return "cuda", "float16"
    except Exception:
        pass
    return "cpu", "int8"


def ensure_model(model, root):
    """Returns True when a download actually happened."""
    if model not in MODEL_REPOS:
        fail("unknown_model", "Modelo desconocido: %s" % model)
    if is_cached(model, root):
        return False
    stage("downloading_model", model)
    ticker = DownloadTicker(repo_dir(root, MODEL_REPOS[model]))
    ticker.start()
    try:
        from faster_whisper import download_model

        download_model(model, cache_dir=root)
    except Exception as e:
        # Stop first so no stray "download" line lands after the error line.
        ticker.stop()
        fail("model_download_failed", e, traceback.format_exc())
    finally:
        ticker.stop()
    return True


def load_model(model, root, device_pref):
    stage("loading_model")
    try:
        from faster_whisper import WhisperModel
    except Exception as e:
        fail("import_failed", e, traceback.format_exc())

    if ensure_model(model, root):
        stage("loading_model")

    device, compute = pick_device(device_pref)
    try:
        return WhisperModel(model, device=device, compute_type=compute, download_root=root), device, compute
    except Exception as e:
        if device == "cpu":
            fail("decode_failed", e, traceback.format_exc())
        # CUDA present but unusable (missing cuDNN 9 is the common case) — retry on CPU.
        stage("cuda_fallback", str(e))
        try:
            return WhisperModel(model, device="cpu", compute_type="int8", download_root=root), "cpu", "int8"
        except Exception as e2:
            fail("decode_failed", e2, traceback.format_exc())


def cmd_probe(args):
    info = {
        "type": "probe",
        "python": platform.python_version(),
        "faster_whisper": None,
        "models": [],
        "cuda": None,
        "import_ok": False,
    }
    # Deliberately avoids importing faster_whisper: metadata lookup is ~300ms
    # versus ~2s for the real import, and this runs on every app boot.
    try:
        info["faster_whisper"] = importlib.metadata.version("faster-whisper")
        info["import_ok"] = True
    except Exception:
        pass

    if info["import_ok"]:
        info["models"] = [m for m in MODEL_REPOS if is_cached(m, args.download_root)]
        if args.with_cuda:
            try:
                import ctranslate2

                info["cuda"] = ctranslate2.get_cuda_device_count() > 0
            except Exception:
                info["cuda"] = False

    emit(info)


def cmd_download(args):
    try:
        import faster_whisper  # noqa: F401
    except Exception as e:
        fail("import_failed", e, traceback.format_exc())
    ensure_model(args.model, args.download_root)
    emit({"type": "done", "text": "", "language": None, "duration": 0.0})


def cmd_transcribe(args):
    model, device, compute = load_model(args.model, args.download_root, args.device)

    try:
        segments, info = model.transcribe(
            args.audio,
            language=None if args.language == "auto" else args.language,
            beam_size=5,
            # Silence would otherwise yield hallucinated text.
            vad_filter=True,
            # Prevents runaway repetition loops on long audio.
            condition_on_previous_text=False,
        )
    except Exception as e:
        fail(oom_or_decode(e), e, traceback.format_exc())

    # Emitted before "info" so the resolved device stays visible until the first
    # segment arrives instead of being overwritten by the stage line.
    stage("transcribing")

    # info.duration is the original timeline; duration_after_vad would make
    # segment-based progress overshoot 100%.
    duration = float(info.duration or 0.0)
    emit({
        "type": "info",
        "duration": duration,
        "language": info.language,
        "language_probability": info.language_probability,
        "device": device,
        "compute_type": compute,
    })

    parts = []
    ratio = 0.0
    try:
        for seg in segments:
            text = seg.text.strip()
            if duration > 0:
                ratio = max(ratio, min(1.0, float(seg.end) / duration))
            if text:
                parts.append(text)
            emit({
                "type": "segment",
                "start": float(seg.start),
                "end": float(seg.end),
                "text": text,
                "ratio": round(ratio, 4),
            })
    except KeyboardInterrupt:
        fail("interrupted", "Interrumpido")
    except Exception as e:
        fail(oom_or_decode(e), e, traceback.format_exc())

    stage("finalizing")
    emit({
        "type": "done",
        "text": " ".join(parts),
        "language": info.language,
        "duration": duration,
    })


def oom_or_decode(err):
    blob = str(err).lower()
    if "out of memory" in blob or "cuda_error_out_of_memory" in blob:
        return "out_of_memory"
    return "decode_failed"


def main():
    parser = argparse.ArgumentParser(add_help=False)
    subs = parser.add_subparsers(dest="mode", required=True)

    p = subs.add_parser("probe", add_help=False)
    p.add_argument("--download-root", required=True)
    p.add_argument("--with-cuda", action="store_true")
    p.set_defaults(func=cmd_probe)

    d = subs.add_parser("download", add_help=False)
    d.add_argument("--download-root", required=True)
    d.add_argument("--model", required=True)
    d.set_defaults(func=cmd_download)

    t = subs.add_parser("transcribe", add_help=False)
    t.add_argument("--download-root", required=True)
    t.add_argument("--model", required=True)
    t.add_argument("--audio", required=True)
    t.add_argument("--language", default="auto")
    t.add_argument("--device", default="auto", choices=["auto", "cpu", "cuda"])
    t.set_defaults(func=cmd_transcribe)

    args = parser.parse_args()
    os.makedirs(args.download_root, exist_ok=True)
    try:
        args.func(args)
    except SystemExit:
        raise
    except KeyboardInterrupt:
        fail("interrupted", "Interrumpido")
    except Exception as e:
        fail("unknown", e, traceback.format_exc())


if __name__ == "__main__":
    main()
