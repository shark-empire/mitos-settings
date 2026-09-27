//! A minimal, hand-rolled JSON value type, encoder, and parser -- just
//! enough to speak mitos-network's wire protocol (see `network::client`)
//! without pulling in serde_json. This is the one point where staying
//! dependency-free (see `Cargo.toml`, and `grants::service_client`'s doc
//! comment for the same call made there, for a *plain-text* protocol)
//! runs into a daemon whose own protocol happens to be JSON. Not a
//! general-purpose JSON library -- no arbitrary-precision numbers, no
//! comments, no streaming -- just enough of the spec to round-trip the
//! specific request/response shapes this crate actually sends and reads.
//!
//! See "How to connect network to gui and settings.txt" (mitos-network's
//! own integration guide, copied into `docs/network-integration.md`) for
//! the wire format this exists to speak: `Request`/`Response`/`Event`
//! all serialize as serde's default "externally tagged" enum
//! representation -- a bare JSON string for a fieldless variant
//! (`"ListBluetoothDevices"`), a single-key object for one carrying
//! fields (`{"ConnectWifi":{"device":"wlan0", ...}}`).

use std::fmt::Write as _;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn string(s: impl Into<String>) -> Json {
        Json::String(s.into())
    }

    /// The single-key `{"Variant":payload}` shape for an enum variant
    /// carrying fields.
    pub fn variant(name: &str, payload: Json) -> Json {
        Json::Object(vec![(name.to_string(), payload)])
    }

    /// The bare-string shape for a fieldless (unit) enum variant --
    /// serde's default representation doesn't wrap these in an object.
    pub fn unit_variant(name: &str) -> Json {
        Json::String(name.to_string())
    }

    pub fn object(fields: Vec<(&str, Json)>) -> Json {
        Json::Object(
            fields
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        )
    }

    pub fn encode(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Number(n) => {
                // Prefer a bare integer literal ("80") over "80.0" when
                // it's exact -- unambiguously valid JSON for either an
                // integer- or float-typed field on the other end, unlike
                // the reverse.
                if n.fract() == 0.0 && n.is_finite() && n.abs() < 1e15 {
                    let _ = write!(out, "{}", *n as i64);
                } else {
                    let _ = write!(out, "{n}");
                }
            }
            Json::String(s) => write_json_string(s, out),
            Json::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Object(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_json_string(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }

    /// The single key of a `{"Variant":payload}`-shaped object and the
    /// payload under it -- how a `Response`/`Event`'s variant name and
    /// contents are read back out. `None` for anything that isn't a
    /// single-key object (a fieldless `Response` would be a bare string
    /// instead -- use `as_str` for that case).
    pub fn as_single_variant(&self) -> Option<(&str, &Json)> {
        match self {
            Json::Object(fields) if fields.len() == 1 => Some((fields[0].0.as_str(), &fields[0].1)),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(fields) => fields
                .iter()
                .find(|(k, _)| k.as_str() == key)
                .map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(items) => Some(items.as_slice()),
            _ => None,
        }
    }
}

fn write_json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Parses one JSON value from `input`, ignoring leading/trailing
/// whitespace. Errors on anything left over after the value, so a
/// malformed or truncated frame is caught here rather than silently
/// read as a partial value.
pub fn parse(input: &str) -> Result<Json, String> {
    let mut chars = input.chars().peekable();
    skip_whitespace(&mut chars);
    let value = parse_value(&mut chars)?;
    skip_whitespace(&mut chars);
    if chars.next().is_some() {
        return Err("trailing data after JSON value".to_string());
    }
    Ok(value)
}

fn skip_whitespace(chars: &mut Peekable<Chars>) {
    while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
        chars.next();
    }
}

