//! The dynamically-typed value a setting can hold, plus the plain-text wire
//! format used both for on-disk persistence and for the IPC protocol.
//!
//! There is no external serialization crate here on purpose: the value
//! space is small and closed (bool/int/float/str/strlist), so a hand-rolled
//! `kind:payload` encoding is enough and keeps the whole project
//! dependency-free.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    StrList(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Bool,
    Int,
    Float,
    Str,
    StrList,
}

impl fmt::Display for ValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ValueKind::Bool => "bool",
            ValueKind::Int => "int",
            ValueKind::Float => "float",
            ValueKind::Str => "str",
            ValueKind::StrList => "strlist",
        };
        write!(f, "{s}")
    }
}

impl Value {
    pub fn kind(&self) -> ValueKind {
        match self {
            Value::Bool(_) => ValueKind::Bool,
            Value::Int(_) => ValueKind::Int,
            Value::Float(_) => ValueKind::Float,
            Value::Str(_) => ValueKind::Str,
            Value::StrList(_) => ValueKind::StrList,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Ints coerce to float too, since range-checking treats both numeric
    /// kinds uniformly (see settings::validation).
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(x) => Some(*x),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_str_list(&self) -> Option<&[String]> {
        match self {
            Value::StrList(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Parses user-facing text (CLI args, interactive app input) into a
    /// `Value` of the requested kind.
    pub fn parse(kind: ValueKind, raw: &str) -> Result<Value, String> {
        match kind {
            ValueKind::Bool => match raw.trim().to_ascii_lowercase().as_str() {
                "true" | "on" | "yes" | "1" => Ok(Value::Bool(true)),
                "false" | "off" | "no" | "0" => Ok(Value::Bool(false)),
                other => Err(format!("'{other}' is not a valid boolean (try true/false)")),
            },
            ValueKind::Int => raw
                .trim()
                .parse::<i64>()
                .map(Value::Int)
                .map_err(|e| format!("'{}' is not a whole number: {e}", raw.trim())),
            ValueKind::Float => raw
                .trim()
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|e| format!("'{}' is not a number: {e}", raw.trim())),
            ValueKind::Str => Ok(Value::Str(raw.to_string())),
            ValueKind::StrList => Ok(Value::StrList(
                raw.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
            )),
        }
    }

    /// Encodes as `"<kind>:<payload>"` for on-disk storage and IPC framing.
    pub fn encode(&self) -> String {
        match self {
            Value::Bool(b) => format!("bool:{b}"),
            Value::Int(i) => format!("int:{i}"),
            Value::Float(x) => format!("float:{x}"),
            Value::Str(s) => format!("str:{}", s.replace('\\', "\\\\").replace('\n', "\\n")),
            // 0x1F (unit separator) can't appear in typed user input, so it's
            // a safe, simple list delimiter that never needs escaping.
            Value::StrList(items) => format!("strlist:{}", items.join("\u{1F}")),
        }
    }

    pub fn decode(raw: &str) -> Result<Value, String> {
        let (kind, data) = raw
            .split_once(':')
            .ok_or_else(|| format!("malformed value '{raw}' (expected kind:payload)"))?;
        match kind {
            "bool" => data
                .parse::<bool>()
                .map(Value::Bool)
                .map_err(|e| e.to_string()),
            "int" => data.parse::<i64>().map(Value::Int).map_err(|e| e.to_string()),
            "float" => data.parse::<f64>().map(Value::Float).map_err(|e| e.to_string()),
            "str" => Ok(Value::Str(data.replace("\\n", "\n").replace("\\\\", "\\"))),
            "strlist" => Ok(Value::StrList(if data.is_empty() {
                Vec::new()
            } else {
                data.split('\u{1F}').map(|s| s.to_string()).collect()
            })),
            other => Err(format!("unknown value kind '{other}'")),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::StrList(items) => write!(f, "{}", items.join(", ")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_every_kind() {
        let values = vec![
            Value::Bool(true),
            Value::Int(-42),
            Value::Float(3.5),
            Value::Str("hello world".into()),
            Value::StrList(vec!["a".into(), "b".into(), "c".into()]),
        ];
        for v in values {
            let encoded = v.encode();
            let decoded = Value::decode(&encoded).unwrap();
            assert_eq!(v, decoded, "round trip failed for {encoded}");
        }
    }

    #[test]
    fn parses_bool_synonyms() {
        assert_eq!(Value::parse(ValueKind::Bool, "on").unwrap(), Value::Bool(true));
        assert_eq!(Value::parse(ValueKind::Bool, "OFF").unwrap(), Value::Bool(false));
        assert!(Value::parse(ValueKind::Bool, "maybe").is_err());
    }

    #[test]
    fn strlist_parses_comma_separated() {
        let v = Value::parse(ValueKind::StrList, "a, b,  c").unwrap();
        assert_eq!(v.as_str_list().unwrap(), &["a".to_string(), "b".to_string(), "c".to_string()]);
    }
}
