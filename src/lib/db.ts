import { invoke } from "@tauri-apps/api/core";
import type {
  AppEnvironment,
  Collection,
  CookieType,
  FormDataField,
  HeaderType,
  HistoryEntry,
  SavedRequest,
} from "@/features/apiTester/types";

interface DbCollection {
  id: string;
  name: string;
  createdAt: number;
}

interface DbSavedRequest {
  id: string;
  collectionId: string;
  name: string;
  method: string;
  url: string;
  headers: string;
  params: string;
  body: string;
  bodyType: string;
  authType: string;
  bearerToken: string;
  basicUser: string;
  basicPass: string;
  apiKeyName: string;
  apiKeyValue: string;
  apiKeyLocation: string;
  formDataFields: string;
  cookieString: string;
  cookies: string;
  preRequestScript: string;
  testScript: string;
}

interface DbHistoryEntry {
  id: string;
  timestamp: number;
  method: string;
  url: string;
  status: number;
  duration: number;
  request: string;
}

interface DbEnvironment {
  id: string;
  name: string;
  variables: string;
  isActive: boolean;
}

const tryParse = <T>(json: string, fallback: T): T => {
  try {
    return JSON.parse(json) as T;
  } catch {
    return fallback;
  }
};

function dbToCollection(db: DbCollection): Collection {
  return { id: db.id, name: db.name, createdAt: db.createdAt };
}

function dbToSavedRequest(db: DbSavedRequest): SavedRequest {
  return {
    id: db.id,
    collectionId: db.collectionId,
    name: db.name,
    method: db.method,
    url: db.url,
    headers: tryParse<HeaderType[]>(db.headers, []),
    params: tryParse<HeaderType[]>(db.params, []),
    body: db.body,
    bodyType: db.bodyType as SavedRequest["bodyType"],
    authType: db.authType as SavedRequest["authType"],
    bearerToken: db.bearerToken,
    basicUser: db.basicUser,
    basicPass: db.basicPass,
    apiKeyName: db.apiKeyName,
    apiKeyValue: db.apiKeyValue,
    apiKeyLocation: db.apiKeyLocation as SavedRequest["apiKeyLocation"],
    formDataFields: tryParse<FormDataField[]>(db.formDataFields, []),
    cookieString: db.cookieString,
    cookies: tryParse<CookieType[]>(db.cookies, []),
    preRequestScript: db.preRequestScript ?? "",
    testScript: db.testScript ?? "",
  };
}

function savedRequestToDb(req: SavedRequest): DbSavedRequest {
  return {
    id: req.id,
    collectionId: req.collectionId,
    name: req.name,
    method: req.method,
    url: req.url,
    headers: JSON.stringify(req.headers),
    params: JSON.stringify(req.params),
    body: req.body,
    bodyType: req.bodyType,
    authType: req.authType,
    bearerToken: req.bearerToken,
    basicUser: req.basicUser,
    basicPass: req.basicPass,
    apiKeyName: req.apiKeyName,
    apiKeyValue: req.apiKeyValue,
    apiKeyLocation: req.apiKeyLocation,
    formDataFields: JSON.stringify(req.formDataFields),
    cookieString: req.cookieString,
    cookies: JSON.stringify(req.cookies),
    preRequestScript: req.preRequestScript ?? "",
    testScript: req.testScript ?? "",
  };
}

const EMPTY_REQUEST: SavedRequest = {
  id: "",
  collectionId: "",
  name: "",
  method: "GET",
  url: "",
  headers: [],
  params: [],
  body: "",
  bodyType: "none",
  authType: "none",
  bearerToken: "",
  basicUser: "",
  basicPass: "",
  apiKeyName: "",
  apiKeyValue: "",
  apiKeyLocation: "header",
  formDataFields: [],
  cookieString: "",
  cookies: [],
  preRequestScript: "",
  testScript: "",
};

function dbToHistoryEntry(db: DbHistoryEntry): HistoryEntry {
  return {
    id: db.id,
    timestamp: db.timestamp,
    method: db.method,
    url: db.url,
    status: db.status,
    duration: db.duration,
    request: tryParse<SavedRequest>(db.request, EMPTY_REQUEST),
  };
}

function historyToDb(entry: HistoryEntry): DbHistoryEntry {
  return {
    id: entry.id,
    timestamp: entry.timestamp,
    method: entry.method,
    url: entry.url,
    status: entry.status,
    duration: entry.duration,
    request: JSON.stringify(entry.request),
  };
}

function dbToEnvironment(db: DbEnvironment): AppEnvironment {
  return {
    id: db.id,
    name: db.name,
    variables: tryParse<Record<string, string>>(db.variables, {}),
    isActive: db.isActive,
  };
}

