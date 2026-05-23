import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { ApiResponse, ConsoleEntry, ResponseBodyView, ResponseTabType, PanelLayout, TestResult } from "../types";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Clock, Database, XCircle,
  ArrowUpDown, Copy, Check, Search, X, WrapText, AlignLeft,
  PanelBottomOpen, PanelRightOpen, CheckCircle2, AlertCircle,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { bodyGetSlice, bodyGetFull, bodySearchLines, type BodySlice } from "@/lib/db";

interface ResponseViewerProps {
  isLoading: boolean;
  response: ApiResponse | null;
  error: string | null;
  responseTime: number | null;
  responseSize: number | null;
  activeTab: string;
  onTabChange: (tab: ResponseTabType) => void;
  bodyView: ResponseBodyView;
  onBodyViewChange: (view: ResponseBodyView) => void;
  tabId: string;
  testResults?: TestResult[];
  consoleLogs?: ConsoleEntry[];
  layout: PanelLayout;
  onLayoutChange: (l: PanelLayout) => void;
}

type JsonPrimitive = string | number | boolean | null;
interface JsonObject { [key: string]: JsonValue }
type JsonValue = JsonPrimitive | JsonValue[] | JsonObject;

const PAGE_SIZE = 500;
const MAX_DOM_LINES = 1500;
const LINE_HEIGHT_PX = 21; // approx px per line (font-mono text-xs leading-[1.7])
const TREE_THRESHOLD = 30_000; // chars: above this, skip recursive JSON tree

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function escapeRegExp(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function renderHighlightedText(text: string, search: string, markClassName = "bg-yellow-400/40 text-inherit rounded-sm"): ReactNode {
  if (!search) return text;
  const parts = text.split(new RegExp(`(${escapeRegExp(search)})`, "gi"));
  if (parts.length === 1) return text;
  const lowered = search.toLowerCase();
  return parts.map((part, index) =>
    part.toLowerCase() === lowered ? (
      <mark key={`${part}-${index}`} className={markClassName}>{part}</mark>
    ) : (
      <span key={`${part}-${index}`}>{part}</span>
    ),
  );
}

interface JsonNodeProps {
  keyName?: string;
  value: JsonValue;
  depth: number;
  searchTerm: string;
  isLast?: boolean;
}

const LOADING_MESSAGES = [
  "There are only two hard things in software: naming things and cache invalidation.",
  "Most programming is just googling with confidence.",
  "A bug becomes a feature if enough users depend on it.",
  "The code worked yesterday. Nobody knows why.",
  "99% of software engineering is figuring out why it broke.",
  "Every developer has a folder named 'final_final_v2'.",
  "Programming: turning caffeine into confusion since forever.",
  "Fixing one bug usually creates two new ones.",
  "The best code is the code you don’t have to write.",
  "Software developers spend more time debugging than coding.",
  "If it works on the first try, be suspicious.",
  "Nothing is more permanent than a temporary fix.",
  "Computers are fast because they make mistakes very quickly.",
  "Deploying to production builds character.",
  "Half of coding is convincing the compiler you’re right.",
  "One missing semicolon can ruin an entire afternoon.",
  "Git knows what you did. Even if you don’t.",
  "The stack trace is just the app crying for help.",
  "Behind every great app is a developer reading logs at 2 AM.",
  "Programming would be easy if not for the computers.",

  "Built by Animesh. Probably over-engineered on purpose.",
  "Animesh is somewhere optimizing this request right now.",
  "Fun fact: Animesh definitely said 'just one more feature' yesterday.",
  "Made by Animesh with dangerously high confidence levels.",
  "Animesh ships bugs directly to production like a real engineer.",
  "Animesh believes sleep is optional during deployments.",
  "Powered by code, caffeine, and Animesh.",
  "Animesh tested this locally. Brave move.",
  "Another masterpiece from Animesh Industries.",
  "Animesh wrote this. The compiler survived somehow.",
];

export function ResponseViewer({
  isLoading,
  response,
  error,
  responseTime,
  responseSize,
  activeTab,
  onTabChange,
  bodyView,
  onBodyViewChange,
  tabId,
  testResults,
  consoleLogs,
  layout,
  onLayoutChange,
}: ResponseViewerProps) {
  const [copied, setCopied] = useState(false);
  const [search, setSearch] = useState("");
  const [showSearch, setShowSearch] = useState(false);
  const [wordWrap, setWordWrap] = useState(true);
  const loadingMsg = useMemo(
    () => LOADING_MESSAGES[Math.floor(Math.random() * LOADING_MESSAGES.length)],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [isLoading],
  );
  const loadingSvg = useMemo(
    () => Math.random() < 0.5 ? "/The Nyan Cat.svg" : "/Dancing Pallbearers.svg",
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [isLoading],
  );

  const [windowLines, setWindowLines] = useState<string[]>([]);
  const [windowStart, setWindowStart] = useState(0);
  const [rustTotalLines, setRustTotalLines] = useState(0);

  const [rustMatchLines, setRustMatchLines] = useState<number[]>([]);
  const [currentMatchIdx, setCurrentMatchIdx] = useState(0);
  const searchDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingScrollLineRef = useRef<number | null>(null);

  const scrollRef = useRef<HTMLDivElement>(null);

  const isLoadingMoreRef = useRef(false);
  const windowStartRef = useRef(0);
  const windowLinesLenRef = useRef(0);
  const rustTotalRef = useRef(0);
  const rustKeyRef = useRef("");

  const rustKey = `${tabId}:${bodyView === "raw" ? "raw_display" : "pretty"}`;
  const copyKey = `${tabId}:${bodyView === "raw" ? "raw" : "pretty"}`;

  useEffect(() => { windowStartRef.current = windowStart; }, [windowStart]);
  useEffect(() => { windowLinesLenRef.current = windowLines.length; }, [windowLines.length]);
  useEffect(() => { rustTotalRef.current = rustTotalLines; }, [rustTotalLines]);
  useEffect(() => { rustKeyRef.current = rustKey; }, [rustKey]);

  useEffect(() => {
    setWindowLines([]);
    setWindowStart(0);
    setRustTotalLines(0);
    setRustMatchLines([]);
    setSearch("");
    setShowSearch(false);
    setCurrentMatchIdx(0);
    isLoadingMoreRef.current = false;
    windowStartRef.current = 0;
    windowLinesLenRef.current = 0;
    if (!response) return;

    bodyGetSlice(rustKey, 0, PAGE_SIZE)
      .then((slice: BodySlice) => {
        setWindowLines(slice.lines);
        setRustTotalLines(slice.totalLines);
        rustTotalRef.current = slice.totalLines;
        windowLinesLenRef.current = slice.lines.length;
      })
      .catch(console.error);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [response, tabId, bodyView]);

  const handleScroll = useCallback(() => {
    const el = scrollRef.current;
    if (!el || isLoadingMoreRef.current) return;

    const { scrollTop, clientHeight } = el;
    const start = windowStartRef.current;
    const len = windowLinesLenRef.current;
    const total = rustTotalRef.current;
    const key = rustKeyRef.current;
    const windowEnd = start + len;

    // Trigger load-more when viewport bottom approaches the END of loaded content.
    // Compare against loaded-content boundary (not absolute scrollHeight) so phantom
    // padding below doesn't prevent the trigger from firing.
    const loadedBottomPx = windowEnd * LINE_HEIGHT_PX;
    if (scrollTop + clientHeight > loadedBottomPx - 400 && windowEnd < total) {
      isLoadingMoreRef.current = true;
      bodyGetSlice(key, windowEnd, PAGE_SIZE)
        .then((slice: BodySlice) => {
          setWindowLines((prev) => {
            let combined = [...prev, ...slice.lines];
            let newStart = start;
            if (combined.length > MAX_DOM_LINES) {
              const drop = combined.length - MAX_DOM_LINES;
              newStart = start + drop;
              combined = combined.slice(drop);
              el.scrollTop = Math.max(0, el.scrollTop - drop * LINE_HEIGHT_PX);
            }
            windowStartRef.current = newStart;
            windowLinesLenRef.current = combined.length;
            setWindowStart(newStart);
            return combined;
          });
          rustTotalRef.current = slice.totalLines;
          setRustTotalLines(slice.totalLines);
          isLoadingMoreRef.current = false;
        })
        .catch(() => { isLoadingMoreRef.current = false; });
    }

    // Trigger load-more when viewport top approaches the START of loaded content.
    // Include paddingTop offset so the check works even when windowStart > 0.
    const loadedTopPx = start * LINE_HEIGHT_PX;
    if (scrollTop < loadedTopPx + 400 && start > 0 && !isLoadingMoreRef.current) {
      isLoadingMoreRef.current = true;
      const fetchStart = Math.max(0, start - PAGE_SIZE);
      const count = start - fetchStart;
      const prevScrollTop = el.scrollTop;
      bodyGetSlice(key, fetchStart, count)
        .then((slice: BodySlice) => {
          setWindowLines((prev) => {
            let combined = [...slice.lines, ...prev];
            if (combined.length > MAX_DOM_LINES) {
              combined = combined.slice(0, MAX_DOM_LINES);
            }
            windowStartRef.current = fetchStart;
            windowLinesLenRef.current = combined.length;
            setWindowStart(fetchStart);
            requestAnimationFrame(() => {
              if (el) el.scrollTop = prevScrollTop + slice.lines.length * LINE_HEIGHT_PX;
            });
            return combined;
          });
          isLoadingMoreRef.current = false;
        })
        .catch(() => { isLoadingMoreRef.current = false; });
    }
  }, []); // uses only refs — no deps

  useEffect(() => {
    setRustMatchLines([]);
    setCurrentMatchIdx(0);
    if (!search || !response) return;
    if (searchDebounceRef.current) clearTimeout(searchDebounceRef.current);
    searchDebounceRef.current = setTimeout(() => {
      bodySearchLines(rustKey, search)
        .then(setRustMatchLines)
        .catch(() => setRustMatchLines([]));
    }, 300);
    return () => { if (searchDebounceRef.current) clearTimeout(searchDebounceRef.current); };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, response, tabId, bodyView]);

  useEffect(() => {
    if (pendingScrollLineRef.current !== null) {
      const line = pendingScrollLineRef.current;
      pendingScrollLineRef.current = null;
      requestAnimationFrame(() => requestAnimationFrame(() => scrollToLine(line)));
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [windowLines]);

  const scrollToLine = useCallback((globalIdx: number) => {
    const el = scrollRef.current;
    if (!el) return;
    const target = el.querySelector(`[data-line="${globalIdx}"]`) as HTMLElement | null;
    if (target) {
      target.scrollIntoView({ block: "center", behavior: "smooth" });
      target.classList.add("bg-brand-400/20");
      setTimeout(() => target.classList.remove("bg-orange-400/20"), 1200);
    }
  }, []);

  const goToMatch = useCallback(async (delta: number) => {
    if (rustMatchLines.length === 0) return;
    const next = (currentMatchIdx + delta + rustMatchLines.length) % rustMatchLines.length;
    setCurrentMatchIdx(next);
    const targetLine = rustMatchLines[next];

    const winStart = windowStartRef.current;
    const winEnd = winStart + windowLinesLenRef.current;

    if (targetLine >= winStart && targetLine < winEnd) {
      scrollToLine(targetLine);
    } else {
      const fetchStart = Math.max(0, targetLine - Math.floor(PAGE_SIZE / 2));
      try {
        const slice = await bodyGetSlice(rustKeyRef.current, fetchStart, PAGE_SIZE);
        windowStartRef.current = fetchStart;
        windowLinesLenRef.current = slice.lines.length;
        rustTotalRef.current = slice.totalLines;
        pendingScrollLineRef.current = targetLine;
        setWindowStart(fetchStart);
        setWindowLines(slice.lines);
        setRustTotalLines(slice.totalLines);
      } catch { /* ignore */ }
    }
  }, [currentMatchIdx, rustMatchLines, scrollToLine]);

  const bodyText = useMemo(() => {
    if (!response) return "";
    if (bodyView === "pretty") {
      const d = response.data;
      if (typeof d === "object" && d !== null) return JSON.stringify(d, null, 2);
      if (typeof d === "string") { try { return JSON.stringify(JSON.parse(d), null, 2); } catch {} }
      return String(d ?? "");
    }
    const d = response.data;
    if (typeof d === "object" && d !== null) return JSON.stringify(d);
    return String(d ?? "");
  }, [response, bodyView]);

  const parsedJsonData = useMemo<JsonValue | null>(() => {
    if (!response) return null;
    const data = response.data;
    if (data === null || typeof data === "number" || typeof data === "boolean") return data as JsonValue;
    if (typeof data === "object") return data as JsonValue;
    if (typeof data === "string") { try { return JSON.parse(data) as JsonValue; } catch { return null; } }
    return null;
  }, [response]);

  const rustReady = windowLines.length > 0 || rustTotalLines > 0;
  const isLarge = bodyText.length > TREE_THRESHOLD || rustTotalLines > 200;
  const totalMatchCount = rustMatchLines.length;

  const paddingTop = windowStart * LINE_HEIGHT_PX;
  const loadedBottom = windowStart + windowLines.length;
  const paddingBottom = Math.max(0, rustTotalLines - loadedBottom) * LINE_HEIGHT_PX;

  const handleCopy = async () => {
    const full = rustTotalLines > 0
      ? await bodyGetFull(copyKey).catch(() => windowLines.join("\n"))
      : (bodyText || windowLines.join("\n"));
    navigator.clipboard.writeText(full);
    setCopied(true);
    setTimeout(() => setCopied(false), 1800);
  };

  const searchInputRef = useRef<HTMLInputElement>(null);

  const handleSearchToggle = () => {
    setShowSearch((v) => !v);
    setSearch("");
  };

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "f") {
        e.preventDefault();
        setShowSearch(true);
        requestAnimationFrame(() => searchInputRef.current?.focus());
      } else if (e.key === "Escape" && showSearch) {
        setShowSearch(false);
        setSearch("");
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [showSearch]);

  return (
    <div className="flex flex-col bg-stone-50 dark:bg-zinc-950 h-full overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-stone-200 dark:border-zinc-800 bg-white/80 dark:bg-zinc-900/80 shrink-0 gap-3">
        <div className="flex items-center gap-3 min-w-0">
          {isLoading ? (
            <span className="text-xs text-zinc-400 dark:text-zinc-700">Response</span>
          ) : response ? (
            <StatusBadge status={response.status} />
          ) : error ? (
            <div className="flex items-center gap-2 text-red-500 dark:text-red-400">
              <XCircle className="h-3.5 w-3.5" />
              <span className="text-xs font-medium">Error</span>
            </div>
          ) : (
            <span className="text-xs text-zinc-400 dark:text-zinc-700">Response</span>
          )}
        </div>
        <div className="flex items-center gap-3 shrink-0">
          {response && !isLoading && (
            <>
              {responseTime !== null && (
                <div className="flex items-center gap-1 text-xs">
                  <Clock className="h-3 w-3 text-zinc-400 dark:text-zinc-600" />
                  <span className={cn(
                    "tabular-nums font-semibold",
                    responseTime < 300 ? "text-emerald-500 dark:text-emerald-400" :
                    responseTime < 1000 ? "text-yellow-600 dark:text-yellow-400" : "text-red-500 dark:text-red-400"
                  )}>
                    {responseTime < 1000 ? `${responseTime}ms` : `${(responseTime / 1000).toFixed(2)}s`}
                  </span>
                </div>
              )}
              {responseSize !== null && (
                <div className="flex items-center gap-1 text-xs">
                  <Database className="h-3 w-3 text-zinc-400 dark:text-zinc-600" />
                  <span className="tabular-nums text-zinc-500 dark:text-zinc-400">{formatSize(responseSize)}</span>
                </div>
              )}
            </>
          )}
          <button
            onClick={() => onLayoutChange(layout === "vertical" ? "horizontal" : "vertical")}
            title={layout === "vertical" ? "Switch to side-by-side" : "Switch to stacked"}
            className="h-6 w-6 flex items-center justify-center rounded text-zinc-400 dark:text-zinc-600 hover:text-zinc-700 dark:hover:text-zinc-300 hover:bg-stone-100 dark:hover:bg-zinc-800 transition-colors"
          >
            {layout === "vertical"
              ? <PanelRightOpen className="h-3.5 w-3.5" />
              : <PanelBottomOpen className="h-3.5 w-3.5" />}
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-hidden flex flex-col min-h-0">
        {isLoading && (
          <div className="flex-1 flex items-center justify-center">
            <div className="flex flex-col items-center gap-4">
              <img src={loadingSvg} alt="Loading…" className="h-64 w-auto" />
              <span className="text-sm font-semibold text-zinc-500 dark:text-zinc-400">{loadingMsg}</span>
              <span className="text-xs text-zinc-400 dark:text-zinc-600">Nyan nyan nyan nyan nyan nyan…</span>
            </div>
          </div>
        )}

        {!isLoading && error && (
          <div className="flex-1 flex flex-col items-center justify-center gap-3 p-6">
            <XCircle className="h-10 w-10 text-red-500/50" />
            <p className="text-sm font-semibold text-red-500 dark:text-red-400">Request Failed</p>
            <pre className="select-text text-xs text-red-700 dark:text-red-300/80 font-mono bg-white dark:bg-zinc-900 border border-red-500/20 dark:border-red-900/40 rounded-xl px-5 py-3 max-w-lg text-center whitespace-pre-wrap break-all">{error}</pre>
          </div>
        )}

        {!isLoading && !error && !response && (
          <div className="flex-1 flex flex-col items-center justify-center gap-2 text-zinc-400 dark:text-zinc-800">
            <ArrowUpDown className="h-9 w-9" />
            <p className="text-xs">Hit Send to see the response</p>
          </div>
        )}

        {!isLoading && !error && response && (
          <Tabs value={activeTab} onValueChange={onTabChange as (v: string) => void} className="flex flex-col h-full min-h-0">
            <div className="shrink-0 flex items-center border-b border-stone-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 px-2 gap-1">
              <TabsList className="bg-transparent rounded-none h-8 gap-0 p-0">
                {(["body", "headers", "cookies", "tests", "console"] as ResponseTabType[]).map((t) => (
                  <TabsTrigger
                    key={t}
                    value={t}
                    className={cn(
                      "text-xs px-3 py-0 h-8 rounded-none capitalize border-b-2 border-transparent",
                      "data-[state=active]:border-brand-500 data-[state=active]:bg-transparent data-[state=active]:text-brand-400",
                      "text-zinc-500 dark:text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 transition-colors",
                    )}
                  >
                    {t}
                    {t === "headers" && response.headers && (
                      <span className="ml-1.5 text-[10px] bg-stone-100 dark:bg-zinc-800 text-zinc-500 dark:text-zinc-500 rounded-full px-1.5">
                        {Object.keys(response.headers).length}
                      </span>
                    )}
                    {t === "tests" && testResults && testResults.length > 0 && (
                      <span className={cn(
                        "ml-1.5 text-[10px] rounded-full px-1.5",
                        testResults.every((r) => r.passed)
                          ? "bg-emerald-500/15 text-emerald-500"
                          : "bg-red-500/15 text-red-500",
                      )}>
                        {testResults.filter((r) => r.passed).length}/{testResults.length}
                      </span>
                    )}
                    {t === "console" && consoleLogs && consoleLogs.length > 0 && (
                      <span className={cn(
                        "ml-1.5 text-[10px] rounded-full px-1.5",
                        consoleLogs.some((l) => l.level === "error")
                          ? "bg-red-500/15 text-red-500"
                          : consoleLogs.some((l) => l.level === "warn")
                          ? "bg-yellow-500/15 text-yellow-500"
                          : "bg-zinc-500/15 text-zinc-400",
                      )}>
                        {consoleLogs.length}
                      </span>
                    )}
                  </TabsTrigger>
                ))}
              </TabsList>

              {activeTab === "body" && (
                <div className="ml-auto flex items-center gap-1">
                  <div className="flex bg-stone-100 dark:bg-zinc-800 rounded-md p-0.5 gap-0.5">
                    {(["pretty", "raw"] as ResponseBodyView[]).map((v) => (
                      <button
                        key={v}
                        onClick={() => onBodyViewChange(v)}
                        className={cn(
                          "px-2.5 py-0.5 text-xs rounded transition-colors capitalize font-medium",
                          bodyView === v
                            ? "bg-white dark:bg-zinc-700 text-zinc-800 dark:text-zinc-200 shadow-sm"
                            : "text-zinc-500 dark:text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300",
                        )}
                      >
                        {v}
                      </button>
                    ))}
                  </div>
                  <button
                    onClick={() => setWordWrap((v) => !v)}
                    title="Toggle word wrap"
                    className={cn(
                      "h-6 w-6 flex items-center justify-center rounded transition-colors",
                      wordWrap ? "text-orange-400 bg-orange-500/10" : "text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800",
                    )}
                  >
                    <WrapText className="h-3.5 w-3.5" />
                  </button>
                  <button
                    onClick={handleSearchToggle}
                    title="Search in response"
                    className={cn(
                      "h-6 w-6 flex items-center justify-center rounded transition-colors",
                      showSearch ? "text-orange-400 bg-orange-500/10" : "text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800",
                    )}
                  >
                    <Search className="h-3.5 w-3.5" />
                  </button>
                  <button
                    onClick={handleCopy}
                    title="Copy response body"
                    className="h-6 flex items-center gap-1 px-2 rounded text-xs font-medium transition-colors text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800"
                  >
                    {copied ? <Check className="h-3.5 w-3.5 text-emerald-400" /> : <Copy className="h-3.5 w-3.5" />}
                    {copied ? <span className="text-emerald-400">Copied!</span> : <span>Copy</span>}
                  </button>
                </div>
              )}
            </div>

            {activeTab === "body" && showSearch && (
              <div className="shrink-0 flex items-center gap-2 px-3 py-1.5 border-b border-stone-200 dark:border-zinc-800 bg-stone-50/60 dark:bg-zinc-900/60">
                <Search className="h-3.5 w-3.5 text-zinc-500 dark:text-zinc-500 shrink-0" />
                <input
                  ref={searchInputRef}
                  autoFocus
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      goToMatch(e.shiftKey ? -1 : 1);
                    }
                  }}
                  placeholder="Search… (Enter to navigate)"
                  autoCorrect="off"
                  autoCapitalize="none"
                  spellCheck={false}
                  className="flex-1 bg-transparent text-xs text-zinc-800 dark:text-zinc-200 placeholder:text-zinc-400 dark:placeholder:text-zinc-600 focus:outline-none"
                />
                {search && totalMatchCount > 0 && (
                  <div className="flex items-center gap-1 shrink-0">
                    <button onClick={() => goToMatch(-1)} className="text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 px-0.5">↑</button>
                    <button onClick={() => goToMatch(1)} className="text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 px-0.5">↓</button>
                  </div>
                )}
                {search && (
                  <button onClick={() => setSearch("")} className="text-zinc-500 dark:text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 shrink-0">
                    <X className="h-3.5 w-3.5" />
                  </button>
                )}
                {search && (
                  <span className="text-[10px] text-zinc-500 dark:text-zinc-500 shrink-0 tabular-nums">
                    {totalMatchCount > 0 ? `${currentMatchIdx + 1} / ${totalMatchCount}` : "no matches"}
                  </span>
                )}
              </div>
            )}

            <TabsContent value="body" className="flex-1 overflow-hidden m-0 p-0 min-h-0 data-[state=active]:flex data-[state=active]:flex-col">
              {rustReady || bodyText ? (
                <div
                  ref={scrollRef}
                  onScroll={handleScroll}
                  className={cn("flex-1 overflow-auto py-3 text-xs", wordWrap ? "whitespace-pre-wrap break-all" : "whitespace-pre overflow-x-auto")}
                >
                  {/* Top padding for virtual scroll */}
                  {paddingTop > 0 && <div style={{ height: paddingTop }} aria-hidden />}

                  {bodyView === "pretty" && parsedJsonData !== null && !isLarge ? (
                    <JsonTreeViewer data={parsedJsonData} searchTerm={search} />
                  ) : (
                    <HighlightedJson lines={windowLines} search={search} lineOffset={windowStart} plain={bodyView === "raw"} />
                  )}

                  {/* Bottom padding for virtual scroll */}
                  {paddingBottom > 0 && <div style={{ height: paddingBottom }} aria-hidden />}
                </div>
              ) : (
                <div className="flex items-center justify-center h-20 text-xs text-zinc-400 dark:text-zinc-700">
                  <AlignLeft className="h-4 w-4 mr-2" />No response body
                </div>
              )}
              {rustTotalLines > 0 && (
                <div className="shrink-0 flex items-center justify-between px-4 py-1 border-t border-stone-200 dark:border-zinc-800 bg-white/60 dark:bg-zinc-900/60">
                  <span className="text-[10px] text-zinc-400 dark:text-zinc-600 tabular-nums">
                    {windowStart.toLocaleString()}–{(windowStart + windowLines.length).toLocaleString()} of {rustTotalLines.toLocaleString()} lines
                  </span>
                  <span className="text-[10px] text-zinc-400 dark:text-zinc-600">scroll to load more</span>
                </div>
              )}
            </TabsContent>

            <TabsContent value="headers" className="flex-1 overflow-auto m-0 p-0 min-h-0">
              {response.headers && Object.keys(response.headers).length > 0 ? (
                <div className="divide-y divide-stone-200/50 dark:divide-zinc-800/50">
                  {Object.entries(response.headers).map(([key, value]) => (
                    <div key={key} className="flex gap-4 px-4 py-2 hover:bg-stone-50/60 dark:hover:bg-zinc-900/60 group">
                      <span className="text-xs font-medium text-sky-600 dark:text-sky-400/80 shrink-0 w-52 truncate">{key}</span>
                      <span className="text-xs font-mono text-zinc-700 dark:text-zinc-300 break-all select-text flex-1">{value}</span>
                      <button
                        onClick={() => navigator.clipboard.writeText(value)}
                        className="opacity-0 group-hover:opacity-100 shrink-0 text-zinc-400 dark:text-zinc-600 hover:text-zinc-700 dark:hover:text-zinc-300 transition-all"
                      >
                        <Copy className="h-3 w-3" />
                      </button>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="flex items-center justify-center h-20 text-xs text-zinc-400 dark:text-zinc-700">No response headers</div>
              )}
            </TabsContent>

            <TabsContent value="cookies" className="flex-1 overflow-auto m-0 p-0 min-h-0">
              {response.cookies ? (
                <pre className="text-xs p-4 font-mono text-zinc-700 dark:text-zinc-300 whitespace-pre-wrap break-all select-text leading-relaxed">
                  {response.cookies}
                </pre>
              ) : (
                <div className="flex items-center justify-center h-20 text-xs text-zinc-400 dark:text-zinc-700">No cookies</div>
              )}
            </TabsContent>

            <TabsContent value="tests" className="flex-1 overflow-auto m-0 p-0 min-h-0">
              {testResults && testResults.length > 0 ? (
                <div className="flex flex-col divide-y divide-stone-200/50 dark:divide-zinc-800/50">
                  <div className="flex items-center gap-3 px-4 py-2 bg-stone-50/60 dark:bg-zinc-900/60">
                    <span className="text-xs text-emerald-500 font-semibold">
                      {testResults.filter((r) => r.passed).length} passed
                    </span>
                    <span className="text-zinc-400 dark:text-zinc-600 text-xs">/</span>
                    <span className="text-xs text-zinc-500 dark:text-zinc-400">
                      {testResults.length} total
                    </span>
                    {testResults.some((r) => !r.passed) && (
                      <span className="text-xs text-red-500 font-semibold ml-auto">
                        {testResults.filter((r) => !r.passed).length} failed
                      </span>
                    )}
                  </div>
                  {testResults.map((result, i) => (
                    <div
                      key={i}
                      className={cn(
                        "flex items-start gap-3 px-4 py-2.5 hover:bg-stone-50/60 dark:hover:bg-zinc-900/60",
                        !result.passed && "bg-red-500/5",
                      )}
                    >
                      {result.passed
                        ? <CheckCircle2 className="h-3.5 w-3.5 text-emerald-500 shrink-0 mt-0.5" />
                        : <AlertCircle className="h-3.5 w-3.5 text-red-500 shrink-0 mt-0.5" />}
                      <div className="flex-1 min-w-0">
                        <p className={cn(
                          "text-xs font-medium",
                          result.passed ? "text-zinc-700 dark:text-zinc-300" : "text-red-600 dark:text-red-400",
                        )}>
                          {result.name}
                        </p>
                        {result.error && (
                          <p className="text-[11px] font-mono text-red-500/80 mt-0.5 break-all">{result.error}</p>
                        )}
                      </div>
                      {result.duration !== undefined && (
                        <span className="text-[10px] text-zinc-400 dark:text-zinc-600 shrink-0 tabular-nums">
                          {result.duration}ms
                        </span>
                      )}
                    </div>
                  ))}
                </div>
              ) : (
                <div className="flex flex-col items-center justify-center h-32 gap-2 text-zinc-400 dark:text-zinc-700">
                  <CheckCircle2 className="h-6 w-6" />
                  <p className="text-xs">No test results — add tests in the Scripts tab</p>
                </div>
              )}
            </TabsContent>

            <TabsContent value="console" className="flex-1 overflow-auto m-0 p-0 min-h-0">
              {consoleLogs && consoleLogs.length > 0 ? (
                <div className="flex flex-col divide-y divide-stone-200/50 dark:divide-zinc-800/50 font-mono text-xs">
                  {consoleLogs.map((entry, i) => (
                    <div
                      key={i}
                      className={cn(
                        "flex items-start gap-2.5 px-3 py-2 hover:bg-stone-50/60 dark:hover:bg-zinc-900/60",
                        entry.level === "error" && "bg-red-500/5",
                        entry.level === "warn" && "bg-yellow-500/5",
                      )}
                    >
                      <span className={cn(
                        "shrink-0 mt-0.5 text-[10px] font-bold uppercase tracking-wide w-8",
                        entry.level === "log"   && "text-zinc-400",
                        entry.level === "info"  && "text-blue-500",
                        entry.level === "warn"  && "text-yellow-500",
                        entry.level === "error" && "text-red-500",
                      )}>
                        {entry.level}
                      </span>
                      <span className={cn(
                        "flex-1 min-w-0 break-all whitespace-pre-wrap leading-relaxed",
                        entry.level === "error" ? "text-red-500" :
                        entry.level === "warn"  ? "text-yellow-600 dark:text-yellow-400" :
                        entry.level === "info"  ? "text-blue-500" :
                        "text-zinc-700 dark:text-zinc-300",
                      )}>
                        {entry.args.join(" ")}
                      </span>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="flex flex-col items-center justify-center h-32 gap-2 text-zinc-400 dark:text-zinc-700">
                  <p className="text-xs">No output — use <code className="font-mono bg-zinc-100 dark:bg-zinc-800 px-1 rounded">console.log()</code> in a script</p>
                </div>
              )}
            </TabsContent>
          </Tabs>
        )}
      </div>
    </div>
  );
}

function HighlightedJson({ lines, search, lineOffset = 0, plain = false }: {
  lines: string[];
  search: string;
  lineOffset?: number;
  plain?: boolean;
}) {
  const lsearch = search.toLowerCase();
  const colorMap: Record<string, string> = {
    key: "text-sky-600 dark:text-sky-300",
    string: "text-emerald-600 dark:text-emerald-300",
    number: "text-orange-600 dark:text-orange-300",
    literal: "text-purple-600 dark:text-purple-400",
    punct: "text-zinc-500 dark:text-zinc-500",
    ws: "",
    text: "text-zinc-700 dark:text-zinc-300",
  };

  return (
    <code className="block font-mono text-xs leading-[1.7] select-text text-zinc-700 dark:text-zinc-300">
      {lines.map((line, li) => {
        const globalIdx = lineOffset + li;

        if (plain) {
          return (
            <div key={li} data-line={globalIdx} className="flex hover:bg-stone-100 dark:hover:bg-white/[0.02]">
              <span className="select-none shrink-0 w-12 text-right pr-4 text-zinc-400/40 dark:text-zinc-600/60 font-mono border-r border-zinc-200/40 dark:border-zinc-700/40 mr-3">
                {globalIdx + 1}
              </span>
              <span className="flex-1">
                {lsearch && line.toLowerCase().includes(lsearch)
                  ? renderHighlightedText(line, search)
                  : (line || "\u00a0")}
              </span>
            </div>
          );
        }
        const tokens: { type: string; value: string }[] = [];
        const re = /("(?:[^"\\]|\\.)*")\s*:|("(?:[^"\\]|\\.)*")|(true|false|null)|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)|([{}\[\],:])|(\s+)/g;
        let last = 0;
        let m: RegExpExecArray | null;
        while ((m = re.exec(line)) !== null) {
          if (m.index > last) tokens.push({ type: "text", value: line.slice(last, m.index) });
          if (m[1]) tokens.push({ type: "key", value: m[1] + line.slice(m.index + m[1].length, re.lastIndex) });
          else if (m[2]) tokens.push({ type: "string", value: m[2] });
          else if (m[3]) tokens.push({ type: "literal", value: m[3] });
          else if (m[4]) tokens.push({ type: "number", value: m[4] });
          else if (m[5]) tokens.push({ type: "punct", value: m[5] });
          else if (m[6]) tokens.push({ type: "ws", value: m[6] });
          last = re.lastIndex;
        }
        if (last < line.length) tokens.push({ type: "text", value: line.slice(last) });

        const rendered = tokens.map((tok, ti) => (
          <span key={ti} className={colorMap[tok.type] ?? ""}>
            {lsearch && tok.value.toLowerCase().includes(lsearch)
              ? renderHighlightedText(tok.value, search)
              : tok.value}
          </span>
        ));

        return (
          <div
            key={li}
            data-line={globalIdx}
            className="flex hover:bg-stone-100 dark:hover:bg-white/[0.02]"
          >
            <span className="select-none shrink-0 w-12 text-right pr-4 text-zinc-400/40 dark:text-zinc-600/60 font-mono border-r border-zinc-200/40 dark:border-zinc-700/40 mr-3">
              {globalIdx + 1}
            </span>
            <span className="flex-1">{rendered.length > 0 ? rendered : "\u00a0"}</span>
          </div>
        );
      })}
    </code>
  );
}

function JsonTreeNode({ keyName, value, depth, searchTerm, isLast = true }: JsonNodeProps) {
  const isObject = typeof value === "object" && value !== null && !Array.isArray(value);
  const isArray = Array.isArray(value);
  const isCollapsible = isObject || isArray;
  const [collapsed, setCollapsed] = useState(false);

  const normalizedSearch = searchTerm.trim().toLowerCase();
  const forceExpand = normalizedSearch.length > 0;
  const isExpanded = forceExpand || !collapsed;
  const rowHasMatch = normalizedSearch.length > 0 && (
    (keyName?.toLowerCase().includes(normalizedSearch) ?? false) ||
    JSON.stringify(value).toLowerCase().includes(normalizedSearch)
  );

  const renderPrimitive = (val: string | number | boolean | null) => {
    const str = JSON.stringify(val);
    const colorClass =
      typeof val === "string"
        ? "text-emerald-600 dark:text-emerald-300"
        : typeof val === "number"
          ? "text-sky-600 dark:text-orange-300"
          : typeof val === "boolean"
            ? "text-orange-600 dark:text-purple-400"
            : "text-zinc-500 dark:text-zinc-400";

    return <span className={colorClass}>{renderHighlightedText(str, searchTerm)}</span>;
  };

  const keyEl = keyName !== undefined ? (
    <span className="text-zinc-500 dark:text-zinc-400 mr-1 shrink-0">
      "{renderHighlightedText(keyName, searchTerm)}"
      <span className="text-zinc-400 dark:text-zinc-500">: </span>
    </span>
  ) : null;

  if (isCollapsible) {
    const openBrace = isArray ? "[" : "{";
    const closeBrace = isArray ? "]" : "}";
    const items = isArray
      ? (value as JsonValue[]).map((item, index) => ({ key: index, value: item }))
      : Object.entries(value as Record<string, JsonValue>).map(([entryKey, entryValue]) => ({ key: entryKey, value: entryValue }));

    return (
      <div style={{ marginLeft: depth > 0 ? "1rem" : 0 }}>
        <div
          className={cn(
            "flex items-center rounded px-0.5 -mx-0.5",
            !forceExpand && "cursor-pointer hover:bg-stone-100 dark:hover:bg-zinc-800/40",
            rowHasMatch && "bg-yellow-400/10",
          )}
          onClick={() => !forceExpand && setCollapsed((prev) => !prev)}
        >
          {keyEl}
          <span className="text-zinc-500 text-xs mr-1 w-3 inline-block text-center select-none">
            {isExpanded ? "▾" : "▸"}
          </span>
          <span className="text-zinc-700 dark:text-zinc-300">{openBrace}</span>
          {!isExpanded && (
            <span className="text-zinc-500 text-xs ml-1">
              {isArray ? `${items.length} items` : `${items.length} keys`}
            </span>
          )}
          {!isExpanded && <span className="text-zinc-700 dark:text-zinc-300 ml-1">{closeBrace}</span>}
          {!isExpanded && !isLast && <span className="text-zinc-400 dark:text-zinc-600">,</span>}
        </div>
        {isExpanded && (
          <>
            <div>
              {items.map((item, index) => (
                <JsonTreeNode
                  key={String(item.key)}
                  keyName={isArray ? undefined : String(item.key)}
                  value={item.value}
                  depth={depth + 1}
                  searchTerm={searchTerm}
                  isLast={index === items.length - 1}
                />
              ))}
            </div>
            <div>
              <span className="text-zinc-700 dark:text-zinc-300">{closeBrace}</span>
              {!isLast && <span className="text-zinc-400 dark:text-zinc-600">,</span>}
            </div>
          </>
        )}
      </div>
    );
  }

  return (
    <div
      style={{ marginLeft: depth > 0 ? "1rem" : 0 }}
      className={cn("flex items-start", rowHasMatch && "bg-yellow-400/10 rounded")}
    >
      {keyEl}
      {renderPrimitive(value as string | number | boolean | null)}
      {!isLast && <span className="text-zinc-400 dark:text-zinc-600">,</span>}
    </div>
  );
}

function JsonTreeViewer({ data, searchTerm }: { data: JsonValue; searchTerm: string }) {
  return (
    <div className="font-mono text-xs leading-5 p-3 text-zinc-700 dark:text-zinc-300 select-text">
      <JsonTreeNode value={data} depth={0} searchTerm={searchTerm} />
    </div>
  );
}

function StatusBadge({ status }: { status?: number }) {
  if (!status) return null;
  const [bg, label] =
    status < 200 ? ["bg-zinc-500/15 text-zinc-500 dark:text-zinc-400 border-zinc-500/30", "INFO"] :
    status < 300 ? ["bg-emerald-500/15 text-emerald-500 dark:text-emerald-400 border-emerald-500/30", "OK"] :
    status < 400 ? ["bg-sky-500/15 text-sky-500 dark:text-sky-400 border-sky-500/30", "REDIRECT"] :
    status < 500 ? ["bg-orange-500/15 text-orange-500 dark:text-orange-400 border-orange-500/30", "CLIENT ERR"] :
                   ["bg-red-500/15 text-red-500 dark:text-red-400 border-red-500/30", "SERVER ERR"];
  return (
    <div className="flex items-center gap-1.5">
      <span className={cn("inline-flex items-center px-2 py-0.5 rounded-md border text-xs font-bold tabular-nums", bg)}>
        {status}
      </span>
      <span className="text-[10px] text-zinc-400 dark:text-zinc-600 font-medium tracking-wider">{label}</span>
    </div>
  );
}
