// Utility functions for API Tester
import { ParsedCurl } from "../types";
import { tauriProxyFetch } from "./tauriProxy";

// Export tauriProxyFetch so it can be used elsewhere
export { tauriProxyFetch };

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
 * Parses a cURL command and returns an object with method, URL, headers, and body
 */
export function parseCurlCommand(curlCmd: string): ParsedCurl {
  const result: ParsedCurl = { header: {} };
  let cmd = curlCmd.trim();
  if (cmd.startsWith('curl ')) {
    cmd = cmd.substring(5);
  }

  const methodMatch = cmd.match(/-X\s+([^\s]+)/i);
  if (methodMatch) {
    result.method = methodMatch[1].toUpperCase();
  } else if (cmd.includes(' --data') || cmd.includes(' -d ') || cmd.includes('--data-raw')) {
    result.method = 'POST';
  } else {
    result.method = 'GET';
  }

  // URL: Find the first argument that is not a flag or a value for a flag that expects one
  const parts = cmd.match(/(?:[^\s"']+|"[^"]*"|'[^']*')+/g) || [];
  let urlFound = false;
  for (let i = 0; i < parts.length; i++) {
    const part = parts[i];
    if (part.startsWith('-')) {
      // Skip flags and their potential values if they are known to take one
      const flag = part.match(/^(-[a-zA-Z]|--[a-zA-Z-]+)/)?.[0];
      const flagsWithValue = ['-X', '--request', '-H', '--header', '-d', '--data', '--data-raw', '-u', '--user', '--url'];
      if (flag && flagsWithValue.includes(flag) && i + 1 < parts.length) {
        i++; // Skip next part as it's a value for this flag
      }
      continue;
    }
    if (part.includes('://') || (part.startsWith('http') && !parts[i-1]?.startsWith('-'))) {
      result.url = part.replace(/^["']|["']$/g, '');
      urlFound = true;
      break;
    }
  }
   if (!urlFound) { // Fallback for URLs without protocol, assuming it's the first non-flag
    for (const part of parts) {
        if (!part.startsWith('-')) {
            result.url = part.replace(/^["']|["']$/g, '');
            break;
        }
    }
   }

  const headerRegex = /(-H|--header)\s*(['"])(.+?)\2/g;
  let match;
  while ((match = headerRegex.exec(cmd)) !== null) {
    const headerLine = match[3];
    const [name, ...valueParts] = headerLine.split(/:\s*/);
    const value = valueParts.join(':').trim();
    if (name && value && result.header) {
      result.header[name.trim()] = value;
    }
  }

  const dataMatch = cmd.match(/(-d|--data|--data-raw)\s*(['"])([\s\S]*?)\2/s) || cmd.match(/(-d|--data|--data-raw)\s+'([\s\S]*?)'/s) || cmd.match(/(-d|--data|--data-raw)\s+([^'"\s][\s\S]*)/s) ;

  if (dataMatch) {
    result.body = dataMatch[3] || dataMatch[2] || dataMatch[1];
    if (dataMatch[0].includes("--data-raw")){
        // keep as is
    } else if (dataMatch[0].includes("-d") || dataMatch[0].includes("--data")){
        // if it's urlencoded, it might need decoding, but for now, we'll keep it as is
        // as Postman also imports it as raw if it's not clearly JSON.
    }
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
