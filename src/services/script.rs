use std::collections::HashMap;

use boa_engine::{Context, Source};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::domain::environment::AppEnvironment;
use crate::domain::request::KeyValue;
use crate::domain::response::{ConsoleEntry, ConsoleLevel, HttpResponse, TestResult};

const PRELUDE: &str = r#"
var __tests = [];
var __logs = [];
function __ser(a) { if (typeof a === 'string') return a; try { return JSON.stringify(a); } catch (e) { return String(a); } }
function __args(a) { return Array.prototype.map.call(a, __ser).join(' '); }
var console = {
  log: function () { __logs.push({ level: 'log', message: __args(arguments) }); },
  info: function () { __logs.push({ level: 'info', message: __args(arguments) }); },
  warn: function () { __logs.push({ level: 'warn', message: __args(arguments) }); },
  error: function () { __logs.push({ level: 'error', message: __args(arguments) }); }
};
function __eq(a, b) { return JSON.stringify(a) === JSON.stringify(b); }
function __expect(actual) {
  function build(neg) {
    function fail(msg) { if (!neg) throw new Error(msg); }
    function failNot(msg) { if (neg) throw new Error(msg); }
    var api = {};
    api.equal = function (e) { if (actual === e) { failNot('expected ' + __ser(actual) + ' to not equal ' + __ser(e)); } else { fail('expected ' + __ser(actual) + ' to equal ' + __ser(e)); } return api; };
    api.equals = api.equal;
    api.eql = function (e) { if (__eq(actual, e)) { failNot('expected values to differ'); } else { fail('expected ' + __ser(actual) + ' to deeply equal ' + __ser(e)); } return api; };
    api.above = function (n) { if (actual > n) { failNot('expected not above ' + n); } else { fail('expected ' + __ser(actual) + ' to be above ' + n); } return api; };
    api.least = function (n) { if (actual >= n) { failNot('expected below ' + n); } else { fail('expected ' + __ser(actual) + ' to be at least ' + n); } return api; };
    api.below = function (n) { if (actual < n) { failNot('expected not below ' + n); } else { fail('expected ' + __ser(actual) + ' to be below ' + n); } return api; };
    api.most = function (n) { if (actual <= n) { failNot('expected above ' + n); } else { fail('expected ' + __ser(actual) + ' to be at most ' + n); } return api; };
    api.include = function (x) { var ok = actual != null && typeof actual.indexOf === 'function' && actual.indexOf(x) !== -1; if (ok) { failNot('expected not to include ' + __ser(x)); } else { fail('expected ' + __ser(actual) + ' to include ' + __ser(x)); } return api; };
    api.a = function (t) { if (typeof actual === t) { failNot('expected type to differ from ' + t); } else { fail('expected type ' + t + ' but got ' + (typeof actual)); } return api; };
    api.oneOf = function (arr) { if (arr.indexOf(actual) !== -1) { failNot('expected not one of ' + __ser(arr)); } else { fail('expected ' + __ser(actual) + ' to be one of ' + __ser(arr)); } return api; };
    api.to = api; api.be = api; api.been = api; api.is = api; api.have = api; api.has = api; api.that = api; api.which = api; api.deep = api; api.with = api;
    api.an = api.a;
    Object.defineProperty(api, 'true', { get: function () { if (actual === true) { failNot('expected not true'); } else { fail('expected ' + __ser(actual) + ' to be true'); } return api; } });
    Object.defineProperty(api, 'false', { get: function () { if (actual === false) { failNot('expected not false'); } else { fail('expected ' + __ser(actual) + ' to be false'); } return api; } });
    Object.defineProperty(api, 'null', { get: function () { if (actual === null) { failNot('expected not null'); } else { fail('expected ' + __ser(actual) + ' to be null'); } return api; } });
    Object.defineProperty(api, 'undefined', { get: function () { if (actual === undefined) { failNot('expected defined'); } else { fail('expected ' + __ser(actual) + ' to be undefined'); } return api; } });
    Object.defineProperty(api, 'ok', { get: function () { if (actual) { failNot('expected falsy'); } else { fail('expected ' + __ser(actual) + ' to be truthy'); } return api; } });
    Object.defineProperty(api, 'empty', { get: function () { var ok = actual == null || actual.length === 0; if (ok) { failNot('expected not empty'); } else { fail('expected ' + __ser(actual) + ' to be empty'); } return api; } });
    if (!neg) { var negApi = build(true); Object.defineProperty(api, 'not', { get: function () { return negApi; } }); }
    return api;
  }
  return build(false);
}
var pm = {
  environment: {
    _vars: (__seed.env || {}),
    get: function (k) { return this._vars[k]; },
    set: function (k, v) { this._vars[k] = String(v); },
    unset: function (k) { delete this._vars[k]; },
    has: function (k) { return Object.prototype.hasOwnProperty.call(this._vars, k); }
  },
  expect: __expect,
  test: function (name, fn) { try { fn(); __tests.push({ name: name, passed: true }); } catch (e) { __tests.push({ name: name, passed: false, error: (e && e.message) ? e.message : String(e) }); } }
};
pm.variables = pm.environment;
pm.globals = pm.environment;
pm.collectionVariables = pm.environment;
if (__seed.request) {
  pm.request = { method: __seed.request.method, url: __seed.request.url, headers: (__seed.request.headers || {}), body: __seed.request.body };
}
if (__seed.response) {
  var __res = __seed.response;
  pm.response = {
    code: __res.code,
    status: __res.status,
    responseTime: __res.responseTime,
    headers: (__res.headers || {}),
    _body: __res.body,
    text: function () { return this._body; },
    json: function () { return JSON.parse(this._body); }
  };
  pm.response.to = {
    have: {
      status: function (c) { if (pm.response.code !== c) { throw new Error('expected status ' + c + ' but got ' + pm.response.code); } return pm.response.to; },
      header: function (h) { if (!(h in pm.response.headers)) { throw new Error('expected response to have header ' + h); } return pm.response.to; }
    },
    be: {
      success: function () { if (!(pm.response.code >= 200 && pm.response.code < 300)) { throw new Error('expected 2xx but got ' + pm.response.code); } return pm.response.to; }
    }
  };
}
function __output() {
  var out = { tests: __tests, logs: __logs, env: pm.environment._vars };
  if (pm.request) { out.request = { method: pm.request.method, url: pm.request.url, headers: pm.request.headers, body: pm.request.body }; }
  return out;
}
"#;

