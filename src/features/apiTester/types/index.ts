// Types for API Tester
export interface ParsedCurl {
  method?: string;
  url?: string;
  header?: Record<string, string>;
  body?: string;
  cookies?: Record<string, string>;
}

export interface Environment {
  name: string;
  variables: {
    baseUrl: string;
  };
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

export type AuthType = 'none' | 'basic' | 'bearer' | 'apikey' | 'jwt-user' | 'cookie';
export type RequestBodyType = 'json' | 'text' | 'form-data' | 'none';
export type ApiKeyLocation = 'header' | 'query';
export type ResponseBodyView = 'pretty' | 'raw';
export type RequestTabType = 'params' | 'headers' | 'body' | 'auth';
export type ResponseTabType = 'body' | 'headers' | 'cookies';

export interface ApiResponse {
  status?: number;
  statusText?: string;
  headers?: Record<string, string>;
  data?: any;
  cookies?: string | null;
  error?: string;
}
