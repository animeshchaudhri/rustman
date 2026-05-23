import type { ApiResponse, TestResult } from "../types";

function makePmExpect(val: unknown): Record<string, unknown> {
  const assert = (condition: boolean, msg: string) => {
    if (!condition) throw new Error(msg);
  };

  const chain: Record<string, unknown> = {};

  chain.to = {
    equal: (expected: unknown) => {
      assert(val === expected, `Expected ${JSON.stringify(expected)}, got ${JSON.stringify(val)}`);
      return chain;
    },
    include: (str: string) => {
      assert(String(val).includes(str), `Expected "${val}" to include "${str}"`);
      return chain;
    },
    be: new Proxy(
      {
        ok: () => { assert(Boolean(val), `Expected truthy value, got ${String(val)}`); return chain; },
        below: (n: number) => { assert((val as number) < n, `Expected ${String(val)} to be below ${n}`); return chain; },
        above: (n: number) => { assert((val as number) > n, `Expected ${String(val)} to be above ${n}`); return chain; },
        equal: (expected: unknown) => { assert(val === expected, `Expected ${JSON.stringify(expected)}, got ${JSON.stringify(val)}`); return chain; },
      } as Record<string, unknown>,
      {
        get(target, prop: string) {
          return prop in target ? target[prop] : () => chain;
        },
      },
    ),
    have: {
      property: (key: string) => {
        assert(
          typeof val === "object" && val !== null && key in (val as object),
          `Expected object to have property "${key}"`,
        );
        return chain;
      },
      status: (s: number) => {
        assert(
          (val as { status?: number }).status === s,
          `Expected status ${s}, got ${(val as { status?: number }).status}`,
        );
        return chain;
      },
    },
  };

  return chain;
}

export async function runPreRequestScript(
  script: string,
  envVars: Record<string, string>,
): Promise<Record<string, string>> {
  if (!script.trim()) return envVars;
  const updatedVars = { ...envVars };
  const pm = {
    environment: {
      get: (key: string) => updatedVars[key] ?? "",
      set: (key: string, value: string) => { updatedVars[key] = String(value); },
    },
  };
  try {
    const AsyncFn = Object.getPrototypeOf(async function () {}).constructor as new (
      ...args: string[]
    ) => (...args: unknown[]) => Promise<void>;
    const fn = new AsyncFn("pm", script);
    await fn(pm);
  } catch (e) {
    console.warn("[pre-request script]", e);
  }
  return updatedVars;
}

export function runTestScript(
  script: string,
  apiResponse: ApiResponse,
  duration: number,
  envVars: Record<string, string>,
): TestResult[] {
  if (!script.trim()) return [];
  const results: TestResult[] = [];
  const updatedVars = { ...envVars };

  const contentType =
    Object.entries(apiResponse.headers ?? {}).find(([k]) => k.toLowerCase() === "content-type")?.[1] ?? "";
  const isJson = contentType.includes("application/json");

  const pmResponse = {
    status: apiResponse.status,
    responseTime: duration,
    json: () => {
      const d = apiResponse.data;
      if (typeof d === "string") { try { return JSON.parse(d); } catch { return d; } }
      return d;
    },
    text: () => {
      const d = apiResponse.data;
      if (typeof d === "string") return d;
      return JSON.stringify(d ?? "");
    },
    to: {
      have: {
        status: (s: number) => {
          if (apiResponse.status !== s)
            throw new Error(`Expected status ${s}, got ${apiResponse.status ?? "none"}`);
        },
      },
      be: { json: isJson },
    },
  };

  const pm = {
    response: pmResponse,
    test: (name: string, fn: () => void) => {
      const start = performance.now();
      try {
        fn();
        results.push({ name, passed: true, duration: Math.round(performance.now() - start) });
      } catch (e) {
        results.push({
          name,
          passed: false,
          error: e instanceof Error ? e.message : String(e),
          duration: Math.round(performance.now() - start),
        });
      }
    },
    expect: makePmExpect,
    environment: {
      get: (key: string) => updatedVars[key] ?? "",
      set: (key: string, value: string) => { updatedVars[key] = String(value); },
    },
  };

  try {
    // eslint-disable-next-line @typescript-eslint/no-implied-eval
    const fn = new Function("pm", script);
    fn(pm);
  } catch (e) {
    console.warn("[test script]", e);
  }

  return results;
}
