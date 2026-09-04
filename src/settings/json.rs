//! JSON export of the schema and current values — the integration surface
//! for any MITOS component that *isn't* Rust (or doesn't want to link this
//! crate): `mitos-docs` for auto-generating a settings reference,
//! `mitos-gui`/`mitos-installer`/anything else that wants to discover or
//! snapshot settings without parsing this crate's internal `kind:payload`
//! store format. See `INTEGRATION.md` at the repo root.
//!
//! This is a small, hand-rolled encoder, not `serde_json` — deliberately.
//! The shapes here are simple and fixed (flat objects, strings, numbers,
//! bools, arrays of strings), so a hand-written encoder is easy to verify
//! by hand, and it keeps the project dependency-free. If this ever needs
//! to encode something more complex, that's the signal to pull in
//! `serde`+`serde_json` instead of growing this file — see the "On
//! dependencies" section of INTEGRATION.md.

use crate::settings::manager::SettingsManager;
use crate::settings::schema::{Schema, SettingSpec, ValueFormat};
use crate::settings::value::Value;

/// Escapes a string for safe embedding in a JSON string literal, per
/// RFC 8259 §7. Handles the characters that actually occur in this
/// project's data (labels, descriptions, paths) — not a general-purpose
/// escaper, but correct for everything this crate ever stores.
pub fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", escape_str(s))
}

fn value_to_json(value: &Value) -> String {
    match value {
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Str(s) => quoted(s),
        Value::StrList(items) => format!("[{}]", items.iter().map(|s| quoted(s)).collect::<Vec<_>>().join(", ")),
    }
}

/// String name for a `ValueFormat`, used in the JSON export so a non-Rust
/// consumer can tell e.g. "this needs a hex color string" without any
/// Rust-specific knowledge. Add a new arm here whenever `ValueFormat`
/// grows a variant.
fn format_name(format: ValueFormat) -> &'static str {
    match format {
        ValueFormat::HexColor => "hex_color",
    }
}

/// The full, static schema — every setting's key, type, default,
/// privilege, and constraints — as JSON. Doesn't touch persisted values at
/// all; this is "what can be configured," not "what is configured right
/// now" (see `values_to_json` for that). Iteration order matches
/// `Schema::all`, which is sorted by key (it's backed by a `BTreeMap`), so
/// output is stable across runs — safe to diff or check into another
/// repo.
pub fn schema_to_json(schema: &Schema) -> String {
    let specs: Vec<&SettingSpec> = schema.all().collect();
    let mut out = String::from("{\n  \"settings\": [\n");

    for (i, spec) in specs.iter().enumerate() {
        out.push_str("    {\n");
        out.push_str(&format!("      \"key\": {},\n", quoted(spec.key)));
        out.push_str(&format!("      \"category\": {},\n", quoted(spec.category)));
        out.push_str(&format!("      \"label\": {},\n", quoted(spec.label)));
        out.push_str(&format!("      \"description\": {},\n", quoted(spec.description)));
        out.push_str(&format!("      \"kind\": {},\n", quoted(&spec.kind.to_string())));
        out.push_str(&format!("      \"default\": {},\n", value_to_json(&spec.default)));
        out.push_str(&format!("      \"privilege\": {},\n", quoted(&spec.privilege.to_string())));
        out.push_str(&format!("      \"read_only\": {}", spec.read_only));

        if let Some(choices) = spec.choices {
            let joined = choices.iter().map(|c| quoted(c)).collect::<Vec<_>>().join(", ");
            out.push_str(&format!(",\n      \"choices\": [{joined}]"));
        }
        if let Some((lo, hi)) = spec.range {
            out.push_str(&format!(",\n      \"range\": [{lo}, {hi}]"));
        }
        if let Some(format) = spec.format {
            out.push_str(&format!(",\n      \"format\": {}", quoted(format_name(format))));
        }

        out.push_str("\n    }");
        if i + 1 < specs.len() {
            out.push(',');
        }
        out.push('\n');
    }

    out.push_str("  ]\n}\n");
    out
}

