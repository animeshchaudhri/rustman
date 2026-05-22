import { useCallback, useMemo, useState } from "react";

import type {
  ApiKeyLocation,
  AuthType,
  CookieType,
  FormDataField,
  HeaderType,
  RequestBodyType,
  SavedRequest,
} from "../types";

export interface RequestTab {
  id: string;
  name: string;
  isDirty: boolean;
  savedRequestId?: string;
  savedCollectionId?: string;
  method: string;
  urlInput: string;
  headers: HeaderType[];
  params: HeaderType[];
  body: string;
  bodyType: RequestBodyType;
  authType: AuthType;
  bearerToken: string;
  basicUser: string;
  basicPass: string;
  apiKeyName: string;
  apiKeyValue: string;
  apiKeyLocation: ApiKeyLocation;
  cookieString: string;
  cookies: CookieType[];
  formDataFields: FormDataField[];
  preRequestScript: string;
  testScript: string;
}

type UpdateRequestTab = Partial<RequestTab> | ((tab: RequestTab) => RequestTab);

const createHeader = (): HeaderType => ({
  id: crypto.randomUUID(),
  key: "",
  value: "",
  enabled: true,
});

const createCookie = (): CookieType => ({
  id: crypto.randomUUID(),
  name: "",
  value: "",
  enabled: true,
});

export const createFormDataField = (): FormDataField => ({
  id: crypto.randomUUID(),
  key: "",
  value: "",
  type: "text",
  enabled: true,
});

const cloneHeaders = (headers: HeaderType[]) => headers.map((header) => ({ ...header }));
const cloneCookies = (cookies: CookieType[]) => cookies.map((cookie) => ({ ...cookie }));
const cloneFormData = (fields: FormDataField[]) => fields.map((field) => ({ ...field }));

const requestStateKeys: Array<keyof RequestTab> = [
  "method",
  "urlInput",
  "headers",
  "params",
  "body",
  "bodyType",
  "authType",
  "bearerToken",
  "basicUser",
  "basicPass",
  "apiKeyName",
  "apiKeyValue",
  "apiKeyLocation",
  "cookieString",
  "cookies",
  "formDataFields",
];

