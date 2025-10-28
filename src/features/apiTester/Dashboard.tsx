// Main Dashboard component for API Tester
import { useState, useEffect, useCallback } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Authentication,
  CodeGeneration,
  RequestBody,
  RequestHeaders,
  RequestParams,
  ResponseViewer,
  UrlBar
} from "./components";
import { useEnvironment } from "./hooks/useEnvironment";
import { 
  enhancedFetch, 
  parseJwt, 
  parseCurlCommand, 
  generateJsCode,
  parseCookies,
  extractAccessTokenFromCookies
} from "./utils";
import { 
  HeaderType, 
  RequestTabType,
  ResponseTabType,
  ApiResponse,
  ResponseBodyView,
  RequestBodyType,
  AuthType,
  ApiKeyLocation,
  ParsedCurl,
  CookieType
} from "./types";
import { ChevronRight, Clock, Folder, Plus, Settings } from "lucide-react";
import { Separator } from "@radix-ui/react-select";
import { Button } from "@/components/ui/button";

export default function ApiTester() {
  // Basic request state
  const [method, setMethod] = useState("GET");
  const [urlInput, setUrlInput] = useState("https://jsonplaceholder.typicode.com/posts/1");
  const [endpoint, setEndpoint] = useState("/posts/1");

  // Response state
  const [isLoading, setIsLoading] = useState(false);
  const [response, setResponse] = useState<ApiResponse | null>(null);
  const [responseTime, setResponseTime] = useState<number | null>(null);
  const [responseSize, setResponseSize] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  
  // Request body state
  const [requestBody, setRequestBody] = useState("");
  const [requestBodyType, setRequestBodyType] = useState<RequestBodyType>('json');

  // Headers and params state
  const [headers, setHeaders] = useState<HeaderType[]>([
    { id: crypto.randomUUID(), key: "Content-Type", value: "application/json", enabled: true },
  ]);
  const [params, setParams] = useState<HeaderType[]>([
    { id: crypto.randomUUID(), key: "", value: "", enabled: true },
  ]);
  const [cookies, setCookies] = useState<CookieType[]>([
    { id: crypto.randomUUID(), name: "", value: "", enabled: true },
  ]);

  // Authentication state
  const [authType, setAuthType] = useState<AuthType>('none');
  const [bearerToken, setBearerToken] = useState<string>("");
  const [basicUser, setBasicUser] = useState<string>("");
  const [basicPass, setBasicPass] = useState<string>("");
  const [apiKeyName, setApiKeyName] = useState<string>("");
  const [apiKeyValue, setApiKeyValue] = useState<string>("");
  const [apiKeyLocation, setApiKeyLocation] = useState<ApiKeyLocation>('header');
  const [userDetail, setUserDetail] = useState<string>("");
  const [cookieString, setCookieString] = useState<string>("");

  // UI state
  const [activeRequestTab, setActiveRequestTab] = useState<RequestTabType>("params");
  const [activeResponseTab, setActiveResponseTab] = useState<ResponseTabType>("body");
  const [responseBodyView, setResponseBodyView] = useState<ResponseBodyView>('pretty');
  const [generatedCurl, setGeneratedCurl] = useState<string>("");
  const [generatedJs, setGeneratedJs] = useState<string>("");

  // Use environment hook
  const { 
    environments,
    activeEnvName,
    setActiveEnvName,
    currentBaseUrl,
    setCurrentBaseUrl,
    handleEnvironmentChange,
    handleUrlInputChange: rawHandleUrlInputChange,
  } = useEnvironment();

  // Wrapper for environment change handler to include all needed values
  const handleEnvChange = useCallback((newEnvName: string) => {
    console.log(`Changing environment from ${activeEnvName} to ${newEnvName}`);
    console.log(`Current URL: ${urlInput}`);
    console.log(`Current bearer token: ${bearerToken ? '(exists)' : '(empty)'}`);
    console.log(`Authorization header: ${headers.find(h => h.key.toLowerCase() === 'authorization')?.value || 'none'}`);
    
    handleEnvironmentChange(
      newEnvName,
      urlInput, 
      bearerToken,
      headers,
      setUrlInput,
      setHeaders,
      setUserDetail
    );
  }, [activeEnvName, handleEnvironmentChange, urlInput, bearerToken, headers]);

  // Wrapper for URL input change handler
  const handleUrlInputChange = useCallback((newUrl: string) => {
    rawHandleUrlInputChange(newUrl, setUrlInput, setEndpoint);
  }, [rawHandleUrlInputChange]);

  // Handle JWT token changes
  const handleJwtChange = useCallback((token: string) => {
    setBearerToken(token);
    if (activeEnvName === "Local Dev") {
      try {
        const parsedJwt = parseJwt(token);
        if (parsedJwt) {
          setUserDetail(JSON.stringify(parsedJwt, null, 2));
          // Update x-user-detail header with parsed JWT
          const xUserDetailHeader = headers.find(h => h.key.toLowerCase() === 'x-user-detail');
          if (xUserDetailHeader) {
            setHeaders(headers.map(h => 
              h.id === xUserDetailHeader.id 
                ? { ...h, value: JSON.stringify(parsedJwt), enabled: true }
                : h
            ));
          } else {
            setHeaders([...headers, {
              id: crypto.randomUUID(),
              key: 'x-user-detail',
              value: JSON.stringify(parsedJwt),
              enabled: true
            }]);
          }
          
          // Remove Authorization and x-authorization headers when in local
          const filteredHeaders = headers.filter((header) => 
            header.key.toLowerCase() !== 'authorization' && 
            header.key.toLowerCase() !== 'x-authorization'
          );
          setHeaders(filteredHeaders);
        }
      } catch (error) {
        console.error('Error parsing JWT:', error);
      }
    } else if (activeEnvName === "Production" || activeEnvName === "No Environment") {
      // Check for existing authorization headers
      const authHeader = headers.find(h => h.key.toLowerCase() === 'authorization');
      const xAuthHeader = headers.find(h => h.key.toLowerCase() === 'x-authorization');
      
      if (xAuthHeader) {
        // If x-authorization was present, update it
        setHeaders(headers.map((h) => 
          h.id === xAuthHeader.id 
            ? { ...h, value: token, enabled: true }
            : h
        ));
      } else if (authHeader) {
        // If authorization was present, update it
        setHeaders(headers.map((h) => 
          h.id === authHeader.id 
            ? { ...h, value: token, enabled: true }
            : h
        ));
      } else {
        // If no auth header was present, add a standard Authorization header
        setHeaders([...headers, {
          id: crypto.randomUUID(),
          key: 'Authorization',
          value: token,
          enabled: true
        }]);
      }
      
      // Remove x-user-detail header in production
      const filteredHeaders = headers.filter((header) => 
        header.key.toLowerCase() !== 'x-user-detail'
      );
      setHeaders(filteredHeaders);
    }
  }, [activeEnvName, headers]);

  // --- Effects ---
  useEffect(() => {
    const activeEnv = environments.find(env => env.name === activeEnvName);
    if (activeEnv) {
      setCurrentBaseUrl(activeEnv.variables.baseUrl || "");
      if (activeEnv.variables.baseUrl && urlInput.startsWith(activeEnv.variables.baseUrl)) {
        setEndpoint(urlInput.substring(activeEnv.variables.baseUrl.length));
      } else if (!activeEnv.variables.baseUrl && !urlInput.includes("://")) {
        // No base URL, input is likely an endpoint
        setEndpoint(urlInput);
      } else if (urlInput.includes("://")) {
        // Full URL in input, extract endpoint
        try {
          const parsed = new URL(urlInput);
          setEndpoint(parsed.pathname + parsed.search + parsed.hash);
          if (!activeEnv.variables.baseUrl) { // If "No Environment", adopt this base
            setCurrentBaseUrl(parsed.origin);
          }
        } catch (e) { /* ignore invalid URL during typing */ }
      }
    }
  }, [activeEnvName, environments, urlInput]);

  // Utility to get the full URL from base and endpoint
  const getFullUrl = useCallback(() => {
    let fullUrl = "";
    if (currentBaseUrl) {
      fullUrl = currentBaseUrl.replace(/\/$/, "") + (endpoint.startsWith('/') ? endpoint : '/' + endpoint);
    } else if (endpoint.includes("://")) { // If endpoint itself is a full URL (No Environment case)
      fullUrl = endpoint;
    } else if (urlInput.includes("://")) { // Fallback to urlInput if it's a full URL
      fullUrl = urlInput;
    } else {
      return ""; // Cannot determine full URL
    }

    const activeParams = params
      .filter((param) => param.key && param.enabled)
      .map((param) => `${encodeURIComponent(param.key)}=${encodeURIComponent(param.value)}`)
      .join("&");

    if (activeParams) {
      fullUrl += (fullUrl.includes("?") ? "&" : "?") + activeParams;
    }
    return fullUrl;
  }, [currentBaseUrl, endpoint, params, urlInput]);

  // --- Event Handlers for Key-Value inputs (Headers, Params, Cookies) ---
  const handleAddItem = (type: "header" | "param" | "cookie") => {
    const newItem = { id: crypto.randomUUID(), key: "", value: "", enabled: true };
    const newCookie = { id: crypto.randomUUID(), name: "", value: "", enabled: true };
    if (type === "header") setHeaders([...headers, newItem]);
    else if (type === "param") setParams([...params, newItem]);
    else if (type === "cookie") setCookies([...cookies, newCookie]);
  };

  const handleItemChange = (
    type: "header" | "param" | "cookie",
    id: string,
    field: "key" | "value" | "enabled" | "name",
    value: string | boolean
  ) => {
    if (type === "header") {
      setHeaders((prevItems) =>
        prevItems.map((item) =>
          item.id === id ? { ...item, [field]: value } : item
        )
      );
    } else if (type === "param") {
      setParams((prevItems) =>
        prevItems.map((item) =>
          item.id === id ? { ...item, [field]: value } : item
        )
      );
    } else if (type === "cookie") {
      setCookies((prevItems) =>
        prevItems.map((item) =>
          item.id === id ? { ...item, [field]: value } : item
        )
      );
    }
  };

  const handleRemoveItem = (type: "header" | "param" | "cookie", id: string) => {
    if (type === "header") {
      setHeaders((prevItems) => prevItems.filter((item) => item.id !== id));
    } else if (type === "param") {
      setParams((prevItems) => prevItems.filter((item) => item.id !== id));
    } else if (type === "cookie") {
      setCookies((prevItems) => prevItems.filter((item) => item.id !== id));
    }
  };

  // Handle cookie string changes and extract access token
  const handleCookieStringChange = useCallback((newCookieString: string) => {
    setCookieString(newCookieString);
    
    // Parse cookies and set them in the cookies state
    const parsedCookies = parseCookies(newCookieString);
    const cookieItems: CookieType[] = Object.entries(parsedCookies).map(([name, value]) => ({
      id: crypto.randomUUID(),
      name,
      value,
      enabled: true
    }));
    setCookies(cookieItems);

    // Extract access token if present
    const accessToken = extractAccessTokenFromCookies(newCookieString);
    if (accessToken) {
      setBearerToken(accessToken);
      // Try to parse JWT if it's a JWT token
      try {
        const parsedJwt = parseJwt(accessToken);
        if (parsedJwt) {
          setUserDetail(JSON.stringify(parsedJwt, null, 2));
        }
      } catch (error) {
        console.log('Token is not a JWT or could not be parsed');
      }
    }
  }, []);

  // Generate cookie header string from cookies state
  const getCookieHeaderValue = useCallback(() => {
    return cookies
      .filter(cookie => cookie.name && cookie.enabled)
      .map(cookie => `${cookie.name}=${cookie.value}`)
      .join('; ');
  }, [cookies]);
  const generateCurl = useCallback(() => {
    const fullUrl = getFullUrl();
    if (!fullUrl) {
      setGeneratedCurl("// URL is not valid or not fully specified.");
      return;
    }
    let cmd = `curl -X ${method} "${fullUrl}" \\\n`;

    const activeHeaders = [...headers]; // Start with UI headers

    // Add auth headers
    if (authType === "bearer" && bearerToken) {
      activeHeaders.push({ id: "auth", key: "Authorization", value: bearerToken, enabled: true });
    } else if (authType === "basic" && basicUser) {
      activeHeaders.push({ id: "auth", key: "Authorization", value: `Basic ${btoa(`${basicUser}:${basicPass}`)}`, enabled: true });
    } else if (authType === "apikey" && apiKeyName && apiKeyValue && apiKeyLocation === 'header') {
      activeHeaders.push({ id: "auth", key: apiKeyName, value: apiKeyValue, enabled: true });
    }

    // Add cookie header if using cookie auth
    if (authType === "cookie" && cookies.length > 0) {
      const cookieHeader = getCookieHeaderValue();
      if (cookieHeader) {
        cmd += `  -b "${cookieHeader}" \\\n`;
      }
    }

    activeHeaders.forEach((header) => {
      if (header.key && header.enabled) {
        cmd += `  -H "${header.key}: ${header.value}" \\\n`;
      }
    });

    if (["POST", "PUT", "PATCH"].includes(method) && requestBody) {
      let bodyForCurl = requestBody;
      // If form-data, it's more complex and this simple curl gen won't be perfect
      if (requestBodyType === 'json' || requestBodyType === 'text') {
        bodyForCurl = requestBody.replace(/'/g, "'\\''"); // Escape single quotes
      }
      cmd += `  -d '${bodyForCurl}'`;
    } else {
      cmd = cmd.replace(/ \\\n$/, ""); // Remove trailing slash if no body
    }
    setGeneratedCurl(cmd);
  }, [
    method, 
    getFullUrl, 
    headers, 
    requestBody, 
    requestBodyType, 
    authType, 
    bearerToken, 
    basicUser, 
    basicPass, 
    apiKeyName, 
    apiKeyValue, 
    apiKeyLocation,
    cookies,
    getCookieHeaderValue
  ]);

  const prepareJsCode = useCallback(() => {
    const fullUrl = getFullUrl();
    if (!fullUrl) {
      setGeneratedJs("// URL is not valid or not fully specified.");
      return;
    }
    const parsed: ParsedCurl = { 
      method, 
      header: {}, 
      body: requestBodyType !== 'none' ? requestBody : undefined 
    };
    
    const tempHeaders: Record<string, string> = {};
    headers.filter(h => h.enabled && h.key).forEach(h => tempHeaders[h.key] = h.value);

    // Add auth headers
    if (authType === "bearer" && bearerToken) {
      tempHeaders["Authorization"] = bearerToken;
    } else if (authType === "basic" && basicUser) {
      tempHeaders["Authorization"] = `Basic ${btoa(`${basicUser}:${basicPass}`)}`;
    } else if (authType === "apikey" && apiKeyName && apiKeyValue && apiKeyLocation === 'header') {
      tempHeaders[apiKeyName] = apiKeyValue;
    }
    parsed.header = tempHeaders;
    setGeneratedJs(generateJsCode(parsed, fullUrl));
  }, [
    method, 
    getFullUrl, 
    headers, 
    requestBody, 
    requestBodyType, 
    authType, 
    bearerToken, 
    basicUser, 
    basicPass, 
    apiKeyName, 
    apiKeyValue, 
    apiKeyLocation
  ]);

  // Handle curl import
  const handleCurlImport = (curlCmd: string) => {
    try {
      const parsed = parseCurlCommand(curlCmd);
      if (parsed.method) setMethod(parsed.method);
      if (parsed.url) {
        setUrlInput(parsed.url); // This will trigger useEffect to set base/endpoint
        // Try to find matching environment or set to "No Environment"
        let foundEnv = false;
        for (const env of environments) {
          if (env.variables.baseUrl && parsed.url.startsWith(env.variables.baseUrl)) {
            setActiveEnvName(env.name);
            setEndpoint(parsed.url.substring(env.variables.baseUrl.length));
            foundEnv = true;
            break;
          }
        }
        if (!foundEnv) {
          setActiveEnvName("No Environment");
          try {
            const urlObj = new URL(parsed.url);
            setCurrentBaseUrl(urlObj.origin);
            setEndpoint(urlObj.pathname + urlObj.search + urlObj.hash);
          } catch {
            setEndpoint(parsed.url); // if not a full URL after all
            setCurrentBaseUrl("");
          }
        }
      }

      const importedHeaders: typeof headers = [];
      if (parsed.header) {
        for (const [key, value] of Object.entries(parsed.header)) {
          // Check for auth headers
          if (key.toLowerCase() === "authorization" || key.toLowerCase() === "x-authorization") {
            if (value.toLowerCase().startsWith("bearer ")) {
              setAuthType("bearer");
              setBearerToken(value); // Store the full token with Bearer prefix
              continue; // Don't add to regular headers
            } else if (value.toLowerCase().startsWith("basic ")) {
              setAuthType("basic");
              // Basic auth parsing is more complex, usually not stored directly in token
              try {
                const decoded = atob(value.substring(6));
                const [user, pass] = decoded.split(':');
                setBasicUser(user || "");
                setBasicPass(pass || "");
              } catch (e) { console.error("Could not decode basic auth from cURL"); }
              continue;
            }
          }
          importedHeaders.push({ id: crypto.randomUUID(), key, value, enabled: true });
        }
      }
      setHeaders(importedHeaders.length > 0 ? importedHeaders : [
        {id: crypto.randomUUID(), key: "Content-Type", value: "application/json", enabled: true}
      ]); 

      // Handle cookies from cURL
      if (parsed.cookies && Object.keys(parsed.cookies).length > 0) {
        setAuthType("cookie");
        const cookieItems: CookieType[] = Object.entries(parsed.cookies).map(([name, value]) => ({
          id: crypto.randomUUID(),
          name,
          value,
          enabled: true
        }));
        setCookies(cookieItems);
        
        // Create cookie string for display
        const cookieStr = Object.entries(parsed.cookies)
          .map(([name, value]) => `${name}=${value}`)
          .join('; ');
        setCookieString(cookieStr);

        // Extract access token if present
        const accessToken = extractAccessTokenFromCookies(cookieStr);
        if (accessToken) {
          setBearerToken(accessToken);
          try {
            const parsedJwt = parseJwt(accessToken);
            if (parsedJwt) {
              setUserDetail(JSON.stringify(parsedJwt, null, 2));
            }
          } catch (error) {
            console.log('Access token is not a JWT or could not be parsed');
          }
        }
      }

      if (parsed.body) {
        setRequestBody(parsed.body);
        // Try to guess body type
        try {
          JSON.parse(parsed.body);
          setRequestBodyType('json');
          const contentTypeHeader = headers.find(h => h.key.toLowerCase() === 'content-type');
          if (contentTypeHeader) {
            handleItemChange('header', contentTypeHeader.id, 'value', 'application/json');
          } else {
            setHeaders([...headers, {
              id: crypto.randomUUID(),
              key: 'Content-Type',
              value: 'application/json',
              enabled: true
            }]);
          }
        } catch {
          if (parsed.body.includes('&') && parsed.body.includes('=')) {
            // setRequestBodyType('form-data'); 
          } else {
            setRequestBodyType('text');
          }
        }
      } else {
        setRequestBody("");
        setRequestBodyType('none');
      }
      setError(null);
    } catch (e: any) {
      console.error("Error parsing cURL:", e);
      setError(`Error parsing cURL: ${e.message}`);
    }
  };
  
  // --- Send Request Logic ---
  const sendRequest = async () => {
    setIsLoading(true);
    setResponse(null);
    setResponseTime(null);
    setResponseSize(null);
    setError(null);

    const fullRequestUrl = getFullUrl();

    if (!fullRequestUrl) {
      setError("Invalid URL. Please check the base URL and endpoint.");
      setIsLoading(false);
      return;
    }

    const requestOptions: RequestInit = { method };
    const headerObj: Record<string, string> = {};
    
    headers.forEach((header) => {
      if (header.key && header.enabled) {
        headerObj[header.key] = header.value;
      }
    });

    // Add Authentication
    if (authType === "bearer" && bearerToken) {
      // Use the existing x-authorization header if present, otherwise use Authorization
      const hasXAuth = headers.some(h => h.key.toLowerCase() === 'x-authorization');
      if (hasXAuth) {
        headerObj["X-Authorization"] = bearerToken;
      } else {
        headerObj["Authorization"] = bearerToken;
      }
    } else if (authType === "basic" && basicUser) {
      headerObj["Authorization"] = `Basic ${btoa(`${basicUser}:${basicPass}`)}`;
    } else if (authType === "apikey" && apiKeyName && apiKeyValue && apiKeyLocation === 'header') {
      headerObj[apiKeyName] = apiKeyValue;
    } else if (authType === "cookie" && cookies.length > 0) {
      // Add cookie header
      const cookieHeader = getCookieHeaderValue();
      if (cookieHeader) {
        headerObj["Cookie"] = cookieHeader;
      }
    }
    // API Key in query params is handled by getFullUrl logic if params were updated by auth UI

    requestOptions.headers = headerObj;

    if (["POST", "PUT", "PATCH"].includes(method)) {
      if (requestBodyType === 'form-data') {
        // TODO: Implement FormData body if needed
        setError("Form-data body type not fully implemented in this simplified version yet.");
        console.warn("Form-data body type not fully implemented yet.");
      } else if (requestBody) {
        requestOptions.body = requestBody;
      }
    }
    
    const startTime = performance.now();
    try {
      console.log(`Sending request to: ${fullRequestUrl}`);
      console.log('Request options:', requestOptions);
      
      // Use enhanced fetch with proper CORS handling
      const res = await enhancedFetch(fullRequestUrl, requestOptions);
      const endTime = performance.now();
      setResponseTime(Math.round(endTime - startTime));

      const responseText = await res.text();
      setResponseSize(new Blob([responseText]).size);

      let responseData;
      const contentType = res.headers.get("content-type");
      if (contentType && contentType.includes("application/json")) {
        try { 
          responseData = JSON.parse(responseText); 
        } catch (e) { 
          responseData = responseText;
        }
      } else {
        responseData = responseText;
      }

      setResponse({
        status: res.status,
        statusText: res.statusText,
        headers: Object.fromEntries(res.headers.entries()),
        data: responseData,
        cookies: res.headers.get('set-cookie') // Basic cookie extraction
      });
    } catch (err: any) {
      setError(err.message || "An unknown error occurred");
      setResponse({ error: err.message });
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="flex h-screen bg-background text-foreground">
        <div className="w-64 border-r bg-muted/40 p-4 ">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-xl font-bold">Collections</h2>
          <Button variant="ghost" size="icon">
            <Plus className="h-4 w-4" />
          </Button>
        </div>
        <div className="space-y-2">
          <div className="flex items-center gap-2 p-2 rounded-md hover:bg-muted cursor-pointer">
            <Folder className="h-4 w-4 text-muted-foreground" />
            <span>My Collection</span>
            <ChevronRight className="h-4 w-4 ml-auto text-muted-foreground" />
          </div>
          <div className="flex items-center gap-2 p-2 rounded-md hover:bg-muted cursor-pointer">
            <Folder className="h-4 w-4 text-muted-foreground" />
            <span>API Tests</span>
            <ChevronRight className="h-4 w-4 ml-auto text-muted-foreground" />
          </div>
        </div>
        <Separator className="my-4" />
        <div className="space-y-2">
          <div className="flex items-center gap-2 p-2 rounded-md hover:bg-muted cursor-pointer">
            <Clock className="h-4 w-4 text-muted-foreground" />
            <span>History</span>
          </div>
          <div className="flex items-center gap-2 p-2 rounded-md hover:bg-muted cursor-pointer">
            <Settings className="h-4 w-4 text-muted-foreground" />
            <span>Settings</span>
          </div>
        </div>
      </div>

      {/* Main content */}
      <div className="flex-1 flex flex-col ">
        {/* Request section */}
        <div className="p-4 border-b">
          {/* URL and Method Row */}
          <UrlBar
            method={method}
            onMethodChange={setMethod}
            urlInput={urlInput}
            onUrlChange={handleUrlInputChange}
            activeEnvName={activeEnvName}
            onEnvChange={handleEnvChange}
            environments={environments}
            isLoading={isLoading}
            onSendRequest={sendRequest}
          />

          {/* Action Buttons Row */}
          <CodeGeneration
            onGenerateCurl={generateCurl}
            onPrepareJsCode={prepareJsCode}
            generatedCurl={generatedCurl}
            generatedJs={generatedJs}
            onCurlImport={handleCurlImport}
          />

          {/* Request Configuration Tabs */}
          <Tabs value={activeRequestTab} onValueChange={setActiveRequestTab as (value: string) => void} className="w-full overflow-x-auto	 scroll">
            <TabsList className="grid grid-cols-4 w-full">
              <TabsTrigger value="params">Params</TabsTrigger>
              <TabsTrigger value="headers">Headers</TabsTrigger>
              <TabsTrigger value="body">Body</TabsTrigger>
              <TabsTrigger value="auth">Auth</TabsTrigger>
            </TabsList>

            {/* Params Tab */}
            <TabsContent value="params" className="mt-2 overflow-x-auto	">
              <RequestParams
                params={params}
                onAddParam={() => handleAddItem("param")}
                onParamChange={(id, field, value) => handleItemChange("param", id, field, value)}
                onRemoveParam={(id) => handleRemoveItem("param", id)}
              />
            </TabsContent>

            {/* Headers Tab */}
            <TabsContent value="headers" className="mt-2">
              <RequestHeaders
                headers={headers}
                onAddHeader={() => handleAddItem("header")}
                onHeaderChange={(id, field, value) => handleItemChange("header", id, field, value)}
                onRemoveHeader={(id) => handleRemoveItem("header", id)}
              />
            </TabsContent>

            {/* Body Tab */}
            <TabsContent value="body" className="mt-2">
              <RequestBody
                bodyType={requestBodyType}
                body={requestBody}
                onBodyChange={setRequestBody}
                onBodyTypeChange={setRequestBodyType}
              />
            </TabsContent>

            {/* Auth Tab */}
            <TabsContent value="auth" className="mt-2">
              <Authentication
                authType={authType}
                onAuthTypeChange={setAuthType}
                bearerToken={bearerToken}
                onBearerTokenChange={setBearerToken}
                basicUser={basicUser}
                onBasicUserChange={setBasicUser}
                basicPass={basicPass}
                onBasicPassChange={setBasicPass}
                apiKeyName={apiKeyName}
                onApiKeyNameChange={setApiKeyName}
                apiKeyValue={apiKeyValue}
                onApiKeyValueChange={setApiKeyValue}
                apiKeyLocation={apiKeyLocation}
                onApiKeyLocationChange={setApiKeyLocation}
                userDetail={userDetail}
                onJwtTokenChange={handleJwtChange}
                cookieString={cookieString}
                onCookieStringChange={handleCookieStringChange}
                cookies={cookies}
                onAddCookie={() => handleAddItem("cookie")}
                onCookieChange={(id, field, value) => handleItemChange("cookie", id, field, value)}
                onRemoveCookie={(id) => handleRemoveItem("cookie", id)}
              />
            </TabsContent>
          </Tabs>
        </div>

        {/* Response section */}
        <ResponseViewer
          isLoading={isLoading}
          response={response}
          error={error}
          responseTime={responseTime}
          responseSize={responseSize}
          activeTab={activeResponseTab}
          onTabChange={setActiveResponseTab}
          bodyView={responseBodyView}
          onBodyViewChange={setResponseBodyView}
        />
      </div>
    </div>
  );
}
