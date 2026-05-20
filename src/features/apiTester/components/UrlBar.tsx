import { useRef, useState } from "react";
import { cn } from "@/lib/utils";
import { useTheme } from "@/contexts/ThemeContext";
import { Code, Copy, Loader2, Save, Send } from "lucide-react";
import Editor from "@monaco-editor/react";

const METHOD_COLORS: Record<string, string> = {
  GET: "text-emerald-500 dark:text-emerald-400 border-emerald-500/30 bg-emerald-500/10",
  POST: "text-orange-500 dark:text-orange-400 border-orange-500/30 bg-orange-500/10",
  PUT: "text-blue-500 dark:text-blue-400 border-blue-500/30 bg-blue-500/10",
  PATCH: "text-teal-500 dark:text-teal-400 border-teal-500/30 bg-teal-500/10",
  DELETE: "text-red-500 dark:text-red-400 border-red-500/30 bg-red-500/10",
  HEAD: "text-purple-500 dark:text-purple-400 border-purple-500/30 bg-purple-500/10",
  OPTIONS: "text-sky-500 dark:text-sky-400 border-sky-500/30 bg-sky-500/10",
};

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

type UrlSyncParam = {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
};

interface UrlBarProps {
  method: string;
  onMethodChange: (method: string) => void;
  urlInput: string;
  onUrlChange: (url: string) => void;
  onUrlSync?: (cleanUrl: string, params: UrlSyncParam[]) => void;
  onCurlImport: (curlCommand: string) => void;
  isLoading: boolean;
  onSendRequest: () => void;
  onSaveRequest: () => void;
  generatedCurl?: string;
  generatedJs?: string;
  onGenerateCode?: () => void;
  disabled?: boolean;
}

function isCurlCommand(text: string): boolean {
  return /^\s*curl\s/i.test(text);
}

function parseUrlSyncPayload(rawValue: string): { cleanUrl: string; params: UrlSyncParam[] } | null {
  const trimmed = rawValue.trim();
  if (!trimmed || !trimmed.includes("?")) return null;

  try {
    const fullUrl = /^https?:\/\//i.test(trimmed)
      ? trimmed
      : `http://dummy.com${trimmed.startsWith("/") ? trimmed : `/${trimmed}`}`;
    const parsed = new URL(fullUrl);
    const params = Array.from(parsed.searchParams.entries()).map(([key, value]) => ({
      id: crypto.randomUUID(),
      key,
      value,
      enabled: true,
    }));

    if (params.length === 0) return null;

    return {
      cleanUrl: trimmed.split("?")[0],
      params,
    };
  } catch {
    return null;
  }
}

