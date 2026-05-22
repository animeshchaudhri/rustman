import { useEffect, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { Download, X, RefreshCw } from "lucide-react";

export function UpdateChecker() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [dismissed, setDismissed] = useState(false);
  const [state, setState] = useState<"idle" | "downloading" | "done">("idle");
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    const timeout = setTimeout(() => {
      check()
        .then((u) => { if (u) setUpdate(u); })
        .catch(() => {});
    }, 3000);
    return () => clearTimeout(timeout);
  }, []);

  if (!update || dismissed) return null;

  const handleInstall = async () => {
    setState("downloading");
    let downloaded = 0;
    let total = 0;
    try {
      await update.downloadAndInstall((ev) => {
        if (ev.event === "Started") { total = ev.data.contentLength ?? 0; }
        if (ev.event === "Progress") {
          downloaded += ev.data.chunkLength;
          if (total > 0) setProgress(Math.round((downloaded / total) * 100));
        }
        if (ev.event === "Finished") setState("done");
      });
      await relaunch();
    } catch {
      setState("idle");
    }
  };

  return (
    <div className="fixed bottom-4 right-4 z-50 w-80 rounded-xl border border-orange-500/30 bg-zinc-900 shadow-xl shadow-black/40 text-sm overflow-hidden">
      <div className="flex items-start gap-3 p-4">
        <div className="mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-orange-500/15">
          <RefreshCw className="h-3.5 w-3.5 text-orange-400" />
        </div>
        <div className="flex-1 min-w-0">
          <p className="font-semibold text-zinc-100">Update available</p>
          <p className="text-xs text-zinc-400 mt-0.5">
            Rustman <span className="text-orange-400 font-mono">{update.version}</span> is ready to install.
          </p>
          {update.body && (
            <p className="text-xs text-zinc-500 mt-1 line-clamp-2">{update.body}</p>
          )}
          {state === "downloading" && (
            <div className="mt-2">
              <div className="h-1 w-full rounded-full bg-zinc-800">
                <div
                  className="h-1 rounded-full bg-orange-500 transition-all duration-200"
                  style={{ width: `${progress}%` }}
                />
              </div>
              <p className="text-[10px] text-zinc-500 mt-1">{progress > 0 ? `${progress}%` : "Preparing…"}</p>
            </div>
          )}
          {state === "done" && (
            <p className="text-xs text-emerald-400 mt-1">Installing… app will restart.</p>
          )}
        </div>
        {state === "idle" && (
          <button
            onClick={() => setDismissed(true)}
            className="shrink-0 text-zinc-600 hover:text-zinc-400 transition-colors"
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>
      {state === "idle" && (
        <div className="border-t border-zinc-800 px-4 py-2.5 flex justify-end">
          <button
            onClick={handleInstall}
            className="flex items-center gap-1.5 rounded-lg bg-orange-500 hover:bg-orange-400 transition-colors px-3 py-1.5 text-xs font-semibold text-white"
          >
            <Download className="h-3.5 w-3.5" />
            Install &amp; Restart
          </button>
        </div>
      )}
    </div>
  );
}
