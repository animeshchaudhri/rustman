# Rustman scripting language

Rustman supports two script slots per request — a **pre-request script** (runs
before the request is sent) and a **test script** (runs after the response
comes back). Both are written in a small custom language implemented in
`vendor/rustman-engine`, not JavaScript. There is no `pm.*` API, no Node
builtins, no `require`/`import`.

This language is deliberately tiny. It exists to be *generated*, not
hand-written — the target audience for actually typing this syntax is an AI
being asked "write a script that extracts the token from this response," not
a person memorizing a grammar. This document is that AI's spec: it defines
every construct that exists, and nothing else exists. **Do not invent syntax,
functions, or objects that are not listed below** — an unknown identifier or
function name is a runtime error, not a fallback.

## Global scripts

The Settings panel has a **Global Scripts** section — a pre-request/test
script pair that runs for *every* request, so common setup (an auth header
every request needs, environment bootstrapping) doesn't need copy-pasting
into each request's own Scripts tab.

The global script always runs first, then the request's own script runs
second and can see (and override) anything the global script already did —
its `set_header`/`set_body` effects are folded into what `header()`/`body()`
return inside the per-request script. Test-script results and `print(...)`
logs from both accumulate in that order. If either script fails to run, that
phase's error is reported labeled ("Global pre-request script error: ..." vs.
"Pre-request script error: ...") so it's clear which one broke; for a
pre-request failure, the request itself does not send.

## Where a script runs and what it can see

