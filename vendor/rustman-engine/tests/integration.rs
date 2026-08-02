use rustman_engine::{run, Effect, HostInput, ResponseInput};

#[test]
fn pre_request_injects_bearer_header_from_env() {
    let script = r#"
        let token = env("access_token")
        if token != "" {
            set_header("Authorization", "Bearer " + token)
        }
    "#;
    let input = HostInput {
        env_vars: vec![("access_token".to_owned(), "abc123".to_owned())],
        ..Default::default()
    };

    let outcome = run(script, input).expect("script should run");

    assert_eq!(
        outcome.effects,
        vec![Effect::SetHeader("Authorization".to_owned(), "Bearer abc123".to_owned())]
    );
}

#[test]
fn pre_request_skips_header_when_env_var_missing() {
    let script = r#"
        let token = env("access_token")
        if token != "" {
            set_header("Authorization", "Bearer " + token)
        }
    "#;
    let outcome = run(script, HostInput::default()).expect("script should run");
    assert!(outcome.effects.is_empty());
}

#[test]
fn test_script_decodes_jwt_and_extracts_claims_to_env() {
    // {"alg":"HS256","typ":"JWT"} . {"sub":"user-42","access_token":"tok-xyz"} . signature
    let header_b64 = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
    let payload_b64 = "eyJzdWIiOiJ1c2VyLTQyIiwiYWNjZXNzX3Rva2VuIjoidG9rLXh5eiJ9";
    let jwt = format!("{header_b64}.{payload_b64}.sig");

    let script = r#"
        let claims = jwt_decode(header("Authorization"))
        set_env("user_id", claims.payload.sub)
        set_env("access_token", claims.payload.access_token)
        test("status is 200", response.status == 200)
        test("has user id", claims.payload.sub != null)
    "#;

    let input = HostInput {
        headers: vec![("Authorization".to_owned(), format!("Bearer {jwt}"))],
        response: Some(ResponseInput { status: 200, body: "{}".to_owned() }),
        ..Default::default()
    };

    let outcome = run(script, input).expect("script should run");

    assert_eq!(
        outcome.effects,
        vec![
            Effect::SetEnv("user_id".to_owned(), "user-42".to_owned()),
            Effect::SetEnv("access_token".to_owned(), "tok-xyz".to_owned()),
            Effect::Test { name: "status is 200".to_owned(), passed: true },
            Effect::Test { name: "has user id".to_owned(), passed: true },
        ]
    );
}

#[test]
fn test_script_reads_response_json_body() {
    let script = r#"
        let body = response.json()
        test("id is 7", body.id == 7)
        test("name matches", body.name == "ferris")
    "#;
    let input = HostInput {
        response: Some(ResponseInput {
            status: 200,
            body: r#"{"id": 7, "name": "ferris"}"#.to_owned(),
        }),
        ..Default::default()
    };

    let outcome = run(script, input).expect("script should run");

    assert_eq!(
        outcome.effects,
        vec![
            Effect::Test { name: "id is 7".to_owned(), passed: true },
            Effect::Test { name: "name matches".to_owned(), passed: true },
        ]
    );
}

#[test]
fn if_else_both_branches_work() {
    let script = r#"
        let status = response.status
        if status >= 200 && status < 300 {
            test("ok range", true)
        } else {
            test("ok range", false)
        }
    "#;
    let input = HostInput {
        response: Some(ResponseInput { status: 404, body: String::new() }),
        ..Default::default()
    };

    let outcome = run(script, input).expect("script should run");

    assert_eq!(outcome.effects, vec![Effect::Test { name: "ok range".to_owned(), passed: false }]);
}

#[test]
fn cookie_lookup_is_case_insensitive() {
    let script = r#"test("has session", cookie("Session-Id") != "")"#;
    let input = HostInput {
        cookies: vec![("session-id".to_owned(), "abc".to_owned())],
        ..Default::default()
    };
    let outcome = run(script, input).expect("script should run");
    assert_eq!(outcome.effects, vec![Effect::Test { name: "has session".to_owned(), passed: true }]);
}

