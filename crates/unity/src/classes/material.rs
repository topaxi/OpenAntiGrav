use super::{field, pptr, text};
use crate::error::{Error, Result};
use crate::value::{PPtr, Value};

/// One texture slot of a material.
#[derive(Debug, Clone)]
pub struct TexEnv {
    /// The shader property name, such as `_MainTex`.
    pub name: String,
    /// The texture (null when the slot is empty).
    pub texture: PPtr,
    /// UV scale.
    pub scale: [f32; 2],
    /// UV offset.
    pub offset: [f32; 2],
}

/// A `Material`: its shader, texture slots by name, and plain properties.
#[derive(Debug, Clone)]
pub struct Material {
    /// Its name.
    pub name: String,
    /// The shader asset.
    pub shader: PPtr,
    /// Texture slots in file order.
    pub textures: Vec<TexEnv>,
    /// Float properties.
    pub floats: Vec<(String, f32)>,
    /// Colour properties as `(r, g, b, a)`.
    pub colors: Vec<(String, [f32; 4])>,
}

impl Material {
    /// The texture bound to a slot, if the slot exists and is not empty.
    #[must_use]
    pub fn texture(&self, slot: &str) -> Option<PPtr> {
        self.textures
            .iter()
            .find(|t| t.name == slot && !t.texture.is_null())
            .map(|t| t.texture)
    }

    /// Reads a `Material` value.
    pub fn from_value(v: &Value) -> Result<Self> {
        const C: &str = "Material";
        let bad = |field| Error::Field { class: C, field };
        let props = field(v, C, "m_SavedProperties")?;
        let list = |name: &'static str| field(props, C, name)?.as_array().ok_or_else(|| bad(name));
        // The key is a plain string in the content seen, a `FastPropertyName`
        // struct holding one in older files.
        let key = |p: &Value| -> Result<String> {
            let first = p.get("first").ok_or_else(|| bad("pair.first"))?;
            first
                .as_str()
                .or_else(|| first.get("name").and_then(Value::as_str))
                .map(str::to_string)
                .ok_or_else(|| bad("pair.first"))
        };
        fn second(p: &Value) -> Result<&Value> {
            p.get("second").ok_or(Error::Field {
                class: "Material",
                field: "pair.second",
            })
        }
        let vec2 = |x: &Value| -> Result<[f32; 2]> {
            Ok([
                x.get("x")
                    .and_then(Value::as_f32)
                    .ok_or_else(|| bad("Vector2f"))?,
                x.get("y")
                    .and_then(Value::as_f32)
                    .ok_or_else(|| bad("Vector2f"))?,
            ])
        };
        let mut textures = Vec::new();
        for p in list("m_TexEnvs")? {
            let env = second(p)?;
            textures.push(TexEnv {
                name: key(p)?,
                texture: pptr(env, C, "m_Texture")?,
                scale: vec2(field(env, C, "m_Scale")?)?,
                offset: vec2(field(env, C, "m_Offset")?)?,
            });
        }
        let mut floats = Vec::new();
        for p in list("m_Floats")? {
            floats.push((key(p)?, second(p)?.as_f32().ok_or_else(|| bad("m_Floats"))?));
        }
        let mut colors = Vec::new();
        for p in list("m_Colors")? {
            let c = second(p)?;
            let get = |n: &str| {
                c.get(n)
                    .and_then(Value::as_f32)
                    .ok_or_else(|| bad("m_Colors"))
            };
            colors.push((key(p)?, [get("r")?, get("g")?, get("b")?, get("a")?]));
        }
        Ok(Self {
            name: text(v, C, "m_Name")?,
            shader: pptr(v, C, "m_Shader")?,
            textures,
            floats,
            colors,
        })
    }
}
