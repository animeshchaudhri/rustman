import { useMemo, useState } from "react";
import { ApiResponse, ResponseBodyView, ResponseTabType } from "../types";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Loader2, Clock, Database, Zap, XCircle,
  ArrowUpDown, Copy, Check, Search, X, WrapText, AlignLeft,
} from "lucide-react";
import { cn } from "@/lib/utils";

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
}

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function prettyBody(data: unknown): string {
  if (typeof data === "object" && data !== null) {
    return JSON.stringify(data, null, 2);
  }
  if (typeof data === "string") {
    try { return JSON.stringify(JSON.parse(data), null, 2); } catch {   }
  }
  return String(data ?? "");
}

function rawBody(data: unknown): string {
  if (typeof data === "object" && data !== null) return JSON.stringify(data);
  return String(data ?? "");
}

function HighlightedJson({ text, search }: { text: string; search: string }) {
  const lines = text.split("\n");
  const lsearch = search.toLowerCase();

  return (
    <code className="block font-mono text-xs leading-[1.7] select-text">
      {lines.map((line, li) => {
        
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

        const colorMap: Record<string, string> = {
          key: "text-sky-300",
          string: "text-emerald-300",
          number: "text-orange-300",
          literal: "text-purple-400",
          punct: "text-zinc-500",
          ws: "",
          text: "text-zinc-300",
        };

        const rendered = tokens.map((tok, ti) => (
          <span key={ti} className={colorMap[tok.type] ?? ""}>
            {lsearch && tok.value.toLowerCase().includes(lsearch)
              ? tok.value.split(new RegExp(`(${search.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`, "gi")).map((part, pi) =>
                  part.toLowerCase() === lsearch
                    ? <mark key={pi} className="bg-orange-500/40 text-orange-200 rounded-sm">{part}</mark>
                    : part
                )
              : tok.value}
          </span>
        ));

        return <div key={li} className="hover:bg-white/[0.02] px-4">{rendered || " "}</div>;
      })}
    </code>
  );
}

