use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEnvironment {
    pub id: String,
    pub name: String,
    pub variables: HashMap<String, String>,
    pub is_active: bool,
}

impl AppEnvironment {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            variables: HashMap::new(),
            is_active: false,
        }
    }
}

pub fn substitute(text: &str, env: Option<&AppEnvironment>) -> String {
    let Some(env) = env else { return text.to_owned() };
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("}}") {
            Some(end) => {
                let key = after[..end].trim();
                match env.variables.get(key) {
                    Some(value) => out.push_str(value),
                    None => out.push_str(&rest[start..start + end + 4]),
                }
                rest = &after[end + 2..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(vars: &[(&str, &str)]) -> AppEnvironment {
        let mut e = AppEnvironment::new("test");
        for (k, v) in vars {
            e.variables.insert((*k).to_owned(), (*v).to_owned());
        }
        e
    }

    #[test]
    fn replaces_known_variables() {
        let e = env(&[("base_url", "https://api.example.com")]);
        assert_eq!(
            substitute("{{base_url}}/posts", Some(&e)),
            "https://api.example.com/posts"
        );
    }

    #[test]
    fn tolerates_whitespace_inside_braces() {
        let e = env(&[("token", "abc")]);
        assert_eq!(substitute("Bearer {{ token }}", Some(&e)), "Bearer abc");
    }

    #[test]
    fn leaves_unknown_variables_literal() {
        let e = env(&[("a", "1")]);
        assert_eq!(substitute("{{missing}}/x", Some(&e)), "{{missing}}/x");
    }

    #[test]
    fn no_env_returns_input() {
        assert_eq!(substitute("{{a}}", None), "{{a}}");
    }

    #[test]
    fn handles_multiple_and_unclosed() {
        let e = env(&[("a", "1"), ("b", "2")]);
        assert_eq!(substitute("{{a}}-{{b}}-{{", Some(&e)), "1-2-{{");
    }
}
