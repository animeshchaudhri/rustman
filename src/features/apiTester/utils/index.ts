// Utility functions for API Tester
import { ParsedCurl } from "../types";
import { tauriProxyFetch } from "./tauriProxy";

// Export tauriProxyFetch so it can be used elsewhere
export { tauriProxyFetch };

/**
 * Replaces {{variable}} placeholders with values from an env vars map.
 * Unresolved variables are left as-is.
 */
export function replaceVariables(str: string, vars: Record<string, string>): string {
  return str.replace(/\{\{(\w+)\}\}/g, (match, key) => vars[key] ?? match);
}

/**
 * Parses a JWT token and returns the payload as an object
 */
export function parseJwt(token: string) {
  try {
    const base64Url = token.split('.')[1];
    const base64 = base64Url.replace(/-/g, '+').replace(/_/g, '/');
    const jsonPayload = decodeURIComponent(atob(base64).split('').map(c => 
      '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2)
    ).join(''));
    return JSON.parse(jsonPayload);
  } catch (e) {
    console.error('Failed to parse JWT:', e);
    return null;
  }
}

/**
 * Parses cookies from a cookie string and returns an object
 */
export function parseCookies(cookieString: string): Record<string, string> {
  const cookies: Record<string, string> = {};
  if (!cookieString) return cookies;
  
  cookieString.split(';').forEach(cookie => {
    const [name, ...rest] = cookie.trim().split('=');
    if (name && rest.length > 0) {
      cookies[name.trim()] = rest.join('=').trim();
    }
  });
  
  return cookies;
}

/**
 * Extracts access token from cookies
 */
export function extractAccessTokenFromCookies(cookieString: string): string | null {
  const cookies = parseCookies(cookieString);
  return cookies.accessToken || null;
}

/**
 * Tokenizes a shell-like string respecting single-quoted, double-quoted, and
 * unquoted tokens. Returns an array of unquoted token strings.
 */
function shellTokenize(str: string): string[] {
  const tokens: string[] = [];
  let i = 0;
  const len = str.length;

  while (i < len) {
    // Skip whitespace
    while (i < len && /\s/.test(str[i])) i++;
    if (i >= len) break;

    const q = str[i];
    if (q === "'" || q === '"') {
      i++; // skip opening quote
      let token = "";
      while (i < len && str[i] !== q) {
        if (q === '"' && str[i] === '\\' && i + 1 < len) {
          i++;
          token += str[i++];
        } else {
          token += str[i++];
        }
      }
      if (i < len) i++; // skip closing quote
      tokens.push(token);
    } else {
      let token = "";
      while (i < len && !/\s/.test(str[i])) {
        if (str[i] === '\\' && i + 1 < len) {
          i++;
          token += str[i++];
        } else {
          token += str[i++];
        }
      }
      tokens.push(token);
    }
  }
  return tokens;
}

/**
 * Parses a cURL command (including multi-line with \ continuations) and returns
 * an object with method, URL, headers, body, and cookies.
 */
export function parseCurlCommand(curlCmd: string): ParsedCurl {
  const result: ParsedCurl = { header: {}, cookies: {} };

  // 1. Collapse backslash-newline continuations (handles both \n and \r\n)
  let cmd = curlCmd
    .replace(/\\\r\n/g, " ")
    .replace(/\\\n/g, " ")
    .trim();

  // 2. Strip leading "curl" token
  if (/^curl\s/i.test(cmd)) cmd = cmd.replace(/^curl\s+/i, "");

  // 3. Tokenize (respects quoted strings)
  const tokens = shellTokenize(cmd);

  // 4. Walk token pairs
  let i = 0;
  const next = () => tokens[++i];

  while (i < tokens.length) {
    const tok = tokens[i];

    // Method
    if (tok === "-X" || tok === "--request") {
      result.method = next()?.toUpperCase();
      i++;
      continue;
    }

    // URL via --url flag
    if (tok === "--url") {
      result.url = next();
      i++;
      continue;
    }

    // Headers
    if (tok === "-H" || tok === "--header") {
      const raw = next() ?? "";
      i++;
      const colon = raw.indexOf(":");
      if (colon !== -1) {
        const key = raw.slice(0, colon).trim();
        const value = raw.slice(colon + 1).trim();
        if (key && result.header) result.header[key] = value;
      }
      continue;
    }

    // Cookies  (-b / --cookie)
    if (tok === "-b" || tok === "--cookie") {
      const raw = next() ?? "";
      i++;
      const parsed = parseCookies(raw);
      result.cookies = { ...result.cookies, ...parsed };
      continue;
    }

    // Body  (-d / --data / --data-raw / --data-binary / --data-urlencode)
    if (
      tok === "-d" ||
      tok === "--data" ||
      tok === "--data-raw" ||
      tok === "--data-binary" ||
      tok === "--data-urlencode"
    ) {
      result.body = next() ?? "";
      i++;
      continue;
    }

    // User (-u / --user) — skip value
    if (tok === "-u" || tok === "--user") {
      next();
      i++;
      continue;
    }

    // Form data (-F / --form) — skip value for now
    if (tok === "-F" || tok === "--form") {
      next();
      i++;
      continue;
    }

    // Flags without values that we safely skip
    if (tok === "-s" || tok === "--silent" ||
        tok === "-v" || tok === "--verbose" ||
        tok === "-L" || tok === "--location" ||
        tok === "-k" || tok === "--insecure" ||
        tok === "-i" || tok === "--include" ||
        tok === "--compressed") {
      i++;
      continue;
    }

    // Any unrecognised flag with = syntax (e.g. --output=file) — skip
    if (tok.startsWith("-") && tok.includes("=")) {
      i++;
      continue;
    }

    // Any other unrecognised flag — skip (and its potential value)
    if (tok.startsWith("-")) {
      i++;
      continue;
    }

    // Bare token — treat as URL if it looks like one and we don't have a URL yet
    if (!result.url && (tok.includes("://") || tok.startsWith("http"))) {
      result.url = tok;
    }

    i++;
  }

  // 5. Infer method
  if (!result.method) {
    result.method = result.body ? "POST" : "GET";
  }

  return result;
}

