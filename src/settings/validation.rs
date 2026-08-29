//! Validates a candidate `Value` against the constraints declared on its
//! `SettingSpec` (type, allowed choices, numeric range, read-only-ness)
//! before `SettingsManager` ever persists or applies it.

use crate::settings::schema::SettingSpec;
use crate::settings::value::Value;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    TypeMismatch { expected: String, found: String },
    NotAChoice { value: String, allowed: Vec<String> },
    OutOfRange { value: f64, min: f64, max: f64 },
    ReadOnly(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::TypeMismatch { expected, found } => {
                write!(f, "expected a {expected} value, found {found}")
            }
            ValidationError::NotAChoice { value, allowed } => {
                write!(f, "'{value}' is not one of: {}", allowed.join(", "))
            }
            ValidationError::OutOfRange { value, min, max } => {
                write!(f, "{value} is outside the allowed range {min}..={max}")
            }
            ValidationError::ReadOnly(key) => write!(f, "'{key}' is read-only"),
        }
    }
}

impl std::error::Error for ValidationError {}

pub fn validate(spec: &SettingSpec, value: &Value) -> Result<(), ValidationError> {
    if spec.read_only {
        return Err(ValidationError::ReadOnly(spec.key.to_string()));
    }

    if value.kind() != spec.kind {
        return Err(ValidationError::TypeMismatch {
            expected: spec.kind.to_string(),
            found: value.kind().to_string(),
        });
    }

    if let Some(choices) = spec.choices {
        if let Some(s) = value.as_str() {
            if !choices.contains(&s) {
                return Err(ValidationError::NotAChoice {
                    value: s.to_string(),
                    allowed: choices.iter().map(|c| c.to_string()).collect(),
                });
            }
        }
    }

    if let Some((min, max)) = spec.range {
        if let Some(n) = value.as_float() {
            if n < min || n > max {
                return Err(ValidationError::OutOfRange { value: n, min, max });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::PrivilegeLevel;
    use crate::settings::value::ValueKind;

    fn spec() -> SettingSpec {
        SettingSpec::new(
            "display.brightness",
            "display",
            "Brightness",
            "desc",
            ValueKind::Int,
            Value::Int(80),
            PrivilegeLevel::User,
        )
        .range(0.0, 100.0)
    }

    #[test]
    fn accepts_value_in_range() {
        assert!(validate(&spec(), &Value::Int(50)).is_ok());
    }

    #[test]
    fn rejects_out_of_range() {
        assert!(matches!(validate(&spec(), &Value::Int(150)), Err(ValidationError::OutOfRange { .. })));
    }

    #[test]
    fn rejects_wrong_type() {
        assert!(matches!(
            validate(&spec(), &Value::Str("nope".into())),
            Err(ValidationError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn rejects_choice_not_in_list() {
        let s = SettingSpec::new(
            "power.profile",
            "power",
            "Profile",
            "desc",
            ValueKind::Str,
            Value::Str("balanced".into()),
            PrivilegeLevel::User,
        )
        .choices(&["power-saver", "balanced", "performance"]);
        assert!(validate(&s, &Value::Str("balanced".into())).is_ok());
        assert!(matches!(
            validate(&s, &Value::Str("turbo".into())),
            Err(ValidationError::NotAChoice { .. })
        ));
    }

    #[test]
    fn rejects_writes_to_read_only() {
        let s = spec().read_only();
        assert!(matches!(validate(&s, &Value::Int(50)), Err(ValidationError::ReadOnly(_))));
    }
}
