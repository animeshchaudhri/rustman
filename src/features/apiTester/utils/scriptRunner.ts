import { invoke } from "@tauri-apps/api/core";
import type { ApiResponse, ConsoleEntry, TestResult } from "../types";

interface ScriptInput {
  script: string;
  body: string | null;
  env_vars: Record<string, string>;
  response: {
    status: number | null;
    headers: Record<string, string>;
    body: string | null;
    duration: number | null;
  } | null;
}

interface ScriptOutput {
  vars: Record<string, string>;
  body: string | null;
  logs: ConsoleEntry[];
  results: TestResult[];
  error: string | null;
}

export async function runPreRequestScript(
  script: string,
  envVars: Record<string, string>,
  body = "",
): Promise<{ vars: Record<string, string>; logs: ConsoleEntry[]; body?: string }> {
  if (!script.trim()) return { vars: envVars, logs: [] };

  const input: ScriptInput = {
    script,
    body: body || null,
    env_vars: envVars,
    response: null,
  };

  try {
    const output = await invoke<ScriptOutput>("run_script", { input });
    return {
      vars: output.vars ?? envVars,
      logs: output.logs ?? [],
      body: output.body ?? undefined,
    };
  } catch (e) {
    return {
      vars: envVars,
      logs: [{ level: "error", args: [String(e)], timestamp: Date.now() }],
    };
  }
}

export async function runTestScript(
  script: string,
  apiResponse: ApiResponse,
  duration: number,
  envVars: Record<string, string>,
): Promise<{ results: TestResult[]; logs: ConsoleEntry[] }> {
  if (!script.trim()) return { results: [], logs: [] };

  const bodyStr =
    typeof apiResponse.data === "string"
      ? apiResponse.data
      : JSON.stringify(apiResponse.data ?? "");

  const input: ScriptInput = {
    script,
    body: null,
    env_vars: envVars,
    response: {
      status: apiResponse.status ?? null,
      headers: (apiResponse.headers as Record<string, string>) ?? {},
      body: bodyStr,
      duration,
    },
  };

  try {
    const output = await invoke<ScriptOutput>("run_script", { input });
    return {
      results: output.results ?? [],
      logs: output.logs ?? [],
    };
  } catch (e) {
    return {
      results: [],
      logs: [{ level: "error", args: [String(e)], timestamp: Date.now() }],
    };
  }
}
