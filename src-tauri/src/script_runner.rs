use aes::Aes256;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
use hmac::{Hmac, Mac};
use rand::RngCore;
use rquickjs::{Context, Function, Runtime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type Aes256CbcEnc = cbc::Encryptor<Aes256>;
type Aes256CbcDec = cbc::Decryptor<Aes256>;
type HmacSha256 = Hmac<sha2::Sha256>;
type HmacSha512 = Hmac<sha2::Sha512>;

// ─── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptInput {
    pub script: String,
    pub body: Option<String>,
    pub env_vars: HashMap<String, String>,
    pub response: Option<ResponseData>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseData {
    pub status: Option<u16>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub duration: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ScriptOutput {
    pub vars: HashMap<String, String>,
    pub body: Option<String>,
    pub logs: Vec<LogEntry>,
    pub results: Vec<TestResult>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: String,
    pub args: Vec<String>,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub duration: u64,
}

// ─── Native crypto dispatcher ─────────────────────────────────────────────────

fn pad_key(bytes: &[u8], size: usize) -> Vec<u8> {
    let mut out = vec![0u8; size];
    out[..bytes.len().min(size)].copy_from_slice(&bytes[..bytes.len().min(size)]);
    out
}

fn algo_key_size(algo: &str) -> usize {
    if algo.contains("256") { 32 } else if algo.contains("192") { 24 } else { 16 }
}

fn iv_arr(bytes: &[u8]) -> [u8; 16] {
    let mut arr = [0u8; 16];
    arr[..bytes.len().min(16)].copy_from_slice(&bytes[..bytes.len().min(16)]);
    arr
}

fn dispatch(call_json: String) -> String {
    #[derive(Deserialize)]
    struct Call {
        func: String,
        args: Vec<serde_json::Value>,
    }

    let call: Call = match serde_json::from_str(&call_json) {
        Ok(c) => c,
        Err(e) => return format!("__ERR__:{e}"),
    };

    let a = &call.args;
    let str_arg = |i: usize| a.get(i).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let int_arg = |i: usize| a.get(i).and_then(|v| v.as_i64()).unwrap_or(0) as usize;

    match call.func.as_str() {
        "rand_bytes" => {
            let size = int_arg(0).max(1);
            let mut buf = vec![0u8; size];
            rand::thread_rng().fill_bytes(&mut buf);
            hex::encode(buf)
        }

        "aes_enc" => {
            let (key_h, iv_h, data_h, algo) = (str_arg(0), str_arg(1), str_arg(2), str_arg(3));
            let key = match hex::decode(&key_h) { Ok(b) => b, Err(e) => return format!("__ERR__:key:{e}") };
            let iv  = match hex::decode(&iv_h)  { Ok(b) => b, Err(e) => return format!("__ERR__:iv:{e}") };
            let data = match hex::decode(&data_h) { Ok(b) => b, Err(e) => return format!("__ERR__:data:{e}") };
            let pkey = pad_key(&key, algo_key_size(&algo));
            match Aes256CbcEnc::new_from_slices(&pkey, &iv_arr(&iv)) {
                Ok(enc) => hex::encode(enc.encrypt_padded_vec_mut::<Pkcs7>(&data)),
                Err(e)  => format!("__ERR__:{e}"),
            }
        }

        "aes_dec" => {
            let (key_h, iv_h, data_h, algo) = (str_arg(0), str_arg(1), str_arg(2), str_arg(3));
            let key  = match hex::decode(&key_h)  { Ok(b) => b, Err(e) => return format!("__ERR__:key:{e}") };
            let iv   = match hex::decode(&iv_h)   { Ok(b) => b, Err(e) => return format!("__ERR__:iv:{e}") };
            let data = match hex::decode(&data_h) { Ok(b) => b, Err(e) => return format!("__ERR__:data:{e}") };
            let pkey = pad_key(&key, algo_key_size(&algo));
            match Aes256CbcDec::new_from_slices(&pkey, &iv_arr(&iv)) {
                Ok(dec) => match dec.decrypt_padded_vec_mut::<Pkcs7>(&data) {
                    Ok(pt) => hex::encode(pt),
                    Err(e) => format!("__ERR__:decrypt:{e}"),
                },
                Err(e) => format!("__ERR__:{e}"),
            }
        }

        "hash" => {
            use sha2::Digest;
            let algo = str_arg(0);
            let data = match hex::decode(str_arg(1)) { Ok(b) => b, Err(e) => return format!("__ERR__:{e}") };
            match algo.to_lowercase().replace('-', "").as_str() {
                "sha256" => { let mut h = sha2::Sha256::new(); h.update(&data); hex::encode(h.finalize()) }
                "sha512" => { let mut h = sha2::Sha512::new(); h.update(&data); hex::encode(h.finalize()) }
                _ => format!("__ERR__:unsupported hash: {algo}"),
            }
        }

        "hmac" => {
            let algo    = str_arg(0);
            let key     = match hex::decode(str_arg(1)) { Ok(b) => b, Err(e) => return format!("__ERR__:{e}") };
            let data    = match hex::decode(str_arg(2)) { Ok(b) => b, Err(e) => return format!("__ERR__:{e}") };
            match algo.to_lowercase().replace('-', "").as_str() {
                "sha256" => {
                    let mut mac = match HmacSha256::new_from_slice(&key) { Ok(m) => m, Err(e) => return format!("__ERR__:{e}") };
                    mac.update(&data);
                    hex::encode(mac.finalize().into_bytes())
                }
                "sha512" => {
                    let mut mac = match HmacSha512::new_from_slice(&key) { Ok(m) => m, Err(e) => return format!("__ERR__:{e}") };
                    mac.update(&data);
                    hex::encode(mac.finalize().into_bytes())
                }
                _ => format!("__ERR__:unsupported hmac: {algo}"),
            }
        }

        "b64enc" => {
            let bytes = match hex::decode(str_arg(0)) { Ok(b) => b, Err(e) => return format!("__ERR__:{e}") };
            B64.encode(bytes)
        }

        "b64dec" => {
            let bytes = match B64.decode(str_arg(0).as_bytes()) { Ok(b) => b, Err(e) => return format!("__ERR__:{e}") };
            hex::encode(bytes)
        }

        "utf8hex" => hex::encode(str_arg(0).as_bytes()),

        "hexutf8" => {
            let bytes = match hex::decode(str_arg(0)) { Ok(b) => b, Err(e) => return format!("__ERR__:{e}") };
            String::from_utf8_lossy(&bytes).to_string()
        }

        _ => format!("__ERR__:unknown:{}", call.func),
    }
}

// ─── JavaScript preamble ──────────────────────────────────────────────────────

const PREAMBLE: &str = r#"
const __rc = (func, ...args) => __rustCall(JSON.stringify({ func, args }));

class Buffer {
  constructor(hexStr) { this._hex = hexStr || ''; }

  static from(data, encoding) {
    if (data instanceof Buffer) return data;
    encoding = encoding || 'utf8';
    if (typeof data === 'string') {
      if (encoding === 'base64') return new Buffer(__rc('b64dec', data));
      if (encoding === 'hex')    return new Buffer(data);
      return new Buffer(__rc('utf8hex', data));
    }
    if (Array.isArray(data) || (data && typeof data.length === 'number')) {
      let h = '';
      for (let i = 0; i < data.length; i++) h += ('0' + (data[i] & 0xFF).toString(16)).slice(-2);
      return new Buffer(h);
    }
    return new Buffer('');
  }

  static alloc(size, fill) {
    const byte = fill !== undefined ? ('0' + (fill & 0xFF).toString(16)).slice(-2) : '00';
    return new Buffer(byte.repeat(size));
  }

  static isBuffer(obj) { return obj instanceof Buffer; }

  get length() { return this._hex.length >> 1; }

  toString(encoding) {
    encoding = encoding || 'utf8';
    if (encoding === 'hex')    return this._hex;
    if (encoding === 'base64') return __rc('b64enc', this._hex);
    return __rc('hexutf8', this._hex);
  }
}

const crypto = {
  randomBytes(size) {
    return new Buffer(__rc('rand_bytes', size));
  },

  createCipheriv(algorithm, key, iv) {
    const keyHex = key instanceof Buffer ? key._hex : __rc('utf8hex', String(key));
    const ivHex  = iv  instanceof Buffer ? iv._hex  : __rc('utf8hex', String(iv));
    let acc = '';
    return {
      update(data, inputEnc) {
        if (!data) return '';
        let h;
        if (inputEnc === 'hex')    h = data instanceof Buffer ? data._hex : data;
        else if (inputEnc === 'base64') h = __rc('b64dec', String(data));
        else h = data instanceof Buffer ? data._hex : __rc('utf8hex', String(data));
        acc += h;
        return '';
      },
      final(outputEnc) {
        const r = __rc('aes_enc', keyHex, ivHex, acc, algorithm);
        if (r.startsWith('__ERR__:')) throw new Error(r.slice(8));
        if (outputEnc === 'hex')    return r;
        if (outputEnc === 'base64') return __rc('b64enc', r);
        return __rc('hexutf8', r);
      }
    };
  },

  createDecipheriv(algorithm, key, iv) {
    const keyHex = key instanceof Buffer ? key._hex : __rc('utf8hex', String(key));
    const ivHex  = iv  instanceof Buffer ? iv._hex  : __rc('utf8hex', String(iv));
    let acc = '';
    return {
      update(data, inputEnc) {
        if (!data) return '';
        let h;
        if (inputEnc === 'base64') h = __rc('b64dec', String(data));
        else if (inputEnc === 'hex') h = data instanceof Buffer ? data._hex : data;
        else h = data instanceof Buffer ? data._hex : __rc('utf8hex', String(data));
        acc += h;
        return '';
      },
      final(outputEnc) {
        const r = __rc('aes_dec', keyHex, ivHex, acc, algorithm);
        if (r.startsWith('__ERR__:')) throw new Error(r.slice(8));
        if (outputEnc === 'hex')    return r;
        if (outputEnc === 'base64') return __rc('b64enc', r);
        return __rc('hexutf8', r);
      }
    };
  },

  createHash(algorithm) {
    let acc = '';
    return {
      update(data) {
        acc += data instanceof Buffer ? __rc('hexutf8', data._hex) : String(data);
        return this;
      },
      digest(outputEnc) {
        outputEnc = outputEnc || 'hex';
        const r = __rc('hash', algorithm, __rc('utf8hex', acc));
        if (r.startsWith('__ERR__:')) throw new Error(r.slice(8));
        if (outputEnc === 'base64') return __rc('b64enc', r);
        return r;
      }
    };
  },

  createHmac(algorithm, key) {
    const keyHex = key instanceof Buffer ? key._hex : __rc('utf8hex', String(key));
    let acc = '';
    return {
      update(data) {
        acc += data instanceof Buffer ? __rc('hexutf8', data._hex) : String(data);
        return this;
      },
      digest(outputEnc) {
        outputEnc = outputEnc || 'hex';
        const r = __rc('hmac', algorithm, keyHex, __rc('utf8hex', acc));
        if (r.startsWith('__ERR__:')) throw new Error(r.slice(8));
        if (outputEnc === 'base64') return __rc('b64enc', r);
        return r;
      }
    };
  }
};

function require(mod) {
  if (mod === 'crypto') return crypto;
  if (mod === 'buffer') return { Buffer };
  throw new Error('Cannot find module: ' + mod);
}

const console = (() => {
  const push = (level) => function() {
    const args = [];
    for (let i = 0; i < arguments.length; i++) {
      const a = arguments[i];
      if (a === null || a === undefined) args.push(String(a));
      else if (typeof a === 'object') { try { args.push(JSON.stringify(a)); } catch(e) { args.push(String(a)); } }
      else args.push(String(a));
    }
    __state__.logs.push({ level, args, timestamp: Date.now() });
  };
  return { log: push('log'), warn: push('warn'), error: push('error'), info: push('info') };
})();

const pm = {
  environment: {
    get(key) { return __state__.vars[key] !== undefined ? __state__.vars[key] : ''; },
    set(key, value) { __state__.vars[key] = String(value); }
  },
  test(name, fn) {
    const start = Date.now();
    try {
      fn();
      __state__.results.push({ name, passed: true, error: null, duration: Date.now() - start });
    } catch(e) {
      __state__.results.push({ name, passed: false, error: e.message || String(e), duration: Date.now() - start });
    }
  },
  expect(val) {
    const chain = {};
    chain.to = {
      equal(expected) { if (val !== expected) throw new Error('Expected ' + JSON.stringify(expected) + ' got ' + JSON.stringify(val)); return chain; },
      include(str) { if (String(val).indexOf(str) === -1) throw new Error('Expected "' + val + '" to include "' + str + '"'); return chain; },
      have: {
        property(key) { if (!val || !(key in Object(val))) throw new Error('Expected object to have property "' + key + '"'); return chain; },
        status(s) { if (!val || val.status !== s) throw new Error('Expected status ' + s + ', got ' + (val && val.status)); return chain; }
      },
      be: {
        ok() { if (!val) throw new Error('Expected truthy, got ' + val); return chain; },
        above(n) { if (!(val > n)) throw new Error(val + ' not above ' + n); return chain; },
        below(n) { if (!(val < n)) throw new Error(val + ' not below ' + n); return chain; },
        equal(e) { if (val !== e) throw new Error('Expected ' + JSON.stringify(e) + ' got ' + JSON.stringify(val)); return chain; }
      }
    };
    return chain;
  },
  get response() {
    const r = __state__.response || {};
    const headers = r.headers || {};
    return {
      status: r.status || 0,
      responseTime: r.duration || 0,
      json() { const d = r.body || ''; if (typeof d === 'string') { try { return JSON.parse(d); } catch(e) { return d; } } return d; },
      text() { const d = r.body || ''; return typeof d === 'string' ? d : JSON.stringify(d); }
    };
  }
};

const req = {
  getBody() {
    if (!__state__.body) return '';
    try { return JSON.parse(__state__.body); } catch(e) { return __state__.body; }
  },
  setBody(val) {
    __state__.body = typeof val === 'string' ? val : JSON.stringify(val);
  }
};

const res = (() => {
  const r = __state__.response || {};
  const headers = r.headers || {};
  return {
    status: r.status || 0,
    headers,
    json() { const d = r.body || ''; if (typeof d === 'string') { try { return JSON.parse(d); } catch(e) { return d; } } return d; },
    text() { const d = r.body || ''; return typeof d === 'string' ? d : JSON.stringify(d); },
    getHeader(key) {
      const k = String(key).toLowerCase();
      const found = Object.keys(headers).find(h => h.toLowerCase() === k);
      return found ? headers[found] : '';
    }
  };
})();
"#;

// ─── Script execution ─────────────────────────────────────────────────────────

pub fn execute_script(input: ScriptInput) -> ScriptOutput {
    let rt = match Runtime::new() {
        Ok(r) => r,
        Err(e) => return ScriptOutput { error: Some(e.to_string()), ..Default::default() },
    };
    let ctx = match Context::full(&rt) {
        Ok(c) => c,
        Err(e) => return ScriptOutput { error: Some(e.to_string()), ..Default::default() },
    };

    let env_json      = serde_json::to_string(&input.env_vars).unwrap_or_else(|_| "{}".into());
    let body_json     = serde_json::to_string(&input.body).unwrap_or_else(|_| "null".into());
    let response_json = serde_json::to_string(&input.response).unwrap_or_else(|_| "null".into());

    let init_code = format!(
        "var __state__ = {{ vars: {env_json}, body: {body_json}, logs: [], results: [], error: null, response: {response_json} }};"
    );

    let user_script = input.script.clone();

    let result = ctx.with(|ctx| -> Result<ScriptOutput, rquickjs::Error> {
        // Register the single native dispatcher
        let dispatcher = Function::new(ctx.clone(), |call: String| -> String { dispatch(call) })?;
        ctx.globals().set("__rustCall", dispatcher)?;

        // Init state + preamble
        ctx.eval::<(), _>(format!("{init_code}\n{PREAMBLE}"))?;

        // Async IIFE so user scripts can use `await` (e.g. await cipher.final(...))
        // Errors inside the async fn are caught via .catch(); sync throws by the outer try.
        let wrapped = format!(
            "try {{ (async function() {{ {user_script} }})().catch(function(e) {{ __state__.error = e.message || String(e); }}); }} catch(e) {{ __state__.error = e.message || String(e); }}"
        );
        ctx.eval::<(), _>(wrapped)?;

        Ok(ScriptOutput::default()) // placeholder; state is read after job drain below
    });

    // Drain pending jobs (needed if user script uses async/await)
    loop {
        match rt.execute_pending_job() {
            Ok(true) => continue,
            _ => break,
        }
    }

    // Read final state
    let state_json = ctx.with(|ctx| -> Result<String, rquickjs::Error> {
        ctx.eval("JSON.stringify(__state__)")
    });

    match state_json {
        Ok(json) => parse_state(&json, &input.env_vars),
        Err(e) => {
            // If we already got an error from the init phase, report that
            if let Err(init_err) = result {
                ScriptOutput { error: Some(init_err.to_string()), ..Default::default() }
            } else {
                ScriptOutput { error: Some(e.to_string()), ..Default::default() }
            }
        }
    }
}

fn parse_state(json: &str, fallback_vars: &HashMap<String, String>) -> ScriptOutput {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => return ScriptOutput { error: Some(e.to_string()), vars: fallback_vars.clone(), ..Default::default() },
    };

    let vars: HashMap<String, String> =
        serde_json::from_value(v["vars"].clone()).unwrap_or_else(|_| fallback_vars.clone());
    let body: Option<String> = v["body"].as_str().map(|s| s.to_string());
    let logs: Vec<LogEntry>  = serde_json::from_value(v["logs"].clone()).unwrap_or_default();
    let results: Vec<TestResult> = serde_json::from_value(v["results"].clone()).unwrap_or_default();
    let error: Option<String> = v["error"].as_str().filter(|s| !s.is_empty()).map(|s| s.to_string());

    ScriptOutput { vars, body, logs, results, error }
}

// ─── Tauri command ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn run_script(input: ScriptInput) -> Result<ScriptOutput, String> {
    Ok(execute_script(input))
}
