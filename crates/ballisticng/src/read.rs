//! Typed access to a `MonoBehaviour`'s generic tree, with a named error for
//! every absent or mistyped field.

use crate::error::{Error, Result};
use oag_unity::value::{PPtr, Value};

pub(crate) fn get<'v>(v: &'v Value, c: &'static str, n: &'static str) -> Result<&'v Value> {
    v.get(n).ok_or(Error::Field(c, n))
}

pub(crate) fn int(v: &Value, c: &'static str, n: &'static str) -> Result<i64> {
    get(v, c, n)?.as_i64().ok_or(Error::Field(c, n))
}

pub(crate) fn index(v: &Value, c: &'static str, n: &'static str) -> Result<usize> {
    usize::try_from(int(v, c, n)?).map_err(|_| Error::Field(c, n))
}

pub(crate) fn float(v: &Value, c: &'static str, n: &'static str) -> Result<f32> {
    get(v, c, n)?.as_f32().ok_or(Error::Field(c, n))
}

pub(crate) fn flag(v: &Value, c: &'static str, n: &'static str) -> Result<bool> {
    get(v, c, n)?.as_bool().ok_or(Error::Field(c, n))
}

pub(crate) fn vec3(v: &Value, c: &'static str, n: &'static str) -> Result<[f32; 3]> {
    get(v, c, n)?.as_vec3().ok_or(Error::Field(c, n))
}

pub(crate) fn pptr(v: &Value, c: &'static str, n: &'static str) -> Result<PPtr> {
    get(v, c, n)?.as_pptr().ok_or(Error::Field(c, n))
}

pub(crate) fn array<'v>(v: &'v Value, c: &'static str, n: &'static str) -> Result<&'v [Value]> {
    get(v, c, n)?.as_array().ok_or(Error::Field(c, n))
}

pub(crate) fn vec3_array(v: &Value, c: &'static str, n: &'static str) -> Result<Vec<[f32; 3]>> {
    array(v, c, n)?
        .iter()
        .map(|e| e.as_vec3().ok_or(Error::Field(c, n)))
        .collect()
}

pub(crate) fn text(v: &Value, c: &'static str, n: &'static str) -> Result<String> {
    get(v, c, n)?
        .as_str()
        .map(str::to_string)
        .ok_or(Error::Field(c, n))
}