/// Current values (not the static schema) as a flat JSON object, optionally
/// filtered to one category. This is a snapshot: whoever calls this owns
/// re-fetching it if they need to notice later changes (for a live push
/// model instead, see `home.conf`, or the IPC daemon's `LIST` request).
pub fn values_to_json(manager: &SettingsManager, category: Option<&str>) -> String {
    let mut specs: Vec<&SettingSpec> = manager.schema().all().collect();
    if let Some(cat) = category {
        specs.retain(|s| s.category == cat);
    }

    let mut out = String::from("{\n");
    for (i, spec) in specs.iter().enumerate() {
        let value_json = manager.get(spec.key).map(value_to_json).unwrap_or_else(|_| "null".to_string());
        out.push_str(&format!("  {}: {value_json}", quoted(spec.key)));
        if i + 1 < specs.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_str_handles_quotes_backslashes_and_control_chars() {
        assert_eq!(escape_str("hello"), "hello");
        assert_eq!(escape_str("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_str("back\\slash"), "back\\\\slash");
        assert_eq!(escape_str("line\nbreak"), "line\\nbreak");
        assert_eq!(escape_str("tab\ttab"), "tab\\ttab");
        assert_eq!(escape_str("bell\x07here"), "bell\\u0007here");
    }

    #[test]
    fn a_format_constrained_spec_reports_its_format_in_json() {
        use crate::permissions::PrivilegeLevel;
        use crate::settings::value::ValueKind;

        let mut schema = Schema::new();
        schema.register(
            SettingSpec::new(
                "test.color",
                "test",
                "Color",
                "desc",
                ValueKind::Str,
                Value::Str("#ffffff".into()),
                PrivilegeLevel::User,
            )
            .format(ValueFormat::HexColor),
        );
        let json = schema_to_json(&schema);
        assert!(json.contains("\"format\": \"hex_color\""));
    }

    #[test]
    fn schema_to_json_includes_every_registered_key_exactly_once() {
        let mut schema = Schema::new();
        crate::categories::register_all(&mut schema);
        let json = schema_to_json(&schema);

        for spec in schema.all() {
            let needle = format!("\"key\": \"{}\"", spec.key);
            let count = json.matches(&needle).count();
            assert_eq!(count, 1, "expected exactly one occurrence of {} in schema JSON, found {count}", spec.key);
        }
    }

    #[test]
    fn schema_to_json_is_valid_enough_to_bracket_match() {
        // Not a full JSON parser -- just a structural sanity check that
        // catches an obviously malformed encoder (unbalanced brackets).
        let mut schema = Schema::new();
        crate::categories::register_all(&mut schema);
        let json = schema_to_json(&schema);

        let mut depth = 0i32;
        for c in json.chars() {
            match c {
                '{' | '[' => depth += 1,
                '}' | ']' => depth -= 1,
                _ => {}
            }
            assert!(depth >= 0, "unbalanced brackets in generated JSON");
        }
        assert_eq!(depth, 0, "unbalanced brackets in generated JSON");
    }

    #[test]
    fn values_to_json_filters_by_category() {
        use crate::settings::manager::test_support::isolated_manager;
        use crate::settings::manager::Mode;

        let (manager, dir) = isolated_manager(Mode::Standalone);
        let json = values_to_json(&manager, Some("sound"));

        assert!(json.contains("\"sound.volume\""));
        assert!(!json.contains("\"display.brightness\""));

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn values_to_json_reflects_a_changed_value() {
        use crate::settings::manager::test_support::isolated_manager;
        use crate::settings::manager::Mode;

        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        manager.set("sound.volume", Value::Int(42)).unwrap();
        let json = values_to_json(&manager, None);

        assert!(json.contains("\"sound.volume\": 42"));
        std::fs::remove_dir_all(dir).ok();
    }
}