#[test]
fn missing_env_header_cookie_are_empty_string_not_null() {
    // Deliberate convention: env()/header()/cookie() return "" when unset,
    // not null, so the natural `if x != ""` pattern works without scripts
    // needing to know about a separate null type for these string lookups.
    let script = r#"
        test("env missing", env("nope") == "")
        test("header missing", header("Nope") == "")
        test("cookie missing", cookie("nope") == "")
    "#;
    let outcome = run(script, HostInput::default()).expect("script should run");
    assert_eq!(
        outcome.effects,
        vec![
            Effect::Test { name: "env missing".to_owned(), passed: true },
            Effect::Test { name: "header missing".to_owned(), passed: true },
            Effect::Test { name: "cookie missing".to_owned(), passed: true },
        ]
    );
}

#[test]
fn base64_round_trip() {
    let script = r#"
        let encoded = base64_encode("hello")
        let decoded = base64_decode(encoded)
        test("round trips", decoded == "hello")
    "#;
    let outcome = run(script, HostInput::default()).expect("script should run");
    assert_eq!(outcome.effects, vec![Effect::Test { name: "round trips".to_owned(), passed: true }]);
}

#[test]
fn json_stringify_round_trips_through_json_parse() {
    let script = r#"
        let parsed = json_parse("{\"id\": 7, \"name\": \"ferris\"}")
        let text = json_stringify(parsed)
        let reparsed = json_parse(text)
        test("id survived the round trip", reparsed.id == 7)
        test("name survived the round trip", reparsed.name == "ferris")
    "#;
    let outcome = run(script, HostInput::default()).expect("script should run");
    assert_eq!(
        outcome.effects,
        vec![
            Effect::Test { name: "id survived the round trip".to_owned(), passed: true },
            Effect::Test { name: "name survived the round trip".to_owned(), passed: true },
        ]
    );
}

