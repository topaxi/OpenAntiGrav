//! The typed tree of values a typetree walk produces.

use std::sync::Arc;

/// A reference to another object: a file (0 = this one, `n` = externals\[n-1\])
/// and a path id within it. **Both** are needed: path ids collide across the
/// files of one bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PPtr {
    /// 0 for the same file, else a 1-based index into the externals table.
    pub file_id: i32,
    /// The object's path id within that file. 0 with file 0 is null.
    pub path_id: i64,
}

impl PPtr {
    /// A pointer to nothing.
    #[must_use]
    pub fn is_null(&self) -> bool {
        self.path_id == 0
    }
}

/// A named struct: its typetree type name and its fields in file order.
#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    /// The node's type name, such as `Vector3f` or `Mesh`.
    pub type_name: Arc<str>,
    /// Fields in serialization order.
    pub fields: Vec<(Arc<str>, Value)>,
}

/// One decoded value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `bool`.
    Bool(bool),
    /// Any signed integer.
    Int(i64),
    /// Any unsigned integer.
    UInt(u64),
    /// `float`.
    Float(f32),
    /// `double`.
    Double(f64),
    /// `string`.
    String(String),
    /// A byte run: `TypelessData`, or an array of 8-bit integers. Arrays of
    /// `SInt8` land here too, so the sign is the reader's to apply.
    Bytes(Vec<u8>),
    /// An array of any other element.
    Array(Vec<Value>),
    /// A struct.
    Struct(Struct),
    /// A `PPtr<...>`.
    PPtr(PPtr),
}

impl Value {
    /// A struct field by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Value> {
        match self {
            Self::Struct(s) => s.fields.iter().find(|(n, _)| &**n == name).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Any integer as `i64` (an out-of-range unsigned is `None`).
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Self::Int(v) => Some(v),
            Self::UInt(v) => i64::try_from(v).ok(),
            Self::Bool(b) => Some(i64::from(b)),
            _ => None,
        }
    }

    /// Any non-negative integer as `u64`.
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match *self {
            Self::UInt(v) => Some(v),
            Self::Int(v) => u64::try_from(v).ok(),
            Self::Bool(b) => Some(u64::from(b)),
            _ => None,
        }
    }

    /// A float of either width as `f32`.
    #[must_use]
    pub fn as_f32(&self) -> Option<f32> {
        match *self {
            Self::Float(v) => Some(v),
            Self::Double(v) => Some(v as f32),
            _ => None,
        }
    }

    /// A boolean (or 0/1 integer).
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Self::Bool(b) => Some(b),
            Self::Int(v) => Some(v != 0),
            Self::UInt(v) => Some(v != 0),
            _ => None,
        }
    }

    /// A string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// A byte run.
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(b) => Some(b),
            _ => None,
        }
    }

    /// An array (an empty byte run counts as an empty array).
    #[must_use]
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(a) => Some(a),
            Self::Bytes(b) if b.is_empty() => Some(&[]),
            _ => None,
        }
    }

    /// A `PPtr`.
    #[must_use]
    pub fn as_pptr(&self) -> Option<PPtr> {
        match *self {
            Self::PPtr(p) => Some(p),
            _ => None,
        }
    }

    /// A struct of three floats named `x`, `y`, `z` (`Vector3f`).
    #[must_use]
    pub fn as_vec3(&self) -> Option<[f32; 3]> {
        Some([
            self.get("x")?.as_f32()?,
            self.get("y")?.as_f32()?,
            self.get("z")?.as_f32()?,
        ])
    }

    /// A struct of four floats named `x`, `y`, `z`, `w` (`Quaternionf`).
    #[must_use]
    pub fn as_vec4(&self) -> Option<[f32; 4]> {
        Some([
            self.get("x")?.as_f32()?,
            self.get("y")?.as_f32()?,
            self.get("z")?.as_f32()?,
            self.get("w")?.as_f32()?,
        ])
    }
}
