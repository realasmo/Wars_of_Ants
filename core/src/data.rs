//! The shipped ruleset lives in `data/*.jsonc` at the repo root and is
//! embedded into the binary at compile time — one source of truth shared
//! by the wasm client, the native tests, and the replay tooling, so all
//! three can never disagree about the shipped balance. Files are JSON
//! with comments (JSONC); `strip_jsonc` also runs in `Sim::parse_rules`,
//! so the admin drawer's JSON box accepts the same annotated documents
//! and comments never reach the rules digest.
//!
//! The four-file split (ants / resources / world / colony) is thematic
//! convention for human editors: the loader merges top-level keys
//! key-by-key, so a key still parses wherever it lives — but each key in
//! exactly one file, or the load fails loudly.

use std::sync::LazyLock;

use crate::rules::GameRules;

/// (file name, embedded text) in a fixed order — the order only matters
/// for duplicate-key error messages.
pub const DATA_FILES: &[(&str, &str)] = &[
    ("ants.jsonc", include_str!("../../data/ants.jsonc")),
    (
        "resources.jsonc",
        include_str!("../../data/resources.jsonc"),
    ),
    ("world.jsonc", include_str!("../../data/world.jsonc")),
    ("colony.jsonc", include_str!("../../data/colony.jsonc")),
];

/// Strip `// line` and `/* block */` comments from JSONC text. String-aware:
/// comment markers inside string literals are kept verbatim (including
/// escape sequences). Everything else must remain strict JSON — this adds
/// no leniency beyond comments (no trailing commas, no unquoted keys).
pub fn strip_jsonc(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    let mut in_string = false;
    while i < b.len() {
        let c = b[i];
        if in_string {
            out.push(c);
            if c == b'\\' && i + 1 < b.len() {
                // escaped char passes through verbatim (\" \\ \/ …)
                out.push(b[i + 1]);
                i += 2;
                continue;
            }
            if c == b'"' {
                in_string = false;
            }
            i += 1;
        } else if c == b'"' {
            in_string = true;
            out.push(c);
            i += 1;
        } else if c == b'/' && b.get(i + 1) == Some(&b'/') {
            // line comment: skip to the newline (kept, so error line
            // numbers still match the source file)
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if c == b'/' && b.get(i + 1) == Some(&b'*') {
            // block comment: becomes one space, newlines kept
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                if b[i] == b'\n' {
                    out.push(b'\n');
                }
                i += 1;
            }
            i = (i + 2).min(b.len());
            out.push(b' ');
        } else {
            out.push(c);
            i += 1;
        }
    }
    String::from_utf8(out).expect("stripping ASCII comment bytes keeps UTF-8 valid")
}

/// Merge JSONC rule files into one validated `GameRules`. Every failure
/// mode is loud: a file that isn't a JSON object, a key provided by two
/// files, a missing or unknown field, a validation violation — collected,
/// never silently dropped or defaulted.
pub fn merge_rules(files: &[(&str, &str)]) -> Result<GameRules, Vec<String>> {
    let mut merged = serde_json::Map::new();
    let mut errs = Vec::new();
    for (name, text) in files {
        match serde_json::from_str::<serde_json::Value>(&strip_jsonc(text)) {
            Ok(serde_json::Value::Object(map)) => {
                for (key, value) in map {
                    if merged.contains_key(&key) {
                        errs.push(format!(
                            "{name}: '{key}' is already provided by an earlier file — each key in exactly one file"
                        ));
                    } else {
                        merged.insert(key, value);
                    }
                }
            }
            Ok(_) => errs.push(format!("{name}: top level must be a JSON object")),
            Err(e) => errs.push(format!("{name}: {e}")),
        }
    }
    if !errs.is_empty() {
        return Err(errs);
    }
    match serde_json::from_value::<GameRules>(serde_json::Value::Object(merged)) {
        Ok(rules) => rules.validate().map(|()| rules),
        Err(e) => Err(vec![format!("rules: {e}")]),
    }
}

/// The shipped rules: the four `data/*.jsonc` files, merged and validated.
pub fn load_shipped_rules() -> Result<GameRules, Vec<String>> {
    merge_rules(DATA_FILES)
}

/// The shipped rules, parsed and validated once. Panics on invalid data
/// files: shipping files that don't parse is a build-time-class mistake,
/// and every test run exercises this path through `GameRules::default()`.
pub fn shipped() -> GameRules {
    static RULES: LazyLock<GameRules> = LazyLock::new(|| {
        load_shipped_rules()
            .unwrap_or_else(|errs| panic!("invalid shipped data files: {}", errs.join("; ")))
    });
    RULES.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_line_and_block_comments_outside_strings() {
        let src = "{\n  // line comment\n  \"a\": 1, /* block\n  comment */ \"b\": 2\n}\n";
        // comments vanish; the indentation before them and all newlines
        // (including the block comment's inner ones) stay
        assert_eq!(strip_jsonc(src), "{\n  \n  \"a\": 1, \n  \"b\": 2\n}\n");
    }

    #[test]
    fn keeps_comment_markers_inside_strings_and_escapes() {
        let src = r#"{"a": "x // not a comment", "b": "y /* nope */", "c": "quote \" then // still string", "d": "backslash \\ before end"}"#;
        assert_eq!(strip_jsonc(src), src);
    }

    #[test]
    fn stripped_documents_still_parse_as_strict_json() {
        let src = "{\n  // tuned 2026-10\n  \"max_ants\": 24 /* hands off */\n}";
        let v: serde_json::Value = serde_json::from_str(&strip_jsonc(src)).unwrap();
        assert_eq!(v["max_ants"], 24);
    }

    #[test]
    fn unterminated_comment_degrades_to_a_parse_error_not_a_panic() {
        let v = serde_json::from_str::<serde_json::Value>(&strip_jsonc("{\"a\": 1 /* oops"));
        assert!(v.is_err());
    }
}
