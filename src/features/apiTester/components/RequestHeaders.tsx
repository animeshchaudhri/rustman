import { useState } from "react";
import type { HeaderType } from "../types";
import { cn } from "@/lib/utils";
import { AlignLeft, Key, Plus, Table, Trash2, ToggleLeft, ToggleRight } from "lucide-react";

interface RequestHeadersProps {
  headers: HeaderType[];
  onAddHeader: () => void;
  onHeaderChange: (id: string, field: "key" | "value" | "enabled", value: string | boolean) => void;
  onRemoveHeader: (id: string) => void;
  onExtractFromCookie?: () => void;
}

const COMMON_HEADERS = [
  "Content-Type", "Authorization", "Accept", "Accept-Language",
  "Cache-Control", "X-Requested-With", "X-API-Key", "X-Request-ID",
  "Origin", "Referer", "User-Agent", "Cookie",
];

export function RequestHeaders({ headers, onAddHeader, onHeaderChange, onRemoveHeader, onExtractFromCookie }: RequestHeadersProps) {
  const [bulkMode, setBulkMode] = useState(false);
  const [bulkText, setBulkText] = useState("");
  const [showSuggestions, setShowSuggestions] = useState<string | null>(null);

  const enabledCount = headers.filter((h) => h.key && h.enabled).length;

  const toBulkText = (hdrs: HeaderType[]) =>
    hdrs.filter((h) => h.key).map((h) => `${h.key}: ${h.value}`).join("\n");

  const fromBulkText = (text: string): HeaderType[] =>
    text.split("\n")
      .map((line) => line.trim())
      .filter((line) => line && !line.startsWith("#"))
      .map((line) => {
        const colon = line.indexOf(":");
        if (colon === -1) return { id: crypto.randomUUID(), key: line.trim(), value: "", enabled: true };
        return {
          id: crypto.randomUUID(),
          key: line.slice(0, colon).trim(),
          value: line.slice(colon + 1).trim(),
          enabled: true,
        };
      });

  const enterBulkMode = () => {
    setBulkText(toBulkText(headers));
    setBulkMode(true);
  };

  const commitBulkMode = () => {
    const parsed = fromBulkText(bulkText);

    setBulkMode(false);

    const existing = [...headers];
    parsed.forEach((newH, i) => {
      if (i < existing.length) {
        onHeaderChange(existing[i].id, "key", newH.key);
        onHeaderChange(existing[i].id, "value", newH.value);
        onHeaderChange(existing[i].id, "enabled", true);
      } else {
        onAddHeader();
        setTimeout(() => {
        }, 0);
      }
    });

    for (let i = parsed.length; i < existing.length; i++) {
      onRemoveHeader(existing[i].id);
    }
  };

  const toggleAll = () => {
    const allEnabled = headers.every((h) => h.enabled);
    headers.forEach((h) => onHeaderChange(h.id, "enabled", !allEnabled));
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-stone-200 dark:border-zinc-800 shrink-0">
        <div className="flex items-center gap-1">
          <span className="text-xs text-zinc-500 dark:text-zinc-500">
            {enabledCount > 0 && <span className="text-orange-400 font-medium">{enabledCount}</span>}
            {enabledCount > 0 ? " active" : "No active headers"}
          </span>
        </div>
        <div className="flex items-center gap-1">
          {headers.length > 0 && (
            <button
              onClick={toggleAll}
              title="Toggle all"
              className="flex items-center gap-1 px-2 py-0.5 text-xs text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800 rounded transition-colors"
            >
              {headers.every((h) => h.enabled)
                ? <ToggleRight className="h-3.5 w-3.5" />
                : <ToggleLeft className="h-3.5 w-3.5" />}
              Toggle all
            </button>
          )}
          <button
            onClick={bulkMode ? commitBulkMode : enterBulkMode}
            className={cn(
              "flex items-center gap-1 px-2 py-0.5 text-xs rounded transition-colors",
              bulkMode
                ? "bg-orange-600/20 text-orange-400 border border-orange-500/30"
                : "text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800",
            )}
          >
            {bulkMode ? <Table className="h-3.5 w-3.5" /> : <AlignLeft className="h-3.5 w-3.5" />}
            {bulkMode ? "Table" : "Bulk Edit"}
          </button>
          {onExtractFromCookie && (
            <button
              onClick={onExtractFromCookie}
              title="Decode JWT from Cookie header and inject x-user-detail"
              className="flex items-center gap-1 px-2 py-0.5 text-xs text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800 rounded transition-colors"
            >
              <Key className="h-3.5 w-3.5" />
              JWT→Detail
            </button>
          )}
          <button
            onClick={onAddHeader}
            className="flex items-center gap-1 px-2 py-0.5 text-xs text-zinc-500 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100 hover:bg-stone-100 dark:hover:bg-zinc-800 rounded transition-colors"
          >
            <Plus className="h-3.5 w-3.5" />
            Add
          </button>
        </div>
      </div>

      {bulkMode ? (
        <div className="flex-1 flex flex-col p-3 gap-2">
          <p className="text-xs text-zinc-500 dark:text-zinc-500">One header per line: <code className="text-zinc-500 dark:text-zinc-400">Key: Value</code></p>
          <textarea
            autoFocus
            value={bulkText}
            onChange={(e) => setBulkText(e.target.value)}
            placeholder={"Content-Type: application/json\nAuthorization: Bearer <token>\nX-API-Key: abc123"}
            autoCorrect="off"
            autoCapitalize="none"
            spellCheck={false}
            className="flex-1 bg-white dark:bg-zinc-800 border border-stone-300 dark:border-zinc-700 rounded-lg p-3 text-xs font-mono text-zinc-800 dark:text-zinc-200 placeholder:text-zinc-400 dark:placeholder:text-zinc-600 focus:outline-none focus:border-orange-500/50 resize-none"
          />
          <div className="flex gap-2">
            <button
              onClick={commitBulkMode}
              className="px-4 py-1.5 bg-orange-600 hover:bg-orange-500 text-white text-xs font-semibold rounded-lg transition-colors"
            >
              Apply
            </button>
            <button
              onClick={() => setBulkMode(false)}
              className="px-4 py-1.5 bg-white dark:bg-zinc-800 hover:bg-stone-100 dark:hover:bg-zinc-700 text-zinc-700 dark:text-zinc-300 text-xs rounded-lg border border-stone-300 dark:border-zinc-700 transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto">
          {headers.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-10 gap-2 text-zinc-400 dark:text-zinc-700">
              <p className="text-xs">No headers. Click Add or Bulk Edit.</p>
            </div>
          ) : (
            <>
              <div className="grid grid-cols-[20px_1fr_1fr_28px] gap-1 px-3 py-1 border-b border-stone-200/60 dark:border-zinc-800/60">
                <div />
                <div className="text-[10px] uppercase tracking-wider text-zinc-400 dark:text-zinc-600 font-medium">Key</div>
                <div className="text-[10px] uppercase tracking-wider text-zinc-400 dark:text-zinc-600 font-medium">Value</div>
                <div />
              </div>

              {headers.map((h) => (
                <div
                  key={h.id}
                  className={cn(
                    "grid grid-cols-[20px_1fr_1fr_28px] gap-1 px-3 py-1 items-center",
                    "hover:bg-stone-100/40 dark:hover:bg-zinc-800/40 group border-b border-stone-200/30 dark:border-zinc-800/30",
                    !h.enabled && "opacity-40",
                  )}
                >
                  <input
                    type="checkbox"
                    checked={h.enabled}
                    onChange={(e) => onHeaderChange(h.id, "enabled", e.target.checked)}
                    className="w-3.5 h-3.5 accent-orange-500 cursor-pointer"
                  />

                  <div className="relative">
                    <input
                      value={h.key}
                      onChange={(e) => onHeaderChange(h.id, "key", e.target.value)}
                      onFocus={() => setShowSuggestions(h.id)}
                      onBlur={() => setTimeout(() => setShowSuggestions(null), 150)}
                      placeholder="Key"
                      disabled={!h.enabled}
                      autoCorrect="off"
                      autoCapitalize="none"
                      spellCheck={false}
                      className="w-full bg-transparent border-b border-transparent hover:border-stone-300 dark:hover:border-zinc-700 focus:border-orange-500/60 px-1 py-0.5 text-xs font-medium text-zinc-700 dark:text-zinc-300 placeholder:text-zinc-500 dark:placeholder:text-zinc-700 focus:outline-none transition-colors"
                    />
                    {showSuggestions === h.id && h.key.length >= 1 && (
                      <div className="absolute top-full left-0 z-20 mt-0.5 bg-white dark:bg-zinc-800 border border-stone-300 dark:border-zinc-700 rounded-lg shadow-xl max-h-40 overflow-y-auto min-w-[200px]">
                        {COMMON_HEADERS.filter((s) => s.toLowerCase().includes(h.key.toLowerCase()) && s !== h.key).map((s) => (
                          <button
                            key={s}
                            onMouseDown={() => { onHeaderChange(h.id, "key", s); setShowSuggestions(null); }}
                            className="block w-full text-left px-3 py-1.5 text-xs text-zinc-700 dark:text-zinc-300 hover:bg-stone-100 dark:hover:bg-zinc-700 hover:text-zinc-900 dark:hover:text-zinc-100"
                          >
                            {s}
                          </button>
                        ))}
                      </div>
                    )}
                  </div>

                  <input
                    value={h.value}
                    onChange={(e) => onHeaderChange(h.id, "value", e.target.value)}
                    placeholder="Value"
                    disabled={!h.enabled}
                    autoCorrect="off"
                    autoCapitalize="none"
                    spellCheck={false}
                    className="w-full bg-transparent border-b border-transparent hover:border-stone-300 dark:hover:border-zinc-700 focus:border-orange-500/60 px-1 py-0.5 text-xs font-mono text-zinc-500 dark:text-zinc-400 placeholder:text-zinc-500 dark:placeholder:text-zinc-700 focus:outline-none transition-colors"
                  />

                  <button
                    onClick={() => onRemoveHeader(h.id)}
                    className="opacity-0 group-hover:opacity-100 flex items-center justify-center text-zinc-400 dark:text-zinc-600 hover:text-red-400 transition-all"
                  >
                    <Trash2 className="h-3 w-3" />
                  </button>
                </div>
              ))}

              <div
                onClick={onAddHeader}
                className="grid grid-cols-[20px_1fr_1fr_28px] gap-1 px-3 py-2 items-center cursor-pointer hover:bg-stone-100/40 dark:hover:bg-zinc-800/40 border-b border-stone-200/30 dark:border-zinc-800/30"
              >
                <div />
                <span className="text-xs text-zinc-400 dark:text-zinc-700 italic">+ Add header</span>
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
