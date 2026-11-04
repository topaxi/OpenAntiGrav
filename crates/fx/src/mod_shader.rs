//! A mod's own fragment shader: a WGSL file of the mod, drawn on the surfaces a mod can name
//! (an effect's particles, a beam between two crafts, a screen fill). The contract a shader is
//! written against, and what is refused and why, is
//! `docs/architecture/wasm-extensions/mod-shaders.md`.
//!
//! A shader is one file whose leading `//!` lines are a TOML header
//! (`oag_gpu::shader_file`, the shape of a screen filter) and whose body defines
//!
//! ```wgsl
//! fn mod_fragment(input: ModInput) -> vec4<f32>
//! ```
//!
//! against the host text in `shaders/mod_shader.wesl`. [`compile`] needs no device, so it
//! runs on the load thread; a file it refuses draws nothing and the load report quotes why.
//! [`Stage`] is the GPU half. Everything here is an output: never hashed, never read back.

mod check;
mod pipeline;

pub use pipeline::{Drawer, Drawn, Fill, Frame, Shader, Stage};

use serde::Deserialize;

/// How many scalars a shader has: one `vec4<f32>` of the uniform.
pub const MAX_PARAMS: usize = 4;

/// The longest file: a shader is a screenful of text, and the driver compiles it mid-load.
const MAX_SOURCE_BYTES: usize = 64 * 1024;

/// The host's side of every shader, as linked and validated at build.
const HOST: &str = include_str!(concat!(env!("OUT_DIR"), "/mod_shader.wgsl"));

/// What the host calls the fragment the mod's file replaces.
const MOD_FRAGMENT: &str = "fn mod_fragment(";

/// How a shader's result lands on the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Blend {
    /// Over the picture by its alpha.
    #[default]
    Alpha,
    /// Added to the picture, weighted by its alpha.
    Add,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    #[serde(default)]
    blend: Blend,
    /// A PNG of the mod, read by `mod_texture`.
    texture: Option<String>,
    #[serde(default)]
    param: Vec<Param>,
}

/// One scalar a header declares.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Param {
    name: String,
    #[serde(default)]
    default: f32,
}

/// A file [`compile`] accepted.
#[derive(Debug, Clone, PartialEq)]
pub struct Compiled {
    /// The whole module, the mod's text first.
    pub wgsl: String,
    /// Its header's `blend`.
    pub blend: Blend,
    /// Its header's `texture`, a path in the mod.
    pub texture: Option<String>,
    /// Its scalars' defaults, unset ones zero.
    pub defaults: [f32; MAX_PARAMS],
}

/// The module with `body` where the host's own fragment was. The host's stays as a function
/// nothing calls, under a name of its own, and the mod's text comes first so that a line in
/// naga's report is a line of the mod's file.
fn module(body: &str, accessors: &str) -> String {
    let host = HOST.replacen(MOD_FRAGMENT, "fn oag_flat_fragment(", 1);
    let mut out = String::with_capacity(body.len() + host.len() + accessors.len() + 2);
    out.push_str(body);
    out.push('\n');
    out.push_str(accessors);
    out.push_str(&host);
    out
}

/// What a draw with no shader of its own runs: the colour it was given.
#[must_use]
pub fn flat() -> Compiled {
    Compiled {
        wgsl: HOST.to_string(),
        blend: Blend::Alpha,
        texture: None,
        defaults: [0.0; MAX_PARAMS],
    }
}

/// Reads the file at `path` (a name for the report, the bytes are `source`) into a module the
/// device can be handed.
///
/// # Errors
///
/// Why it is refused, naga's annotated report for a module that does not build: never a panic.
pub fn compile(path: &str, source: &str) -> Result<Compiled, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "{path}: {} bytes, the most is {MAX_SOURCE_BYTES}",
            source.len()
        ));
    }
    let (header, _) = oag_gpu::shader_file::split_header(source);
    let header: Header = toml::from_str(&header).map_err(|e| format!("{path}: the header: {e}"))?;
    if header.param.len() > MAX_PARAMS {
        return Err(format!(
            "{path}: {} params, the uniform holds {MAX_PARAMS}",
            header.param.len()
        ));
    }
    let mut defaults = [0.0; MAX_PARAMS];
    for (index, param) in header.param.iter().enumerate() {
        if !oag_gpu::shader_file::is_identifier(&param.name) {
            return Err(format!(
                "{path}: param {:?} is not a WGSL identifier",
                param.name
            ));
        }
        defaults[index] = param.default;
    }
    if !oag_gpu::shader_file::defines(source, "mod_fragment") {
        return Err(format!(
            "{path}: defines no `fn mod_fragment(input: ModInput) -> vec4<f32>`"
        ));
    }
    let accessors = oag_gpu::shader_file::param_accessors(
        "oag_frame.params",
        header.param.iter().map(|param| param.name.as_str()),
    );
    let wgsl = module(source, &accessors);
    let parsed = oag_gpu::shader_file::validate(&wgsl, path)?;
    check::refuse(&parsed).map_err(|why| format!("{path}: {why}"))?;
    Ok(Compiled {
        wgsl,
        blend: header.blend,
        texture: header.texture,
        defaults,
    })
}

#[cfg(test)]
mod tests;
