//! The built-in classes a scene is made of, read from a [`crate::value::Value`].
//!
//! Each `from_value` names the field it could not find rather than guessing. A
//! `MonoBehaviour` is deliberately not interpreted: [`MonoBehaviour`] hands back
//! its script pointer and the generic tree, and a content layer reads the rest.

mod gameobject;
mod material;
mod mesh;
mod texture;
mod transform;

pub use gameobject::GameObject;
pub use material::{Material, TexEnv};
pub use mesh::{Channel, IndexFormat, Mesh, SubMesh, VertexFormat};
pub use texture::{StreamRef, Texture2d, TextureFormat};
pub use transform::Transform;

use crate::error::{Error, Result};
use crate::value::{PPtr, Value};

/// A `MonoBehaviour`: the script it runs and its fields, uninterpreted.
#[derive(Debug, Clone)]
pub struct MonoBehaviour {
    /// The `GameObject` it is attached to.
    pub game_object: PPtr,
    /// The `MonoScript` it instantiates.
    pub script: PPtr,
    /// Its name (usually empty).
    pub name: String,
    /// The whole serialized value, for a content layer to read.
    pub value: Value,
}

impl MonoBehaviour {
    /// Reads a `MonoBehaviour` value.
    pub fn from_value(v: &Value) -> Result<Self> {
        Ok(Self {
            game_object: pptr(v, "MonoBehaviour", "m_GameObject")?,
            script: pptr(v, "MonoBehaviour", "m_Script")?,
            name: v
                .get("m_Name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            value: v.clone(),
        })
    }
}

fn field<'v>(v: &'v Value, class: &'static str, name: &'static str) -> Result<&'v Value> {
    v.get(name).ok_or(Error::Field { class, field: name })
}

fn pptr(v: &Value, class: &'static str, name: &'static str) -> Result<PPtr> {
    field(v, class, name)?
        .as_pptr()
        .ok_or(Error::Field { class, field: name })
}

fn int(v: &Value, class: &'static str, name: &'static str) -> Result<i64> {
    field(v, class, name)?
        .as_i64()
        .ok_or(Error::Field { class, field: name })
}

fn uint(v: &Value, class: &'static str, name: &'static str) -> Result<u64> {
    field(v, class, name)?
        .as_u64()
        .ok_or(Error::Field { class, field: name })
}

fn text(v: &Value, class: &'static str, name: &'static str) -> Result<String> {
    field(v, class, name)?
        .as_str()
        .map(str::to_string)
        .ok_or(Error::Field { class, field: name })
}
