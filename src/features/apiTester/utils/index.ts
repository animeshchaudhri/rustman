import { ParsedCurl } from "../types";
import { tauriProxyFetch, ProxyFormField } from "./tauriProxy";

export { tauriProxyFetch };
export type { ProxyFormField };

 
export function replaceVariables(str: string, vars: Record<string, string>): string {
  return str.replace(/\{\{(\w+)\}\}/g, (match, key) => vars[key] ?? match);
}

 
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

 
export function parseCookies(cookieString: string): Record<string, string> {
  const cookies: Record<string, string> = {};
  if (!cookieString) return cookies;
  
  cookieString.split(';').forEach(cookie => {
    const [name, ...rest] = cookie.trim().split('=');
    if (name && rest.length > 0) {
      const raw = rest.join('=').trim();
      let value = raw;
      try { value = decodeURIComponent(raw); } catch { value = raw; }
      cookies[name.trim()] = value;
    }
  });
  
  return cookies;
}

 
export function extractAccessTokenFromCookies(cookieString: string): string | null {
  const cookies = parseCookies(cookieString);
  return cookies.accessToken || null;
}

 
function shellTokenize(str: string): string[] {
  const tokens: string[] = [];
  let i = 0;
  const len = str.length;

  while (i < len) {
    
    while (i < len && /\s/.test(str[i])) i++;
    if (i >= len) break;

    const q = str[i];
    if (q === "'" || q === '"') {
      i++; 
      let token = "";
      while (i < len && str[i] !== q) {
        if (q === '"' && str[i] === '\\' && i + 1 < len) {
          i++;
          token += str[i++];
        } else {
          token += str[i++];
        }
      }
      if (i < len) i++; 
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

function fixCurlJsonHeaders(cmd: string): string {
  const out: string[] = [];
  let i = 0;
  const len = cmd.length;

  while (i < len) {
    const isShort = cmd[i] === '-' && i + 1 < len && cmd[i + 1] === 'H';
    const isLong = cmd.slice(i, i + 8) === '--header';

    if (!isShort && !isLong) {
      out.push(cmd[i++]);
      continue;
    }

    const flagEnd = isShort ? i + 2 : i + 8;
    out.push(cmd.slice(i, flagEnd));
    i = flagEnd;

    while (i < len && (cmd[i] === ' ' || cmd[i] === '\t')) out.push(cmd[i++]);

    const q = i < len ? cmd[i] : '';
    if (q !== '"' && q !== "'") continue;

    out.push(q);
    i++;

    let colonIdx = -1;
    for (let k = i; k < len; k++) {
      if (cmd[k] === q || cmd[k] === '\n') break;
      if (cmd[k] === ':') { colonIdx = k; break; }
    }

    if (colonIdx < 0) {
      while (i < len && cmd[i] !== q && cmd[i] !== '\n') out.push(cmd[i++]);
      if (i < len && cmd[i] === q) { out.push(q); i++; }
      continue;
    }

    out.push(cmd.slice(i, colonIdx + 1));
    i = colonIdx + 1;

    while (i < len && cmd[i] === ' ') out.push(cmd[i++]);

    if (i >= len || (cmd[i] !== '{' && cmd[i] !== '[')) {
      while (i < len && cmd[i] !== q && cmd[i] !== '\n') {
        if (cmd[i] === '\\' && i + 1 < len) { out.push(cmd[i++]); out.push(cmd[i++]); }
        else out.push(cmd[i++]);
      }
      if (i < len && cmd[i] === q) { out.push(q); i++; }
      continue;
    }

    let depth = 0;
    let inStr = false;
    let esc = false;
    const jsonStart = i;

    while (i < len) {
      const ch = cmd[i];
      if (esc) {
        esc = false;
      } else if (ch === '\\') {
        if (inStr) esc = true;
      } else if (ch === '"') {
        inStr = !inStr;
      } else if (!inStr) {
        if (ch === '{' || ch === '[') depth++;
        else if (ch === '}' || ch === ']') { depth--; if (depth === 0) { i++; break; } }
      }
      i++;
    }

    if (depth !== 0) {
      out.push(cmd.slice(jsonStart, i));
      while (i < len && cmd[i] !== q && cmd[i] !== '\n') i++;
      if (i < len && cmd[i] === q) i++;
      out.push(q);
      continue;
    }

    const jsonContent = cmd.slice(jsonStart, i);
    const escaped = q === '"' ? jsonContent.replace(/"/g, '\\"') : jsonContent;
    out.push(escaped, q);

    while (i < len && cmd[i] !== ' ' && cmd[i] !== '\t' && cmd[i] !== '\\' && cmd[i] !== '\n') {
      if (cmd[i] === q) { i++; break; }
      i++;
    }
  }

  return out.join('');
}

 
export function parseCurlCommand(curlCmd: string): ParsedCurl {
  const result: ParsedCurl = { header: {}, cookies: {} };

  
  let cmd = curlCmd
    .replace(/\\\r\n/g, " ")
    .replace(/\\\n/g, " ")
    .trim();

  cmd = fixCurlJsonHeaders(cmd);

  
  if (/^curl\s/i.test(cmd)) cmd = cmd.replace(/^curl\s+/i, "");

  
  const tokens = shellTokenize(cmd);

  
  let i = 0;
  const next = () => tokens[++i];

  while (i < tokens.length) {
    const tok = tokens[i];

    
    if (tok === "-X" || tok === "--request") {
      result.method = next()?.toUpperCase();
      i++;
      continue;
    }

    
    if (tok === "--url") {
      result.url = next();
      i++;
      continue;
    }

    
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

    
    if (tok === "-b" || tok === "--cookie") {
      const raw = next() ?? "";
      i++;
      const parsed = parseCookies(raw);
      result.cookies = { ...result.cookies, ...parsed };
      continue;
    }

    
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

    
    if (tok === "-u" || tok === "--user") {
      next();
      i++;
      continue;
    }

    
    if (tok === "-F" || tok === "--form") {
      next();
      i++;
      continue;
    }

    
    if (tok === "-s" || tok === "--silent" ||
        tok === "-v" || tok === "--verbose" ||
        tok === "-L" || tok === "--location" ||
        tok === "-k" || tok === "--insecure" ||
        tok === "-i" || tok === "--include" ||
        tok === "--compressed") {
      i++;
      continue;
    }

    
    if (tok.startsWith("-") && tok.includes("=")) {
      i++;
      continue;
    }

    
    if (tok.startsWith("-")) {
      i++;
      continue;
    }

    
    if (!result.url && (tok.includes("://") || tok.startsWith("http"))) {
      result.url = tok;
    }

    i++;
  }

  
  if (!result.method) {
    result.method = result.body ? "POST" : "GET";
  }

  return result;
}

 
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
    code += "    const data = await response.json();\n";
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

 
export const beautifyJson = (jsonStr: string) => {
  try {
    const obj = JSON.parse(jsonStr);
    return JSON.stringify(obj, null, 2);
  } catch (e) {
    return jsonStr; 
  }
};

 
export const enhancedFetch = async (url: string, options: RequestInit, formFields?: ProxyFormField[], tabId?: string) => {
  try {
    const response = await tauriProxyFetch(url, options, formFields, tabId);
    return response;
  } catch (error) {
    console.error('Enhanced fetch error:', error);
    throw error;
  }
};
