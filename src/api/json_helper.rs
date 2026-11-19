//! Lightweight JSON utility for zero-dependency Rust web API

use std::collections::HashMap;

pub fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

#[derive(Debug, Clone)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(map) => map.get(key),
            _ => None,
        }
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_number(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.as_number())
    }
}

pub fn parse_json(input: &str) -> Option<JsonValue> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let mut idx = 0;
    parse_value(&chars, &mut idx)
}

fn skip_whitespace(chars: &[char], idx: &mut usize) {
    while *idx < chars.len() && (chars[*idx].is_whitespace() || chars[*idx] == '\r' || chars[*idx] == '\n') {
        *idx += 1;
    }
}

fn parse_value(chars: &[char], idx: &mut usize) -> Option<JsonValue> {
    skip_whitespace(chars, idx);
    if *idx >= chars.len() {
        return None;
    }

    match chars[*idx] {
        '{' => parse_object(chars, idx),
        '[' => parse_array(chars, idx),
        '"' => parse_string(chars, idx).map(JsonValue::String),
        't' | 'f' => parse_bool(chars, idx),
        'n' => parse_null(chars, idx),
        '-' | '0'..='9' => parse_number(chars, idx),
        _ => None,
    }
}

fn parse_object(chars: &[char], idx: &mut usize) -> Option<JsonValue> {
    if chars[*idx] != '{' {
        return None;
    }
    *idx += 1;
    let mut map = HashMap::new();

    loop {
        skip_whitespace(chars, idx);
        if *idx >= chars.len() {
            return None;
        }
        if chars[*idx] == '}' {
            *idx += 1;
            return Some(JsonValue::Object(map));
        }

        let key = parse_string(chars, idx)?;
        skip_whitespace(chars, idx);
        if *idx >= chars.len() || chars[*idx] != ':' {
            return None;
        }
        *idx += 1; // skip ':'

        let val = parse_value(chars, idx)?;
        map.insert(key, val);

        skip_whitespace(chars, idx);
        if *idx < chars.len() && chars[*idx] == ',' {
            *idx += 1;
        } else if *idx < chars.len() && chars[*idx] == '}' {
            *idx += 1;
            return Some(JsonValue::Object(map));
        } else {
            return None;
        }
    }
}

fn parse_array(chars: &[char], idx: &mut usize) -> Option<JsonValue> {
    if chars[*idx] != '[' {
        return None;
    }
    *idx += 1;
    let mut arr = Vec::new();

    loop {
        skip_whitespace(chars, idx);
        if *idx >= chars.len() {
            return None;
        }
        if chars[*idx] == ']' {
            *idx += 1;
            return Some(JsonValue::Array(arr));
        }

        let val = parse_value(chars, idx)?;
        arr.push(val);

        skip_whitespace(chars, idx);
        if *idx < chars.len() && chars[*idx] == ',' {
            *idx += 1;
        } else if *idx < chars.len() && chars[*idx] == ']' {
            *idx += 1;
            return Some(JsonValue::Array(arr));
        } else {
            return None;
        }
    }
}

fn parse_string(chars: &[char], idx: &mut usize) -> Option<String> {
    if chars[*idx] != '"' {
        return None;
    }
    *idx += 1;
    let mut s = String::new();

    while *idx < chars.len() {
        let c = chars[*idx];
        if c == '"' {
            *idx += 1;
            return Some(s);
        } else if c == '\\' {
            *idx += 1;
            if *idx >= chars.len() {
                return None;
            }
            match chars[*idx] {
                '"' => s.push('"'),
                '\\' => s.push('\\'),
                '/' => s.push('/'),
                'n' => s.push('\n'),
                'r' => s.push('\r'),
                't' => s.push('\t'),
                'b' => s.push('\x08'),
                'f' => s.push('\x0c'),
                _ => s.push(chars[*idx]),
            }
            *idx += 1;
        } else {
            s.push(c);
            *idx += 1;
        }
    }
    None
}

fn parse_bool(chars: &[char], idx: &mut usize) -> Option<JsonValue> {
    let rest: String = chars[*idx..].iter().take(5).collect();
    if rest.starts_with("true") {
        *idx += 4;
        Some(JsonValue::Bool(true))
    } else if rest.starts_with("false") {
        *idx += 5;
        Some(JsonValue::Bool(false))
    } else {
        None
    }
}

fn parse_null(chars: &[char], idx: &mut usize) -> Option<JsonValue> {
    let rest: String = chars[*idx..].iter().take(4).collect();
    if rest.starts_with("null") {
        *idx += 4;
        Some(JsonValue::Null)
    } else {
        None
    }
}

fn parse_number(chars: &[char], idx: &mut usize) -> Option<JsonValue> {
    let mut s = String::new();
    while *idx < chars.len() {
        let c = chars[*idx];
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
            s.push(c);
            *idx += 1;
        } else {
            break;
        }
    }
    s.parse::<f64>().ok().map(JsonValue::Number)
}
