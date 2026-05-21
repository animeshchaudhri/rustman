import { useState, useEffect, useRef, useMemo } from "react";
import { Search, Clock, BookOpen, LayoutList } from "lucide-react";
import { cn } from "@/lib/utils";
import type { SavedRequest, HistoryEntry, Collection } from "../types";
import type { RequestTab } from "../hooks/useRequestTabs";

const METHOD_COLORS: Record<string, string> = {
  GET: "text-emerald-500 bg-emerald-500/10",
  POST: "text-orange-500 bg-orange-500/10",
  PUT: "text-blue-500 bg-blue-500/10",
  PATCH: "text-teal-500 bg-teal-500/10",
  DELETE: "text-red-500 bg-red-500/10",
  HEAD: "text-purple-500 bg-purple-500/10",
  OPTIONS: "text-sky-500 bg-sky-500/10",
};

interface PaletteItem {
  id: string;
  method: string;
  name: string;
  url: string;
  source: string;
  sourceType: "collection" | "history" | "tab";
  request: SavedRequest;
  tabId?: string;
}

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  requests: SavedRequest[];
  collections: Collection[];
  history: HistoryEntry[];
  openTabs: RequestTab[];
  activeTabId: string;
  onOpen: (req: SavedRequest) => void;
  onSwitchTab: (tabId: string) => void;
}

function scoreMatch(query: string, text: string): number {
  if (!text) return 0;
  const q = query.toLowerCase();
  const t = text.toLowerCase();
  if (t === q) return 100;
  if (t.startsWith(q)) return 80;
  if (t.includes(q)) return 60;
  let qi = 0;
  for (let ti = 0; ti < t.length && qi < q.length; ti++) {
    if (t[ti] === q[qi]) qi++;
  }
  return qi === q.length ? 30 : 0;
}