fn parse_value(chars: &mut Peekable<Chars>) -> Result<Json, String> {
    skip_whitespace(chars);
    match chars.peek() {
        Some('"') => parse_string(chars).map(Json::String),
        Some('{') => parse_object(chars),
        Some('[') => parse_array(chars),
        Some('t') | Some('f') => parse_bool(chars),
        Some('n') => parse_null(chars),
        Some(c) if *c == '-' || c.is_ascii_digit() => parse_number(chars),
        Some(c) => Err(format!("unexpected character '{c}' at start of JSON value")),
        None => Err("unexpected end of input while expecting a JSON value".to_string()),
    }
}

fn expect(chars: &mut Peekable<Chars>, expected: char) -> Result<(), String> {
    match chars.next() {
        Some(c) if c == expected => Ok(()),
        Some(c) => Err(format!("expected '{expected}', found '{c}'")),
        None => Err(format!("expected '{expected}', found end of input")),
    }
}

fn parse_literal(chars: &mut Peekable<Chars>, literal: &str, value: Json) -> Result<Json, String> {
    for expected in literal.chars() {
        match chars.next() {
            Some(c) if c == expected => {}
            _ => return Err(format!("expected literal '{literal}'")),
        }
    }
    Ok(value)
}

fn parse_bool(chars: &mut Peekable<Chars>) -> Result<Json, String> {
    match chars.peek() {
        Some('t') => parse_literal(chars, "true", Json::Bool(true)),
        Some('f') => parse_literal(chars, "false", Json::Bool(false)),
        _ => Err("expected 'true' or 'false'".to_string()),
    }
}

fn parse_null(chars: &mut Peekable<Chars>) -> Result<Json, String> {
    parse_literal(chars, "null", Json::Null)
}

fn parse_string(chars: &mut Peekable<Chars>) -> Result<String, String> {
    expect(chars, '"')?;
    let mut s = String::new();
    loop {
        match chars.next() {
            None => return Err("unterminated string".to_string()),
            Some('"') => return Ok(s),
            Some('\\') => match chars.next() {
                Some('"') => s.push('"'),
                Some('\\') => s.push('\\'),
                Some('/') => s.push('/'),
                Some('n') => s.push('\n'),
                Some('t') => s.push('\t'),
                Some('r') => s.push('\r'),
                Some('b') => s.push('\u{0008}'),
                Some('f') => s.push('\u{000C}'),
                Some('u') => {
                    let mut hex = String::new();
                    for _ in 0..4 {
                        match chars.next() {
                            Some(h) => hex.push(h),
                            None => return Err("truncated \\u escape".to_string()),
                        }
                    }
                    let code = u32::from_str_radix(&hex, 16)
                        .map_err(|_| format!("invalid \\u escape '{hex}'"))?;
                    match char::from_u32(code) {
                        Some(c) => s.push(c),
                        // A lone UTF-16 surrogate half -- not expected in
                        // anything this daemon actually sends (device
                        // names, SSIDs, etc.), so this is a parse error
                        // rather than attempting surrogate pairing.
                        None => {
                            return Err(format!("invalid \\u{hex} escape (unpaired surrogate)"))
                        }
                    }
                }
                Some(other) => return Err(format!("invalid escape '\\{other}'")),
                None => return Err("truncated escape sequence".to_string()),
            },
            Some(c) => s.push(c),
        }
    }
}

fn parse_number(chars: &mut Peekable<Chars>) -> Result<Json, String> {
    let mut s = String::new();
    if matches!(chars.peek(), Some('-')) {
        s.push(chars.next().unwrap());
    }
    while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
        s.push(chars.next().unwrap());
    }
    if matches!(chars.peek(), Some('.')) {
        s.push(chars.next().unwrap());
        while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
            s.push(chars.next().unwrap());
        }
    }
    if matches!(chars.peek(), Some('e') | Some('E')) {
        s.push(chars.next().unwrap());
        if matches!(chars.peek(), Some('+') | Some('-')) {
            s.push(chars.next().unwrap());
        }
        while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
            s.push(chars.next().unwrap());
        }
    }
    s.parse::<f64>()
        .map(Json::Number)
        .map_err(|_| format!("invalid number '{s}'"))
}

