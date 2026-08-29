//! Runs `settings::validation::validate` against specs pulled from the
//! real, fully-registered schema (not hand-built toy specs — those are
//! already covered by the unit tests inside `settings::validation`
//! itself).

use mitos_settings::categories;
use mitos_settings::settings::schema::Schema;
use mitos_settings::settings::validation::{validate, ValidationError};
use mitos_settings::settings::value::Value;

fn full_schema() -> Schema {
    let mut schema = Schema::new();
    categories::register_all(&mut schema);
    schema
}

#[test]
fn range_constraints_hold_for_a_registered_setting() {
    let schema = full_schema();
    let spec = schema.get("display.brightness").expect("display.brightness should be registered");
    assert!(validate(spec, &Value::Int(50)).is_ok());
    assert!(matches!(validate(spec, &Value::Int(200)), Err(ValidationError::OutOfRange { .. })));
    assert!(matches!(validate(spec, &Value::Int(-5)), Err(ValidationError::OutOfRange { .. })));
}

#[test]
fn choice_constraints_hold_for_a_registered_setting() {
    let schema = full_schema();
    let spec = schema.get("theme.mode").expect("theme.mode should be registered");
    assert!(validate(spec, &Value::Str("dark".into())).is_ok());
    assert!(matches!(validate(spec, &Value::Str("rainbow".into())), Err(ValidationError::NotAChoice { .. })));
}

#[test]
fn type_mismatches_are_rejected() {
    let schema = full_schema();
    let spec = schema.get("sound.volume").expect("sound.volume should be registered");
    assert!(matches!(
        validate(spec, &Value::Str("loud".into())),
        Err(ValidationError::TypeMismatch { .. })
    ));
}

#[test]
fn read_only_specs_reject_every_value() {
    let schema = full_schema();
    let spec = schema.get("security.secure_boot_status").expect("should be registered");
    assert!(spec.read_only);
    assert!(matches!(validate(spec, &spec.default.clone()), Err(ValidationError::ReadOnly(_))));
}

/// The strongest sanity check here: every default value shipped for every
/// non-read-only setting must itself satisfy that setting's own
/// constraints. A failure here means a category registered an
/// inconsistent spec (e.g. a default outside its own declared range).
#[test]
fn every_writable_settings_default_passes_its_own_validation() {
    let schema = full_schema();
    for spec in schema.all() {
        if spec.read_only {
            continue;
        }
        assert!(
            validate(spec, &spec.default).is_ok(),
            "default for '{}' fails its own validation constraints",
            spec.key
        );
    }
}
