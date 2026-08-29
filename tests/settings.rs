//! Tests for the data model layer: `Value` parsing/encoding and `Schema`
//! lookups, independent of persistence or the manager.

use mitos_settings::categories;
use mitos_settings::permissions::PrivilegeLevel;
use mitos_settings::settings::schema::Schema;
use mitos_settings::settings::value::{Value, ValueKind};

fn full_schema() -> Schema {
    let mut schema = Schema::new();
    categories::register_all(&mut schema);
    schema
}

#[test]
fn value_encode_decode_round_trips_every_kind() {
    let values = [
        Value::Bool(false),
        Value::Int(1234),
        Value::Float(-0.5),
        Value::Str("multi\nline\\value".to_string()),
        Value::StrList(vec!["one".into(), "two, still one item until parsed".into()]),
    ];
    for v in values {
        let decoded = Value::decode(&v.encode()).unwrap();
        assert_eq!(v, decoded);
    }
}

#[test]
fn value_parse_rejects_malformed_input_per_kind() {
    assert!(Value::parse(ValueKind::Int, "not a number").is_err());
    assert!(Value::parse(ValueKind::Float, "not a number").is_err());
    assert!(Value::parse(ValueKind::Bool, "maybe").is_err());
    assert!(Value::parse(ValueKind::Str, "anything at all").is_ok());
}

#[test]
fn schema_contains_every_advertised_category() {
    let schema = full_schema();
    assert_eq!(schema.categories().len(), categories::all().len());
}

#[test]
fn schema_lookup_by_category_only_returns_that_categorys_keys() {
    let schema = full_schema();
    for spec in schema.by_category("sound") {
        assert_eq!(spec.category, "sound");
    }
    assert!(schema.by_category("sound").count() > 0);
}

#[test]
fn admin_and_root_settings_exist_alongside_user_settings() {
    let schema = full_schema();
    let levels: std::collections::HashSet<PrivilegeLevel> = schema.all().map(|s| s.privilege).collect();
    assert!(levels.contains(&PrivilegeLevel::User));
    assert!(levels.contains(&PrivilegeLevel::Admin));
}

#[test]
fn unknown_key_returns_none_not_a_panic() {
    let schema = full_schema();
    assert!(schema.get("totally.made.up.key").is_none());
}