#[derive(Debug, Default, Clone)]
pub struct ScriptResult {
    pub logs: Vec<ConsoleEntry>,
    pub tests: Vec<TestResult>,
    pub env_updates: HashMap<String, String>,
    pub request: Option<RequestOverride>,
}

#[derive(Debug, Clone)]
pub struct RequestOverride {
    pub method: String,
    pub url: String,
    pub headers: Vec<KeyValue>,
    pub body: String,
}

#[derive(Deserialize, Default)]
struct RawOutput {
    #[serde(default)]
    tests: Vec<RawTest>,
    #[serde(default)]
    logs: Vec<RawLog>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    request: Option<RawRequest>,
}

#[derive(Deserialize)]
struct RawTest {
    name: String,
    passed: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct RawLog {
    level: String,
    message: String,
}

#[derive(Deserialize)]
struct RawRequest {
    method: String,
    url: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: String,
}

pub fn run_pre_request(
    script: &str,
    method: &str,
    url: &str,
    headers: &[KeyValue],
    body: &str,
    env: Option<&AppEnvironment>,
) -> ScriptResult {
    let seed = json!({
        "env": env_seed(env),
        "request": request_seed(method, url, headers, body),
    });
    into_result(evaluate(&seed, script), true)
}

pub fn run_test(
    script: &str,
    method: &str,
    url: &str,
    headers: &[KeyValue],
    body: &str,
    response: &HttpResponse,
    env: Option<&AppEnvironment>,
) -> ScriptResult {
    let seed = json!({
        "env": env_seed(env),
        "request": request_seed(method, url, headers, body),
        "response": {
            "code": response.status,
            "status": response.status_text,
            "responseTime": response.duration_ms,
            "headers": response.headers,
            "body": response.body,
        },
    });
    into_result(evaluate(&seed, script), false)
}

fn env_seed(env: Option<&AppEnvironment>) -> Value {
    match env {
        Some(e) => json!(e.variables),
        None => json!({}),
    }
}

fn request_seed(method: &str, url: &str, headers: &[KeyValue], body: &str) -> Value {
    let mut header_map = serde_json::Map::new();
    for h in headers {
        if h.enabled && !h.key.is_empty() {
            header_map.insert(h.key.clone(), Value::String(h.value.clone()));
        }
    }
    json!({ "method": method, "url": url, "headers": header_map, "body": body })
}

fn evaluate(seed: &Value, user_script: &str) -> RawOutput {
    let program = format!(
        "var __seed = {seed};\n{PRELUDE}\ntry {{\n{user_script}\n}} catch (__err) {{ __logs.push({{ level: 'error', message: 'Error: ' + ((__err && __err.message) ? __err.message : String(__err)) }}); }}\nJSON.stringify(__output());"
    );

    let mut context = Context::default();
    match context.eval(Source::from_bytes(program.as_bytes())) {
        Ok(value) => value
            .to_string(&mut context)
            .ok()
            .map(|s| s.to_std_string_escaped())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        Err(err) => {
            let mut out = RawOutput::default();
            out.logs.push(RawLog { level: "error".to_owned(), message: format!("Script error: {err}") });
            out
        }
    }
}

fn into_result(raw: RawOutput, with_request: bool) -> ScriptResult {
    let logs = raw
        .logs
        .into_iter()
        .map(|l| ConsoleEntry {
            level: match l.level.as_str() {
                "warn" => ConsoleLevel::Warn,
                "error" => ConsoleLevel::Error,
                "info" => ConsoleLevel::Info,
                _ => ConsoleLevel::Log,
            },
            message: l.message,
            timestamp: 0,
        })
        .collect();

    let tests = raw
        .tests
        .into_iter()
        .map(|t| TestResult { name: t.name, passed: t.passed, error: t.error, duration_ms: None })
        .collect();

    let request = if with_request {
        raw.request.map(|r| RequestOverride {
            method: r.method,
            url: r.url,
            headers: r
                .headers
                .into_iter()
                .map(|(key, value)| KeyValue {
                    id: uuid::Uuid::new_v4().to_string(),
                    key,
                    value,
                    enabled: true,
                })
                .collect(),
            body: r.body,
        })
    } else {
        None
    };

    ScriptResult { logs, tests, env_updates: raw.env, request }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_passes_and_captures_logs() {
        let response = HttpResponse { status: 200, ..Default::default() };
        let result = run_test(
            "pm.test('status is 200', function () { pm.response.to.have.status(200); pm.expect(1).to.equal(1); }); console.log('hello', 42);",
            "GET",
            "https://example.com",
            &[],
            "",
            &response,
            None,
        );
        assert_eq!(result.tests.len(), 1);
        assert!(result.tests[0].passed);
        assert!(result.logs.iter().any(|l| l.message.contains("hello")));
    }

    #[test]
    fn failing_assertion_marks_test_failed() {
        let response = HttpResponse { status: 500, ..Default::default() };
        let result = run_test(
            "pm.test('ok', function () { pm.response.to.have.status(200); });",
            "GET",
            "https://example.com",
            &[],
            "",
            &response,
            None,
        );
        assert_eq!(result.tests.len(), 1);
        assert!(!result.tests[0].passed);
        assert!(result.tests[0].error.is_some());
    }

    #[test]
    fn pre_request_sets_env_and_mutates_request() {
        let result = run_pre_request(
            "pm.environment.set('token', 'abc'); pm.request.headers['X-Test'] = '1'; pm.request.url = 'https://changed.example';",
            "GET",
            "https://example.com",
            &[],
            "",
            None,
        );
        assert_eq!(result.env_updates.get("token").map(String::as_str), Some("abc"));
        let override_req = result.request.expect("request override present");
        assert_eq!(override_req.url, "https://changed.example");
        assert!(override_req.headers.iter().any(|h| h.key == "X-Test" && h.value == "1"));
    }
}
