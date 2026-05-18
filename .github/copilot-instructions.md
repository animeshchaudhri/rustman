# Copilot Instructions

## Project Overview

A desktop API testing tool (like Postman) built with **Tauri v2**, **React 18 + TypeScript**, and a **Rust** backend. The key architectural distinction is that all HTTP requests are proxied through the Rust backend to fully bypass CORS restrictions.

## Commands

```bash
# Run frontend dev server only (Vite on port 1420)
yarn dev

# Run full Tauri desktop app (preferred for development)
yarn tauri dev

# Type-check + build frontend
yarn build

# Build distributable desktop app
yarn tauri build

# Add a new shadcn/ui component
npx shadcn@latest add <component-name>
```

There are no tests currently in this project.

## Architecture

### Request Flow
All HTTP requests go through a single pipeline:

```
Dashboard.tsx (state) → enhancedFetch() → tauriProxyFetch() → Tauri invoke('proxy_request') → Rust reqwest → external API
```

- `src/features/apiTester/utils/tauriProxy.ts` — bridges TypeScript to the Rust backend via `invoke<ProxyResponse>('proxy_request', { request })`
- `src-tauri/src/proxy.rs` — the `#[tauri::command] proxy_request` function using `reqwest`; returns `ProxyResponse { status, headers, body, error }`
- `src-tauri/src/lib.rs` — registers Tauri commands; add new commands here and in `tauri::generate_handler![]`

Never use the browser's `fetch` directly for making API calls — always go through `enhancedFetch`.

### Frontend Structure

```
src/
  features/apiTester/
    Dashboard.tsx          # Single top-level component holding all request/response state
    components/            # UI panels, all barrel-exported via index.ts
    hooks/useEnvironment.tsx
    utils/
      index.ts             # parseCurlCommand, generateJsCode, enhancedFetch, parseJwt, etc.
      tauriProxy.ts        # Tauri IPC bridge
    types/index.ts         # All shared TypeScript interfaces and union types
  components/ui/           # shadcn/ui primitives (do not edit manually; use CLI to add)
  lib/utils.ts             # cn() utility for Tailwind class merging
```

### Environment System (`useEnvironment` hook)

Three hardcoded environments: **Production**, **Local Dev**, **No Environment**.

Switching environments has side effects:
- URLs are automatically rewritten by swapping the `baseUrl` prefix.
- Switching **to Local Dev**: JWT bearer token is decoded and injected as an `x-user-detail` header; `Authorization` header is renamed to `X-Authorization`.
- Switching **to Production/No Environment**: `x-user-detail` header is removed; `X-Authorization` may be converted back to `Authorization`.

## Key Conventions

### Import Alias
`@/` resolves to `src/`. Always use this for non-relative imports (e.g., `@/components/ui/button`, `@/lib/utils`).

### UI Components
- Uses **shadcn/ui "new-york" style** with CSS variables (`cssVariables: true`). Tailwind base color is `neutral`.
- Add components via `npx shadcn@latest add <name>` — they land in `src/components/ui/`.
- Icons come from `lucide-react`.

### TypeScript Types
All shared types live in `src/features/apiTester/types/index.ts`. Key union types:
- `AuthType`: `'none' | 'basic' | 'bearer' | 'apikey' | 'jwt-user' | 'cookie'`
- `RequestBodyType`: `'json' | 'text' | 'form-data' | 'none'`
- Row-based state (headers, params, cookies) uses `{ id: string; key/name: string; value: string; enabled: boolean }` with `crypto.randomUUID()` for IDs.

### Adding a New Tauri Command (Rust ↔ TypeScript)
1. Implement in `src-tauri/src/` (new file or `proxy.rs`)
2. Annotate with `#[tauri::command]`
3. Register in `lib.rs` inside `tauri::generate_handler![]`
4. Call from TypeScript with `invoke<ReturnType>('command_name', { args })`

### Rust Backend
- Uses `reqwest` (blocking + async features) for outbound HTTP.
- Error handling uses `anyhow` + `thiserror`. Tauri commands return `Result<T, String>`.
- `ProxyRequest` / `ProxyResponse` structs in `proxy.rs` must stay in sync with the `ProxyRequest` / `ProxyResponse` interfaces in `tauriProxy.ts`.

### Tailwind CSS
Uses Tailwind v4 via the `@tailwindcss/vite` plugin — there is **no `tailwind.config.js`**. Configuration (theme, base color) is done via CSS variables in `src/App.css`.