#[test]
fn json_stringify_produces_real_quoted_json_not_the_debug_format() {
    // Regression check: json_stringify must emit valid JSON (quoted keys and
    // string values), unlike Value's Display impl, which is a human-readable
    // debug format such as `{name: ferris}` with no quotes at all.
    let script = r#"
        let obj = jwt_decode("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyLTQyIn0.sig")
        set_env("claims_json", json_stringify(obj.payload))
    "#;
    let outcome = run(script, HostInput::default()).expect("script should run");
    assert_eq!(
        outcome.effects,
        vec![Effect::SetEnv("claims_json".to_owned(), r#"{"sub":"user-42"}"#.to_owned())]
    );
}

#[test]
fn aes_round_trip_decrypts_back_to_the_original_plaintext() {
    let script = r#"
        let ciphertext = aes_encrypt("top secret", "my-key")
        let plaintext = aes_decrypt(ciphertext, "my-key")
        test("round trips", plaintext == "top secret")
    "#;
    let outcome = run(script, HostInput::default()).expect("script should run");
    assert_eq!(outcome.effects, vec![Effect::Test { name: "round trips".to_owned(), passed: true }]);
}

#[test]
fn aes_decrypt_with_the_wrong_key_fails_closed_not_open() {
    let script = r#"
        let ciphertext = aes_encrypt("top secret", "right-key")
        let plaintext = aes_decrypt(ciphertext, "wrong-key")
        test("decrypt with wrong key returns null", plaintext == null)
    "#;
    let outcome = run(script, HostInput::default()).expect("script should run");
    assert_eq!(
        outcome.effects,
        vec![Effect::Test { name: "decrypt with wrong key returns null".to_owned(), passed: true }]
    );
}

#[test]
fn set_body_encrypts_the_outgoing_request_body() {
    let script = r#"
        let encrypted = aes_encrypt(body(), env("body_key"))
        set_body(encrypted)
    "#;
    let input = HostInput {
        env_vars: vec![("body_key".to_owned(), "secret".to_owned())],
        body: r#"{"amount": 42}"#.to_owned(),
        ..Default::default()
    };
    let outcome = run(script, input).expect("script should run");
    assert_eq!(outcome.effects.len(), 1);
    match &outcome.effects[0] {
        Effect::SetBody(encrypted) => {
            assert_ne!(encrypted, r#"{"amount": 42}"#, "body should no longer be the plaintext");
        }
        other => panic!("expected a SetBody effect, got {other:?}"),
    }
}

#[test]
fn set_header_auto_stringifies_an_object_argument_instead_of_erroring() {
    // Regression test: passing the whole decoded JWT object (not a string)
    // to set_header used to be a hard runtime error. It should now be
    // auto-converted to JSON, same as calling json_stringify by hand.
    let script = r#"
        let claims = jwt_decode("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyLTQyIn0.sig")
        set_header("X-User-Detail", claims.payload)
    "#;
    let outcome = run(script, HostInput::default()).expect("script should run");
    assert_eq!(
        outcome.effects,
        vec![Effect::SetHeader("X-User-Detail".to_owned(), r#"{"sub":"user-42"}"#.to_owned())]
    );
}

#[test]
fn set_env_auto_stringifies_numbers_and_booleans_via_display() {
    let script = r#"
        set_env("count", 7)
        set_env("enabled", true)
    "#;
    let outcome = run(script, HostInput::default()).expect("script should run");
    assert_eq!(
        outcome.effects,
        vec![
            Effect::SetEnv("count".to_owned(), "7".to_owned()),
            Effect::SetEnv("enabled".to_owned(), "true".to_owned()),
        ]
    );
}

#[test]
fn print_logs_a_stringlike_value_for_debugging() {
    let script = r#"
        let claims = jwt_decode("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyLTQyIn0.sig")
        print("status: " + response.status)
        print(claims.payload)
    "#;
    let input = HostInput {
        response: Some(ResponseInput { status: 200, body: String::new() }),
        ..Default::default()
    };
    let outcome = run(script, input).expect("script should run");
    assert_eq!(
        outcome.effects,
        vec![
            Effect::Log("status: 200".to_owned()),
            Effect::Log(r#"{"sub":"user-42"}"#.to_owned()),
        ]
    );
}

#[test]
fn headers_returns_every_header_as_an_object_for_debugging() {
    let script = r#"
        let all = headers()
        print(all)
        test("can field-access a specific header", all.Authorization == "Bearer abc")
    "#;
    let input = HostInput {
        headers: vec![
            ("Authorization".to_owned(), "Bearer abc".to_owned()),
            ("Content-Type".to_owned(), "application/json".to_owned()),
        ],
        ..Default::default()
    };
    let outcome = run(script, input).expect("script should run");
    assert_eq!(
        outcome.effects,
        vec![
            Effect::Log(r#"{"Authorization":"Bearer abc","Content-Type":"application/json"}"#.to_owned()),
            Effect::Test { name: "can field-access a specific header".to_owned(), passed: true },
        ]
    );
}

#[test]
fn url_and_body_are_available_in_both_pre_request_and_test_scripts() {
    let pre_request_script = r#"
        test("url visible", url() == "https://api.test/orders")
        test("body visible", body() == "{\"qty\": 1}")
    "#;
    let input = HostInput {
        url: "https://api.test/orders".to_owned(),
        body: r#"{"qty": 1}"#.to_owned(),
        ..Default::default()
    };
    let outcome = run(pre_request_script, input).expect("script should run");
    assert_eq!(
        outcome.effects,
        vec![
            Effect::Test { name: "url visible".to_owned(), passed: true },
            Effect::Test { name: "body visible".to_owned(), passed: true },
        ]
    );

    // Same two builtins, now in a test-script-shaped HostInput (response set,
    // url/body still carrying what was actually sent) -- this is the part
    // that used to be impossible: debugging what a request actually sent
    // from inside its own test script.
    let test_script = r#"
        test("url visible in test script", url() == "https://api.test/orders")
        test("body visible in test script", body() == "{\"qty\": 1}")
    "#;
    let input = HostInput {
        url: "https://api.test/orders".to_owned(),
        body: r#"{"qty": 1}"#.to_owned(),
        response: Some(ResponseInput { status: 201, body: "{}".to_owned() }),
        ..Default::default()
    };
    let outcome = run(test_script, input).expect("script should run");
    assert_eq!(
        outcome.effects,
        vec![
            Effect::Test { name: "url visible in test script".to_owned(), passed: true },
            Effect::Test { name: "body visible in test script".to_owned(), passed: true },
        ]
    );
}

#[test]
fn syntax_error_reports_a_line_number() {
    let script = "let x = ";
    let err = run(script, HostInput::default()).unwrap_err();
    assert!(err.line.is_some(), "expected a line number, got: {err}");
}

#[test]
fn unknown_function_is_a_runtime_error_not_a_panic() {
    let script = "does_not_exist()";
    let err = run(script, HostInput::default()).unwrap_err();
    assert!(err.message.contains("does_not_exist"), "message was: {}", err.message);
}

#[test]
fn print_handles_response_json_object_directly() {
    let script = r#"
        let body = response.json()
        print(body)
    "#;
    let input = HostInput {
        response: Some(ResponseInput { status: 200, body: r#"{"id": 7, "name": "ferris"}"#.to_owned() }),
        ..Default::default()
    };
    let outcome = run(script, input).expect("script should run");
    assert_eq!(outcome.effects, vec![Effect::Log(r#"{"id":7,"name":"ferris"}"#.to_owned())]);
}

#[test]
fn print_handles_response_json_inline_without_a_let_binding() {
    let script = r#"print(response.json())"#;
    let input = HostInput {
        response: Some(ResponseInput { status: 200, body: r#"{"id": 7}"#.to_owned() }),
        ..Default::default()
    };
    let outcome = run(script, input).expect("script should run");
    assert_eq!(outcome.effects, vec![Effect::Log(r#"{"id":7}"#.to_owned())]);
}

#[test]
fn print_handles_json_parse_object_and_array() {
    let script = r#"
        print(json_parse("{\"a\": 1}"))
        print(json_parse("[1, 2, 3]"))
    "#;
    let outcome = run(script, HostInput::default()).expect("script should run");
    assert_eq!(
        outcome.effects,
        vec![
            Effect::Log(r#"{"a":1}"#.to_owned()),
            Effect::Log("[1,2,3]".to_owned()),
        ]
    );
}

#[test]
fn print_on_invalid_json_correctly_shows_null_not_a_bug() {
    // json_parse legitimately returns null for invalid JSON -- printing
    // "null" here is correct behavior, not a print() bug.
    let script = r#"print(json_parse("not valid json"))"#;
    let outcome = run(script, HostInput::default()).expect("script should run");
    assert_eq!(outcome.effects, vec![Effect::Log("null".to_owned())]);
}

#[test]
fn print_nested_field_access_on_an_object() {
    let script = r#"
        let claims = jwt_decode("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyLTQyIn0.sig")
        print(claims)
        print(claims.payload)
        print(claims.payload.sub)
    "#;
    let outcome = run(script, HostInput::default()).expect("script should run");
    assert_eq!(
        outcome.effects,
        vec![
            Effect::Log(r#"{"header":{"alg":"HS256"},"payload":{"sub":"user-42"}}"#.to_owned()),
            Effect::Log(r#"{"sub":"user-42"}"#.to_owned()),
            Effect::Log("user-42".to_owned()),
        ]
    );
}
