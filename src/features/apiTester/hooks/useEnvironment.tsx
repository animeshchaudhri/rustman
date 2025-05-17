// Custom hook for handling API environment functionality
import { useState, useCallback } from "react";
import { Environment, HeaderType } from "../types";
import { parseJwt } from "../utils";

export function useEnvironment() {
  // Environment state
  const [environments] = useState<Environment[]>([
    { name: "No Environment", variables: { baseUrl: "" } },
    { name: "Production", variables: { baseUrl: "https://jsonplaceholder.typicode.com" } },
    { name: "Local Dev", variables: { baseUrl: "http://localhost:3000" } },
  ]);
  const [activeEnvName, setActiveEnvName] = useState<string>("Production");
  const [currentBaseUrl, setCurrentBaseUrl] = useState<string>("");

  // Automatically convert URLs between environments
  const convertUrl = useCallback((url: string, fromEnvName: string, toEnvName: string) => {
    const fromEnv = environments.find(env => env.name === fromEnvName);
    const toEnv = environments.find(env => env.name === toEnvName);
    
    // If the target environment doesn't have a baseUrl, just return the original URL
    if (!toEnv?.variables.baseUrl) return url;
    
    // If coming from an environment with a baseUrl and URL starts with it
    if (fromEnv?.variables.baseUrl && url.startsWith(fromEnv.variables.baseUrl)) {
      const endpoint = url.substring(fromEnv.variables.baseUrl.length);
      return toEnv.variables.baseUrl + endpoint;
    } 
    // Handle case where we're coming from "No Environment" or URL doesn't match fromEnv baseUrl
    else {
      try {
        // Try to parse the URL to extract just the path and query
        const parsedUrl = new URL(url);
        const pathAndQuery = parsedUrl.pathname + parsedUrl.search + parsedUrl.hash;
        return toEnv.variables.baseUrl + pathAndQuery;
      } catch (e) {
        // If URL parsing fails, it might be just a path already
        const path = url.startsWith('/') ? url : '/' + url;
        return toEnv.variables.baseUrl + path;
      }
    }
  }, [environments]);

  // Handle environment change with URL conversion
  const handleEnvironmentChange = useCallback((
    newEnvName: string,
    urlInput: string,
    bearerToken: string,
    headers: HeaderType[],
    setUrlInput: (url: string) => void,
    setHeaders: (headers: HeaderType[]) => void,
    setUserDetail: (detail: string) => void
  ) => {
    const oldEnvName = activeEnvName;
    setActiveEnvName(newEnvName);
    
    // Convert the current URL to the new environment
    const newUrl = convertUrl(urlInput, oldEnvName, newEnvName);
    setUrlInput(newUrl);

    // Handle any Authorization or X-Authorization header that might exist
    const authHeader = headers.find(h => h.key.toLowerCase() === 'authorization');
    const xAuthHeader = headers.find(h => h.key.toLowerCase() === 'x-authorization');
    let existingToken = '';
    let tokenSource = '';
    
    // Extract token from Authorization header if it exists
    if (authHeader && authHeader.value.toLowerCase().startsWith('bearer ')) {
      existingToken = authHeader.value.substring(7);
      tokenSource = 'authorization';
    } 
    // Extract token from X-Authorization header if it exists
    else if (xAuthHeader && xAuthHeader.value.toLowerCase().startsWith('bearer ')) {
      existingToken = xAuthHeader.value.substring(7);
      tokenSource = 'x-authorization';
    } else if (xAuthHeader) {
      // If there's an x-authorization header without 'Bearer ' prefix
      existingToken = xAuthHeader.value;
      tokenSource = 'x-authorization';
    } else if (authHeader) {
      // If there's an authorization header without 'Bearer ' prefix
      existingToken = authHeader.value;
      tokenSource = 'authorization';
    }
    
    // Use the passed bearer token or the one from the header
    const token = bearerToken || existingToken;

    // If switching to local, add x-user-detail but don't remove the original token
    if (newEnvName === "Local Dev") {
      try {
        let parsedData = null;
        if (token) {
          // Parse the JWT if we have a token
          parsedData = parseJwt(token);
          setUserDetail(JSON.stringify(parsedData, null, 2));
        }
        
        // Create a new headers array for modification
        let updatedHeaders = [...headers];
        
        if (parsedData) {
          // Find and update or add x-user-detail header
          const xUserDetailHeader = headers.find(h => h.key.toLowerCase() === 'x-user-detail');
          if (xUserDetailHeader) {
            updatedHeaders = updatedHeaders.map((h) => 
              h.id === xUserDetailHeader.id 
                ? { ...h, value: JSON.stringify(parsedData), enabled: true }
                : h
            );
          } else {
            updatedHeaders.push({
              id: crypto.randomUUID(),
              key: 'x-user-detail',
              value: JSON.stringify(parsedData),
              enabled: true
            });
          }
        } else if (oldEnvName === "No Environment") {
          // If no token and coming from No Environment, add an empty x-user-detail header
          updatedHeaders.push({
            id: crypto.randomUUID(),
            key: 'x-user-detail',
            value: '{}',
            enabled: true
          });
        }
        
        // IMPORTANT: Don't remove the x-authorization header, we want to keep it
        // Only remove the authorization header as we prefer x-authorization in local
        if (tokenSource !== 'x-authorization') {
          // Remove only the standard Authorization header if we have an x-authorization
          updatedHeaders = updatedHeaders.filter(h => 
            h.key.toLowerCase() !== 'authorization'
          );
          
          // If we have a token but no x-authorization header, add it
          if (token && !xAuthHeader) {
            updatedHeaders.push({
              id: crypto.randomUUID(),
              key: 'X-Authorization',
              value: token,
              enabled: true
            });
          }
        }
        
        setHeaders(updatedHeaders);
      } catch (error) {
        console.error('Error parsing JWT:', error);
      }
    } 
    // If switching to production or no environment, use bearer token if available
    else if ((newEnvName === "Production" || newEnvName === "No Environment") && token) {
      // Handle authorization header based on which one was present originally
      if (xAuthHeader || tokenSource === 'x-authorization') {
        // If x-authorization was present or preferred, update it or keep using it
        const updatedHeaders = headers.map((h) => 
          h.key.toLowerCase() === 'x-authorization'
            ? { ...h, value: token, enabled: true }
            : h
        );
        
        // Remove x-user-detail header in production
        const filteredHeaders = updatedHeaders.filter((header) => 
          header.key.toLowerCase() !== 'x-user-detail'
        );
        
        setHeaders(filteredHeaders);
      } else {
        // Otherwise use standard Authorization header
        const updatedHeaders = authHeader 
          ? headers.map((h) => 
              h.id === authHeader.id 
                ? { ...h, value: token, enabled: true } 
                : h
            )
          : [...headers, {
              id: crypto.randomUUID(),
              key: 'Authorization',
              value: token,
              enabled: true
            }];
        
        // Remove x-user-detail header in production
        const filteredHeaders = updatedHeaders.filter((header) => 
          header.key.toLowerCase() !== 'x-user-detail'
        );
        
        setHeaders(filteredHeaders);
      }
    }
  }, [activeEnvName, convertUrl]);

  // Handle URL input changes with environment detection
  const handleUrlInputChange = useCallback((
    newUrl: string, 
    setUrlInput: (url: string) => void, 
    setEndpoint: (endpoint: string) => void
  ) => {
    setUrlInput(newUrl);
    try {
      const url = new URL(newUrl);
      const newEndpoint = url.pathname + url.search;
      setEndpoint(newEndpoint);
      
      // Auto-detect environment based on the URL
      const matchingEnv = environments.find(env => 
        env.variables.baseUrl && url.origin === new URL(env.variables.baseUrl).origin
      );
      
      // Update current base URL regardless of environment change
      if (matchingEnv) {
        setCurrentBaseUrl(matchingEnv.variables.baseUrl);
      } else {
        // If no matching environment, set to the origin of the current URL
        setCurrentBaseUrl(url.origin);
      }
      
      console.log('URL changed, matching environment:', matchingEnv?.name);
    } catch (e) {
      // If URL is not valid, just update the input
      setEndpoint(newUrl);
    }
  }, [environments]);

  return {
    environments,
    activeEnvName,
    setActiveEnvName,
    currentBaseUrl,
    setCurrentBaseUrl,
    convertUrl,
    handleEnvironmentChange,
    handleUrlInputChange,
  };
}