function StatusBadge({ status }: { status?: number }) {
  if (!status) return null;
  const [bg, label] =
    status < 200 ? ["bg-zinc-500/15 text-zinc-400 border-zinc-500/30", "INFO"] :
    status < 300 ? ["bg-emerald-500/15 text-emerald-400 border-emerald-500/30", "OK"] :
    status < 400 ? ["bg-sky-500/15 text-sky-400 border-sky-500/30", "REDIRECT"] :
    status < 500 ? ["bg-orange-500/15 text-orange-400 border-orange-500/30", "CLIENT ERR"] :
                   ["bg-red-500/15 text-red-400 border-red-500/30", "SERVER ERR"];
  return (
    <div className="flex items-center gap-1.5">
      <span className={cn("inline-flex items-center px-2 py-0.5 rounded-md border text-xs font-bold tabular-nums", bg)}>
        {status}
      </span>
      <span className="text-[10px] text-zinc-600 font-medium tracking-wider">{label}</span>
    </div>
  );
}

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
}: ResponseViewerProps) {
  const [copied, setCopied] = useState(false);
  const [search, setSearch] = useState("");
  const [showSearch, setShowSearch] = useState(false);
  const [wordWrap, setWordWrap] = useState(true);

  const bodyText = useMemo(() => {
    if (!response?.data) return "";
    return bodyView === "pretty" ? prettyBody(response.data) : rawBody(response.data);
  }, [response?.data, bodyView]);

  const handleCopy = () => {
    navigator.clipboard.writeText(bodyText);
    setCopied(true);
    setTimeout(() => setCopied(false), 1800);
  };

  const handleSearchToggle = () => {
    setShowSearch((v) => !v);
    setSearch("");
  };

  return (
    <div className="flex flex-col bg-zinc-950 h-full overflow-hidden">
      { }
      <div className="flex items-center justify-between px-4 py-2 border-b border-zinc-800 bg-zinc-900/80 shrink-0 gap-3">
        <div className="flex items-center gap-3 min-w-0">
          {isLoading ? (
            <div className="flex items-center gap-2 text-zinc-400">
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              <span className="text-xs font-medium">Sending…</span>
            </div>
          ) : response ? (
            <StatusBadge status={response.status} />
          ) : error ? (
            <div className="flex items-center gap-2 text-red-400">
              <XCircle className="h-3.5 w-3.5" />
              <span className="text-xs font-medium">Error</span>
            </div>
          ) : (
            <span className="text-xs text-zinc-700">Response</span>
          )}
        </div>

        {response && !isLoading && (
          <div className="flex items-center gap-3 shrink-0">
            {responseTime !== null && (
              <div className="flex items-center gap-1 text-xs">
                <Clock className="h-3 w-3 text-zinc-600" />
                <span className={cn("tabular-nums font-semibold",
                  responseTime < 300 ? "text-emerald-400" :
                  responseTime < 1000 ? "text-yellow-400" : "text-red-400")}>
                  {responseTime < 1000 ? `${responseTime}ms` : `${(responseTime/1000).toFixed(2)}s`}
                </span>
              </div>
            )}
            {responseSize !== null && (
              <div className="flex items-center gap-1 text-xs">
                <Database className="h-3 w-3 text-zinc-600" />
                <span className="tabular-nums text-zinc-400">{formatSize(responseSize)}</span>
              </div>
            )}
          </div>
        )}
      </div>

      { }
      <div className="flex-1 overflow-hidden flex flex-col min-h-0">
        {isLoading && (
          <div className="flex-1 flex items-center justify-center">
            <div className="flex flex-col items-center gap-3 text-zinc-700">
              <div className="relative">
                <Zap className="h-9 w-9" />
                <Loader2 className="h-4 w-4 animate-spin text-orange-500 absolute -bottom-1 -right-1" />
              </div>
              <span className="text-xs text-zinc-600">Waiting for response…</span>
            </div>
          </div>
        )}

        {!isLoading && error && (
          <div className="flex-1 flex flex-col items-center justify-center gap-3 p-6">
            <XCircle className="h-10 w-10 text-red-500/50" />
            <p className="text-sm font-semibold text-red-400">Request Failed</p>
            <pre className="select-text text-xs text-red-300/80 font-mono bg-zinc-900 border border-red-900/40 rounded-xl px-5 py-3 max-w-lg text-center whitespace-pre-wrap break-all">{error}</pre>
          </div>
        )}

        {!isLoading && !error && !response && (
          <div className="flex-1 flex flex-col items-center justify-center gap-2 text-zinc-800">
            <ArrowUpDown className="h-9 w-9" />
            <p className="text-xs">Hit Send to see the response</p>
          </div>
        )}

        {!isLoading && !error && response && (
          <Tabs
            value={activeTab}
            onValueChange={onTabChange as (v: string) => void}
            className="flex flex-col h-full min-h-0"
          >
            { }
            <div className="shrink-0 flex items-center border-b border-zinc-800 bg-zinc-900 px-2 gap-1">
              <TabsList className="bg-transparent rounded-none h-8 gap-0 p-0">
                {(["body", "headers", "cookies"] as ResponseTabType[]).map((t) => (
                  <TabsTrigger
                    key={t}
                    value={t}
                    className={cn(
                      "text-xs px-3 py-0 h-8 rounded-none capitalize border-b-2 border-transparent",
                      "data-[state=active]:border-orange-500 data-[state=active]:bg-transparent data-[state=active]:text-orange-400",
                      "text-zinc-500 hover:text-zinc-300 transition-colors",
                    )}
                  >
                    {t}
                    {t === "headers" && response.headers && (
                      <span className="ml-1.5 text-[10px] bg-zinc-800 text-zinc-500 rounded-full px-1.5">
                        {Object.keys(response.headers).length}
                      </span>
                    )}
                  </TabsTrigger>
                ))}
              </TabsList>

              { }
              {activeTab === "body" && (
                <div className="ml-auto flex items-center gap-1">
                  { }
                  <div className="flex bg-zinc-800 rounded-md p-0.5 gap-0.5">
                    {(["pretty", "raw"] as ResponseBodyView[]).map((v) => (
                      <button
                        key={v}
                        onClick={() => onBodyViewChange(v)}
                        className={cn(
                          "px-2.5 py-0.5 text-xs rounded transition-colors capitalize font-medium",
                          bodyView === v ? "bg-zinc-700 text-zinc-200" : "text-zinc-500 hover:text-zinc-300",
                        )}
                      >
                        {v}
                      </button>
                    ))}
                  </div>

                  { }
                  <button
                    onClick={() => setWordWrap(v => !v)}
                    title="Toggle word wrap"
                    className={cn(
                      "h-6 w-6 flex items-center justify-center rounded transition-colors",
                      wordWrap ? "text-orange-400 bg-orange-500/10" : "text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800",
                    )}
                  >
                    <WrapText className="h-3.5 w-3.5" />
                  </button>

                  { }
                  <button
                    onClick={handleSearchToggle}
                    title="Search in response"
                    className={cn(
                      "h-6 w-6 flex items-center justify-center rounded transition-colors",
                      showSearch ? "text-orange-400 bg-orange-500/10" : "text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800",
                    )}
                  >
                    <Search className="h-3.5 w-3.5" />
                  </button>

                  { }
                  <button
                    onClick={handleCopy}
                    title="Copy response body"
                    className="h-6 flex items-center gap-1 px-2 rounded text-xs font-medium transition-colors text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800"
                  >
                    {copied ? <Check className="h-3.5 w-3.5 text-emerald-400" /> : <Copy className="h-3.5 w-3.5" />}
                    {copied ? <span className="text-emerald-400">Copied!</span> : <span>Copy</span>}
                  </button>
                </div>
              )}
            </div>

            { }
            {activeTab === "body" && showSearch && (
              <div className="shrink-0 flex items-center gap-2 px-3 py-1.5 border-b border-zinc-800 bg-zinc-900/60">
                <Search className="h-3.5 w-3.5 text-zinc-500 shrink-0" />
                <input
                  autoFocus
                  value={search}
                  onChange={e => setSearch(e.target.value)}
                  placeholder="Search in response…"
                  className="flex-1 bg-transparent text-xs text-zinc-200 placeholder:text-zinc-600 focus:outline-none"
                />
                {search && (
                  <button onClick={() => setSearch("")} className="text-zinc-500 hover:text-zinc-200">
                    <X className="h-3.5 w-3.5" />
                  </button>
                )}
                {search && (
                  <span className="text-[10px] text-zinc-500 shrink-0">
                    {(bodyText.split(search).length - 1)} match{bodyText.split(search).length !== 2 ? "es" : ""}
                  </span>
                )}
              </div>
            )}

            { }
            <TabsContent value="body" className="flex-1 overflow-auto m-0 p-0 min-h-0 data-[state=active]:flex data-[state=active]:flex-col">
              {bodyText ? (
                <div className={cn("py-3 text-xs", wordWrap ? "whitespace-pre-wrap break-all" : "whitespace-pre overflow-x-auto")}>
                  {bodyView === "pretty" ? (
                    <HighlightedJson text={bodyText} search={search} />
                  ) : (
                    <pre className="px-4 select-text font-mono text-zinc-300 text-xs">{bodyText}</pre>
                  )}
                </div>
              ) : (
                <div className="flex items-center justify-center h-20 text-xs text-zinc-700">
                  <AlignLeft className="h-4 w-4 mr-2" />No response body
                </div>
              )}
            </TabsContent>

            { }
            <TabsContent value="headers" className="flex-1 overflow-auto m-0 p-0 min-h-0">
              {response.headers && Object.keys(response.headers).length > 0 ? (
                <div className="divide-y divide-zinc-800/50">
                  {Object.entries(response.headers).map(([key, value]) => (
                    <div key={key} className="flex gap-4 px-4 py-2 hover:bg-zinc-900/60 group">
                      <span className="text-xs font-medium text-sky-400/80 shrink-0 w-52 truncate">{key}</span>
                      <span className="text-xs font-mono text-zinc-300 break-all select-text flex-1">{value}</span>
                      <button
                        onClick={() => navigator.clipboard.writeText(value)}
                        className="opacity-0 group-hover:opacity-100 shrink-0 text-zinc-600 hover:text-zinc-300 transition-all"
                      >
                        <Copy className="h-3 w-3" />
                      </button>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="flex items-center justify-center h-20 text-xs text-zinc-700">No response headers</div>
              )}
            </TabsContent>

            { }
            <TabsContent value="cookies" className="flex-1 overflow-auto m-0 p-0 min-h-0">
              {response.cookies ? (
                <pre className="text-xs p-4 font-mono text-zinc-300 whitespace-pre-wrap break-all select-text leading-relaxed">
                  {response.cookies}
                </pre>
              ) : (
                <div className="flex items-center justify-center h-20 text-xs text-zinc-700">No cookies</div>
              )}
            </TabsContent>
          </Tabs>
        )}
      </div>
    </div>
  );
}
