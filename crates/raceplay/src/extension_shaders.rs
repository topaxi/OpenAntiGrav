//! A mod's own shaders: read on the load thread with the race, drawn by
//! [`oag_fx::mod_shader::Stage`] on what a module or an effect file names. A file that fails to
//! read or validate draws nothing; the loader report quotes why and the race runs. Outputs:
//! never hashed, never in the simulation.

use oag_core::math::Vec3;
use oag_fx::mod_shader::{self, Fill, MAX_PARAMS, Shader};
use oag_mesh::mesh::GpuVertex;

use crate::Race;
use crate::extensions::Source;

/// Where the flat shader sits: what a beam naming no shader draws.
const FLAT: usize = 0;

const _: () = assert!(MAX_PARAMS == oag_ext_abi::MAX_SHADER_PARAMS as usize);

/// Every shader of the race's mods at its place in the race's [`mod_shader::Stage`], and each
/// mod's `[shaders]`, in name order, as those places. A place whose file did not load is `None`.
#[derive(Debug, Clone, Default)]
pub struct ModShaders {
    places: Vec<(String, Vec<usize>)>,
    list: Vec<Option<Shader>>,
}

impl ModShaders {
    /// The stage's list; empty in a race with no mod, which then builds no stage.
    #[must_use]
    pub fn list(&self) -> &[Option<Shader>] {
        &self.list
    }

    /// The place of shader `index` of mod `mod_id`.
    #[must_use]
    pub fn place(&self, mod_id: &str, index: u16) -> Option<usize> {
        let (_, places) = self.places.iter().find(|(id, _)| id == mod_id)?;
        places.get(usize::from(index)).copied()
    }

    /// The place an effect file's `shader = "<name>"` of `source` means.
    #[cfg(feature = "extensions")]
    pub(crate) fn place_by_name(&self, source: &Source, name: &str) -> Result<u16, String> {
        let index = source
            .shaders
            .iter()
            .position(|(have, _)| have == name)
            .ok_or_else(|| "the mod's [shaders] has no such name".to_string())?;
        let place = self
            .place(&source.mod_id, index as u16)
            .ok_or_else(|| "not loaded".to_string())?;
        u16::try_from(place).map_err(|e| e.to_string())
    }
}

/// Reads and compiles the `[shaders]` of every mod that is on for this race. A file that is
/// missing or refused keeps its place, is named in `report` with the reason, and draws nothing.
#[must_use]
pub fn load(
    sources: &[Source],
    title: &str,
    foreign_track: Option<&std::path::Path>,
    report: &mut Vec<String>,
) -> ModShaders {
    #[cfg(not(feature = "extensions"))]
    {
        let _ = (sources, title, foreign_track, report);
        ModShaders::default()
    }
    #[cfg(feature = "extensions")]
    {
        let mut out = ModShaders::default();
        let mut seen = std::collections::BTreeSet::new();
        for source in crate::extensions::for_race(sources, title, foreign_track) {
            if out.list.is_empty() {
                out.list.push(Some(Shader {
                    compiled: mod_shader::flat(),
                    texture: None,
                }));
            }
            if !seen.insert(source.mod_id.clone()) {
                continue;
            }
            let mut places = Vec::new();
            for (name, path) in &source.shaders {
                let label = format!("mod {}: shader {name}", source.mod_id);
                let read = read(source, path);
                match &read {
                    Ok(_) => report.push(format!("{label}: {path} loaded")),
                    Err(why) => {
                        let line = format!("{label} left out: {why}; draws nothing");
                        log::warn!("{line}");
                        report.push(line);
                    }
                }
                places.push(out.list.len());
                out.list.push(read.ok());
            }
            out.places.push((source.mod_id.clone(), places));
        }
        out
    }
}

#[cfg(feature = "extensions")]
fn read(source: &Source, path: &str) -> Result<Shader, String> {
    let bytes = (source.files)(path)?;
    let text = String::from_utf8(bytes).map_err(|e| format!("{path}: {e}"))?;
    let compiled = mod_shader::compile(path, &text)?;
    let texture = match &compiled.texture {
        None => None,
        Some(png) => {
            let (width, height, rgba) = crate::extensions::decode_png(&(source.files)(png)?)
                .map_err(|why| format!("{png}: {why}"))?;
            Some((width, height, rgba))
        }
    };
    Ok(Shader { compiled, texture })
}

impl Race {
    /// The race's shaders, for [`crate::Scene::attach_mod_shaders`].
    #[must_use]
    pub fn mod_shaders(&self) -> &[Option<Shader>] {
        self.view.handles.shaders().list()
    }

    /// The view being drawn's triangles per shader: the particles of the effects a shader
    /// draws, then the beams. `right`, `up` and `eye` are that view's camera.
    pub(crate) fn mod_surfaces(
        &self,
        right: Vec3,
        up: Vec3,
        eye: Vec3,
    ) -> Vec<(usize, Vec<GpuVertex>)> {
        let mut surfaces = Vec::new();
        let guard = self.stage_guard_band();
        self.view
            .stage
            .extend_shaded(&mut surfaces, right, up, guard.as_ref());
        for (mod_id, beam) in self.extension_beams() {
            let place = match beam.shader {
                None => Some(FLAT),
                Some(shader) => self.view.handles.shaders().place(mod_id, shader),
            };
            let in_world = |slot: usize| {
                usize::from(self.ship_count()) > slot && self.sim.world.ships[slot].in_world()
            };
            let (Some(place), true) = (place, in_world(beam.from) && in_world(beam.to)) else {
                continue;
            };
            let end = |slot| self.ship_model_matrix_of(slot).w_axis.truncate();
            let at = match surfaces.iter().position(|(at, _)| *at == place) {
                Some(at) => at,
                None => {
                    surfaces.push((place, Vec::new()));
                    surfaces.len() - 1
                }
            };
            mod_shader::ribbon(
                end(beam.from),
                end(beam.to),
                beam.half_width,
                eye,
                beam.rgba,
                &mut surfaces[at].1,
            );
        }
        surfaces
    }

    /// The view being drawn's fill through a shader. One that names a shader the mod lacks
    /// is no fill.
    pub(crate) fn mod_fill(&self) -> Option<Fill> {
        let (mod_id, shader, colour) = self.extension_shaded_fill(self.viewed_slot())?;
        Some(Fill {
            shader: self.view.handles.shaders().place(mod_id, shader)?,
            colour,
        })
    }

    /// The scalars the modules sent for the view being drawn, by shader place.
    pub(crate) fn mod_shader_params(&self) -> Vec<[Option<f32>; MAX_PARAMS]> {
        let shaders = self.view.handles.shaders();
        let mut params = vec![[None; MAX_PARAMS]; shaders.list().len()];
        for (mod_id, shader, index, value) in self.extension_shader_params(self.viewed_slot()) {
            if let Some(place) = shaders.place(mod_id, shader) {
                params[place][usize::from(index)] = Some(value);
            }
        }
        params
    }
}
