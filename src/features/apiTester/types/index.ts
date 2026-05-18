// Types for API Tester

export interface ParsedCurl {
  method?: string;
  url?: string;
  header?: Record<string, string>;
  body?: string;
  cookies?: Record<string, string>;
}

export interface HeaderType {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
}

export interface CookieType {
  id: string;
  name: string;
  value: string;
  enabled: boolean;
}

export interface FormDataField {
  id: string;
  key: string;
  value: string;
  type: "text" | "file";
  enabled: boolean;
  fileName?: string;
}

export type AuthType = "none" | "basic" | "bearer" | "apikey" | "jwt-user" | "cookie";
export type RequestBodyType = "json" | "text" | "form-data" | "none";
export type ApiKeyLocation = "header" | "query";
export type ResponseBodyView = "pretty" | "raw";
export type RequestTabType = "params" | "headers" | "body" | "auth" | "scripts";
export type ResponseTabType = "body" | "headers" | "cookies";

export interface ApiResponse {
  status?: number;
  statusText?: string;
  headers?: Record<string, string>;
  data?: unknown;
  cookies?: string | null;
  error?: string;
}

export interface SavedRequest {
  id: string;
  collectionId: string;
  name: string;
  method: string;
  url: string;
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
  formDataFields: FormDataField[];
  cookieString: string;
  cookies: CookieType[];
  preRequestScript: string;
  testScript: string;
}

export interface Collection {
  id: string;
  name: string;
  createdAt: number;
}

export interface HistoryEntry {
  id: string;
  timestamp: number;
  method: string;
  url: string;
  status: number;
  duration: number;
  request: SavedRequest;
}

export interface AppEnvironment {
  id: string;
  name: string;
  variables: Record<string, string>;
  isActive: boolean;
}

// ── Postman import types (v2.1) ───────────────────────────────────────────────

export interface PostmanCollectionV21 {
  info: {
    name: string;
    _postman_id?: string;
    schema?: string;
  };
  item: PostmanItem[];
  variable?: Array<{ key: string; value: string }>;
}

export type PostmanItem = PostmanFolder | PostmanRequestItem;

export interface PostmanFolder {
  name: string;
  item: PostmanItem[];
  description?: string;
}

export interface PostmanRequestItem {
  name: string;
  request: {
    method?: string;
    header?: Array<{ key: string; value: string; disabled?: boolean }>;
    url?:
      | string
      | {
          raw: string;
          query?: Array<{ key: string; value: string; disabled?: boolean }>;
        };
    body?: {
      mode?: "raw" | "urlencoded" | "formdata" | "file" | "graphql";
      raw?: string;
      urlencoded?: Array<{ key: string; value: string; disabled?: boolean }>;
      formdata?: Array<{
        key: string;
        value?: string;
        type?: "text" | "file";
        disabled?: boolean;
      }>;
      options?: { raw?: { language?: string } };
    };
    auth?: {
      type: string;
      bearer?: Array<{ key: string; value: string }>;
      basic?: Array<{ key: string; value: string }>;
      apikey?: Array<{ key: string; value: string }>;
    };
  };
}
