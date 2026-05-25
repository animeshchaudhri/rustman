import { useCallback, useEffect, useRef, startTransition, useState } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Authentication,
  CollectionsSidebar,
  CommandPalette,
  EnvironmentsSidebar,
  HistorySidebar,
  RequestBody,
  RequestHeaders,
  RequestParams,
  ResponseViewer,
  SaveRequestDialog,
  ScriptsTab,
  UrlBar,
} from "./components";
import { useCollections } from "./hooks/useCollections";
import { useHistory } from "./hooks/useHistory";
import {
  buildTabName,
  createRequestTab,
  savedRequestToRequestTab,
  useRequestTabs,
} from "./hooks/useRequestTabs";
import {
  enhancedFetch,
  parseCurlCommand,
  parseCookies,
  extractAccessTokenFromCookies,
  parseJwt,
  generateJsCode,
  replaceVariables,
  type ProxyFormField,
} from "./utils";
import {
  clearSession,
  deleteEnvironment,
  getEnvironments,
  getSession,
  saveEnvironment,
  saveSession,
  bodyClearPrefix,
  parseCurl,
  generateCurlCmd,
  type GenerateCurlInput,
} from "@/lib/db";
import type {
  ApiKeyLocation,
  ApiResponse,
  AppEnvironment,
  AuthType,
  Collection,
  CookieType,
  FormDataField,
  HeaderType,
  HistoryEntry,
  PanelLayout,
  ConsoleEntry,
  ParsedCurl,
  RequestBodyType,
  RequestTabType,
  ResponseBodyView,
  ResponseTabType,
  SavedRequest,
  TestResult,
} from "./types";
import { runPreRequestScript, runTestScript } from "./utils/scriptRunner";
import { cn } from "@/lib/utils";
import { useTheme, type Theme, BRAND_THEMES } from "@/contexts/ThemeContext";
import { METHOD_BADGE_DOT as METHOD_BADGE } from "./constants";
import {
  BookOpen,
  Clock,
  Globe,
  Plus,
  Settings,
  X,
  Github,
  Globe2,
  Mail,
  Zap,
  Database,
  Layers,
  Heart,
} from "lucide-react";

type SidePanel = "collections" | "history" | "environments" | "settings" | null;

interface TabResponse {
  response: ApiResponse | null;
  responseTime: number | null;
  responseSize: number | null;
  isLoading: boolean;
  error: string | null;
  testResults: TestResult[];
  consoleLogs: ConsoleEntry[];
}

function formatRelativeTime(ts: number): string {
  const d = Date.now() - ts;
  if (d < 60000) return "just now";
  if (d < 3600000) return `${Math.floor(d / 60000)}m ago`;
  if (d < 86400000) return `${Math.floor(d / 3600000)}h ago`;
  return `${Math.floor(d / 86400000)}d ago`;
}

