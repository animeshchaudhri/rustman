import { useState } from "react";
import type { HeaderType } from "../types";
import { cn } from "@/lib/utils";
import { Plus, Trash2, Link, Copy, Check } from "lucide-react";

interface RequestParamsProps {
  params: HeaderType[];
  onAddParam: () => void;
  onParamChange: (id: string, field: "key" | "value" | "enabled", value: string | boolean) => void;
  onRemoveParam: (id: string) => void;
  urlInput?: string;
}

export function RequestParams({ params, onAddParam, onParamChange, onRemoveParam, urlInput = "" }: RequestParamsProps) {
  const [copied, setCopied] = useState(false);

  const enabledCount = params.filter((p) => p.key && p.enabled).length;

  const queryString = params
    .filter((p) => p.key && p.enabled)
    .map((p) => `${encodeURIComponent(p.key)}=${encodeURIComponent(p.value)}`)
    .join("&");

  const fullPreview = queryString ? `${urlInput.split("?")[0]}?${queryString}` : urlInput.split("?")[0];

  const copyUrl = () => {
    navigator.clipboard.writeText(fullPreview);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-stone-200 dark:border-zinc-800 shrink-0">
        <span className="text-xs text-zinc-500 dark:text-zinc-500">
          {enabledCount > 0 ? (
            <>
              <span className="text-brand-400 font-medium">{enabledCount}</span> active param{enabledCount !== 1 ? "s" : ""}
            </>
          ) : (
            "No active params"
          )}
        </span>
        <button
          onClick={onAddParam}
          className="flex items-center gap-1 px-2 py-0.5 text-xs text-zinc-500 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100 hover:bg-stone-100 dark:hover:bg-zinc-800 rounded transition-colors"
        >
          <Plus className="h-3.5 w-3.5" />
          Add
        </button>
      </div>

      <div className="flex-1 overflow-y-auto">
        {params.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-10 text-zinc-400 dark:text-zinc-700">
            <p className="text-xs">No query params. Click Add to start.</p>
          </div>
        ) : (
          <>
            <div className="grid grid-cols-[20px_1fr_1fr_28px] gap-1 px-3 py-1 border-b border-stone-200/60 dark:border-zinc-800/60">
              <div />
              <div className="text-[10px] uppercase tracking-wider text-zinc-400 dark:text-zinc-600 font-medium">Key</div>
              <div className="text-[10px] uppercase tracking-wider text-zinc-400 dark:text-zinc-600 font-medium">Value</div>
              <div />
            </div>

            {params.map((p) => (
              <div
                key={p.id}
                className={cn(
                  "grid grid-cols-[20px_1fr_1fr_28px] gap-1 px-3 py-1 items-center",
                  "hover:bg-stone-100/40 dark:hover:bg-zinc-800/40 group border-b border-stone-200/30 dark:border-zinc-800/30",
                  !p.enabled && "opacity-40",
                )}
              >
                <input
                  type="checkbox"
                  checked={p.enabled}
                  onChange={(e) => onParamChange(p.id, "enabled", e.target.checked)}
                  className="w-3.5 h-3.5 accent-brand-500 cursor-pointer"
                />
                <input
                  value={p.key}
                  onChange={(e) => onParamChange(p.id, "key", e.target.value)}
                  placeholder="Key"
                  disabled={!p.enabled}
                  autoCorrect="off"
                  autoCapitalize="none"
                  spellCheck={false}
                  className="w-full bg-transparent border-b border-transparent hover:border-stone-300 dark:hover:border-zinc-700 focus:border-brand-500/60 px-1 py-0.5 text-xs font-medium text-zinc-700 dark:text-zinc-300 placeholder:text-zinc-500 dark:placeholder:text-zinc-700 focus:outline-none transition-colors"
                />
                <input
                  value={p.value}
                  onChange={(e) => onParamChange(p.id, "value", e.target.value)}
                  placeholder="Value"
                  disabled={!p.enabled}
                  autoCorrect="off"
                  autoCapitalize="none"
                  spellCheck={false}
                  className="w-full bg-transparent border-b border-transparent hover:border-stone-300 dark:hover:border-zinc-700 focus:border-brand-500/60 px-1 py-0.5 text-xs font-mono text-zinc-500 dark:text-zinc-400 placeholder:text-zinc-500 dark:placeholder:text-zinc-700 focus:outline-none transition-colors"
                />
                <button
                  onClick={() => onRemoveParam(p.id)}
                  className="opacity-0 group-hover:opacity-100 flex items-center justify-center text-zinc-400 dark:text-zinc-600 hover:text-red-400 transition-all"
                >
                  <Trash2 className="h-3 w-3" />
                </button>
              </div>
            ))}

            <div
              onClick={onAddParam}
              className="grid grid-cols-[20px_1fr_1fr_28px] gap-1 px-3 py-2 cursor-pointer hover:bg-stone-100/40 dark:hover:bg-zinc-800/40 border-b border-stone-200/30 dark:border-zinc-800/30"
            >
              <div />
              <span className="text-xs text-zinc-400 dark:text-zinc-700 italic">+ Add parameter</span>
            </div>
          </>
        )}
      </div>

      {enabledCount > 0 && (
        <div className="shrink-0 border-t border-stone-200 dark:border-zinc-800 px-3 py-2 flex items-center gap-2">
          <Link className="h-3 w-3 text-zinc-400 dark:text-zinc-600 shrink-0" />
          <span className="flex-1 text-[11px] font-mono text-zinc-500 dark:text-zinc-500 truncate">{fullPreview}</span>
          <button onClick={copyUrl} className="shrink-0 text-zinc-400 dark:text-zinc-600 hover:text-zinc-700 dark:hover:text-zinc-300 transition-colors">
            {copied ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
          </button>
        </div>
      )}
    </div>
  );
}
