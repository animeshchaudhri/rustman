import {  invoke } from '@tauri-apps/api/core';

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
}

interface ProxyResponse {
  status: number;
  headers: Record<string, string>;
  body: string;
  error?: string;
}

 
export const tauriProxyFetch = async (url: string, options: RequestInit = {}, formFields?: ProxyFormField[]) => {
  try {
    
    const proxyRequest: ProxyRequest = {
      url,
      method: options.method || 'GET',
      timeout: options.signal instanceof AbortSignal ? undefined : 90000000,
    };

    
    if (options.headers) {
      proxyRequest.headers = {};
      if (options.headers instanceof Headers) {
        options.headers.forEach((value, key) => {
          if (proxyRequest.headers) proxyRequest.headers[key] = value;
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

    
    const getStatusText = (status: number): string => {
      const statusTexts: Record<number, string> = {
        200: 'OK',
        201: 'Created',
        202: 'Accepted',
        204: 'No Content',
        400: 'Bad Request',
        401: 'Unauthorized',
        403: 'Forbidden',
        404: 'Not Found',
        405: 'Method Not Allowed',
        500: 'Internal Server Error',
        502: 'Bad Gateway',
        503: 'Service Unavailable',
        504: 'Gateway Timeout',
      };
      return statusTexts[status] || 'Unknown Status';
    };

    
    const fetchResponse = {
      status: response.status,
      statusText: getStatusText(response.status),
      ok: response.status >= 200 && response.status < 300,
      headers: responseHeaders,
      
      
      text: async () => response.body,
      
      
      json: async () => {
        try {
          return JSON.parse(response.body);
        } catch (e) {
          throw new Error('Failed to parse response as JSON');
        }
      },
      
      
    };

    return fetchResponse;
  } catch (error) {
    console.error('Tauri proxy error:', error);
    throw error;
  }
};