function AboutPanel() {
  const { theme, setTheme, brand, setBrand } = useTheme();

  const stack = [
    { icon: Zap, label: "Tauri v2", desc: "Desktop runtime" },
    { icon: Layers, label: "React 18", desc: "UI framework" },
    { icon: Database, label: "SQLite", desc: "via rusqlite (bundled)" },
  ];

  const links = [
    { icon: Github, label: "GitHub", href: "https://github.com/animeshchaudhri", color: "text-zinc-700 dark:text-zinc-300" },
    { icon: Globe2, label: "animesh.us", href: "https://animesh.us", color: "text-sky-500 dark:text-sky-400" },
    { icon: Mail, label: "ac04@duck.com", href: "mailto:ac04@duck.com", color: "text-brand-500 dark:text-brand-400" },
  ];

  return (
    <div className="flex flex-col h-full overflow-y-auto">
      <div className="px-4 py-3 border-b border-stone-200 dark:border-zinc-800">
        <p className="text-[10px] uppercase tracking-widest text-zinc-500 dark:text-zinc-600 font-semibold mb-0.5">About</p>
        <p className="text-sm font-bold text-zinc-900 dark:text-zinc-100">Rustman</p>
      </div>

      <div className="flex flex-col gap-5 p-4">
        <div className="flex flex-col items-center gap-3 py-5 bg-stone-100 dark:bg-zinc-800/40 rounded-xl border border-stone-200 dark:border-zinc-800">
          <img src="/rustman-logo.svg" alt="Rustman" className="w-16 h-16" />
          <div className="text-center">
            <p className="text-base font-bold text-zinc-900 dark:text-zinc-100 tracking-tight">
              RUST<span className="text-brand-400">MAN</span>
            </p>
            <p className="text-[10px] text-zinc-500 dark:text-zinc-600 tracking-widest mt-0.5">API TESTING TOOL</p>
          </div>
          <span className="text-[10px] bg-stone-200 dark:bg-zinc-700 text-zinc-500 dark:text-zinc-400 px-2 py-0.5 rounded-full">v0.2.0</span>
        </div>

        <div className="bg-stone-100 dark:bg-zinc-800/40 rounded-xl border border-stone-200 dark:border-zinc-800 overflow-hidden">
          <div className="px-4 py-2.5 border-b border-stone-200 dark:border-zinc-800">
            <p className="text-[10px] uppercase tracking-widest text-zinc-500 dark:text-zinc-600 font-semibold">Made by</p>
          </div>
          <div className="px-4 py-3 flex items-center gap-3">
            <img
              src="https://github.com/animeshchaudhri.png"
              alt="Animesh Chaudhri"
              className="w-10 h-10 rounded-full border-2 border-brand-500/30"
              onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
            />
            <div>
              <p className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">Animesh Chaudhri</p>
              <p className="text-[11px] text-zinc-500 dark:text-zinc-400">Full-Stack · Rust · AI · Distributed Systems</p>
              <p className="text-[10px] text-zinc-500 dark:text-zinc-600 mt-0.5">Pune, India</p>
            </div>
          </div>

          <div className="border-t border-stone-200 dark:border-zinc-800 divide-y divide-stone-200/60 dark:divide-zinc-800/60">
            {links.map(({ icon: Icon, label, href, color }) => (
              <a
                key={href}
                href={href}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-3 px-4 py-2.5 hover:bg-stone-200/60 dark:hover:bg-zinc-800/60 transition-colors group"
              >
                <Icon className={cn("h-3.5 w-3.5 shrink-0", color)} />
                <span className="text-xs text-zinc-500 dark:text-zinc-400 group-hover:text-zinc-700 dark:group-hover:text-zinc-200 transition-colors">{label}</span>
              </a>
            ))}
          </div>
        </div>

        <div className="bg-stone-100 dark:bg-zinc-800/40 rounded-xl border border-stone-200 dark:border-zinc-800 overflow-hidden">
          <div className="px-4 py-2.5 border-b border-stone-200 dark:border-zinc-800">
            <p className="text-[10px] uppercase tracking-widest text-zinc-500 dark:text-zinc-600 font-semibold">Built with</p>
          </div>
          <div className="divide-y divide-stone-200/60 dark:divide-zinc-800/60">
            {stack.map(({ icon: Icon, label, desc }) => (
              <div key={label} className="flex items-center gap-3 px-4 py-2.5">
                <Icon className="h-3.5 w-3.5 text-brand-400/70 shrink-0" />
                <div>
                  <p className="text-xs font-medium text-zinc-700 dark:text-zinc-300">{label}</p>
                  <p className="text-[10px] text-zinc-500 dark:text-zinc-600">{desc}</p>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="bg-stone-100 dark:bg-zinc-800/40 rounded-xl border border-stone-200 dark:border-zinc-800 overflow-hidden">
          <div className="px-4 py-2.5 border-b border-stone-200 dark:border-zinc-800">
            <p className="text-[10px] uppercase tracking-widest text-zinc-500 dark:text-zinc-600 font-semibold">Mode</p>
          </div>
          <div className="flex p-2 gap-1.5">
            {(["dark", "light", "system"] as Theme[]).map((t) => (
              <button
                key={t}
                onClick={() => setTheme(t)}
                className={cn(
                  "flex-1 py-2 rounded-lg text-xs font-medium capitalize transition-colors border",
                  theme === t
                    ? "bg-brand-600/20 text-brand-400 border-brand-500/30"
                    : "text-zinc-500 dark:text-zinc-500 border-stone-300 dark:border-zinc-700 hover:text-zinc-700 dark:hover:text-zinc-300 hover:bg-stone-200 dark:hover:bg-zinc-800",
                )}
              >
                {t === "dark" ? "Dark" : t === "light" ? "Light" : "System"}
              </button>
            ))}
          </div>
        </div>

        <div className="bg-stone-100 dark:bg-zinc-800/40 rounded-xl border border-stone-200 dark:border-zinc-800 overflow-hidden">
          <div className="px-4 py-2.5 border-b border-stone-200 dark:border-zinc-800">
            <p className="text-[10px] uppercase tracking-widest text-zinc-500 dark:text-zinc-600 font-semibold">Accent</p>
          </div>
          <div className="flex flex-wrap gap-2 p-3">
            {BRAND_THEMES.map(({ value, label, color }) => (
              <button
                key={value}
                onClick={() => setBrand(value)}
                title={label}
                className={cn(
                  "flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors border",
                  brand === value
                    ? "border-brand-500/50 bg-brand-600/10 text-zinc-900 dark:text-zinc-100"
                    : "border-stone-300 dark:border-zinc-700 text-zinc-500 dark:text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 hover:bg-stone-200 dark:hover:bg-zinc-800",
                )}
              >
                <span className="w-3 h-3 rounded-full shrink-0" style={{ backgroundColor: color }} />
                {label}
              </button>
            ))}
          </div>
        </div>

        <div className="flex items-center justify-center gap-1.5 text-[10px] text-zinc-400 dark:text-zinc-700 pb-2">
          <span>© 2026 Animesh Chaudhri</span>
          <Heart className="h-2.5 w-2.5 text-red-500/60 fill-red-500/60" />
          <span>MIT License</span>
        </div>
      </div>
    </div>
  );
}

export default function ApiTester() {
  const { resolved } = useTheme();
  
  const [panel, setPanel] = useState<SidePanel>("collections");
  const [splitRatio, setSplitRatio] = useState(0.45);
  const [layout, setLayout] = useState<PanelLayout>(
    () => (localStorage.getItem("rustman-layout") as PanelLayout) || "vertical",
  );
  const layoutRef = useRef<PanelLayout>(layout);
  const isDragging = useRef(false);
  const splitContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    layoutRef.current = layout;
    localStorage.setItem("rustman-layout", layout);
  }, [layout]);

  
  const {
    tabs,
    activeTabId,
    activeTab,
    addTab,
    closeTab,
    duplicateTab,
    setActiveTab,
    updateActiveTab,
    restoreTabs,
    reorderTabs,
  } = useRequestTabs();

  const handleCloseTab = useCallback((tabId: string) => {
    bodyClearPrefix(tabId).catch(console.error);
    closeTab(tabId);
  }, [closeTab]);

  
  const [responses, setResponses] = useState<Record<string, TabResponse>>({});
  const activeResponse = responses[activeTabId] ?? {
    response: null,
    responseTime: null,
    responseSize: null,
    isLoading: false,
    error: null,
    testResults: [],
  };

  const setTabResponse = useCallback(
    (tabId: string, patch: Partial<TabResponse>) => {
      setResponses((prev) => ({
        ...prev,
        [tabId]: { ...{ response: null, responseTime: null, responseSize: null, isLoading: false, error: null, testResults: [], consoleLogs: [] }, ...(prev[tabId] ?? {}), ...patch },
      }));
    },
    [],
  );

  
  const [activeRequestTab, setActiveRequestTab] = useState<RequestTabType>("params");
  const [activeResponseTab, setActiveResponseTab] = useState<ResponseTabType>("body");
  const [responseBodyView, setResponseBodyView] = useState<ResponseBodyView>("pretty");

  
  const [generatedCurl, setGeneratedCurl] = useState("");
  const [generatedJs, setGeneratedJs] = useState("");

  
  const [saveDialogOpen, setSaveDialogOpen] = useState(false);

  const [paletteOpen, setPaletteOpen] = useState(false);

  const [pendingSession, setPendingSession] = useState<{
    tabs: ReturnType<typeof createRequestTab>[];
    activeTabId: string;
    savedAt: number;
  } | null>(null);
  const sessionHydrated = useRef(false);

  const abortRef = useRef<(() => void) | null>(null);

  const dragTabIdRef = useRef<string | null>(null);
  const [dragOverTabId, setDragOverTabId] = useState<string | null>(null);

  
  const {
    collections,
    requests: collectionRequests,
    createCollection,
    deleteCollection,
    saveRequest: saveReqToCollection,
    deleteRequest: deleteReqFromCollection,
    renameCollection,
    renameRequest,
  } = useCollections();

  
  const { history, addToHistory, clearHistory } = useHistory();

  
  const [environments, setEnvironments] = useState<AppEnvironment[]>([]);
  const [activeEnvId, setActiveEnvId] = useState<string | null>(null);
  const [envEnabled, setEnvEnabled] = useState(
    () => localStorage.getItem("envEnabled") === "true",
  );

  useEffect(() => {
    getEnvironments().then(setEnvironments).catch(console.error);
  }, []);

  useEffect(() => {
    getSession()
      .then((raw) => {
        if (!raw) { sessionHydrated.current = true; return; }
        try {
          const data = JSON.parse(raw) as { tabs: unknown[]; activeTabId: string; savedAt: number };
          const restoredTabs = (data.tabs as Parameters<typeof createRequestTab>[0][]).map((t) =>
            createRequestTab(t as Parameters<typeof createRequestTab>[0])
          );
          const hasMeaningful = restoredTabs.some(
            (t) => t.urlInput.trim() || t.body || t.headers.some((h) => h.key),
          );
          if (hasMeaningful) {
            setPendingSession({ tabs: restoredTabs, activeTabId: data.activeTabId, savedAt: data.savedAt });
          } else {
            sessionHydrated.current = true;
          }
        } catch {
          sessionHydrated.current = true;
        }
      })
      .catch(() => { sessionHydrated.current = true; });
  }, []);

  const activeEnv = envEnabled ? (environments.find((e) => e.id === activeEnvId) ?? null) : null;

  const autosaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    if (!sessionHydrated.current) return;
    if (autosaveTimerRef.current) clearTimeout(autosaveTimerRef.current);
    autosaveTimerRef.current = setTimeout(() => {
      const payload = JSON.stringify({ tabs, activeTabId, savedAt: Date.now(), version: 1 });
      saveSession(payload, Date.now()).catch(console.error);
    }, 800);
    return () => { if (autosaveTimerRef.current) clearTimeout(autosaveTimerRef.current); };
  }, [tabs, activeTabId]);

  const handleEnvEnabledChange = (val: boolean) => {
    setEnvEnabled(val);
    localStorage.setItem("envEnabled", String(val));
  };

  const handleSaveEnvironment = async (env: AppEnvironment) => {
    await saveEnvironment({ ...env, isActive: activeEnvId === env.id });
    const updated = await getEnvironments();
    setEnvironments(updated);
  };

  const handleDeleteEnvironment = async (id: string) => {
    await deleteEnvironment(id);
    setEnvironments((prev) => prev.filter((e) => e.id !== id));
    if (activeEnvId === id) setActiveEnvId(null);
  };

  
  useEffect(() => {
    const handleMove = (e: MouseEvent) => {
      if (!isDragging.current || !splitContainerRef.current) return;
      const rect = splitContainerRef.current.getBoundingClientRect();
      const ratio =
        layoutRef.current === "horizontal"
          ? (e.clientX - rect.left) / rect.width
          : (e.clientY - rect.top) / rect.height;
      setSplitRatio(Math.min(0.8, Math.max(0.15, ratio)));
    };
    const handleUp = () => { isDragging.current = false; };
    window.addEventListener("mousemove", handleMove);
    window.addEventListener("mouseup", handleUp);
    return () => {
      window.removeEventListener("mousemove", handleMove);
      window.removeEventListener("mouseup", handleUp);
    };
  }, []);

  
  const getFullUrl = useCallback(
    (tab: typeof activeTab) => {
      const envVars = activeEnv?.variables ?? {};
      let url = tab.urlInput.trim();
      if (!url) return "";

      url = replaceVariables(url, envVars);

      if (envEnabled && envVars.baseUrl && !/^https?:\/\//i.test(url)) {
        const base = envVars.baseUrl.replace(/\/$/, "");
        url = base + (url.startsWith("/") ? url : `/${url}`);
      }

      const enabledParams = tab.params.filter((p) => p.key.trim() && p.enabled);
      if (enabledParams.length > 0) {
        const qs = enabledParams
          .map((p) => `${encodeURIComponent(replaceVariables(p.key, envVars))}=${encodeURIComponent(replaceVariables(p.value, envVars))}`)
          .join("&");
        url += (url.includes("?") ? "&" : "?") + qs;
      }

      return url;
    },
    [envEnabled, activeEnv],
  );

  
  const applyParsedCurl = useCallback(
    (parsed: { method?: string | null; url?: string | null; header?: Record<string, string>; body?: string | null; cookies?: Record<string, string> }) => {
      const importedHeaders: HeaderType[] = [];
      let newAuthType: AuthType = "none";
      let newBearerToken = "";
      let newBasicUser = "";
      let newBasicPass = "";

      for (const [key, value] of Object.entries(parsed.header ?? {})) {
        const lk = key.toLowerCase();
        if (lk === "authorization" || lk === "x-authorization") {
          if (value.toLowerCase().startsWith("bearer ")) {
            newAuthType = "bearer";
            newBearerToken = value.substring(7);
            continue;
          }
          if (value.toLowerCase().startsWith("basic ")) {
            newAuthType = "basic";
            try {
              const decoded = atob(value.substring(6));
              const colonIdx = decoded.indexOf(":");
              newBasicUser = decoded.slice(0, colonIdx);
              newBasicPass = decoded.slice(colonIdx + 1);
            } catch { }
            continue;
          }
        }
        importedHeaders.push({ id: crypto.randomUUID(), key, value, enabled: true });
      }

      let newCookies: CookieType[] = [];
      let newCookieString = "";
      let newCookieAuthType: AuthType = newAuthType;

      const cookiesMap = parsed.cookies ?? {};
      if (Object.keys(cookiesMap).length > 0) {
        newCookieAuthType = "cookie";
        newCookies = Object.entries(cookiesMap).map(([name, value]) => ({
          id: crypto.randomUUID(),
          name,
          value,
          enabled: true,
        }));
        newCookieString = Object.entries(cookiesMap)
          .map(([k, v]) => `${k}=${v}`)
          .join("; ");

        const accessToken = extractAccessTokenFromCookies(newCookieString);
        if (accessToken) {
          newBearerToken = accessToken;
        }
      }

      let newBody = "";
      let newBodyType: RequestBodyType = "none";
      if (parsed.body) {
        newBody = parsed.body;
        try {
          JSON.parse(parsed.body);
          newBodyType = "json";
        } catch {
          newBodyType = "text";
        }
      }

      const newMethod = parsed.method ?? "GET";
      const newUrl = (parsed.url ?? "").replace(/(%27|%22|'|")+$/, "");

      updateActiveTab({
        method: newMethod,
        urlInput: newUrl,
        headers:
          importedHeaders.length > 0
            ? importedHeaders
            : [{ id: crypto.randomUUID(), key: "Content-Type", value: "application/json", enabled: true }],
        authType: newCookieAuthType,
        bearerToken: newBearerToken,
        basicUser: newBasicUser,
        basicPass: newBasicPass,
        body: newBody,
        bodyType: newBodyType,
        cookies: newCookies,
        cookieString: newCookieString,
        name: buildTabName(newMethod, newUrl),
      });
    },
    [updateActiveTab],
  );

  const handleCurlImport = useCallback(
    (curlCmd: string) => {
      parseCurl(curlCmd)
        .then((parsed) => applyParsedCurl(parsed))
        .catch(() => {
          try {
            applyParsedCurl(parseCurlCommand(curlCmd));
          } catch (e) {
            console.error("cURL parse error:", e);
          }
        });
    },
    [applyParsedCurl],
  );

  
  const sendRequest = useCallback(async () => {
    const tab = activeTab;
    const fullUrl = getFullUrl(tab);
    if (!fullUrl) return;

    const tabId = activeTabId;
    setTabResponse(tabId, { isLoading: true, response: null, error: null, responseTime: null, responseSize: null, testResults: [], consoleLogs: [] });

    let aborted = false;
    abortRef.current = () => {
      aborted = true;
      abortRef.current = null;
      setTabResponse(tabId, { isLoading: false, response: null, error: "Request aborted", responseTime: null, responseSize: null });
    };

    const envVars = activeEnv?.variables ?? {};
    const preLogs: ConsoleEntry[] = [];
    let scriptEnv = { ...envVars };
    let scriptBody: string | undefined;
    if (tab.preRequestScript) {
      const scriptResult = await runPreRequestScript(tab.preRequestScript, { ...envVars }, tab.body ?? "");
      preLogs.push(...scriptResult.logs);
      scriptEnv = scriptResult.vars;
      scriptBody = scriptResult.body;
    }

    const headerObj: Record<string, string> = {};
    tab.headers.forEach((h) => {
      if (h.key && h.enabled) headerObj[h.key] = h.value;
    });

    if (tab.authType === "bearer" && tab.bearerToken) {
      headerObj["Authorization"] = `Bearer ${tab.bearerToken}`;
    } else if (tab.authType === "basic" && tab.basicUser) {
      headerObj["Authorization"] = `Basic ${btoa(`${tab.basicUser}:${tab.basicPass}`)}`;
    } else if (tab.authType === "apikey" && tab.apiKeyName && tab.apiKeyValue) {
      if (tab.apiKeyLocation === "header") headerObj[tab.apiKeyName] = tab.apiKeyValue;
    } else if (tab.authType === "cookie") {
      const cookieHeader = tab.cookies
        .filter((c) => c.name && c.enabled)
        .map((c) => `${c.name}=${c.value}`)
        .join("; ");
      if (cookieHeader) headerObj["Cookie"] = cookieHeader;
    } else if (tab.authType === "jwt-user" && tab.bearerToken) {
      const payload = parseJwt(tab.bearerToken);
      if (payload) headerObj["x-user-detail"] = JSON.stringify(payload);
    }

    const requestOptions: RequestInit = { method: tab.method, headers: headerObj };
    let proxyFormFields: ProxyFormField[] | undefined;

    if (tab.bodyType === "json" || tab.bodyType === "text") {
      // Use body modified by pre-request script if available, otherwise use tab body
      const rawBody = scriptBody ?? tab.body;
      if (rawBody) {
        requestOptions.body = rawBody;
        const hasContentType = Object.keys(headerObj).some(
          (k) => k.toLowerCase() === "content-type",
        );
        if (!hasContentType) {
          (requestOptions.headers as Record<string, string>)["Content-Type"] =
            tab.bodyType === "json" ? "application/json" : "text/plain";
        }
      }
    } else if (tab.bodyType === "form-data") {
      const enabledFields = tab.formDataFields.filter((f) => f.enabled && f.key);
      const hasFileField = enabledFields.some((f) => f.type === "file");

      if (hasFileField) {
        
        proxyFormFields = enabledFields.map((f) => ({
          name: f.key,
          value: f.value,
          is_file: f.type === "file",
          file_name: f.fileName,
          file_data_base64: f.fileData,
          mime_type: f.mimeType,
        }));
      } else if (enabledFields.length > 0) {
        
        requestOptions.body = enabledFields
          .map((f) => `${encodeURIComponent(f.key)}=${encodeURIComponent(f.value)}`)
          .join("&");
        const hasContentType = Object.keys(headerObj).some(
          (k) => k.toLowerCase() === "content-type",
        );
        if (!hasContentType) {
          (requestOptions.headers as Record<string, string>)["Content-Type"] =
            "application/x-www-form-urlencoded";
        }
      }
    }

    const startTime = performance.now();
    try {
      const res = await enhancedFetch(fullUrl, requestOptions, proxyFormFields, tabId);
      if (aborted) return;
      const duration = Math.round(performance.now() - startTime);
      const text = await res.text();
      const size = (res as { bodySize?: number }).bodySize ?? text.length;

      let data: unknown = null;
      if (text.length > 0) {
        const ct = res.headers.get("content-type") ?? "";
        try {
          data = ct.includes("application/json") ? JSON.parse(text) : text;
        } catch {
          data = text;
        }
      }

      const apiResponse: ApiResponse = {
        status: res.status,
        statusText: res.statusText,
        headers: Object.fromEntries(res.headers.entries()),
        data,
        cookies: res.headers.get("set-cookie"),
      };

      const { results: testResults, logs: postLogs } = tab.testScript
        ? await runTestScript(tab.testScript, apiResponse, duration, scriptEnv)
        : { results: [], logs: [] };
      const consoleLogs = [...preLogs, ...postLogs];

      startTransition(() => {
        setTabResponse(tabId, {
          isLoading: false,
          response: apiResponse,
          responseTime: duration,
          responseSize: size,
          error: null,
          testResults,
          consoleLogs,
        });
      });
      if (testResults.length > 0) setActiveResponseTab("tests");
      else if (consoleLogs.length > 0) setActiveResponseTab("console");

      
      const historyEntry: HistoryEntry = {
        id: crypto.randomUUID(),
        timestamp: Date.now(),
        method: tab.method,
        url: fullUrl,
        status: res.status,
        duration,
        request: {
          id: tab.savedRequestId ?? crypto.randomUUID(),
          collectionId: "",
          name: tab.name,
          method: tab.method,
          url: tab.urlInput,
          headers: tab.headers,
          params: tab.params,
          body: tab.body,
          bodyType: tab.bodyType,
          authType: tab.authType,
          bearerToken: tab.bearerToken,
          basicUser: tab.basicUser,
          basicPass: tab.basicPass,
          apiKeyName: tab.apiKeyName,
          apiKeyValue: tab.apiKeyValue,
          apiKeyLocation: tab.apiKeyLocation as ApiKeyLocation,
          formDataFields: tab.formDataFields,
          cookieString: tab.cookieString,
          cookies: tab.cookies,
          preRequestScript: tab.preRequestScript,
          testScript: tab.testScript,
        },
      };
      addToHistory(historyEntry).catch(console.error);
      abortRef.current = null;
    } catch (err: unknown) {
      if (aborted) return;
      const msg = err instanceof Error ? err.message : "Unknown error";
      setTabResponse(tabId, { isLoading: false, response: null, error: msg, responseTime: null, responseSize: null });
      abortRef.current = null;
    }
  }, [activeTab, activeTabId, activeEnv, getFullUrl, setTabResponse, addToHistory]);

  const handleSaveRequest = useCallback(async (name: string, collectionId: string) => {
    const tab = activeTab;
    const req: SavedRequest = {
      id: tab.savedRequestId ?? crypto.randomUUID(),
      collectionId,
      name,
      method: tab.method,
      url: tab.urlInput,
      headers: tab.headers,
      params: tab.params,
      body: tab.body,
      bodyType: tab.bodyType,
      authType: tab.authType,
      bearerToken: tab.bearerToken,
      basicUser: tab.basicUser,
      basicPass: tab.basicPass,
      apiKeyName: tab.apiKeyName,
      apiKeyValue: tab.apiKeyValue,
      apiKeyLocation: tab.apiKeyLocation as ApiKeyLocation,
      formDataFields: tab.formDataFields,
      cookieString: tab.cookieString,
      cookies: tab.cookies,
      preRequestScript: tab.preRequestScript,
      testScript: tab.testScript,
    };
    await saveReqToCollection(req);
    updateActiveTab({ name, isDirty: false, savedRequestId: req.id, savedCollectionId: collectionId });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeTab, saveReqToCollection, updateActiveTab]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const meta = e.metaKey || e.ctrlKey;
      if (!meta) return;
      if (e.key === "Enter") { e.preventDefault(); sendRequest(); }
      else if (e.key === "s") {
        e.preventDefault();
        const tab = tabs.find((t) => t.id === activeTabId);
        if (tab?.savedRequestId) {
          const collId =
            tab.savedCollectionId ??
            Object.entries(collectionRequests).find(([, reqs]) =>
              reqs.some((r) => r.id === tab.savedRequestId),
            )?.[0];
          if (collId) {
            handleSaveRequest(tab.name, collId);
          } else {
            setSaveDialogOpen(true);
          }
        } else {
          setSaveDialogOpen(true);
        }
      }
      else if (e.key === "t") { e.preventDefault(); addTab(); }
      else if (e.key === "w") { e.preventDefault(); handleCloseTab(activeTabId); }
      else if (e.key === "d") { e.preventDefault(); duplicateTab(activeTabId); }
      else if (e.key === "p") { e.preventDefault(); setPaletteOpen(true); }
    };
    const shortcutHandler = (e: Event) => {
      const key = (e as CustomEvent<string>).detail;
      if (key === "new-tab") addTab();
      else if (key === "close-tab") handleCloseTab(activeTabId);
      else if (key === "send") sendRequest();
      else if (key === "duplicate") duplicateTab(activeTabId);
      else if (key === "palette") setPaletteOpen(true);
      else if (key === "save") {
        const tab = tabs.find((t) => t.id === activeTabId);
        if (tab?.savedRequestId) {
          const collId = tab.savedCollectionId ?? Object.entries(collectionRequests).find(([, reqs]) => reqs.some((r) => r.id === tab.savedRequestId))?.[0];
          if (collId) { handleSaveRequest(tab.name, collId); return; }
        }
        setSaveDialogOpen(true);
      }
    };
    window.addEventListener("keydown", handler);
    window.addEventListener("app:shortcut", shortcutHandler);
    return () => {
      window.removeEventListener("keydown", handler);
      window.removeEventListener("app:shortcut", shortcutHandler);
    };
  }, [sendRequest, addTab, handleCloseTab, duplicateTab, activeTabId, tabs, handleSaveRequest, collectionRequests]);

  
  const handleLoadRequest = useCallback(
    (req: SavedRequest) => {
      addTab(savedRequestToRequestTab(req));
    },
    [addTab],
  );

  const handleImportCollection = async (col: Collection, reqs: SavedRequest[]) => {
    await createCollection(col.name).then(async (newCol) => {
      for (const req of reqs) {
        await saveReqToCollection({ ...req, collectionId: newCol.id });
      }
    });
  };

  
  const generateCurl = useCallback(() => {
    const tab = activeTab;
    const fullUrl = getFullUrl(tab);
    if (!fullUrl) return;

    const input: GenerateCurlInput = {
      method: tab.method,
      url: fullUrl,
      headers: tab.headers
        .filter((h) => h.key && h.enabled)
        .map((h) => ({ key: h.key, value: h.value })),
      body: tab.body || undefined,
      cookies: tab.cookies
        .filter((c) => c.name && c.enabled)
        .map((c) => ({ key: c.name, value: c.value })),
      auth_type: tab.authType,
      bearer_token: tab.bearerToken || undefined,
      basic_user: tab.basicUser || undefined,
      basic_pass: tab.basicPass || undefined,
      api_key_name: tab.apiKeyName || undefined,
      api_key_value: tab.apiKeyValue || undefined,
      api_key_location: tab.apiKeyLocation || undefined,
    };

    generateCurlCmd(input)
      .then((cmd) => setGeneratedCurl(cmd))
      .catch(console.error);
  }, [activeTab, getFullUrl]);

  const prepareJsCode = useCallback((): string => {
    const tab = activeTab;
    const fullUrl = getFullUrl(tab);
    if (!fullUrl) return "";
    const headers: Record<string, string> = {};
    tab.headers.filter((h) => h.key && h.enabled).forEach((h) => { headers[h.key] = h.value; });
    if (tab.authType === "bearer" && tab.bearerToken) {
      headers["Authorization"] = `Bearer ${tab.bearerToken}`;
    }
    if (tab.authType === "cookie") {
      const cookieStr = tab.cookies
        .filter((c) => c.name && c.enabled)
        .map((c) => `${c.name}=${c.value}`)
        .join("; ");
      if (cookieStr) headers["Cookie"] = cookieStr;
    }
    const parsed: ParsedCurl = { method: tab.method, header: headers, body: tab.body || undefined };
    const js = generateJsCode(parsed, fullUrl);
    setGeneratedJs(js);
    return js;
  }, [activeTab, getFullUrl]);

  
  const togglePanel = (p: SidePanel) => setPanel((prev) => (prev === p ? null : p));

  
  const u = updateActiveTab;

  const handleExtractFromCookie = useCallback(() => {
    let jwtToken = "";

    const cookieHeader = activeTab.headers.find((h) => h.key.toLowerCase() === "cookie");
    if (cookieHeader?.value) {
      const c = parseCookies(cookieHeader.value);
      jwtToken = c.accessToken || c.token || c.jwt ||
        Object.values(c).find((v) => v.startsWith("eyJ")) || "";
    }

    if (!jwtToken && activeTab.cookieString) {
      const c = parseCookies(activeTab.cookieString);
      jwtToken = c.accessToken || c.token || c.jwt ||
        Object.values(c).find((v) => v.startsWith("eyJ")) || "";
    }

    if (!jwtToken && activeTab.cookies.length > 0) {
      jwtToken = activeTab.cookies
        .filter((c) => c.enabled)
        .find((c) => c.value.startsWith("eyJ"))?.value || "";
    }

    if (!jwtToken && activeTab.bearerToken.startsWith("eyJ")) {
      jwtToken = activeTab.bearerToken;
    }

    if (!jwtToken) {
      for (const h of activeTab.headers) {
        if (!h.enabled) continue;
        if (h.value.startsWith("eyJ")) { jwtToken = h.value; break; }
        if (h.key.toLowerCase() === "authorization" && h.value.toLowerCase().startsWith("bearer ")) {
          const t = h.value.slice(7);
          if (t.startsWith("eyJ")) { jwtToken = t; break; }
        }
      }
    }

    if (!jwtToken) return;
    const decoded = parseJwt(jwtToken);
    if (!decoded) return;
    const xUserDetailValue = JSON.stringify(decoded);
    const existing = activeTab.headers.find((h) => h.key.toLowerCase() === "x-user-detail");
    if (existing) {
      u({ headers: activeTab.headers.map((h) => h.id === existing.id ? { ...h, value: xUserDetailValue, enabled: true } : h) });
    } else {
      u({ headers: [...activeTab.headers, { id: crypto.randomUUID(), key: "x-user-detail", value: xUserDetailValue, enabled: true }] });
    }
  }, [activeTab.headers, activeTab.cookieString, activeTab.cookies, activeTab.bearerToken, u]);

  const syncParamsFromUrl = useCallback(
    (cleanUrl: string, params: Array<{ id: string; key: string; value: string; enabled: boolean }>) => {
      u({ urlInput: cleanUrl, params });
    },
    [u],
  );

  const urlSyncTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    if (urlSyncTimerRef.current) clearTimeout(urlSyncTimerRef.current);
    const raw = activeTab.urlInput;
    if (!raw.includes("?")) return;
    urlSyncTimerRef.current = setTimeout(() => {
      const q = raw.indexOf("?");
      const cleanUrl = raw.slice(0, q);
      try {
        const search = new URLSearchParams(raw.slice(q + 1));
        const params = Array.from(search.entries())
          .filter(([key]) => key.length > 0)
          .map(([key, value]) => ({ id: crypto.randomUUID(), key, value, enabled: true }));
        if (params.length > 0) updateActiveTab({ urlInput: cleanUrl, params });
      } catch { /* ignore */ }
    }, 150);
    return () => { if (urlSyncTimerRef.current) clearTimeout(urlSyncTimerRef.current); };
  }, [activeTab.urlInput, updateActiveTab]);

  return (
    <div className={cn(resolved, "flex h-screen overflow-hidden select-none bg-stone-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100")}>
      <div className="flex flex-col items-center w-12 border-r border-stone-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 py-3 gap-1 shrink-0">
        {(
          [
            { id: "collections", icon: BookOpen, label: "Collections" },
            { id: "history", icon: Clock, label: "History" },
            { id: "environments", icon: Globe, label: "Environments" },
          ] as const
        ).map(({ id, icon: Icon, label }) => (
          <button
            key={id}
            title={label}
            onClick={() => togglePanel(id)}
            className={cn(
              "p-2.5 rounded-lg transition-colors",
              panel === id
                ? "bg-brand-600/20 text-brand-400"
                : "text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800",
            )}
          >
            <Icon className="h-4 w-4" />
          </button>
        ))}
        <div className="flex-1" />
        <button
          title="Settings"
          onClick={() => togglePanel("settings")}
          className={cn(
            "p-2.5 rounded-lg transition-colors",
            panel === "settings"
              ? "bg-brand-600/20 text-brand-400"
              : "text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800",
          )}
        >
          <Settings className="h-4 w-4" />
        </button>
      </div>

      {panel && (
        <div className="w-64 shrink-0 border-r border-stone-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 flex flex-col overflow-hidden">
          {panel === "collections" && (
            <CollectionsSidebar
              collections={collections}
              requests={collectionRequests}
              onLoadRequest={handleLoadRequest}
              onCreateCollection={createCollection}
              onRenameCollection={renameCollection}
              onDeleteCollection={deleteCollection}
              onDeleteRequest={deleteReqFromCollection}
              onRenameRequest={renameRequest}
              onImportCollection={handleImportCollection}
            />
          )}
          {panel === "history" && (
            <HistorySidebar
              history={history}
              onReplayRequest={(entry) => handleLoadRequest(entry.request)}
              onClearHistory={clearHistory}
            />
          )}
          {panel === "environments" && (
            <EnvironmentsSidebar
              environments={environments}
              activeEnvId={activeEnvId}
              envEnabled={envEnabled}
              onEnvEnabledChange={handleEnvEnabledChange}
              onSetActive={setActiveEnvId}
              onSave={handleSaveEnvironment}
              onDelete={handleDeleteEnvironment}
            />
          )}
          {panel === "settings" && <AboutPanel />}
        </div>
      )}

      <div className="flex-1 flex flex-col overflow-hidden min-w-0">
        {pendingSession && (
          <div className="flex items-center gap-3 px-4 py-2 bg-brand-600/10 border-b border-brand-500/20 text-xs shrink-0">
            <span className="text-brand-400 text-sm"></span>
            <span className="text-zinc-700 dark:text-zinc-300 flex-1">
              Restore {pendingSession.tabs.length} tab{pendingSession.tabs.length !== 1 ? "s" : ""} from your last session
              <span className="text-zinc-400 dark:text-zinc-600 ml-1.5">
                ({formatRelativeTime(pendingSession.savedAt)})
              </span>
            </span>
            <button
              onClick={() => {
                sessionHydrated.current = true;
                restoreTabs(pendingSession.tabs, pendingSession.activeTabId);
                setPendingSession(null);
              }}
              className="px-3 py-1 bg-brand-600 hover:bg-brand-500 text-white rounded-md font-semibold transition-colors"
            >
              Restore
            </button>
            <button
              onClick={() => {
                sessionHydrated.current = true;
                clearSession().catch(console.error);
                setPendingSession(null);
              }}
              className="p-1 text-zinc-400 hover:text-zinc-700 dark:hover:text-zinc-200 transition-colors"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        )}
        <div className="flex items-center h-9 border-b border-stone-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 overflow-x-auto shrink-0 scrollbar-thin">
          {tabs.map((tab, index) => (
            <div
              key={tab.id}
              draggable
              onDragStart={(e) => {
                dragTabIdRef.current = tab.id;
                e.dataTransfer.effectAllowed = "move";
                e.dataTransfer.setData("text/plain", tab.id);
              }}
              onDragOver={(e) => {
                e.preventDefault();
                e.dataTransfer.dropEffect = "move";
                setDragOverTabId(tab.id);
              }}
              onDragLeave={() => setDragOverTabId(null)}
              onDrop={(e) => {
                e.preventDefault();
                setDragOverTabId(null);
                const fromId = dragTabIdRef.current;
                dragTabIdRef.current = null;
                if (!fromId || fromId === tab.id) return;
                const fromIndex = tabs.findIndex((t) => t.id === fromId);
                if (fromIndex >= 0) reorderTabs(fromIndex, index);
              }}
              onDragEnd={() => { dragTabIdRef.current = null; setDragOverTabId(null); }}
              onClick={() => setActiveTab(tab.id)}
              className={cn(
                "group flex items-center gap-1.5 px-3 h-full border-r border-stone-200 dark:border-zinc-800 cursor-pointer shrink-0",
                "text-xs transition-colors min-w-0 max-w-[200px]",
                tab.id === activeTabId
                  ? "bg-stone-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 border-t-2 border-t-brand-500"
                  : "text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 hover:bg-stone-50 dark:hover:bg-zinc-800",
                dragOverTabId === tab.id && dragTabIdRef.current !== tab.id && "border-l-2 border-l-brand-400",
              )}
            >
              <span className={cn("text-[10px] font-bold shrink-0", METHOD_BADGE[tab.method] ?? "text-zinc-400")}>
                {tab.method.slice(0, 3)}
              </span>
              <span className="truncate">{tab.name}</span>
              {tab.isDirty && <span className="text-brand-400 shrink-0">●</span>}
              <button
                onClick={(e) => { e.stopPropagation(); handleCloseTab(tab.id); }}
                className={cn(
                  "shrink-0 rounded p-0.5 transition-colors",
                  tab.id === activeTabId
                    ? "text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-200 hover:bg-stone-200 dark:hover:bg-zinc-700"
                    : "opacity-0 group-hover:opacity-100 text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 hover:bg-stone-200 dark:hover:bg-zinc-700",
                )}
              >
                <X className="h-3 w-3" />
              </button>
            </div>
          ))}

          <button
            onClick={() => addTab()}
            className="h-full px-3 text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-200 hover:bg-stone-100 dark:hover:bg-zinc-800 transition-colors shrink-0"
            title="New tab"
          >
            <Plus className="h-3.5 w-3.5" />
          </button>
        </div>

        <UrlBar
          method={activeTab.method}
          onMethodChange={(m) => u({ method: m })}
          urlInput={activeTab.urlInput}
          onUrlChange={(v) => u({ urlInput: v })}
          onUrlSync={syncParamsFromUrl}
          onCurlImport={handleCurlImport}
          isLoading={activeResponse.isLoading}
          onSendRequest={sendRequest}
          onAbort={() => { abortRef.current?.(); }}
          onSaveRequest={() => {
            if (activeTab.savedRequestId) {
              const collId =
                activeTab.savedCollectionId ??
                Object.entries(collectionRequests).find(([, reqs]) =>
                  reqs.some((r) => r.id === activeTab.savedRequestId),
                )?.[0];
              if (collId) { handleSaveRequest(activeTab.name, collId); return; }
            }
            setSaveDialogOpen(true);
          }}
          generatedCurl={generatedCurl}
          generatedJs={generatedJs}
          onGenerateCode={() => { generateCurl(); prepareJsCode(); }}
        />

        <div
          ref={splitContainerRef}
          className={cn(
            "flex-1 overflow-hidden min-h-0",
            layout === "vertical" ? "flex flex-col" : "flex flex-row",
          )}
        >
          <div
            className={cn(
              "flex flex-col overflow-hidden",
              layout === "vertical"
                ? "border-b border-stone-200 dark:border-zinc-800"
                : "border-r border-stone-200 dark:border-zinc-800",
            )}
            style={
              layout === "vertical"
                ? { height: `${splitRatio * 100}%` }
                : { width: `${splitRatio * 100}%` }
            }
          >
            <Tabs
              value={activeRequestTab}
              onValueChange={(v) => setActiveRequestTab(v as RequestTabType)}
              className="flex flex-col h-full"
            >
              <TabsList className="shrink-0 bg-white dark:bg-zinc-900 border-b border-stone-200 dark:border-zinc-800 rounded-none justify-start px-2 h-8 gap-0">
                {(["params", "headers", "body", "auth", "scripts"] as RequestTabType[]).map((t) => (
                  <TabsTrigger
                    key={t}
                    value={t}
                    className={cn(
                      "text-xs px-3 py-0 h-7 rounded-none capitalize border-b-2 border-transparent data-[state=active]:border-brand-500",
                      "data-[state=active]:bg-transparent data-[state=active]:text-brand-400",
                      "text-zinc-500 dark:text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300",
                    )}
                  >
                    {t}
                    {t === "params" &&
                      activeTab.params.filter((p) => p.key && p.enabled).length > 0 && (
                        <span className="ml-1 text-[10px] bg-stone-200 dark:bg-zinc-700 text-zinc-700 dark:text-zinc-300 rounded-full px-1">
                          {activeTab.params.filter((p) => p.key && p.enabled).length}
                        </span>
                      )}
                    {t === "headers" &&
                      activeTab.headers.filter((h) => h.key && h.enabled).length > 0 && (
                        <span className="ml-1 text-[10px] bg-stone-200 dark:bg-zinc-700 text-zinc-700 dark:text-zinc-300 rounded-full px-1">
                          {activeTab.headers.filter((h) => h.key && h.enabled).length}
                        </span>
                      )}
                    {t === "scripts" && (activeTab.preRequestScript || activeTab.testScript) && (
                      <span className="ml-1 w-1.5 h-1.5 rounded-full bg-brand-500 inline-block" />
                    )}
                  </TabsTrigger>
                ))}
              </TabsList>

              <TabsContent value="params" className="flex-1 overflow-auto m-0 p-0 data-[state=active]:flex data-[state=active]:flex-col">
                <RequestParams
                  params={activeTab.params}
                  urlInput={activeTab.urlInput}
                  onAddParam={() => u({ params: [...activeTab.params, { id: crypto.randomUUID(), key: "", value: "", enabled: true }] })}
                  onParamChange={(id, field, value) => u({ params: activeTab.params.map((p) => (p.id === id ? { ...p, [field]: value } : p)) })}
                  onRemoveParam={(id) => u({ params: activeTab.params.filter((p) => p.id !== id) })}
                />
              </TabsContent>

              <TabsContent value="headers" className="flex-1 overflow-auto m-0 p-0 data-[state=active]:flex data-[state=active]:flex-col">
                <RequestHeaders
                  headers={activeTab.headers}
                  onAddHeader={() => u({ headers: [...activeTab.headers, { id: crypto.randomUUID(), key: "", value: "", enabled: true }] })}
                  onHeaderChange={(id, field, value) => u({ headers: activeTab.headers.map((h) => (h.id === id ? { ...h, [field]: value } : h)) })}
                  onRemoveHeader={(id) => u({ headers: activeTab.headers.filter((h) => h.id !== id) })}
                  onSetHeaders={(h) => u({ headers: h })}
                  onExtractFromCookie={handleExtractFromCookie}
                />
              </TabsContent>

              <TabsContent value="body" className="flex-1 overflow-hidden m-0 p-0 data-[state=active]:flex data-[state=active]:flex-col">
                <RequestBody
                  bodyType={activeTab.bodyType}
                  body={activeTab.body}
                  onBodyChange={(v) => u({ body: v })}
                  onBodyTypeChange={(t) => u({ bodyType: t })}
                  formDataFields={activeTab.formDataFields}
                  onFormDataChange={(fields: FormDataField[]) => u({ formDataFields: fields })}
                />
              </TabsContent>

              <TabsContent value="auth" className="flex-1 overflow-auto m-0 p-0 data-[state=active]:flex data-[state=active]:flex-col">
                <Authentication
                  authType={activeTab.authType as AuthType}
                  onAuthTypeChange={(t) => u({ authType: t })}
                  bearerToken={activeTab.bearerToken}
                  onBearerTokenChange={(v) => u({ bearerToken: v })}
                  basicUser={activeTab.basicUser}
                  onBasicUserChange={(v) => u({ basicUser: v })}
                  basicPass={activeTab.basicPass}
                  onBasicPassChange={(v) => u({ basicPass: v })}
                  apiKeyName={activeTab.apiKeyName}
                  onApiKeyNameChange={(v) => u({ apiKeyName: v })}
                  apiKeyValue={activeTab.apiKeyValue}
                  onApiKeyValueChange={(v) => u({ apiKeyValue: v })}
                  apiKeyLocation={activeTab.apiKeyLocation as "header" | "query"}
                  onApiKeyLocationChange={(v) => u({ apiKeyLocation: v })}
                  userDetail=""
                  onJwtTokenChange={(v) => u({ bearerToken: v })}
                  cookieString={activeTab.cookieString}
                  onCookieStringChange={(v) => {
                    const parsed = parseCookies(v);
                    const cookieItems: CookieType[] = Object.entries(parsed).map(([name, value]) => ({
                      id: crypto.randomUUID(), name, value, enabled: true,
                    }));
                    const accessToken = extractAccessTokenFromCookies(v);
                    u({ cookieString: v, cookies: cookieItems, bearerToken: accessToken ?? activeTab.bearerToken });
                  }}
                  cookies={activeTab.cookies}
                  onAddCookie={() => u({ cookies: [...activeTab.cookies, { id: crypto.randomUUID(), name: "", value: "", enabled: true }] })}
                  onCookieChange={(id, field, value) => u({ cookies: activeTab.cookies.map((c) => (c.id === id ? { ...c, [field]: value } : c)) })}
                  onRemoveCookie={(id) => u({ cookies: activeTab.cookies.filter((c) => c.id !== id) })}
                />
              </TabsContent>

              <TabsContent value="scripts" className="flex-1 overflow-hidden m-0 p-0 data-[state=active]:flex data-[state=active]:flex-col">
                <ScriptsTab
                  preRequestScript={activeTab.preRequestScript}
                  onPreRequestScriptChange={(v) => u({ preRequestScript: v })}
                  testScript={activeTab.testScript}
                  onTestScriptChange={(v) => u({ testScript: v })}
                />
              </TabsContent>
            </Tabs>
          </div>

          <div
            className={cn(
              "bg-stone-200 dark:bg-zinc-800 hover:bg-brand-500/40 transition-colors shrink-0 group flex items-center justify-center",
              layout === "vertical"
                ? "h-1.5 cursor-row-resize"
                : "w-1.5 cursor-col-resize",
            )}
            onMouseDown={() => { isDragging.current = true; }}
          >
            <div
              className={cn(
                "bg-stone-400 dark:bg-zinc-600 group-hover:bg-brand-500/60 rounded-full transition-colors",
                layout === "vertical" ? "w-8 h-0.5" : "h-8 w-0.5",
              )}
            />
          </div>

          <div
            className="overflow-hidden flex flex-col min-h-0 flex-1"
          >
            <ResponseViewer
              isLoading={activeResponse.isLoading}
              response={activeResponse.response}
              error={activeResponse.error}
              responseTime={activeResponse.responseTime}
              responseSize={activeResponse.responseSize}
              activeTab={activeResponseTab}
              onTabChange={setActiveResponseTab}
              bodyView={responseBodyView}
              onBodyViewChange={setResponseBodyView}
              tabId={activeTabId}
              testResults={activeResponse.testResults}
              consoleLogs={activeResponse.consoleLogs}
              layout={layout}
              onLayoutChange={setLayout}
            />
          </div>
        </div>
      </div>

      <SaveRequestDialog
        open={saveDialogOpen}
        defaultName={activeTab.name}
        defaultCollectionId={activeTab.savedCollectionId}
        collections={collections}
        onSave={handleSaveRequest}
        onClose={() => setSaveDialogOpen(false)}
        onCreateCollection={createCollection}
      />
      <CommandPalette
        open={paletteOpen}
        onClose={() => setPaletteOpen(false)}
        requests={Object.values(collectionRequests).flat()}
        collections={collections}
        history={history}
        openTabs={tabs}
        activeTabId={activeTabId}
        onOpen={handleLoadRequest}
        onSwitchTab={(tabId) => setActiveTab(tabId)}
      />
    </div>
  );
}
