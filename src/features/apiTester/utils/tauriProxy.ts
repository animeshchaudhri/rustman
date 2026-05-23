import { invoke } from '@tauri-apps/api/core';

export interface ProxyFormField {
  name: string;
  value: string;
  is_file: boolean;
  file_name?: string;
  file_data_base64?: string;
  mime_type?: string;
}

interface ProxyRequest {
  url: string;
  method: string;
  headers?: Record<string, string>;
  body?: string;
  form_fields?: ProxyFormField[];
  timeout?: number;
  tab_id?: string;
}

interface ProxyResponse {
  status: number;
  headers: Record<string, string>;
  body: string;
  body_size: number;
  body_stored: boolean;
  error?: string;
}

const STATUS_TEXTS: Record<number, string> = {
  200: 'OK', 201: 'Created', 202: 'Accepted', 204: 'No Content',
  301: 'Moved Permanently', 302: 'Found', 304: 'Not Modified',
  400: 'Bad Request', 401: 'Unauthorized', 403: 'Forbidden', 404: 'Not Found',
  405: 'Method Not Allowed', 409: 'Conflict', 422: 'Unprocessable Entity',
  429: 'Too Many Requests', 500: 'Internal Server Error',
  502: 'Bad Gateway', 503: 'Service Unavailable', 504: 'Gateway Timeout',
};

export const tauriProxyFetch = async (
  url: string,
  options: RequestInit = {},
  formFields?: ProxyFormField[],
  tabId?: string,
) => {
  const proxyRequest: ProxyRequest = {
    url,
    method: options.method || 'GET',
    timeout: 129_600_000, // 36 hours
    tab_id: tabId,
  };

  if (options.headers) {
    proxyRequest.headers = {};
    if (options.headers instanceof Headers) {
      options.headers.forEach((value, key) => {
        proxyRequest.headers![key] = value;
      });
    } else {
      proxyRequest.headers = options.headers as Record<string, string>;
    }
  }

  if (formFields && formFields.length > 0) {
    proxyRequest.form_fields = formFields;
    if (proxyRequest.headers) {
      const ctKey = Object.keys(proxyRequest.headers).find(
        (k) => k.toLowerCase() === 'content-type',
      );
      if (ctKey) delete proxyRequest.headers[ctKey];
    }
  } else if (options.body) {
    proxyRequest.body = typeof options.body === 'string'
      ? options.body
      : JSON.stringify(options.body);
  }

  const response = await invoke<ProxyResponse>('proxy_request', { request: proxyRequest });

  if (response.error) {
    throw new Error(response.error);
  }

  const responseHeaders = new Headers();
  Object.entries(response.headers).forEach(([key, value]) => {
    responseHeaders.set(key, value);
  });

  return {
    status: response.status,
    statusText: STATUS_TEXTS[response.status] || 'Unknown Status',
    ok: response.status >= 200 && response.status < 300,
    headers: responseHeaders,
    bodySize: response.body_size,
    bodyStored: response.body_stored,
    text: async () => response.body,
    json: async () => {
      try { return JSON.parse(response.body); }
      catch { throw new Error('Failed to parse response as JSON'); }
    },
  };
};
