// TypeScript interface for the Tauri proxy command
import {  invoke } from '@tauri-apps/api/core';


// Interface matching Rust's ProxyRequest
interface ProxyRequest {
  url: string;
  method: string;
  headers?: Record<string, string>;
  body?: string;
  timeout?: number;
}

// Interface matching Rust's ProxyResponse
interface ProxyResponse {
  status: number;
  headers: Record<string, string>;
  body: string;
  error?: string;
}

/**
 * Makes an HTTP request through the Tauri backend to bypass CORS restrictions
 * 
 * @param url The URL to send the request to
 * @param options Standard RequestInit options similar to fetch API
 * @returns A Response-like object with status, headers, and methods to extract body
 */
export const tauriProxyFetch = async (url: string, options: RequestInit = {}) => {
  try {
    // Convert fetch options to proxy request format
    const proxyRequest: ProxyRequest = {
      url,
      method: options.method || 'GET',
      timeout: options.signal instanceof AbortSignal ? undefined : 30000, // 30s default timeout
    };

    // Convert headers from Headers or object to simple Record
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
    console.error('Proxy request headers:', proxyRequest.headers);

    // Handle request body
    if (options.body) {
      proxyRequest.body = typeof options.body === 'string' 
        ? options.body 
        : JSON.stringify(options.body);
    }

    console.log('Sending proxy request:', proxyRequest);
    
    // Invoke the Tauri command
    const response = await invoke<ProxyResponse>('proxy_request', { request: proxyRequest });
    
    // Handle error returned from backend
    if (response.error) {
      throw new Error(response.error);
    }

    // Create a Response-like object that mimics the fetch API Response
    const responseHeaders = new Headers();
    Object.entries(response.headers).forEach(([key, value]) => {
      responseHeaders.set(key, value);
    });

    // Utility function to get status text from status code
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

    // Create a custom Response object with methods similar to fetch API Response
    const fetchResponse = {
      status: response.status,
      statusText: getStatusText(response.status),
      ok: response.status >= 200 && response.status < 300,
      headers: responseHeaders,
      
      // Method to get response as text
      text: async () => response.body,
      
      // Method to get response as JSON
      json: async () => {
        try {
          return JSON.parse(response.body);
        } catch (e) {
          throw new Error('Failed to parse response as JSON');
        }
      },
      
      // Add additional Response-like methods as needed
    };

    return fetchResponse;
  } catch (error) {
    console.error('Tauri proxy error:', error);
    throw error;
  }
};

// Helper function to get status text from status code
function getStatusText(status: number): string {
  const statusTexts: Record<number, string> = {
    200: 'OK',
    201: 'Created',
    204: 'No Content',
    400: 'Bad Request',
    401: 'Unauthorized',
    403: 'Forbidden',
    404: 'Not Found',
    500: 'Internal Server Error',
    // Add more as needed
  };
  
  return statusTexts[status] || 'Unknown Status';
}
