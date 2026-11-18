//! [`CircuitLighting`]: how much of a circuit's authored lighting its plain surfaces compute.

use serde::{Deserialize, Serialize};

/// How much of the authored rig a circuit's plain surfaces compute, on a title whose circuits
/// are shaded from their own materials (Wipeout HD, 2048, Omega).
///
/// `Original` is the default and the original's own shading. `Simplified` is a departure for
/// speed: it drops the specular highlight, the surface's normal map and Wipeout HD's per-vertex
/// SPU lights, and keeps the albedo, the baked light (lightmap or colour set), the ambient, the
/// sun on the surface's own normal, its glows, fog and the craft shadows
/// (`docs/rendering/hires-gpu-levers.md`, "The circuit lighting rung").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum CircuitLighting {
    /// The original's shading.
    #[default]
    Original,
    /// No specular, normal map or SPU vertex lights on a circuit's plain surfaces.
    Simplified,
}

impl CircuitLighting {
    /// The value's name in a settings file, on the command line and in the menu.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Simplified => "simplified",
        }
    }

    /// Every value, in menu order.
    pub const ALL: [Self; 2] = [Self::Original, Self::Simplified];

    /// Whether the plain surfaces drop the terms [`Self::Simplified`] names.
    #[must_use]
    pub fn is_simplified(self) -> bool {
        self == Self::Simplified
    }
}

impl std::str::FromStr for CircuitLighting {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|value| value.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!("{text:?} is not a circuit lighting; try original or simplified")
            })
    }
}

impl std::fmt::Display for CircuitLighting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for CircuitLighting {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<CircuitLighting> for String {
    fn from(value: CircuitLighting) -> Self {
        value.to_string()
    }
}