export const buildTabName = (method: string, urlInput: string, fallback = "Untitled Request") => {
  const trimmedUrl = urlInput.trim();
  if (!trimmedUrl) {
    return fallback;
  }

  try {
    const parsedUrl = new URL(trimmedUrl);
    const path = `${parsedUrl.pathname}${parsedUrl.search}` || "/";
    return `${method.toUpperCase()} ${path}`;
  } catch {
    const withoutProtocol = trimmedUrl.replace(/^https?:\/\//, "");
    const slashIndex = withoutProtocol.indexOf("/");
    const path = slashIndex >= 0 ? withoutProtocol.slice(slashIndex) : trimmedUrl;
    return `${method.toUpperCase()} ${path || fallback}`;
  }
};

const cloneRequestTab = (tab: RequestTab): RequestTab => ({
  ...tab,
  headers: cloneHeaders(tab.headers),
  params: cloneHeaders(tab.params),
  cookies: cloneCookies(tab.cookies),
  formDataFields: cloneFormData(tab.formDataFields),
});

const normalizeHeaders = (headers?: HeaderType[]) =>
  headers && headers.length > 0 ? cloneHeaders(headers) : [createHeader()];

const normalizeParams = (params?: HeaderType[]) =>
  params && params.length > 0 ? cloneHeaders(params) : [createHeader()];

const normalizeCookies = (cookies?: CookieType[]) =>
  cookies && cookies.length > 0 ? cloneCookies(cookies) : [createCookie()];

const normalizeFormData = (fields?: FormDataField[]) =>
  fields && fields.length > 0 ? cloneFormData(fields) : [createFormDataField()];

export const createRequestTab = (overrides: Partial<RequestTab> = {}): RequestTab => {
  const method = overrides.method ?? "GET";
  const urlInput = overrides.urlInput ?? "";

  return {
    id: overrides.id ?? crypto.randomUUID(),
    name: overrides.name ?? buildTabName(method, urlInput),
    isDirty: overrides.isDirty ?? false,
    savedRequestId: overrides.savedRequestId,
    savedCollectionId: overrides.savedCollectionId,
    method,
    urlInput,
    headers: normalizeHeaders(overrides.headers),
    params: normalizeParams(overrides.params),
    body: overrides.body ?? "",
    bodyType: overrides.bodyType ?? "none",
    authType: overrides.authType ?? "none",
    bearerToken: overrides.bearerToken ?? "",
    basicUser: overrides.basicUser ?? "",
    basicPass: overrides.basicPass ?? "",
    apiKeyName: overrides.apiKeyName ?? "",
    apiKeyValue: overrides.apiKeyValue ?? "",
    apiKeyLocation: overrides.apiKeyLocation ?? "header",
    cookieString: overrides.cookieString ?? "",
    cookies: normalizeCookies(overrides.cookies),
    formDataFields: normalizeFormData(overrides.formDataFields),
    preRequestScript: overrides.preRequestScript ?? "",
    testScript: overrides.testScript ?? "",
  };
};

export const savedRequestToRequestTab = (request: SavedRequest): RequestTab =>
  createRequestTab({
    name: request.name,
    isDirty: false,
    savedRequestId: request.id,
    savedCollectionId: request.collectionId,
    method: request.method,
    urlInput: request.url,
    headers: request.headers,
    params: request.params,
    body: request.body,
    bodyType: request.bodyType,
    authType: request.authType,
    bearerToken: request.bearerToken,
    basicUser: request.basicUser,
    basicPass: request.basicPass,
    apiKeyName: request.apiKeyName,
    apiKeyValue: request.apiKeyValue,
    apiKeyLocation: request.apiKeyLocation,
    formDataFields: request.formDataFields,
    cookieString: request.cookieString,
    cookies: request.cookies,
    preRequestScript: request.preRequestScript ?? "",
    testScript: request.testScript ?? "",
  });

const hasRequestStateChange = (previous: RequestTab, next: RequestTab) =>
  requestStateKeys.some((key) => JSON.stringify(previous[key]) !== JSON.stringify(next[key]));

export function useRequestTabs() {
  const [tabs, setTabs] = useState<RequestTab[]>(() => [createRequestTab()]);
  const [activeTabId, setActiveTabId] = useState<string>(() => tabs[0]?.id ?? crypto.randomUUID());

  const activeTab = useMemo(
    () => tabs.find((tab) => tab.id === activeTabId) ?? tabs[0],
    [activeTabId, tabs],
  );

  const addTab = useCallback((initial?: Partial<RequestTab>) => {
    const nextTab = createRequestTab(initial);
    setTabs((prev) => [...prev, nextTab]);
    setActiveTabId(nextTab.id);
    return nextTab;
  }, []);

  const closeTab = useCallback(
    (tabId: string) => {
      setTabs((prev) => {
        if (prev.length === 1) {
          const replacement = createRequestTab();
          setActiveTabId(replacement.id);
          return [replacement];
        }

        const currentIndex = prev.findIndex((tab) => tab.id === tabId);
        const nextTabs = prev.filter((tab) => tab.id !== tabId);

        if (activeTabId === tabId) {
          const fallback = nextTabs[Math.max(0, currentIndex - 1)] ?? nextTabs[0];
          setActiveTabId(fallback.id);
        }

        return nextTabs;
      });
    },
    [activeTabId],
  );

  const updateActiveTab = useCallback(
    (update: UpdateRequestTab) => {
      setTabs((prev) =>
        prev.map((tab) => {
          if (tab.id !== activeTabId) {
            return tab;
          }

          const draft = cloneRequestTab(tab);
          const updated = typeof update === "function" ? update(draft) : { ...draft, ...update };
          const normalized = createRequestTab({ ...tab, ...updated, id: tab.id });

          if (!(typeof update !== "function" && update.name !== undefined)) {
            const usesAutoName =
              !tab.savedRequestId ||
              tab.name === buildTabName(tab.method, tab.urlInput) ||
              tab.name === "Untitled Request";

            if (usesAutoName) {
              normalized.name = buildTabName(normalized.method, normalized.urlInput);
            }
          }

          if (!(typeof update !== "function" && update.isDirty !== undefined) && hasRequestStateChange(tab, normalized)) {
            normalized.isDirty = true;
          }

          return normalized;
        }),
      );
    },
    [activeTabId],
  );

  const duplicateTab = useCallback((tabId: string) => {
    setTabs((prev) => {
      const src = prev.find((t) => t.id === tabId);
      if (!src) return prev;
      const clone = cloneRequestTab(src);
      clone.id = crypto.randomUUID();
      clone.name = `${src.name} (copy)`;
      clone.isDirty = true;
      const idx = prev.findIndex((t) => t.id === tabId);
      const next = [...prev];
      next.splice(idx + 1, 0, clone);
      setActiveTabId(clone.id);
      return next;
    });
  }, []);

  const restoreTabs = useCallback((newTabs: RequestTab[], newActiveTabId: string) => {
    const normalized = newTabs.map((t) => createRequestTab({ ...t, id: t.id }));
    setTabs(normalized);
    setActiveTabId(newActiveTabId);
  }, []);

  const reorderTabs = useCallback((fromIndex: number, toIndex: number) => {
    setTabs((prev) => {
      const next = [...prev];
      const [removed] = next.splice(fromIndex, 1);
      next.splice(toIndex, 0, removed);
      return next;
    });
  }, []);

  return {
    tabs,
    activeTabId,
    activeTab,
    addTab,
    closeTab,
    duplicateTab,
    setActiveTab: setActiveTabId,
    updateActiveTab,
    restoreTabs,
    reorderTabs,
  };
}
