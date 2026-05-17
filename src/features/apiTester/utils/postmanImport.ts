import type {
  AuthType,
  Collection,
  CookieType,
  FormDataField,
  HeaderType,
  PostmanCollectionV21,
  PostmanFolder,
  PostmanItem,
  PostmanRequestItem,
  RequestBodyType,
  SavedRequest,
} from "../types";

export interface PostmanImportResult {
  collection: Collection;
  requests: SavedRequest[];
}

function isFolder(item: PostmanItem): item is PostmanFolder {
  return "item" in item && Array.isArray((item as PostmanFolder).item);
}

function extractUrl(
  url: PostmanRequestItem["request"]["url"],
): { rawUrl: string; queryParams: HeaderType[] } {
  if (!url) return { rawUrl: "", queryParams: [] };

  if (typeof url === "string") {
    try {
      const u = new URL(url);
      const queryParams: HeaderType[] = [];
      u.searchParams.forEach((value, key) => {
        queryParams.push({ id: crypto.randomUUID(), key, value, enabled: true });
      });
      return { rawUrl: url, queryParams };
    } catch {
      return { rawUrl: url, queryParams: [] };
    }
  }

  const rawUrl = url.raw ?? "";
  const queryParams: HeaderType[] = (url.query ?? [])
    .filter((q) => q.key)
    .map((q) => ({
      id: crypto.randomUUID(),
      key: q.key,
      value: q.value ?? "",
      enabled: !q.disabled,
    }));

  return { rawUrl, queryParams };
}

function extractBody(body: PostmanRequestItem["request"]["body"]): {
  bodyStr: string;
  bodyType: RequestBodyType;
  formDataFields: FormDataField[];
} {
  if (!body || !body.mode) return { bodyStr: "", bodyType: "none", formDataFields: [] };

  if (body.mode === "raw") {
    const lang = body.options?.raw?.language;
    const bodyType: RequestBodyType = lang === "json" ? "json" : "text";
    return { bodyStr: body.raw ?? "", bodyType, formDataFields: [] };
  }

  if (body.mode === "urlencoded") {
    const fields: FormDataField[] = (body.urlencoded ?? []).map((f) => ({
      id: crypto.randomUUID(),
      key: f.key,
      value: f.value ?? "",
      type: "text" as const,
      enabled: !f.disabled,
    }));
    return { bodyStr: "", bodyType: "form-data", formDataFields: fields };
  }

  if (body.mode === "formdata") {
    const fields: FormDataField[] = (body.formdata ?? []).map((f) => ({
      id: crypto.randomUUID(),
      key: f.key,
      value: f.value ?? "",
      type: (f.type === "file" ? "file" : "text") as "text" | "file",
      enabled: !f.disabled,
    }));
    return { bodyStr: "", bodyType: "form-data", formDataFields: fields };
  }

  return { bodyStr: "", bodyType: "none", formDataFields: [] };
}

function extractAuth(auth: PostmanRequestItem["request"]["auth"]): {
  authType: AuthType;
  bearerToken: string;
  basicUser: string;
  basicPass: string;
  apiKeyName: string;
  apiKeyValue: string;
} {
  const defaults = {
    authType: "none" as AuthType,
    bearerToken: "",
    basicUser: "",
    basicPass: "",
    apiKeyName: "",
    apiKeyValue: "",
  };
  if (!auth) return defaults;

  if (auth.type === "bearer" && auth.bearer) {
    const token = auth.bearer.find((b) => b.key === "token")?.value ?? "";
    return { ...defaults, authType: "bearer", bearerToken: token };
  }

  if (auth.type === "basic" && auth.basic) {
    const user = auth.basic.find((b) => b.key === "username")?.value ?? "";
    const pass = auth.basic.find((b) => b.key === "password")?.value ?? "";
    return { ...defaults, authType: "basic", basicUser: user, basicPass: pass };
  }

  if (auth.type === "apikey" && auth.apikey) {
    const keyName = auth.apikey.find((b) => b.key === "key")?.value ?? "X-API-Key";
    const keyValue = auth.apikey.find((b) => b.key === "value")?.value ?? "";
    return { ...defaults, authType: "apikey", apiKeyName: keyName, apiKeyValue: keyValue };
  }

  return defaults;
}

function convertRequest(
  item: PostmanRequestItem,
  collectionId: string,
  namePrefix = "",
): SavedRequest {
  const { rawUrl, queryParams } = extractUrl(item.request.url);
  const { bodyStr, bodyType, formDataFields } = extractBody(item.request.body);
  const authData = extractAuth(item.request.auth);

  const headers: HeaderType[] = (item.request.header ?? [])
    .filter((h) => h.key)
    .map((h) => ({
      id: crypto.randomUUID(),
      key: h.key,
      value: h.value ?? "",
      enabled: !h.disabled,
    }));

  return {
    id: crypto.randomUUID(),
    collectionId,
    name: namePrefix ? `${namePrefix} / ${item.name}` : item.name,
    method: (item.request.method ?? "GET").toUpperCase(),
    url: rawUrl,
    headers,
    params: queryParams,
    body: bodyStr,
    bodyType,
    formDataFields,
    cookieString: "",
    cookies: [] as CookieType[],
    ...authData,
    apiKeyLocation: "header" as const,
  };
}

function flattenItems(
  items: PostmanItem[],
  collectionId: string,
  prefix = "",
): SavedRequest[] {
  const result: SavedRequest[] = [];
  for (const item of items) {
    if (isFolder(item)) {
      const folderPrefix = prefix ? `${prefix} / ${item.name}` : item.name;
      result.push(...flattenItems(item.item, collectionId, folderPrefix));
    } else {
      result.push(convertRequest(item, collectionId, prefix));
    }
  }
  return result;
}

export function importPostmanCollection(json: unknown): PostmanImportResult {
  const data = json as PostmanCollectionV21;

  if (!data?.info?.name || !Array.isArray(data.item)) {
    throw new Error(
      "Invalid Postman collection. Expected v2.1 format with info.name and item array.",
    );
  }

  const collectionId = crypto.randomUUID();
  const collection: Collection = {
    id: collectionId,
    name: data.info.name,
    createdAt: Date.now(),
  };

  const requests = flattenItems(data.item, collectionId);
  return { collection, requests };
}