function environmentToDb(env: AppEnvironment): DbEnvironment {
  return {
    id: env.id,
    name: env.name,
    variables: JSON.stringify(env.variables),
    isActive: env.isActive,
  };
}

export async function getCollections(): Promise<Collection[]> {
  const result = await invoke<DbCollection[]>("db_get_collections");
  return result.map(dbToCollection);
}

export async function createCollection(name: string): Promise<Collection> {
  const id = crypto.randomUUID();
  const createdAt = Date.now();
  const result = await invoke<DbCollection>("db_create_collection", {
    id,
    name,
    createdAt,
  });
  return dbToCollection(result);
}

export async function updateCollection(collection: Collection): Promise<Collection> {
  await invoke("db_update_collection", { id: collection.id, name: collection.name });
  return collection;
}

export async function deleteCollection(id: string): Promise<void> {
  await invoke("db_delete_collection", { id });
}

export async function getRequestsForCollection(collectionId: string): Promise<SavedRequest[]> {
  const result = await invoke<DbSavedRequest[]>("db_get_requests", { collectionId });
  return result.map(dbToSavedRequest);
}

export async function saveRequest(request: SavedRequest): Promise<SavedRequest> {
  await invoke("db_save_request", { req: savedRequestToDb(request) });
  return request;
}

export async function updateRequest(request: SavedRequest): Promise<SavedRequest> {
  await invoke("db_save_request", { req: savedRequestToDb(request) });
  return request;
}

export async function deleteRequest(id: string): Promise<void> {
  await invoke("db_delete_request", { id });
}

export async function getHistory(): Promise<HistoryEntry[]> {
  const result = await invoke<DbHistoryEntry[]>("db_get_history");
  return result.map(dbToHistoryEntry);
}

export async function addToHistory(entry: HistoryEntry): Promise<HistoryEntry> {
  await invoke("db_add_history", { entry: historyToDb(entry) });
  return entry;
}

export async function clearHistory(): Promise<void> {
  await invoke("db_clear_history");
}

export async function getEnvironments(): Promise<AppEnvironment[]> {
  const result = await invoke<DbEnvironment[]>("db_get_environments");
  return result.map(dbToEnvironment);
}

export async function saveEnvironment(env: AppEnvironment): Promise<AppEnvironment> {
  await invoke("db_save_environment", { env: environmentToDb(env) });
  return env;
}

export async function deleteEnvironment(id: string): Promise<void> {
  await invoke("db_delete_environment", { id });
}

export async function saveSession(data: string, savedAt: number): Promise<void> {
  await invoke("db_save_session", { data, savedAt });
}

export async function getSession(): Promise<string | null> {
  return await invoke<string | null>("db_get_session");
}

export async function clearSession(): Promise<void> {
  await invoke("db_clear_session");
}

// ── Rust in-memory body store ──────────────────────────────────────────────

export interface BodySlice {
  lines: string[];
  totalLines: number;
}

export async function bodyStore(id: string, text: string): Promise<void> {
  await invoke("body_store", { id, text });
}

export async function bodyGetSlice(id: string, lineStart: number, lineCount: number): Promise<BodySlice> {
  return await invoke<BodySlice>("body_get_slice", { id, lineStart, lineCount });
}

export async function bodySearch(id: string, query: string): Promise<number> {
  return await invoke<number>("body_search", { id, query });
}

export async function bodyGetFull(id: string): Promise<string> {
  return await invoke<string>("body_get_full", { id });
}

export async function bodySearchLines(id: string, query: string): Promise<number[]> {
  return await invoke<number[]>("body_search_lines", { id, query });
}

export async function bodyClearPrefix(prefix: string): Promise<void> {
  await invoke("body_clear_prefix", { prefix });
}

export interface ParsedCurlResult {
  method: string | null;
  url: string | null;
  header: Record<string, string>;
  body: string | null;
  cookies: Record<string, string>;
}

export async function parseCurl(cmd: string): Promise<ParsedCurlResult> {
  return await invoke<ParsedCurlResult>("parse_curl", { cmd });
}

export interface CurlKvPair {
  key: string;
  value: string;
}

export interface GenerateCurlInput {
  method: string;
  url: string;
  headers: CurlKvPair[];
  body?: string;
  cookies: CurlKvPair[];
  auth_type: string;
  bearer_token?: string;
  basic_user?: string;
  basic_pass?: string;
  api_key_name?: string;
  api_key_value?: string;
  api_key_location?: string;
}

export async function generateCurlCmd(input: GenerateCurlInput): Promise<string> {
  return await invoke<string>("generate_curl", { input });
}
