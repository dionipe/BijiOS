//! TOON (Token-Oriented Object Notation) v4 Engine
//! Follows toonformat.dev v4 specification:
//! - Tabular array formatting with header declarations: `name[count]{col1,col2,...}:`
//! - YAML-style indentation for key-value maps
//! - Token-efficient representation for LLMs and persistent storage
//! - Round-trip serialization between TOON text and records

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ToonValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
    Array(Vec<ToonValue>),
    Object(HashMap<String, ToonValue>),
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<ToonValue>>,
    },
}

impl ToonValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ToonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn to_string_lossy(&self) -> String {
        match self {
            ToonValue::String(s) => s.clone(),
            ToonValue::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{:.0}", n)
                } else {
                    format!("{}", n)
                }
            }
            ToonValue::Boolean(b) => b.to_string(),
            ToonValue::Null => "".to_string(),
            ToonValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string_lossy()).collect();
                items.join(",")
            }
            ToonValue::Object(_) => "[Object]".to_string(),
            ToonValue::Table { .. } => "[Table]".to_string(),
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, ToonValue>> {
        match self {
            ToonValue::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_table(&self) -> Option<(&Vec<String>, &Vec<Vec<ToonValue>>)> {
        match self {
            ToonValue::Table { headers, rows } => Some((headers, rows)),
            _ => None,
        }
    }
}

/// Serializer for TOON v4 format
pub struct ToonSerializer;

impl ToonSerializer {
    /// Serialize a table of uniform objects into standard TOON v4 tabular syntax
    /// Format: `<name>[<len>]{<h1,h2,...>}:`
    /// Followed by comma-separated rows.
    pub fn serialize_table(name: &str, headers: &[String], rows: &[Vec<String>]) -> String {
        let mut out = String::new();
        out.push_str(&format!("{}[{}]{{{}}}:\n", name, rows.len(), headers.join(",")));
        for row in rows {
            out.push_str("  ");
            let escaped_row: Vec<String> = row.iter().map(|cell| Self::escape_cell(cell)).collect();
            out.push_str(&escaped_row.join(","));
            out.push('\n');
        }
        out
    }

    /// Serialize key-value metadata in YAML/TOON indentation format
    pub fn serialize_meta(pairs: &[(&str, &str)]) -> String {
        let mut out = String::from("meta:\n");
        for (k, v) in pairs {
            out.push_str(&format!("  {}: {}\n", k, Self::escape_value(v)));
        }
        out
    }

    fn escape_cell(val: &str) -> String {
        if val.contains(',') || val.contains('"') || val.contains('\n') {
            format!("\"{}\"", val.replace('"', "\"\""))
        } else {
            val.to_string()
        }
    }

    fn escape_value(val: &str) -> String {
        if val.contains('\n') || val.starts_with(' ') || val.ends_with(' ') {
            format!("\"{}\"", val.replace('"', "\\\""))
        } else {
            val.to_string()
        }
    }

    /// Calculate estimated token count (simple LLM token metric, approx 4 chars per token)
    pub fn estimate_tokens(text: &str) -> usize {
        (text.len() + 3) / 4
    }
}

/// Parser for TOON v4 format
pub struct ToonParser;

impl ToonParser {
    /// Parse a TOON v4 document containing metadata and tabular data
    pub fn parse(content: &str) -> Result<HashMap<String, ToonValue>, String> {
        let mut root: HashMap<String, ToonValue> = HashMap::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim_end();
            if line.is_empty() || line.starts_with('#') {
                i += 1;
                continue;
            }

            // Check for tabular format: name[count]{col1,col2,...}:
            if let Some(colon_pos) = line.rfind(':') {
                let header_part = line[..colon_pos].trim();
                if let Some(bracket_open) = header_part.find('[') {
                    if let Some(_bracket_close) = header_part.find(']') {
                        if let Some(brace_open) = header_part.find('{') {
                            if let Some(brace_close) = header_part.find('}') {
                                let table_name = header_part[..bracket_open].trim().to_string();
                                let cols_str = &header_part[brace_open + 1..brace_close];
                                let headers: Vec<String> = cols_str
                                    .split(',')
                                    .map(|s| s.trim().to_string())
                                    .collect();

                                i += 1;
                                let mut rows: Vec<Vec<ToonValue>> = Vec::new();

                                while i < lines.len() {
                                    let row_line = lines[i];
                                    if row_line.trim().is_empty() {
                                        i += 1;
                                        continue;
                                    }
                                    // Row lines must be indented
                                    if !row_line.starts_with("  ") && !row_line.starts_with('\t') {
                                        break;
                                    }

                                    let parsed_row = Self::parse_csv_row(row_line.trim());
                                    let row_values = parsed_row
                                        .into_iter()
                                        .map(ToonValue::String)
                                        .collect();
                                    rows.push(row_values);
                                    i += 1;
                                }

                                root.insert(
                                    table_name,
                                    ToonValue::Table { headers, rows },
                                );
                                continue;
                            }
                        }
                    }
                }
            }

            // Check for object/metadata map: name:
            if line.ends_with(':') && !line.starts_with(' ') {
                let section_name = line[..line.len() - 1].trim().to_string();
                i += 1;
                let mut map = HashMap::new();

                while i < lines.len() {
                    let sub_line = lines[i];
                    if sub_line.trim().is_empty() {
                        i += 1;
                        continue;
                    }
                    if !sub_line.starts_with("  ") && !sub_line.starts_with('\t') {
                        break;
                    }

                    let trimmed = sub_line.trim();
                    if let Some(kv_colon) = trimmed.find(':') {
                        let k = trimmed[..kv_colon].trim().to_string();
                        let v = trimmed[kv_colon + 1..].trim();
                        let unquoted = Self::unquote(v);
                        map.insert(k, ToonValue::String(unquoted));
                    }
                    i += 1;
                }
                root.insert(section_name, ToonValue::Object(map));
                continue;
            }

            i += 1;
        }

        Ok(root)
    }

    fn parse_csv_row(line: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut cur = String::new();
        let mut in_quotes = false;
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '"' {
                if in_quotes && chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            } else if c == ',' && !in_quotes {
                result.push(cur.trim().to_string());
                cur = String::new();
            } else {
                cur.push(c);
            }
        }
        result.push(cur.trim().to_string());
        result
    }

    fn unquote(s: &str) -> String {
        let trimmed = s.trim();
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            trimmed[1..trimmed.len() - 1].replace("\"\"", "\"")
        } else {
            trimmed.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toon_table_roundtrip() {
        let headers = vec!["id".into(), "name".into(), "role".into()];
        let rows = vec![
            vec!["1".into(), "Alice Smith".into(), "Admin".into()],
            vec!["2".into(), "Bob, Developer".into(), "Member".into()],
        ];

        let serialized = ToonSerializer::serialize_table("users", &headers, &rows);
        assert!(serialized.contains("users[2]{id,name,role}:"));

        let parsed = ToonParser::parse(&serialized).unwrap();
        if let Some(ToonValue::Table { headers: ph, rows: pr }) = parsed.get("users") {
            assert_eq!(ph.len(), 3);
            assert_eq!(pr.len(), 2);
            assert_eq!(pr[1][1].as_str().unwrap(), "Bob, Developer");
        } else {
            panic!("Expected table 'users'");
        }
    }
}