fn parse_array(chars: &mut Peekable<Chars>) -> Result<Json, String> {
    expect(chars, '[')?;
    let mut items = Vec::new();
    skip_whitespace(chars);
    if matches!(chars.peek(), Some(']')) {
        chars.next();
        return Ok(Json::Array(items));
    }
    loop {
        items.push(parse_value(chars)?);
        skip_whitespace(chars);
        match chars.next() {
            Some(',') => {
                skip_whitespace(chars);
                continue;
            }
            Some(']') => return Ok(Json::Array(items)),
            Some(c) => return Err(format!("expected ',' or ']' in array, found '{c}'")),
            None => return Err("unterminated array".to_string()),
        }
    }
}

fn parse_object(chars: &mut Peekable<Chars>) -> Result<Json, String> {
    expect(chars, '{')?;
    let mut fields = Vec::new();
    skip_whitespace(chars);
    if matches!(chars.peek(), Some('}')) {
        chars.next();
        return Ok(Json::Object(fields));
    }
    loop {
        skip_whitespace(chars);
        let key = parse_string(chars)?;
        skip_whitespace(chars);
        expect(chars, ':')?;
        let value = parse_value(chars)?;
        fields.push((key, value));
        skip_whitespace(chars);
        match chars.next() {
            Some(',') => continue,
            Some('}') => return Ok(Json::Object(fields)),
            Some(c) => return Err(format!("expected ',' or '}}' in object, found '{c}'")),
            None => return Err("unterminated object".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_struct_variant_request() {
        let req = Json::variant(
            "ConnectWifi",
            Json::object(vec![
                ("device", Json::string("wlan0")),
                ("ssid", Json::string("Home")),
                ("security", Json::string("Wpa2Psk")),
                ("passphrase", Json::string("hunter2")),
            ]),
        );
        let encoded = req.encode();
        let parsed = parse(&encoded).unwrap();
        let (name, payload) = parsed.as_single_variant().unwrap();
        assert_eq!(name, "ConnectWifi");
        assert_eq!(payload.get("device").and_then(Json::as_str), Some("wlan0"));
        assert_eq!(payload.get("ssid").and_then(Json::as_str), Some("Home"));
    }

    #[test]
    fn round_trips_a_unit_variant_request() {
        let req = Json::unit_variant("ListBluetoothDevices");
        let encoded = req.encode();
        assert_eq!(encoded, "\"ListBluetoothDevices\"");
        let parsed = parse(&encoded).unwrap();
        assert_eq!(parsed.as_str(), Some("ListBluetoothDevices"));
    }

    #[test]
    fn parses_an_error_response() {
        let parsed = parse(r#"{"Error":"uid 1000 lacks ManageWifi"}"#).unwrap();
        let (name, payload) = parsed.as_single_variant().unwrap();
        assert_eq!(name, "Error");
        assert_eq!(payload.as_str(), Some("uid 1000 lacks ManageWifi"));
    }

    #[test]
    fn parses_an_array_of_objects() {
        let parsed = parse(r#"[{"mac":"AA:BB:CC:DD:EE:FF","name":"Headphones"}]"#).unwrap();
        let items = parsed.as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].get("mac").and_then(Json::as_str),
            Some("AA:BB:CC:DD:EE:FF")
        );
    }

    #[test]
    fn parses_escaped_strings() {
        let parsed = parse(r#""line one\nline two \"quoted\"""#).unwrap();
        assert_eq!(parsed.as_str(), Some("line one\nline two \"quoted\""));
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(parse(r#"{"a":1} garbage"#).is_err());
    }

    #[test]
    fn whole_numbers_encode_without_a_decimal_point() {
        assert_eq!(Json::Number(80.0).encode(), "80");
        assert_eq!(Json::Number(-3.0).encode(), "-3");
        assert_eq!(Json::Number(2.5).encode(), "2.5");
    }
}