export function UrlBar({
  method,
  onMethodChange,
  urlInput,
  onUrlChange,
  onUrlSync,
  onCurlImport,
  isLoading,
  onSendRequest,
  onSaveRequest,
  generatedCurl = "",
  generatedJs = "",
  onGenerateCode,
  disabled = false,
}: UrlBarProps) {
  const { resolved } = useTheme();
  const [codeOpen, setCodeOpen] = useState(false);
  const [codeTab, setCodeTab] = useState<"curl" | "js">("curl");
  const [copied, setCopied] = useState(false);
  const selectRef = useRef<HTMLSelectElement>(null);

  const methodStyle =
    METHOD_COLORS[method] ??
    "text-zinc-500 dark:text-zinc-400 border-stone-300 dark:border-zinc-700 bg-white dark:bg-zinc-800";

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    if (isCurlCommand(val)) {
      onCurlImport(val.trim());
      return;
    }

    // Detect paste via InputEvent.inputType (works in Tauri WebViews where
    // clipboardData.getData() may return empty from the paste event itself)
    const inputType = (e.nativeEvent as InputEvent).inputType;
    const isPaste = inputType === "insertFromPaste" || inputType === "insertFromPasteAsQuotation";

    if (isPaste && val.includes("?") && onUrlSync) {
      const syncPayload = parseUrlSyncPayload(val);
      if (syncPayload) {
        onUrlSync(syncPayload.cleanUrl, syncPayload.params);
        return;
      }
    }

    onUrlChange(val);
  };

  const handlePaste = (e: React.ClipboardEvent<HTMLInputElement>) => {
    const pasted = e.clipboardData.getData("text");
    if (isCurlCommand(pasted)) {
      e.preventDefault();
      onCurlImport(pasted.trim());
      return;
    }

    const syncPayload = parseUrlSyncPayload(pasted);
    if (syncPayload && onUrlSync) {
      e.preventDefault();
      onUrlSync(syncPayload.cleanUrl, syncPayload.params);
    }
  };

  const handleBlur = () => {
    const syncPayload = parseUrlSyncPayload(urlInput);
    if (syncPayload && onUrlSync) {
      onUrlSync(syncPayload.cleanUrl, syncPayload.params);
    }
  };

  const handleOpenCode = () => {
    onGenerateCode?.();
    setCodeOpen(true);
  };

  const copy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const currentCode = codeTab === "curl" ? generatedCurl : generatedJs;

  return (
    <>
      <div className="flex items-center gap-2 px-3 py-2.5 border-b border-stone-200 dark:border-zinc-800 bg-white/80 dark:bg-zinc-900/80 backdrop-blur-sm">
        <div className="relative shrink-0">
          <select
            ref={selectRef}
            value={method}
            onChange={(e) => onMethodChange(e.target.value)}
            className={cn(
              "appearance-none border rounded-lg px-3 py-1.5 pr-7 text-sm font-bold cursor-pointer",
              "focus:outline-none focus:ring-2 focus:ring-orange-500/40 transition-colors min-w-[88px]",
              methodStyle,
            )}
          >
            {METHODS.map((m) => (
              <option key={m} value={m} className="bg-white text-zinc-900 dark:bg-zinc-900 dark:text-zinc-100 font-bold">
                {m}
              </option>
            ))}
          </select>
          <span className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-current opacity-60 text-xs">
            ▾
          </span>
        </div>

        <input
          type="text"
          value={urlInput}
          onChange={handleChange}
          onPaste={handlePaste}
          onBlur={handleBlur}
          placeholder="https://api.example.com/endpoint  —  or paste a cURL command"
          disabled={disabled}
          className={cn(
            "flex-1 bg-white dark:bg-zinc-800/80 border border-stone-300 dark:border-zinc-700 rounded-lg px-4 py-1.5 text-sm",
            "text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-400 dark:placeholder:text-zinc-600",
            "focus:outline-none focus:ring-2 focus:ring-orange-500/30 focus:border-orange-500/50",
            "font-mono transition-all",
            disabled && "opacity-40 cursor-not-allowed",
          )}
        />

        <button
          onClick={handleOpenCode}
          disabled={disabled}
          title="Generate Code"
          className={cn(
            "shrink-0 h-8 w-8 flex items-center justify-center rounded-lg border transition-colors",
            "border-stone-300 dark:border-zinc-700 text-zinc-500 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100 hover:bg-stone-100 dark:hover:bg-zinc-700 hover:border-stone-400 dark:hover:border-zinc-600",
            disabled && "opacity-40 cursor-not-allowed",
          )}
        >
          <Code className="h-3.5 w-3.5" />
        </button>

        <button
          onClick={onSaveRequest}
          disabled={disabled || isLoading}
          title="Save request"
          className={cn(
            "shrink-0 h-8 w-8 flex items-center justify-center rounded-lg border transition-colors",
            "border-stone-300 dark:border-zinc-700 text-zinc-500 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100 hover:bg-stone-100 dark:hover:bg-zinc-700 hover:border-stone-400 dark:hover:border-zinc-600",
            (disabled || isLoading) && "opacity-40 cursor-not-allowed",
          )}
        >
          <Save className="h-3.5 w-3.5" />
        </button>

        <button
          onClick={onSendRequest}
          disabled={disabled || isLoading || !urlInput.trim()}
          className={cn(
            "shrink-0 flex items-center gap-1.5 h-8 px-4 rounded-lg text-sm font-semibold transition-all",
            "bg-orange-600 hover:bg-orange-500 text-white shadow-lg shadow-orange-900/20",
            "disabled:opacity-40 disabled:cursor-not-allowed disabled:shadow-none",
          )}
        >
          {isLoading ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <>
              <Send className="h-3.5 w-3.5" />
              Send
            </>
          )}
        </button>
      </div>

      {codeOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={() => setCodeOpen(false)} />
          <div
            className="relative bg-white dark:bg-zinc-900 border border-stone-300 dark:border-zinc-700 rounded-2xl shadow-2xl w-[720px] max-w-[95vw] overflow-hidden flex flex-col"
            style={{ height: "min(80vh, 600px)" }}
          >
            <div className="flex items-center justify-between px-5 py-3.5 border-b border-stone-200 dark:border-zinc-800 shrink-0">
              <div className="flex items-center gap-2">
                <Code className="h-4 w-4 text-orange-400" />
                <span className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">Generated Code</span>
              </div>
              <div className="flex items-center gap-2">
                <div className="flex bg-stone-100 dark:bg-zinc-800 rounded-lg p-0.5 gap-0.5">
                  {(["curl", "js"] as const).map((t) => (
                    <button
                      key={t}
                      onClick={() => setCodeTab(t)}
                      className={cn(
                        "px-3 py-1 rounded-md text-xs font-medium transition-colors",
                        codeTab === t
                          ? "bg-white dark:bg-zinc-700 text-zinc-900 dark:text-zinc-100 shadow-sm"
                          : "text-zinc-500 dark:text-zinc-400 hover:text-zinc-700 dark:hover:text-zinc-300",
                      )}
                    >
                      {t === "curl" ? "cURL" : "JavaScript"}
                    </button>
                  ))}
                </div>
                <button
                  onClick={() => copy(currentCode)}
                  className="flex items-center gap-1.5 px-3 py-1 bg-white dark:bg-zinc-800 hover:bg-stone-100 dark:hover:bg-zinc-700 border border-stone-300 dark:border-zinc-700 rounded-lg text-xs text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200 transition-colors"
                >
                  <Copy className="h-3 w-3" />
                  {copied ? "Copied!" : "Copy"}
                </button>
                <button
                  onClick={() => setCodeOpen(false)}
                  className="p-1.5 text-zinc-500 dark:text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800 rounded-lg transition-colors text-lg leading-none"
                >
                  ×
                </button>
              </div>
            </div>
            <div className="flex-1 min-h-0">
              <Editor
                height="100%"
                language={codeTab === "curl" ? "shell" : "javascript"}
                value={currentCode || "// Send a request first to generate code"}
                theme={resolved === "dark" ? "vs-dark" : "vs"}
                options={{
                  readOnly: true,
                  minimap: { enabled: false },
                  scrollBeyondLastLine: false,
                  fontSize: 13,
                  lineNumbers: "off",
                  renderLineHighlight: "none",
                  padding: { top: 16, bottom: 16 },
                }}
              />
            </div>
          </div>
        </div>
      )}
    </>
  );
}