export function CommandPalette({
  open,
  onClose,
  requests,
  collections,
  history,
  openTabs,
  activeTabId,
  onOpen,
  onSwitchTab,
}: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (open) {
      setQuery("");
      setActiveIndex(0);
      setTimeout(() => inputRef.current?.focus(), 30);
    }
  }, [open]);

  const collectionMap = useMemo(
    () => new Map(collections.map((c) => [c.id, c.name])),
    [collections],
  );

  const allItems = useMemo<PaletteItem[]>(() => {
    const fromTabs: PaletteItem[] = openTabs
      .filter((t) => t.urlInput.trim() || t.name !== "Untitled Request")
      .map((t) => ({
        id: `tab-${t.id}`,
        method: t.method,
        name: t.name,
        url: t.urlInput,
        source: t.id === activeTabId ? "Active Tab" : "Open Tab",
        sourceType: "tab" as const,
        tabId: t.id,
        request: {
          id: t.savedRequestId ?? t.id,
          collectionId: "",
          name: t.name,
          method: t.method,
          url: t.urlInput,
          headers: t.headers,
          params: t.params,
          body: t.body,
          bodyType: t.bodyType,
          authType: t.authType,
          bearerToken: t.bearerToken,
          basicUser: t.basicUser,
          basicPass: t.basicPass,
          apiKeyName: t.apiKeyName,
          apiKeyValue: t.apiKeyValue,
          apiKeyLocation: t.apiKeyLocation,
          formDataFields: t.formDataFields,
          cookieString: t.cookieString,
          cookies: t.cookies,
          preRequestScript: t.preRequestScript,
          testScript: t.testScript,
        },
      }));

    const fromRequests: PaletteItem[] = requests.map((req) => ({
      id: `req-${req.id}`,
      method: req.method,
      name: req.name,
      url: req.url,
      source: collectionMap.get(req.collectionId) ?? "Collection",
      sourceType: "collection" as const,
      request: req,
    }));

    const seenHistory = new Set<string>();
    const fromHistory: PaletteItem[] = [];
    for (const entry of history) {
      const key = `${entry.method}:${entry.url}`;
      if (!seenHistory.has(key)) {
        seenHistory.add(key);
        fromHistory.push({
          id: `hist-${entry.id}`,
          method: entry.method,
          name: entry.request?.name || entry.url,
          url: entry.url,
          source: "History",
          sourceType: "history" as const,
          request: entry.request,
        });
      }
    }

    return [...fromTabs, ...fromRequests, ...fromHistory];
  }, [openTabs, requests, history, collectionMap, activeTabId]);

  const filtered = useMemo<PaletteItem[]>(() => {
    if (!query.trim()) return allItems.slice(0, 25);
    const q = query.trim();
    return allItems
      .map((item) => ({
        item,
        score: Math.max(
          scoreMatch(q, item.name),
          scoreMatch(q, item.url),
          scoreMatch(q, item.method),
          scoreMatch(q, item.source),
        ),
      }))
      .filter(({ score }) => score > 0)
      .sort((a, b) => b.score - a.score)
      .map(({ item }) => item)
      .slice(0, 25);
  }, [query, allItems]);

  useEffect(() => {
    setActiveIndex((prev) => Math.min(prev, Math.max(0, filtered.length - 1)));
  }, [filtered.length]);

  useEffect(() => {
    if (!listRef.current) return;
    const el = listRef.current.querySelector(`[data-idx="${activeIndex}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [activeIndex]);

  const handleSelect = (item: PaletteItem) => {
    if (item.sourceType === "tab" && item.tabId) {
      onSwitchTab(item.tabId);
    } else {
      onOpen(item.request);
    }
    onClose();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActiveIndex((i) => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActiveIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (filtered[activeIndex]) handleSelect(filtered[activeIndex]);
    } else if (e.key === "Escape") {
      onClose();
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[10vh]">
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={onClose}
      />
      <div className="relative bg-white dark:bg-zinc-900 border border-stone-200 dark:border-zinc-700 rounded-xl shadow-2xl w-[640px] max-w-[95vw] overflow-hidden flex flex-col max-h-[65vh]">
        <div className="flex items-center gap-3 px-4 py-3 border-b border-stone-200 dark:border-zinc-800 shrink-0">
          <Search className="h-4 w-4 text-zinc-400 shrink-0" />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setActiveIndex(0);
            }}
            onKeyDown={handleKeyDown}
            placeholder="Search requests by name, URL, or method…"
            className="flex-1 bg-transparent text-sm text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-400 dark:placeholder:text-zinc-600 focus:outline-none"
            autoCorrect="off"
            autoCapitalize="none"
            spellCheck={false}
          />
          <kbd className="text-[10px] text-zinc-400 dark:text-zinc-600 border border-stone-300 dark:border-zinc-700 rounded px-1.5 py-0.5 font-mono shrink-0">
            esc
          </kbd>
        </div>

        <div ref={listRef} className="overflow-y-auto flex-1 min-h-0">
          {filtered.length === 0 ? (
            <div className="py-16 text-center text-sm text-zinc-400 dark:text-zinc-600">
              {query ? "No results found" : "Start typing to search…"}
            </div>
          ) : (
            filtered.map((item, idx) => (
              <button
                key={item.id}
                data-idx={idx}
                onClick={() => handleSelect(item)}
                onMouseMove={() => setActiveIndex(idx)}
                className={cn(
                  "w-full flex items-center gap-3 px-4 py-2.5 text-left transition-colors border-b border-stone-100/60 dark:border-zinc-800/60",
                  idx === activeIndex
                    ? "bg-orange-500/10"
                    : "hover:bg-stone-50 dark:hover:bg-zinc-800/60",
                )}
              >
                <span
                  className={cn(
                    "text-[10px] font-bold px-1.5 py-0.5 rounded shrink-0 min-w-[42px] text-center",
                    METHOD_COLORS[item.method] ??
                      "text-zinc-500 bg-stone-200 dark:bg-zinc-700",
                  )}
                >
                  {item.method}
                </span>
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-medium text-zinc-800 dark:text-zinc-200 truncate">
                    {item.name}
                  </div>
                  {item.url && (
                    <div className="text-[11px] text-zinc-400 dark:text-zinc-600 truncate font-mono">
                      {item.url}
                    </div>
                  )}
                </div>
                <div className="flex items-center gap-1 shrink-0">
                  {item.sourceType === "history" ? (
                    <Clock className="h-3 w-3 text-zinc-400" />
                  ) : item.sourceType === "tab" ? (
                    <LayoutList className="h-3 w-3 text-zinc-400" />
                  ) : (
                    <BookOpen className="h-3 w-3 text-zinc-400" />
                  )}
                  <span className="text-[10px] text-zinc-400 dark:text-zinc-600 max-w-[90px] truncate">
                    {item.source}
                  </span>
                </div>
              </button>
            ))
          )}
        </div>

        <div className="flex items-center justify-between px-4 py-2 border-t border-stone-100 dark:border-zinc-800 bg-stone-50/80 dark:bg-zinc-900/80 shrink-0">
          <span className="text-[10px] text-zinc-400 dark:text-zinc-600">
            {filtered.length} result{filtered.length !== 1 ? "s" : ""}
          </span>
          <div className="flex items-center gap-3 text-[10px] text-zinc-400 dark:text-zinc-600">
            <span>
              <kbd className="font-mono border border-stone-300 dark:border-zinc-700 rounded px-1">
                ↑↓
              </kbd>{" "}
              navigate
            </span>
            <span>
              <kbd className="font-mono border border-stone-300 dark:border-zinc-700 rounded px-1">
                ↵
              </kbd>{" "}
              open
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
