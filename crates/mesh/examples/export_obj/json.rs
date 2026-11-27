//! The little JSON `nodes.json` needs: a value is its own serialised text.

use std::fmt;

#[derive(Debug, Clone)]
pub struct Value(String);

impl Value {
    pub fn null() -> Self {
        Self("null".into())
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<usize> for Value {
    fn from(n: usize) -> Self {
        Self(n.to_string())
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self(b.to_string())
    }
}

pub fn text(s: &str) -> Value {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    Value(out)
}

pub fn array(items: impl IntoIterator<Item = Value>) -> Value {
    let items: Vec<String> = items.into_iter().map(|v| v.0).collect();
    Value(format!("[{}]", items.join(",")))
}

pub fn numbers(values: &[f32]) -> Value {
    array(values.iter().map(|v| {
        if v.is_finite() {
            Value(v.to_string())
        } else {
            Value::null()
        }
    }))
}

pub fn object_of(fields: Vec<(&str, Value)>) -> Value {
    let fields: Vec<String> = fields
        .into_iter()
        .map(|(k, v)| format!("{}:{}", text(k).0, v.0))
        .collect();
    Value(format!("{{{}}}\n", fields.join(",")))
}

pub fn object<const N: usize>(fields: [(&str, Value); N]) -> Value {
    object_of(fields.into())
}