| | Pre-request script | Test script |
|---|---|---|
| Runs | Before the request is sent | After the response is received |
| `env(...)` | Active environment's variables | Active environment's variables |
| `header(...)` | The request's own headers (as currently configured) | The **response's** headers |
| `cookie(...)` | The request's cookie jar | The request's cookie jar |
| `response` | Not available (doesn't exist yet) | Available — status + body |
| `body()`, `url()` | The request's own body/URL | Same — what was actually sent (not the response) |
| `headers()` | All of the request's own headers | All of the **response's** headers |
| Effects that matter | `set_header`, `set_body`, `set_env` | `test`, `set_env` |

A pre-request script's `set_header` calls are merged into the outgoing
request's headers, and a `set_body` call replaces the outgoing body. A test
script's `test` calls populate the response's **Tests** tab. `set_env` works
in either script and writes to the active environment (persisted
immediately, same as editing it by hand). None of this touches what's saved
in the request editor — a `set_body`/`set_header` override only affects the
one outgoing request, not the tab's stored body/headers.

If a script fails to parse or throws a runtime error, the request still
happens (for a pre-request script) or the response still displays (for a test
script) — the error is shown to the user instead of any effects being
applied. Nothing partial happens: if a script has run some `set_header` or
`test` calls and then hits an error on a later line, none of that run's
effects are applied.

## Syntax

### Statements

```
let name = <expr>
if <expr> { <statements> }
if <expr> { <statements> } else { <statements> }
<expr>
```

A trailing `;` after a statement is allowed but optional and has no effect —
use whichever reads better. There are no loops (`for`/`while`) and no
user-defined functions. Scripts are a short, flat (plus `if`/`else`) sequence
of statements; keep generated scripts to that shape rather than working
around the missing loop/function support.

### Comments

`// rest of line` — there is no block comment syntax.

### Literals

- Numbers: `42`, `3.14` (always floating point internally; `-5` is **not**
  valid syntax — there's no unary minus for numbers, write `0 - 5` if you
  need a negative literal)
- Strings: `"double-quoted"`, with escapes `\n`, `\t`, `\"`, `\\`
- Booleans: `true`, `false`
- `null`

### Variables

`let x = <expr>` binds `x` for the rest of the script (no re-declaration
keyword needed to reassign — `let x = ...` again just rebinds it). There is
no block scoping: everything is one flat scope for the whole script,
including names bound inside an `if`/`else` branch.

### Operators (highest to lowest precedence)

1. `.field` / `.method()` / `fn(args)` — field access and calls
2. `!` — logical not (prefix)
3. `+` `-` — addition/subtraction (also see string concatenation below)
4. `<` `>` `<=` `>=` — numeric comparison only (runtime error on non-numbers)
5. `==` `!=` — equality (works on any two values of the same type; different
   types are always unequal, never coerced)
6. `&&` — logical and (short-circuits)
7. `||` — logical or (short-circuits)

Parentheses `( <expr> )` group as usual.

### String concatenation

`+` on two numbers adds them. `+` where **either** side isn't a number
falls back to string concatenation (via each value's display form), so
`"Bearer " + token` and `"count: " + 3` both work directly. There is no
separate concatenation operator.

### Field access

`value.field` reads a field off an object (the result of `jwt_decode(...)`,
`json_parse(...)`, or `response.json()`). Accessing a field that doesn't
exist, or accessing a field on a non-object, evaluates to `null` — it is
**not** an error. This is a different convention from `env()`/`header()`/
`cookie()` (see below): JSON payload shape is expected to vary request to
request, so a missing field is a normal, silent `null`; a missing env var or
header is instead surfaced as `""` because scripts commonly branch on it with
`!= ""`.

## The `response` object (test scripts only)

- `response.status` — a number, the HTTP status code
- `response.json()` — parses the body as JSON and returns it as an object/
  array/value tree you can field-access into; returns `null` if the body
  isn't valid JSON
- `response.text()` — the raw response body as a string

There is no `response.headers` field access — read response headers with the
`header(...)` builtin (see below), which is fed the response's headers in
test scripts specifically so the same builtin name works in both script
slots.

## Built-in functions

| Function | Signature | Notes |
|---|---|---|
| `env(name)` | `(string) -> string` | Active environment variable. **`""` if unset**, not `null`. |
| `set_env(name, value)` | `(string, any) -> null` | Writes the active environment variable (effect, persisted). `value` can be any type — objects/arrays are JSON-stringified, everything else uses its display form. |
| `header(name)` | `(string) -> string` | Case-insensitive lookup. Request headers pre-request, response headers in a test script. **`""` if absent.** |
| `headers()` | `() -> object` | Every header at once, same request-vs-response split as `header(name)` — for dumping everything at once instead of naming each one. |
| `set_header(name, value)` | `(string, any) -> null` | Adds/overrides a request header. `value` can be any type, same auto-conversion as `set_env` — `set_header("X-User-Detail", claims.payload)` works directly, no `json_stringify` needed. |
| `cookie(name)` | `(string) -> string` | Case-insensitive lookup against the request's cookie jar. **`""` if absent.** |
| `body()` | `() -> string` | The request's own body — available in **both** script slots (a test script inspecting what it actually sent is a normal debugging need; use `response.text()`/`response.json()` for what came back). |
| `set_body(value)` | `(any) -> null` | Replaces the outgoing request body (pre-request scripts only). Same auto-conversion as `set_env`/`set_header`. |
| `url()` | `() -> string` | The request's own URL — available in both script slots. |
| `test(name, condition)` | `(string, bool) -> null` | Records a pass/fail result shown on the response's Tests tab. `condition` is evaluated for truthiness — see below. |
| `print(value)` | `(any) -> null` | Logs `value` (any type, auto-converted like `set_env`) to the Tests tab's console section — for debugging a script without needing an assertion. |
| `base64_encode(s)` | `(string) -> string` | Standard base64 (not URL-safe). |
| `base64_decode(s)` | `(string) -> string` | Accepts standard **or** URL-safe-no-pad base64. Returns `null` if the input isn't valid base64 or isn't valid UTF-8 once decoded. |
| `jwt_decode(token)` | `(string) -> object` | No signature verification — decodes claims only, does not authenticate. Accepts a bare token or one prefixed with `"Bearer "` (leading/trailing whitespace is trimmed). Returns `{ header: <object>, payload: <object> }`. Returns `null` if the token doesn't have at least a `header.payload` shape. |
| `json_parse(s)` | `(string) -> value` | Parses arbitrary JSON text into a value tree. Returns `null` on invalid JSON. |
| `json_stringify(value)` | `(value) -> string` | Serializes any value back into real, properly-quoted JSON text — the inverse of `json_parse`/`jwt_decode`'s output. |
| `aes_encrypt(plaintext, key)` | `(string, string) -> string` | AES-256-GCM, keyed by the SHA-256 hash of `key` (so any string works as a key, whatever length). Returns `base64(nonce \|\| ciphertext)`. |
| `aes_decrypt(ciphertext, key)` | `(string, string) -> string` | Reverses `aes_encrypt` with the same `key`. Returns `null` if the key is wrong or the input is malformed — decryption fails closed, not open. |

`aes_encrypt`/`aes_decrypt` round-trip with each other, but the exact framing
(SHA-256 key derivation, `nonce || ciphertext` layout) is this engine's own
convention — it is **not guaranteed to match some other system's AES-GCM
implementation**. If a real external API dictates its own key derivation or
nonce handling, treat these as a starting point, not a drop-in match.

Truthiness (used by `if`, `!`, `&&`, `||`, and `test`'s second argument):
`null` and `false` are falsy; `0` is falsy, any other number is truthy;
`""` is falsy, any other string is truthy; an empty array is falsy; objects
are always truthy.

## Worked examples

### Inject a bearer token from the environment, only if one is set

```
let token = env("access_token")
if token != "" {
    set_header("Authorization", "Bearer " + token)
}
```

### Decode a JWT from the response and stash claims into the environment

```
let claims = jwt_decode(header("Authorization"))
set_env("user_id", claims.payload.sub)
set_env("access_token", claims.payload.access_token)
test("status is 200", response.status == 200)
test("has user id", claims.payload.sub != null)
```

### Assert on the JSON response body

```
let body = response.json()
test("id is 7", body.id == 7)
test("name matches", body.name == "ferris")
```

### Range check with if/else

```
let status = response.status
if status >= 200 && status < 300 {
    test("ok range", true)
} else {
    test("ok range", false)
}
```

### Check a cookie was set

```
test("has session", cookie("session_id") != "")
```

### Forward a decoded JWT cookie's claims as a header

A common gateway/BFF pattern: a session JWT lives in a cookie, and the
backend wants the decoded user details forwarded as a single header rather
than re-decoding the JWT itself.

```
let claims = jwt_decode(cookie("accessToken"))
set_header("X-User-Detail", claims.payload)
```

`set_header` auto-converts a non-string value, so passing the object
directly JSON-stringifies it the same as calling `json_stringify(claims.payload)`
by hand — spelling it out explicitly is only useful when you want to build
the string yourself (e.g. combining it with other text).

If only one field is actually needed, send just that field — it's already a string:

```
let claims = jwt_decode(cookie("accessToken"))
set_header("X-User-Detail", claims.payload.masterPhrId)
```

### Debug what a request actually sent, from its own test script

```
print(url())
print(headers())
print(body())
```

### Encrypt the outgoing body, decrypt the response to assert on it

```
// pre-request script
let plaintext = body()
set_body(aes_encrypt(plaintext, env("body_key")))
```

```
// test script
let plaintext = aes_decrypt(response.text(), env("body_key"))
let parsed = json_parse(plaintext)
test("status is 200", response.status == 200)
test("order id present", parsed.order_id != null)
```

## What this language deliberately does not have

No loops, no user-defined functions, no arrays/object literals (you can only
*receive* arrays/objects from `jwt_decode`/`json_parse`/`response.json()`, not
construct them), no `try`/`catch` (a runtime error just aborts the script and
surfaces as an error), no string indexing/slicing, no regex. If a task needs
any of these, it's out of scope for a script — solve it with headers/env vars/
assertions in the shapes above instead of reaching for a workaround.

## Prompt for an LLM

Don't hand-write these scripts — copy the block below into ChatGPT, Claude,
or any other LLM, add your own request at the end, and let it write the
script for you.

```
You write scripts in Rustman's scripting language: a small custom language
(not JavaScript, no pm.* API). Only use what's listed below — nothing else
exists at runtime, and inventing syntax or functions will just fail.

STATEMENTS: `let x = <expr>`, `if <expr> { ... }`, `if <expr> { ... } else { ... }`,
or a bare expression. Trailing `;` optional. No loops, no user-defined functions.

LITERALS: numbers (no negative literals — write `0 - 5`, not `-5`), "double-quoted
strings" (\n \t \" \\ escapes), true, false, null.

OPERATORS, low to high precedence: || , && , == != , < > <= >= (numbers only) ,
+ - (numbers add; if either side isn't a number, + concatenates as text) ,
! (prefix not) , .field / .method() / call(...).

COMMENTS: `// rest of line`.

BUILT-IN FUNCTIONS:
  env(name) -> string                    ("" if unset)
  set_env(name, value)
  header(name) -> string                 ("" if absent; request headers pre-request,
                                           response headers in a test script)
  headers() -> object                    (every header at once, same split as header())
  set_header(name, value)
  cookie(name) -> string                 ("" if absent)
  body() -> string                       (the request's own body; works in both script slots)
  set_body(value)                        (pre-request scripts only, replaces the
                                           outgoing body)
  url() -> string                        (the request's own URL; works in both script slots)
  test(name, condition)                  (records a pass/fail)
  print(value)                           (debug log, no assertion)
  base64_encode(s) / base64_decode(s)
  jwt_decode(token) -> {header, payload}  (no signature check; accepts a bare
                                           token or "Bearer <token>")
  json_parse(s) -> value
  json_stringify(value) -> string
  aes_encrypt(text, key) / aes_decrypt(text, key)   (AES-256-GCM, key is any
                                           string, SHA-256 derived, base64(nonce||ciphertext))
  response.status (test scripts only, number) / response.json() / response.text()

set_env/set_header/set_body accept ANY value type — objects/arrays are
auto-JSON-stringified, everything else uses its plain text form. So
`set_header("X-User-Detail", claims.payload)` is valid directly.

CONVENTIONS: env()/header()/cookie() return "" for a missing value — check with
`!= ""`. Field access on an object from json_parse/jwt_decode/response.json()
returns null for a missing field — check that with `!= null`, not `!= ""`.

There's a GLOBAL pre-request/test script pair that runs before every
request's own script and can be overridden by it — mention this only if the
user's request is clearly about something that should apply to every
request, not just one.

Output ONLY the script itself — no explanation, no markdown code fences, no
JS syntax, no pm.* API.

My request: <describe what you want the script to do>
```
