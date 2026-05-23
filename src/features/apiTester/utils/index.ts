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
    const jsonPayload = decodeURIComponent(
      atob(base64).split('').map(c =>
        '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2)
      ).join('')
    );
    return JSON.parse(jsonPayload);
  } catch {
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

function shellTokenize(s: string): string[] {
  const tokens: string[] = [];
  let i = 0;
  while (i < s.length) {
    while (i < s.length && /\s/.test(s[i])) i++;
    if (i >= s.length) break;
    let token = '';
    while (i < s.length) {
      if (s[i] === "'") {
        i++;
        while (i < s.length && s[i] !== "'") token += s[i++];
        if (i < s.length) i++;
      } else if (s[i] === '"') {
        i++;
        while (i < s.length && s[i] !== '"') {
          if (s[i] === '\\' && i + 1 < s.length) { i++; token += s[i++]; }
          else token += s[i++];
        }
        if (i < s.length) i++;
      } else if (s[i] === '\\' && i + 1 < s.length) {
        i++; token += s[i++];
      } else if (/\s/.test(s[i])) {
        break;
      } else {
        token += s[i++];
      }
    }
    tokens.push(token);
  }
  return tokens;
}

export function parseCurlCommand(curlCmd: string): ParsedCurl {
  const result: ParsedCurl = { header: {}, cookies: {} };

  let cmd = curlCmd.replace(/\\\r\n/g, ' ').replace(/\\\n/g, ' ').trim();
  if (/^curl\s/i.test(cmd)) cmd = cmd.replace(/^curl\s+/i, '');

  const tokens = shellTokenize(cmd);
  let i = 0;
  const next = () => tokens[++i];

  while (i < tokens.length) {
    const tok = tokens[i];

    if (tok === '-X' || tok === '--request') { result.method = next()?.toUpperCase(); i++; continue; }
    if (tok === '--url') { result.url = next(); i++; continue; }

    if (tok === '-H' || tok === '--header') {
      const raw = next() ?? '';
      i++;
      const colon = raw.indexOf(':');
      if (colon !== -1) {
        const key = raw.slice(0, colon).trim();
        const value = raw.slice(colon + 1).trim();
        if (key && result.header) result.header[key] = value;
      }
      continue;
    }

    if (tok === '-b' || tok === '--cookie') {
      const raw = next() ?? '';
      i++;
      const parsed = parseCookies(raw);
      result.cookies = { ...result.cookies, ...parsed };
      continue;
    }

    if (tok === '-d' || tok === '--data' || tok === '--data-raw' || tok === '--data-binary' || tok === '--data-urlencode') {
      result.body = next() ?? '';
      i++;
      continue;
    }

    if (
      tok === '-u' || tok === '--user' || tok === '-A' || tok === '--user-agent' ||
      tok === '-m' || tok === '--max-time' || tok === '--connect-timeout' ||
      tok === '-o' || tok === '--output' || tok === '-e' || tok === '--referer' ||
      tok === '-F' || tok === '--form' || tok === '--proxy' || tok === '-x'
    ) {
      next(); i++; continue;
    }

    if (tok === '-G' || tok === '--get') { result.method = 'GET'; i++; continue; }
    if (tok === '-I' || tok === '--head') { result.method = 'HEAD'; i++; continue; }

    if (tok.startsWith('-')) { i++; continue; }

    if (!result.url && (tok.includes('://') || tok.startsWith('http'))) {
      result.url = tok;
    }
    i++;
  }

  if (!result.method) {
    result.method = result.body ? 'POST' : 'GET';
  }

  return result;
}

export function generateJsCode(parsed: ParsedCurl, fullUrl: string): string {
  const method = parsed.method || 'GET';
  const headers = parsed.header || {};
  const hasBody = !!parsed.body && ['POST', 'PUT', 'PATCH', 'DELETE'].includes(method);

  const headerEntries = Object.entries(headers);
  const headerStr = headerEntries.length > 0
    ? `\n  headers: {\n${headerEntries.map(([k, v]) => `    '${k}': '${v.replace(/'/g, "\\'")}'`).join(',\n')},\n  },`
    : '';

  let bodyStr = '';
  if (hasBody && parsed.body) {
    try {
      JSON.parse(parsed.body);
      bodyStr = `\n  body: JSON.stringify(${parsed.body}),`;
    } catch {
      bodyStr = `\n  body: \`${parsed.body.replace(/`/g, '\\`').replace(/\${/g, '\\${')}\`,`;
    }
  }

  return `const response = await fetch('${fullUrl.replace(/'/g, "\\'")}', {\n  method: '${method}',${headerStr}${bodyStr}\n});\nconst data = await response.json();\nconsole.log(data);`;
}

export const beautifyJson = (jsonStr: string) => {
  try {
    return JSON.stringify(JSON.parse(jsonStr), null, 2);
  } catch {
    return jsonStr;
  }
};

export const enhancedFetch = async (
  url: string,
  options: RequestInit,
  formFields?: ProxyFormField[],
  tabId?: string,
) => tauriProxyFetch(url, options, formFields, tabId);
