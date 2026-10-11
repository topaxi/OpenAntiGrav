use super::{field, int, text};
use crate::error::{Error, Result};
use crate::value::{PPtr, Value};

/// A `GameObject`: a name, a layer, and the components attached to it.
#[derive(Debug, Clone)]
pub struct GameObject {
    /// Its name.
    pub name: String,
    /// Its layer.
    pub layer: u32,
    /// Its tag.
    pub tag: u16,
    /// Whether it is active.
    pub active: bool,
    /// Pointers to its components, in order.
    pub components: Vec<PPtr>,
}

impl GameObject {
    /// Reads a `GameObject` value.
    pub fn from_value(v: &Value) -> Result<Self> {
        const C: &str = "GameObject";
        let list = field(v, C, "m_Component")?.as_array().ok_or(Error::Field {
            class: C,
            field: "m_Component",
        })?;
        let mut components = Vec::with_capacity(list.len());
        for entry in list {
            // 5.5 and later: `ComponentPair { component }`; older: `pair { first, second }`.
            let p = entry
                .get("component")
                .or_else(|| entry.get("second"))
                .and_then(Value::as_pptr)
                .ok_or(Error::Field {
                    class: C,
                    field: "m_Component[].component",
                })?;
            components.push(p);
        }
        Ok(Self {
            name: text(v, C, "m_Name")?,
            layer: u32::try_from(int(v, C, "m_Layer")?).map_err(|_| Error::OutOfRange("layer"))?,
            tag: u16::try_from(int(v, C, "m_Tag")?).map_err(|_| Error::OutOfRange("tag"))?,
            active: field(v, C, "m_IsActive")?.as_bool().ok_or(Error::Field {
                class: C,
                field: "m_IsActive",
            })?,
            components,
        })
    }
}
