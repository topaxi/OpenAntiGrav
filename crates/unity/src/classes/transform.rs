use super::{field, pptr};
use crate::error::{Error, Result};
use crate::value::{PPtr, Value};

/// A `Transform`: a local pose and its place in the hierarchy.
#[derive(Debug, Clone)]
pub struct Transform {
    /// The `GameObject` it belongs to.
    pub game_object: PPtr,
    /// Local rotation as `(x, y, z, w)`.
    pub rotation: [f32; 4],
    /// Local position.
    pub position: [f32; 3],
    /// Local scale.
    pub scale: [f32; 3],
    /// Child transforms.
    pub children: Vec<PPtr>,
    /// Parent transform (null at a root).
    pub father: PPtr,
}

impl Transform {
    /// Reads a `Transform` (or `RectTransform`) value.
    pub fn from_value(v: &Value) -> Result<Self> {
        const C: &str = "Transform";
        let bad = |field| Error::Field { class: C, field };
        let children = field(v, C, "m_Children")?
            .as_array()
            .ok_or_else(|| bad("m_Children"))?
            .iter()
            .map(|c| c.as_pptr().ok_or_else(|| bad("m_Children[]")))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            game_object: pptr(v, C, "m_GameObject")?,
            rotation: field(v, C, "m_LocalRotation")?
                .as_vec4()
                .ok_or_else(|| bad("m_LocalRotation"))?,
            position: field(v, C, "m_LocalPosition")?
                .as_vec3()
                .ok_or_else(|| bad("m_LocalPosition"))?,
            scale: field(v, C, "m_LocalScale")?
                .as_vec3()
                .ok_or_else(|| bad("m_LocalScale"))?,
            children,
            father: pptr(v, C, "m_Father")?,
        })
    }
}