/**
 * Generates JavaScript code from a parsed cURL object and URL
 */
export function generateJsCode(parsed: ParsedCurl, fullUrl: string): string {
  try {
    let code = "// Generated JavaScript fetch code\n";
    code += "async function fetchData() {\n";
    code += "  const url = \"" + fullUrl.replace(/"/g, '\\"') + "\";\n";
    code += "  const options = {\n";
    code += `    method: "${parsed.method || 'GET'}",\n`;

    if (parsed.header && Object.keys(parsed.header).length > 0) {
      code += "    headers: {\n";
      for (const [key, value] of Object.entries(parsed.header)) {
        code += `      "${key.replace(/"/g, '\\"')}": "${value.replace(/"/g, '\\"')}",\n`;
      }
      code += "    },\n";
    }

    if (parsed.body && (parsed.method === 'POST' || parsed.method === 'PUT' || parsed.method === 'PATCH')) {
      code += "    body: ";
      try {
        JSON.parse(parsed.body); // Check if it's valid JSON
        code += `JSON.stringify(${parsed.body})`; // If JSON, stringify the original string literal
      } catch (e) {
        code += `\`${parsed.body.replace(/`/g, '\\`').replace(/\${/g, '\\${')}\``; // As template literal
      }
      code += ",\n";
    }

    code += "  };\n\n";
    code += "  try {\n";
    code += "    const response = await fetch(url, options);\n";
    code += "    if (!response.ok) {\n";
    code += "      throw new Error(`HTTP error! status: ${response.status}`);\n";
    code += "    }\n";
    code += "    const data = await response.json(); // or response.text() for non-JSON\n";
    code += "    console.log(data);\n";
    code += "    return data;\n";
    code += "  } catch (error) {\n";
    code += "    console.error('Error fetching data:', error);\n";
    code += "  }\n";
    code += "}\n\n";
    code += "fetchData();\n";

    return code;
  } catch (e: any) {
    return `// Error generating JavaScript code: ${e.message}\n// Original parsed: ${JSON.stringify(parsed)}`;
  }
}

/**
 * Beautifies JSON string with proper indentation
 */
export const beautifyJson = (jsonStr: string) => {
  try {
    const obj = JSON.parse(jsonStr);
    return JSON.stringify(obj, null, 2);
  } catch (e) {
    return jsonStr; // Return original if not valid JSON
  }
};

/**
 * Enhanced fetch function that bypasses CORS by using the Tauri backend proxy
 * This implementation replaces the browser-based fetch with a Tauri command
 */
export const enhancedFetch = async (url: string, options: RequestInit) => {
  try {
    console.log('Enhanced fetch using Tauri proxy for request to:', url);
    console.log('Request options:', options);
    
    // Use the Tauri proxy fetch implementation which goes through our Rust backend
    // This bypasses CORS entirely since requests are made from the desktop app
    const response = await tauriProxyFetch(url, options);
    
    return response;
  } catch (error) {
    console.error('Enhanced fetch error:', error);
    throw error;
  }
};
