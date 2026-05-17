import { useCallback, useEffect, useRef, useState } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Authentication,
  CollectionsSidebar,
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
} from "./utils";
import {
  deleteEnvironment,
  getEnvironments,
  saveEnvironment,
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
  ParsedCurl,
  RequestBodyType,
  RequestTabType,
  ResponseBodyView,
  ResponseTabType,
  SavedRequest,
} from "./types";
import { cn } from "@/lib/utils";
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

// ── Types ────────────────────────────────────────────────────────────────────

type SidePanel = "collections" | "history" | "environments" | "settings" | null;

interface TabResponse {
  response: ApiResponse | null;
  responseTime: number | null;
  responseSize: number | null;
  isLoading: boolean;
  error: string | null;
}

const METHOD_BADGE: Record<string, string> = {
  GET: "text-emerald-400",
  POST: "text-orange-400",
  PUT: "text-blue-400",
  PATCH: "text-teal-400",
  DELETE: "text-red-400",
  HEAD: "text-purple-400",
  OPTIONS: "text-sky-400",
};

// ── About / Settings panel ────────────────────────────────────────────────────

function AboutPanel() {
  const stack = [
    { icon: Zap, label: "Tauri v2", desc: "Desktop runtime" },
    { icon: Layers, label: "React 18", desc: "UI framework" },
    { icon: Database, label: "SQLite", desc: "via rusqlite (bundled)" },
  ];

  const links = [
    { icon: Github, label: "GitHub", href: "https://github.com/animeshchaudhri", color: "text-zinc-300" },
    { icon: Globe2, label: "animesh.us", href: "https://animesh.us", color: "text-sky-400" },
    { icon: Mail, label: "ac04@duck.com", href: "mailto:ac04@duck.com", color: "text-orange-400" },
  ];

  return (
    <div className="flex flex-col h-full overflow-y-auto">
      {/* Header */}
      <div className="px-4 py-3 border-b border-zinc-800">
        <p className="text-[10px] uppercase tracking-widest text-zinc-600 font-semibold mb-0.5">About</p>
        <p className="text-sm font-bold text-zinc-100">Rustman</p>
      </div>

      <div className="flex flex-col gap-5 p-4">
        {/* Logo block */}
        <div className="flex flex-col items-center gap-3 py-5 bg-zinc-800/40 rounded-xl border border-zinc-800">
          <img src="/rustman-logo.svg" alt="Rustman" className="w-16 h-16" />
          <div className="text-center">
            <p className="text-base font-bold text-zinc-100 tracking-tight">
              RUST<span className="text-orange-400">MAN</span>
            </p>
            <p className="text-[10px] text-zinc-600 tracking-widest mt-0.5">API TESTING TOOL</p>
          </div>
          <span className="text-[10px] bg-zinc-700 text-zinc-400 px-2 py-0.5 rounded-full">v0.1.0</span>
        </div>

        {/* Author */}
        <div className="bg-zinc-800/40 rounded-xl border border-zinc-800 overflow-hidden">
          <div className="px-4 py-2.5 border-b border-zinc-800">
            <p className="text-[10px] uppercase tracking-widest text-zinc-600 font-semibold">Made by</p>
          </div>
          <div className="px-4 py-3 flex items-center gap-3">
            <img
              src="https://github.com/animeshchaudhri.png"
              alt="Animesh Chaudhri"
              className="w-10 h-10 rounded-full border-2 border-orange-500/30"
              onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
            />
            <div>
              <p className="text-sm font-semibold text-zinc-100">Animesh Chaudhri</p>
              <p className="text-[11px] text-zinc-500">Full-Stack · Rust · AI · Distributed Systems</p>
              <p className="text-[10px] text-zinc-600 mt-0.5">Pune, India</p>
            </div>
          </div>

          {/* Links */}
          <div className="border-t border-zinc-800 divide-y divide-zinc-800/60">
            {links.map(({ icon: Icon, label, href, color }) => (
              <a
                key={href}
                href={href}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-3 px-4 py-2.5 hover:bg-zinc-800/60 transition-colors group"
              >
                <Icon className={cn("h-3.5 w-3.5 shrink-0", color)} />
                <span className="text-xs text-zinc-400 group-hover:text-zinc-200 transition-colors">{label}</span>
              </a>
            ))}
          </div>
        </div>

        {/* Stack */}
        <div className="bg-zinc-800/40 rounded-xl border border-zinc-800 overflow-hidden">
          <div className="px-4 py-2.5 border-b border-zinc-800">
            <p className="text-[10px] uppercase tracking-widest text-zinc-600 font-semibold">Built with</p>
          </div>
          <div className="divide-y divide-zinc-800/60">
            {stack.map(({ icon: Icon, label, desc }) => (
              <div key={label} className="flex items-center gap-3 px-4 py-2.5">
                <Icon className="h-3.5 w-3.5 text-orange-400/70 shrink-0" />
                <div>
                  <p className="text-xs font-medium text-zinc-300">{label}</p>
                  <p className="text-[10px] text-zinc-600">{desc}</p>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-center gap-1.5 text-[10px] text-zinc-700 pb-2">
          <span>© 2026 Animesh Chaudhri</span>
          <Heart className="h-2.5 w-2.5 text-red-500/60 fill-red-500/60" />
          <span>MIT License</span>
        </div>
      </div>
    </div>
  );
}

// ── Component ─────────────────────────────────────────────────────────────────

export default function ApiTester() {
  // ── Sidebar & UI state ──
  const [panel, setPanel] = useState<SidePanel>("collections");
  const [splitRatio, setSplitRatio] = useState(0.45);
  const isDragging = useRef(false);
  const splitContainerRef = useRef<HTMLDivElement>(null);

  // ── Request tabs ──
  const { tabs, activeTabId, activeTab, addTab, closeTab, duplicateTab, setActiveTab, updateActiveTab } =
    useRequestTabs();

  // ── Per-tab response state ──
  const [responses, setResponses] = useState<Record<string, TabResponse>>({});
  const activeResponse = responses[activeTabId] ?? {
    response: null,
    responseTime: null,
    responseSize: null,
    isLoading: false,
    error: null,
  };

  const setTabResponse = useCallback(
    (tabId: string, patch: Partial<TabResponse>) => {
      setResponses((prev) => ({
        ...prev,
        [tabId]: { ...{ response: null, responseTime: null, responseSize: null, isLoading: false, error: null }, ...(prev[tabId] ?? {}), ...patch },
      }));
    },
    [],
  );

  // ── Per-tab UI state (request/response tabs, view mode) ──
  const [activeRequestTab, setActiveRequestTab] = useState<RequestTabType>("params");
  const [activeResponseTab, setActiveResponseTab] = useState<ResponseTabType>("body");
  const [responseBodyView, setResponseBodyView] = useState<ResponseBodyView>("pretty");

  // ── Code gen ──
  const [generatedCurl, setGeneratedCurl] = useState("");
  const [generatedJs, setGeneratedJs] = useState("");

  // ── Save dialog ──
  const [saveDialogOpen, setSaveDialogOpen] = useState(false);

  // ── Collections ──
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

  // ── History ──
  const { history, addToHistory, clearHistory } = useHistory();

  // ── Environments ──
  const [environments, setEnvironments] = useState<AppEnvironment[]>([]);
  const [activeEnvId, setActiveEnvId] = useState<string | null>(null);
  const [envEnabled, setEnvEnabled] = useState(
    () => localStorage.getItem("envEnabled") === "true",
  );

  useEffect(() => {
    getEnvironments().then(setEnvironments).catch(console.error);
  }, []);

  const activeEnv = envEnabled ? (environments.find((e) => e.id === activeEnvId) ?? null) : null;

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

  // ── Drag handle ──
  useEffect(() => {
    const handleMove = (e: MouseEvent) => {
      if (!isDragging.current || !splitContainerRef.current) return;
      const rect = splitContainerRef.current.getBoundingClientRect();
      const ratio = (e.clientY - rect.top) / rect.height;
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

  // ── URL building with {{variable}} substitution ──
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

  // ── cURL import (called by UrlBar on paste or typed curl) ──
  const handleCurlImport = useCallback(
    (curlCmd: string) => {
      try {
        const parsed: ParsedCurl = parseCurlCommand(curlCmd);

        const importedHeaders: HeaderType[] = [];
        let newAuthType: AuthType = "none";
        let newBearerToken = "";
        let newBasicUser = "";
        let newBasicPass = "";

        if (parsed.header) {
          for (const [key, value] of Object.entries(parsed.header)) {
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
                } catch {/* ignore */}
                continue;
              }
            }
            importedHeaders.push({ id: crypto.randomUUID(), key, value, enabled: true });
          }
        }

        let newCookies: CookieType[] = [];
        let newCookieString = "";
        let newCookieAuthType: AuthType = newAuthType;

        if (parsed.cookies && Object.keys(parsed.cookies).length > 0) {
          newCookieAuthType = "cookie";
          newCookies = Object.entries(parsed.cookies).map(([name, value]) => ({
            id: crypto.randomUUID(),
            name,
            value,
            enabled: true,
          }));
          newCookieString = Object.entries(parsed.cookies)
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
        const newUrl = parsed.url ?? "";

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
      } catch (e) {
        console.error("cURL parse error:", e);
      }
    },
    [updateActiveTab],
  );

  // ── Send request ──
  const sendRequest = useCallback(async () => {
    const tab = activeTab;
    const fullUrl = getFullUrl(tab);
    if (!fullUrl) return;

    const tabId = activeTabId;
    setTabResponse(tabId, { isLoading: true, response: null, error: null, responseTime: null, responseSize: null });

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

    if (["POST", "PUT", "PATCH", "DELETE"].includes(tab.method)) {
      if (tab.bodyType === "json" || tab.bodyType === "text") {
        if (tab.body) requestOptions.body = tab.body;
      } else if (tab.bodyType === "form-data") {
        const textFields = tab.formDataFields.filter((f) => f.enabled && f.key && f.type === "text");
        if (textFields.length > 0) {
          requestOptions.body = textFields
            .map((f) => `${encodeURIComponent(f.key)}=${encodeURIComponent(f.value)}`)
            .join("&");
          if (!headerObj["Content-Type"] && !headerObj["content-type"]) {
            (requestOptions.headers as Record<string, string>)["Content-Type"] =
              "application/x-www-form-urlencoded";
          }
        }
      }
    }

    const startTime = performance.now();
    try {
      const res = await enhancedFetch(fullUrl, requestOptions);
      const duration = Math.round(performance.now() - startTime);
      const text = await res.text();
      const size = new Blob([text]).size;

      let data: unknown;
      const ct = res.headers.get("content-type") ?? "";
      try {
        data = ct.includes("application/json") ? JSON.parse(text) : text;
      } catch {
        data = text;
      }

      const apiResponse: ApiResponse = {
        status: res.status,
        statusText: res.statusText,
        headers: Object.fromEntries(res.headers.entries()),
        data,
        cookies: res.headers.get("set-cookie"),
      };

      setTabResponse(tabId, {
        isLoading: false,
        response: apiResponse,
        responseTime: duration,
        responseSize: size,
        error: null,
      });

      // Auto-save to history
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
        },
      };
      addToHistory(historyEntry).catch(console.error);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : "Unknown error";
      setTabResponse(tabId, { isLoading: false, response: null, error: msg, responseTime: null, responseSize: null });
    }
  }, [activeTab, activeTabId, getFullUrl, setTabResponse, addToHistory]);

  // ── Keyboard shortcuts (placed after sendRequest to avoid TDZ) ──
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const meta = e.metaKey || e.ctrlKey;
      if (!meta) return;
      if (e.key === "Enter") { e.preventDefault(); sendRequest(); }
      else if (e.key === "s") { e.preventDefault(); setSaveDialogOpen(true); }
      else if (e.key === "t") { e.preventDefault(); addTab(); }
      else if (e.key === "w") { e.preventDefault(); closeTab(activeTabId); }
      else if (e.key === "d") { e.preventDefault(); duplicateTab(activeTabId); }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [sendRequest, addTab, closeTab, duplicateTab, activeTabId]);

  // ── Load request into new tab ──
  const handleLoadRequest = useCallback(
    (req: SavedRequest) => {
      addTab(savedRequestToRequestTab(req));
    },
    [addTab],
  );

  // ── Save request ──
  const handleSaveRequest = async (name: string, collectionId: string) => {
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
    };
    await saveReqToCollection(req);
    updateActiveTab({ name, isDirty: false, savedRequestId: req.id });
  };

  // ── Import Postman collection ──
  const handleImportCollection = async (col: Collection, reqs: SavedRequest[]) => {
    await createCollection(col.name).then(async (newCol) => {
      for (const req of reqs) {
        await saveReqToCollection({ ...req, collectionId: newCol.id });
      }
    });
  };

  // ── Code gen ──
  const generateCurl = useCallback((): string => {
    const tab = activeTab;
    const fullUrl = getFullUrl(tab);
    if (!fullUrl) return "";
    let cmd = `curl -X ${tab.method} "${fullUrl}"`;
    tab.headers.filter((h) => h.key && h.enabled).forEach((h) => {
      cmd += ` \\\n  -H "${h.key}: ${h.value}"`;
    });
    if (tab.authType === "bearer" && tab.bearerToken) {
      cmd += ` \\\n  -H "Authorization: Bearer ${tab.bearerToken}"`;
    }
    if (["POST", "PUT", "PATCH"].includes(tab.method) && tab.body) {
      cmd += ` \\\n  -d '${tab.body.replace(/'/g, "'\\''")}'`;
    }
    setGeneratedCurl(cmd);
    return cmd;
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
    const parsed: ParsedCurl = { method: tab.method, header: headers, body: tab.body || undefined };
    const js = generateJsCode(parsed, fullUrl);
    setGeneratedJs(js);
    return js;
  }, [activeTab, getFullUrl]);

  // ── Sidebar toggle ──
  const togglePanel = (p: SidePanel) => setPanel((prev) => (prev === p ? null : p));

  // ── Helpers to update specific tab fields ──
  const u = updateActiveTab;

  return (
    <div className="dark flex h-screen bg-zinc-950 text-zinc-100 overflow-hidden select-none">
      {/* ── Left icon rail ── */}
      <div className="flex flex-col items-center w-12 border-r border-zinc-800 bg-zinc-900 py-3 gap-1 shrink-0">
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
                ? "bg-orange-600/20 text-orange-400"
                : "text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800",
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
              ? "bg-orange-600/20 text-orange-400"
              : "text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800",
          )}
        >
          <Settings className="h-4 w-4" />
        </button>
      </div>

      {/* ── Sidebar panel ── */}
      {panel && (
        <div className="w-64 shrink-0 border-r border-zinc-800 bg-zinc-900 flex flex-col overflow-hidden">
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

      {/* ── Main area ── */}
      <div className="flex-1 flex flex-col overflow-hidden min-w-0">
        {/* Tab bar */}
        <div className="flex items-center h-9 border-b border-zinc-800 bg-zinc-900 overflow-x-auto shrink-0 scrollbar-thin">
          {tabs.map((tab) => (
            <div
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={cn(
                "group flex items-center gap-1.5 px-3 h-full border-r border-zinc-800 cursor-pointer shrink-0",
                "text-xs transition-colors min-w-0 max-w-[200px]",
                tab.id === activeTabId
                  ? "bg-zinc-950 text-zinc-100 border-t-2 border-t-orange-500"
                  : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800",
              )}
            >
              <span
                className={cn(
                  "text-[10px] font-bold shrink-0",
                  METHOD_BADGE[tab.method] ?? "text-zinc-400",
                )}
              >
                {tab.method.slice(0, 3)}
              </span>
              <span className="truncate">{tab.name}</span>
              {tab.isDirty && <span className="text-orange-400 shrink-0">●</span>}
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  closeTab(tab.id);
                }}
                className={cn(
                  "shrink-0 rounded p-0.5 transition-colors",
                  tab.id === activeTabId
                    ? "text-zinc-500 hover:text-zinc-200 hover:bg-zinc-700"
                    : "opacity-0 group-hover:opacity-100 text-zinc-600 hover:text-zinc-300 hover:bg-zinc-700",
                )}
              >
                <X className="h-3 w-3" />
              </button>
            </div>
          ))}

          <button
            onClick={() => addTab()}
            className="h-full px-3 text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 transition-colors shrink-0"
            title="New tab"
          >
            <Plus className="h-3.5 w-3.5" />
          </button>
        </div>

        {/* URL bar — code gen is integrated inside UrlBar */}
        <UrlBar
          method={activeTab.method}
          onMethodChange={(m) => u({ method: m })}
          urlInput={activeTab.urlInput}
          onUrlChange={(v) => u({ urlInput: v })}
          onCurlImport={handleCurlImport}
          isLoading={activeResponse.isLoading}
          onSendRequest={sendRequest}
          onSaveRequest={() => setSaveDialogOpen(true)}
          generatedCurl={generatedCurl}
          generatedJs={generatedJs}
          onGenerateCode={() => { generateCurl(); prepareJsCode(); }}
        />

        {/* Split view: request + response */}
        <div ref={splitContainerRef} className="flex-1 flex flex-col overflow-hidden min-h-0">
          {/* Request config */}
          <div
            className="flex flex-col overflow-hidden border-b border-zinc-800"
            style={{ height: `${splitRatio * 100}%` }}
          >
            <Tabs
              value={activeRequestTab}
              onValueChange={(v) => setActiveRequestTab(v as RequestTabType)}
              className="flex flex-col h-full"
            >
              <TabsList className="shrink-0 bg-zinc-900 border-b border-zinc-800 rounded-none justify-start px-2 h-8 gap-0">
                {(["params", "headers", "body", "auth", "scripts"] as RequestTabType[]).map((t) => (
                  <TabsTrigger
                    key={t}
                    value={t}
                    className={cn(
                      "text-xs px-3 py-0 h-7 rounded-none capitalize border-b-2 border-transparent data-[state=active]:border-orange-500",
                      "data-[state=active]:bg-transparent data-[state=active]:text-orange-400",
                      "text-zinc-500 hover:text-zinc-300",
                    )}
                  >
                    {t}
                    {t === "params" &&
                      activeTab.params.filter((p) => p.key && p.enabled).length > 0 && (
                        <span className="ml-1 text-[10px] bg-zinc-700 text-zinc-300 rounded-full px-1">
                          {activeTab.params.filter((p) => p.key && p.enabled).length}
                        </span>
                      )}
                    {t === "headers" &&
                      activeTab.headers.filter((h) => h.key && h.enabled).length > 0 && (
                        <span className="ml-1 text-[10px] bg-zinc-700 text-zinc-300 rounded-full px-1">
                          {activeTab.headers.filter((h) => h.key && h.enabled).length}
                        </span>
                      )}
                    {t === "scripts" && (activeTab.preRequestScript || activeTab.testScript) && (
                      <span className="ml-1 w-1.5 h-1.5 rounded-full bg-orange-500 inline-block" />
                    )}
                  </TabsTrigger>
                ))}
              </TabsList>

              <TabsContent value="params" className="flex-1 overflow-auto m-0 p-0 data-[state=active]:flex data-[state=active]:flex-col">
                <RequestParams
                  params={activeTab.params}
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

          {/* Drag handle */}
          <div
            className="h-1.5 bg-zinc-800 hover:bg-orange-500/40 cursor-row-resize transition-colors shrink-0 group flex items-center justify-center"
            onMouseDown={() => { isDragging.current = true; }}
          >
            <div className="w-8 h-0.5 bg-zinc-600 group-hover:bg-orange-500/60 rounded-full transition-colors" />
          </div>

          {/* Response */}
          <div
            className="overflow-hidden flex flex-col min-h-0"
            style={{ height: `${(1 - splitRatio) * 100 - 2}%` }}
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
            />
          </div>
        </div>
      </div>

      {/* Save dialog */}
      <SaveRequestDialog
        open={saveDialogOpen}
        defaultName={activeTab.name}
        collections={collections}
        onSave={handleSaveRequest}
        onClose={() => setSaveDialogOpen(false)}
        onCreateCollection={createCollection}
      />
    </div>
  );
}
