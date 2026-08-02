//! Implementations backing the script language's built-in functions.

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use sha2::{Digest, Sha256};

use crate::value::Value;

pub(crate) fn base64_decode(s: &str) -> Value {
    match STANDARD.decode(s).or_else(|_| URL_SAFE_NO_PAD.decode(s)) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => Value::String(text),
            Err(_) => Value::Null,
        },
        Err(_) => Value::Null,
    }
}

pub(crate) fn base64_encode(s: &str) -> String {
    STANDARD.encode(s.as_bytes())
}

pub(crate) fn json_parse(s: &str) -> Value {
    serde_json::from_str::<serde_json::Value>(s)
        .map(|v| Value::from_json(&v))
        .unwrap_or(Value::Null)
}

/// Serializes a script value to real, properly-quoted JSON text — for
/// example, forwarding a whole decoded JWT payload as one header value.
pub(crate) fn json_stringify(v: &Value) -> String {
    serde_json::to_string(&v.to_json()).unwrap_or_default()
}

/// AES-256-GCM, keyed by the SHA-256 hash of an arbitrary-length key string
/// (so scripts can pass any secret from `env(...)` without worrying about
/// exact byte lengths). Output is `base64(nonce || ciphertext+tag)`, so
/// `aes_encrypt`/`aes_decrypt` round-trip with each other directly. This is
/// **not** guaranteed to match some other system's AES-GCM framing — if a
/// real external API dictates its own key derivation or nonce placement,
/// that needs matching builtins rather than these general-purpose ones.
fn derive_key(key: &str) -> Key<Aes256Gcm> {
    let hash = Sha256::digest(key.as_bytes());
    *Key::<Aes256Gcm>::from_slice(&hash)
}

pub(crate) fn aes_encrypt(plaintext: &str, key: &str) -> Value {
    let cipher = Aes256Gcm::new(&derive_key(key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    match cipher.encrypt(&nonce, plaintext.as_bytes()) {
        Ok(ciphertext) => {
            let mut out = nonce.to_vec();
            out.extend_from_slice(&ciphertext);
            Value::String(STANDARD.encode(out))
        }
        Err(_) => Value::Null,
    }
}

pub(crate) fn aes_decrypt(ciphertext_b64: &str, key: &str) -> Value {
    let Ok(data) = STANDARD.decode(ciphertext_b64) else {
        return Value::Null;
    };
    if data.len() < 12 {
        return Value::Null;
    }
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let cipher = Aes256Gcm::new(&derive_key(key));
    let nonce = Nonce::from_slice(nonce_bytes);
    match cipher.decrypt(nonce, ciphertext) {
        Ok(plaintext) => String::from_utf8(plaintext).map(Value::String).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

/// Decodes a JWT's header and payload segments (no signature verification —
/// this is purely for reading claims, not authenticating the token).
/// Accepts a bare token or one prefixed with `"Bearer "`.
pub(crate) fn jwt_decode(token: &str) -> Value {
    let token = token.strip_prefix("Bearer ").unwrap_or(token).trim();
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return Value::Null;
    }

    let decode_segment = |segment: &str| -> Value {
        URL_SAFE_NO_PAD
            .decode(segment)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .map(|text| json_parse(&text))
            .unwrap_or(Value::Null)
    };

    Value::Object(vec![
        ("header".to_owned(), decode_segment(parts[0])),
        ("payload".to_owned(), decode_segment(parts[1])),
    ])
}
