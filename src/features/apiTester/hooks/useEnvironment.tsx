import { useCallback, useMemo } from "react";

import type { AppEnvironment, HeaderType } from "../types";
import { parseJwt } from "../utils";

interface UseEnvironmentOptions {
  environments: AppEnvironment[];
  activeEnvName: string;
  envEnabled: boolean;
}

const isLocalEnvironment = (environment?: AppEnvironment) => {
  if (!environment) {
    return false;
  }

  const baseUrl = environment.variables.baseUrl ?? "";
  return environment.name === "Local Dev" || /localhost|127\.0\.0\.1/i.test(baseUrl);
};

export function useEnvironment({ environments, activeEnvName, envEnabled }: UseEnvironmentOptions) {
  const activeEnvironment = useMemo(
    () => environments.find((environment) => environment.name === activeEnvName) ?? null,
    [activeEnvName, environments],
  );

  const convertUrl = useCallback(
    (url: string, fromEnvName: string, toEnvName: string) => {
      if (!envEnabled) {
        return url;
      }

      const fromEnv = environments.find((environment) => environment.name === fromEnvName);
      const toEnv = environments.find((environment) => environment.name === toEnvName);
      const targetBaseUrl = toEnv?.variables.baseUrl ?? "";

      if (!targetBaseUrl) {
        return url;
      }

      if (fromEnv?.variables.baseUrl && url.startsWith(fromEnv.variables.baseUrl)) {
        const endpoint = url.substring(fromEnv.variables.baseUrl.length);
        return `${targetBaseUrl}${endpoint}`;
      }

      try {
        const parsedUrl = new URL(url);
        return `${targetBaseUrl}${parsedUrl.pathname}${parsedUrl.search}${parsedUrl.hash}`;
      } catch {
        const path = url.startsWith("/") ? url : `/${url}`;
        return `${targetBaseUrl}${path}`;
      }
    },
    [envEnabled, environments],
  );

  const handleEnvironmentChange = useCallback(
    (
      newEnvName: string,
      urlInput: string,
      bearerToken: string,
      headers: HeaderType[],
      setUrlInput: (url: string) => void,
      setHeaders: (headers: HeaderType[]) => void,
      setUserDetail: (detail: string) => void,
    ) => {
      if (!envEnabled) {
        return;
      }

      const oldEnvName = activeEnvName;
      const nextEnvironment = environments.find((environment) => environment.name === newEnvName);
      const nextUrl = convertUrl(urlInput, oldEnvName, newEnvName);
      setUrlInput(nextUrl);

      const authHeader = headers.find((header) => header.key.toLowerCase() === "authorization");
      const xAuthHeader = headers.find((header) => header.key.toLowerCase() === "x-authorization");
      let existingToken = "";
      let tokenSource = "";

      if (authHeader && authHeader.value.toLowerCase().startsWith("bearer ")) {
        existingToken = authHeader.value.slice(7);
        tokenSource = "authorization";
      } else if (xAuthHeader && xAuthHeader.value.toLowerCase().startsWith("bearer ")) {
        existingToken = xAuthHeader.value.slice(7);
        tokenSource = "x-authorization";
      } else if (xAuthHeader) {
        existingToken = xAuthHeader.value;
        tokenSource = "x-authorization";
      } else if (authHeader) {
        existingToken = authHeader.value;
        tokenSource = "authorization";
      }

      const token = bearerToken || existingToken;

      if (isLocalEnvironment(nextEnvironment)) {
        try {
          const parsedData = token ? parseJwt(token) : null;
          setUserDetail(parsedData ? JSON.stringify(parsedData, null, 2) : "");

          let updatedHeaders = [...headers];
          const xUserDetailHeader = updatedHeaders.find(
            (header) => header.key.toLowerCase() === "x-user-detail",
          );

          if (parsedData) {
            if (xUserDetailHeader) {
              updatedHeaders = updatedHeaders.map((header) =>
                header.id === xUserDetailHeader.id
                  ? { ...header, value: JSON.stringify(parsedData), enabled: true }
                  : header,
              );
            } else {
              updatedHeaders.push({
                id: crypto.randomUUID(),
                key: "x-user-detail",
                value: JSON.stringify(parsedData),
                enabled: true,
              });
            }
          }

          updatedHeaders = updatedHeaders.filter(
            (header) => header.key.toLowerCase() !== "authorization",
          );

          if (token) {
            if (xAuthHeader) {
              updatedHeaders = updatedHeaders.map((header) =>
                header.id === xAuthHeader.id
                  ? { ...header, value: token, enabled: true }
                  : header,
              );
            } else {
              updatedHeaders.push({
                id: crypto.randomUUID(),
                key: "X-Authorization",
                value: token,
                enabled: true,
              });
            }
          }

          setHeaders(updatedHeaders);
        } catch (error) {
          console.error("Error parsing JWT:", error);
        }

        return;
      }

      if (!token) {
        setHeaders(headers.filter((header) => header.key.toLowerCase() !== "x-user-detail"));
        setUserDetail("");
        return;
      }

      const normalizedHeaders = headers.filter(
        (header) => header.key.toLowerCase() !== "x-user-detail",
      );

      if (xAuthHeader || tokenSource === "x-authorization") {
        const updatedHeaders = normalizedHeaders.some(
          (header) => header.key.toLowerCase() === "x-authorization",
        )
          ? normalizedHeaders.map((header) =>
              header.key.toLowerCase() === "x-authorization"
                ? { ...header, value: token, enabled: true }
                : header,
            )
          : [
              ...normalizedHeaders,
              {
                id: crypto.randomUUID(),
                key: "X-Authorization",
                value: token,
                enabled: true,
              },
            ];

        setHeaders(updatedHeaders);
        setUserDetail("");
        return;
      }

      const updatedHeaders = authHeader
        ? normalizedHeaders.map((header) =>
            header.id === authHeader.id
              ? { ...header, value: token, enabled: true }
              : header,
          )
        : [
            ...normalizedHeaders,
            {
              id: crypto.randomUUID(),
              key: "Authorization",
              value: token,
              enabled: true,
            },
          ];

      setHeaders(updatedHeaders);
      setUserDetail("");
    },
    [activeEnvName, convertUrl, envEnabled, environments],
  );

  const handleUrlInputChange = useCallback(
    (newUrl: string, setUrlInput: (url: string) => void, setEndpoint: (endpoint: string) => void) => {
      setUrlInput(newUrl);

      try {
        const parsedUrl = new URL(newUrl);
        setEndpoint(`${parsedUrl.pathname}${parsedUrl.search}`);
      } catch {
        setEndpoint(newUrl);
      }
    },
    [],
  );

  return {
    activeEnvironment,
    convertUrl,
    handleEnvironmentChange,
    handleUrlInputChange,
    isLocalEnvironment: isLocalEnvironment(activeEnvironment ?? undefined),
  };
}
