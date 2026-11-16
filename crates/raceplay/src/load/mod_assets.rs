//! What the race's mods add to the effect library, the sheet and the mixer: their shaders and
//! effects land after every title effect, so no title sprite moves.

use oag_assets::Archives;
use oag_fx::psys;

use super::Options;
use crate::extension_effects::ModEffects;
use crate::extension_shaders::ModShaders;
use crate::extensions::Source;

/// The mods a race runs, and the title and options their assets are looked up under.
pub(super) struct ModAssets<'a> {
    sources: &'a [Source],
    title: String,
    options: &'a Options,
}

impl<'a> ModAssets<'a> {
    pub(super) fn new(
        sources: &'a [Source],
        title: &oag_title::Title,
        options: &'a Options,
    ) -> Self {
        Self {
            sources,
            title: title.slug(),
            options,
        }
    }

    /// The mods' `[shaders]`, then their `[effects]` (placed in `effects`, a shader named by
    /// one resolved against the first). A file that is missing or refused is named in `report`
    /// and draws nothing.
    pub(super) fn shaders_and_effects(
        &self,
        archives: &mut Archives,
        effects: &mut psys::Library,
        report: &mut Vec<String>,
    ) -> (ModShaders, ModEffects) {
        let foreign = self.options.foreign_track.as_deref();
        let shaders = crate::extension_shaders::load(self.sources, &self.title, foreign, report);
        let own = crate::extension_effects::load(
            self.sources,
            &self.title,
            foreign,
            &shaders,
            effects,
            &mut |entry, name| {
                crate::foreign::art::host_texture(archives, entry, name)
                    .and_then(|texture| crate::extension_effects::sprite_of(&texture))
            },
            report,
        );
        (shaders, own)
    }

    /// The mods' `[sounds]`, into the race's banks.
    pub(super) fn sounds(&self, banks: &mut oag_sound::sfx::Banks, report: &mut Vec<String>) {
        let foreign = self.options.foreign_track.as_deref();
        banks.mod_sounds =
            crate::extension_sounds::load(self.sources, &self.title, foreign, report);
    }
}
