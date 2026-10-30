//! A mod's own particle effects: what a module's `fire-effect` can name beside the
//! title's. The mod's `[effects]` files are parametric TOML
//! (`oag_fx::psys::parametric`), read on the load thread with the race, added to the
//! race's effect library after every title effect, and played at the craft's nozzle.
//!
//! A file that fails to read, parse or validate draws nothing; the loader report names
//! it and the race runs. Effects are render-side outputs: never hashed, never in the
//! simulation.

/// What a module's `fire-effect` asked for: a title's trigger, or the sending mod's own
/// effect by its place in the mod's `[effects]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fired {
    /// The title's own effect for a trigger.
    Title(oag_title::effects::Trigger),
    /// The mod's own.
    Own {
        /// The sending mod.
        mod_id: String,
        /// Its place in the mod's `[effects]`, in name order.
        index: u16,
    },
}

#[cfg(feature = "view")]
pub use view::*;

#[cfg(feature = "view")]
mod view {
    use std::sync::Arc;

    use oag_core::math::Vec3;
    #[cfg(feature = "extensions")]
    use oag_fx::psys::parametric;
    #[cfg(feature = "extensions")]
    use oag_fx::psys::sprite::Sprite;
    use oag_fx::psys::{Effect, Library};

    use oag_fx::psys::parametric::Axis;

    use super::Fired;
    use crate::extensions::Source;

    /// One loaded effect of a mod and how to aim it.
    #[derive(Debug, Clone)]
    pub struct OwnEffect {
        /// The playable effect, its sprites placed on the race's sheet.
        pub effect: Arc<Effect>,
        /// Where its cone axis points on a craft.
        pub axis: Axis,
    }

    /// Every mod's loaded effects, by mod id, each in the mod's `[effects]` order; a slot
    /// is `None` where the file did not load.
    #[derive(Debug, Clone, Default)]
    pub struct ModEffects(pub Vec<(String, Vec<Option<OwnEffect>>)>);

    impl ModEffects {
        /// The effect `index` of mod `mod_id`, if it loaded.
        #[must_use]
        pub fn get(&self, mod_id: &str, index: u16) -> Option<&OwnEffect> {
            self.0
                .iter()
                .find(|(id, _)| id == mod_id)?
                .1
                .get(usize::from(index))?
                .as_ref()
        }
    }

    /// The key an effect has in the race's [`Library`].
    #[cfg(feature = "extensions")]
    fn key(mod_id: &str, name: &str) -> String {
        format!("mod:{mod_id}/{name}")
    }

    /// Reads, parses and places the `[effects]` of every mod that is on for this race.
    /// Each lands in `library` after everything the title loaded, so no title sprite's
    /// place on the sheet moves. A file that is missing or refused is named in `report`
    /// and draws nothing.
    #[must_use]
    pub fn load(
        sources: &[Source],
        title: &str,
        foreign_track: Option<&std::path::Path>,
        library: &mut Library,
        report: &mut Vec<String>,
    ) -> ModEffects {
        #[cfg(not(feature = "extensions"))]
        {
            let _ = (sources, title, foreign_track, library, report);
            ModEffects::default()
        }
        #[cfg(feature = "extensions")]
        {
            let mut seen = std::collections::BTreeSet::new();
            let mut out = ModEffects::default();
            for source in crate::extensions::for_race(sources, title, foreign_track) {
                if source.effects.is_empty() || !seen.insert(source.mod_id.clone()) {
                    continue;
                }
                let mut loaded = Vec::new();
                for (name, path) in &source.effects {
                    let label = format!("mod {}: effect {name}", source.mod_id);
                    match read(source, path, library, &key(&source.mod_id, name)) {
                        Ok(own) => {
                            report.push(format!("{label}: {path} loaded"));
                            report.extend(crate::assets::sheet_note(
                                &key(&source.mod_id, name),
                                library,
                            ));
                            loaded.push(Some(own));
                        }
                        Err(why) => {
                            let line = format!("{label} left out: {why}; draws nothing");
                            log::info!("{line}");
                            report.push(line);
                            loaded.push(None);
                        }
                    }
                }
                out.0.push((source.mod_id.clone(), loaded));
            }
            out
        }
    }

    #[cfg(feature = "extensions")]
    fn read(
        source: &Source,
        path: &str,
        library: &mut Library,
        key: &str,
    ) -> Result<OwnEffect, String> {
        let bytes = (source.files)(path)?;
        let text = String::from_utf8(bytes).map_err(|e| format!("{path}: {e}"))?;
        let mut texture = |png: &str| -> Option<Sprite> {
            let bytes = (source.files)(png).ok()?;
            let (width, height, rgba) = crate::extensions::decode_png(&bytes).ok()?;
            Some(Sprite {
                width: u16::try_from(width).ok()?,
                height: u16::try_from(height).ok()?,
                rgba: rgba.into(),
            })
        };
        let parsed = parametric::parse(&text, &mut texture).map_err(|e| format!("{path}: {e}"))?;
        library.insert(key, parsed.effect);
        let effect = library
            .get(key)
            .cloned()
            .ok_or_else(|| "the library kept no effect".to_string())?;
        Ok(OwnEffect {
            effect,
            axis: parsed.axis,
        })
    }

    impl crate::Race {
        /// Plays one `fire-effect` at its craft's nozzle (the hull's back end): the
        /// title's effect for a trigger, or the mod's own; one that is absent draws
        /// nothing.
        pub(crate) fn play_fired(&mut self, slot: usize, fired: &Fired) {
            let (effect, axis) = match fired {
                Fired::Title(trigger) => (self.view.handles.get(*trigger).cloned(), Axis::Up),
                Fired::Own { mod_id, index } => {
                    let own = self.view.handles.own(mod_id, *index);
                    (
                        own.map(|own| own.effect.clone()),
                        own.map_or(Axis::Up, |own| own.axis),
                    )
                }
            };
            let Some(effect) = effect else {
                return;
            };
            let ship = &self.sim.world.ships[slot];
            if matches!(fired, Fired::Title(_)) {
                let model = crate::drawable::model_matrix_of(ship);
                let tail = Vec3::new(0.0, 0.0, ship.handling.dimensions.length * 0.5);
                self.view
                    .stage
                    .play(&effect, model.transform_point3(tail), 1.0);
                return;
            }
            // The mod's own: the nozzle is the hull's back end along the body's
            // heading, and a rear effect's cone axis points away from the nose.
            let body = &ship.physics.body;
            let back = -body.forward();
            let at = body.position + back * (ship.handling.dimensions.length * 0.5);
            if let Some(playing) = self.view.stage.play(&effect, at, 1.0) {
                let up = if axis == Axis::Rear { back } else { body.up() };
                self.view.stage.orient(playing, up);
            }
        }
    }
}
