//! The race box's selection screens, over the menu stage: the picker model,
//! the disc's layout for it, and the preview mesh for whatever is selected.
//!
//! Held on [`crate::menu_stage::MenuStage::picker`] rather than as a stage of
//! its own, the same way a modal prompt is: while one is open the menu behind
//! it is a picture, and closing it lands back on the page it opened from with
//! nothing rebuilt. See `oag_ui_screens::picker` for the model and
//! `crate::session::picker` for the flow that opens and closes one.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use log::{debug, warn};
use oag_game::preview::Preview;
use oag_hud::sprite::Sheet;
use oag_mesh::mesh_render::Anisotropy;
use oag_mesh::orbit::Orbit;
use oag_ui::frontend::{Draw, Placed};
use oag_ui_screens::picker::slideshow::Slideshow;
use oag_ui_screens::picker::{self, Picker};

use crate::gpu::Gpu;

/// Where an entry's preview mesh is on the disc.
#[derive(Debug, Clone)]
pub(crate) enum PreviewSource {
    /// The circuit's outline preview: `<location>\FE\forward.vex` /
    /// `\reverse.vex`, which is what `Track Creation`'s info panel shows.
    /// **Pulse only** - Pure's `Track Selection` draws a pre-rendered
    /// sprite, not a mesh, and has no entry here (see
    /// [`Self::entry_name`]). In the same folder, `screen.xml` names the
    /// stills that fill Pulse's hexagonal window. `zone` picks the Zone
    /// chain of stills, the way the original does for a Zone run. See
    /// `docs/formats/race-setup.md`.
    Track {
        location: String,
        reversed: bool,
        zone: bool,
    },
    /// The team's front-end hull, and the skins it declares as `(name,
    /// archive entry)` - the paint the livery row cycles. The hull is
    /// `<location>\ship_FE.vex`. **Pulse only**, for the same reason the
    /// track preview is - see [`Self::entry_name`].
    Ship {
        location: String,
        skins: Vec<(String, String)>,
        /// How a variant's suffix joins the team's directory, where the team
        /// has a variant table - the preview follows the selected variant.
        join: Option<oag_title::VariantJoin>,
    },
}

impl PreviewSource {
    /// The archive entry this preview's mesh is in.
    ///
    /// **Pulse's names, and only Pulse's.** Pure's `Track Selection` and
    /// `Team Selection` do not render a mesh at all: every preview on both
    /// screens is a pre-rendered 256x128 sprite shipped in `FEData.wad`, and
    /// a PPSSPP texture dump matches the pixels the screens draw to those
    /// entries at RMSE 0 - see `docs/formats/race-setup.md`. Pure therefore
    /// has no name to fall through to here, and its pickers draw no preview
    /// until the sprite path exists; [`PickerStage::load_preview`] says so
    /// rather than substituting a mesh, per this project's rule for an asset
    /// that will not resolve.
    ///
    /// This used to return two candidates, Pulse's then Pure's
    /// (`<location>\Ship.vex`, `<location>\track.vex`), on the reading that
    /// Pure drew a real per-entity 3D preview. That reading was wrong, and
    /// the files it named were the in-race hull and the full 4 MB racing
    /// circuit - a stand-in that looked legible, which is exactly how a
    /// wrong picture survives review.
    ///
    /// `None` for a circuit its title draws no model for - see
    /// [`oag_game::preview::track_entry`].
    fn entry_name(
        &self,
        ship_hull: Option<&str>,
        front_end: Option<&oag_title::FrontEnd>,
    ) -> Option<String> {
        match self {
            Self::Track {
                location, reversed, ..
            } => oag_game::preview::track_entry(front_end?, location, *reversed),
            Self::Ship { location, .. } => Some(format!(
                r"{location}\{}",
                ship_hull.unwrap_or("ship_FE.vex")
            )),
        }
    }
}

/// What a title's front end says about how its selection screens preview an
/// entry - the two facts [`PickerStage`] cannot read off the entry itself.
#[derive(Debug, Clone, Default)]
pub(crate) struct Previews {
    /// Whether this title previews with a mesh at all - see
    /// [`oag_title::FrontEnd::preview_meshes`]. `false` on Pure, whose two
    /// screens preview with stills alone; no mesh is loaded and none is
    /// logged as missing, because none is.
    pub(crate) meshes: bool,
    /// The craft file a title with no `ship_FE.vex` draws instead - see
    /// [`oag_title::FrontEnd::ship_preview_hull`]. Ship screens only: a
    /// circuit still needs [`Self::meshes`].
    pub(crate) ship_hull: Option<&'static str>,
    /// The title's front end, for the per-circuit model table - see
    /// [`oag_title::FrontEnd::circuit_models`].
    pub(crate) front_end: Option<&'static oag_title::FrontEnd>,
    /// The front end's own `FEGlobals` table. A per-entity `screen.xml`
    /// declares no globals and still names them for its stills' colour, so
    /// without this Pure's stills tint white - invisible on its white front
    /// end. See [`oag_ui_screens::picker::slideshow::Slideshow::read`].
    pub(crate) globals: Vec<(String, String)>,
    /// The title's own string table, for the panel's `idstring` labels - see
    /// [`oag_ui_screens::picker::slideshow::Slideshow::read`].
    pub(crate) strings: oag_ui::language::StringTable,
}

/// Which axis the ship picker's livery row moves - see
/// `session::picker::open_ship_picker`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LiveryAxis {
    /// The team's own `PI_ModelSkin`s: `race.skin`, the original's row.
    Skin,
    /// The title's variant table: `race.variant`, the RACE page's row.
    Variant,
}

/// Lap lengths measured off the disc on a worker, by track id, filling in as
/// each circuit is read. See [`PickerStage::refresh_info`].
pub(crate) type Distances = Arc<Mutex<HashMap<String, f32>>>;

/// One open selection screen.
pub(crate) struct PickerStage {
    pub(crate) model: Picker,
    pub(crate) layout: picker::Layout,
    pub(crate) livery_axis: LiveryAxis,
    /// Parallel to the model's entries: where each one's preview comes from.
    sources: Vec<PreviewSource>,
    /// The archives the previews are read out of, opened once when the
    /// screen does and kept for its life - a selection change is one archive
    /// read, not a disc reopen.
    archives: oag_assets::Archives,
    /// The selected entry's preview, or `None` when its mesh did not load -
    /// which draws nothing there and says so in the log, per this project's
    /// rule for an asset that will not play.
    pub(crate) preview: Option<Preview>,
    /// Which entry and livery `preview` was built for, so a tick that
    /// changed nothing rebuilds nothing.
    built_for: Option<(usize, Option<String>)>,
    /// The worker's measurements, on a track picker; `None` on a ship one.
    distances: Option<Distances>,
    anisotropy: Anisotropy,
    /// What this title's front end says about previews - see [`Previews`].
    previews: Previews,
    /// The selected entry's slideshow, when its own `screen.xml` authors
    /// one; see [`oag_game::preview::slideshow`]. A circuit's is the picture
    /// behind Pulse's hexagonal window and the whole of Pure's track
    /// preview; a craft's is Pure's craft preview and empty on every other
    /// title. `None` when the file is missing or names no chain, which draws
    /// nothing and says so in the log.
    slideshow: Option<Slideshow>,
    /// The front end's own sheet, as the menus were built with it.
    base: Sheet,
    /// [`Self::base`] extended with the selected circuit's stills - what the
    /// renderer has to be drawing from for [`Self::slideshow_draws`] to show
    /// anything - and whether the renderer has been handed it yet.
    sheet: Option<Sheet>,
    sheet_uploaded: bool,
    /// The race's lap count, for HD's `RACE DISTANCE` row - the length
    /// times this. Set by the caller after [`Self::new`]; `None` when the
    /// race has no lap target, which is the row's `Infinity` glyph.
    pub(crate) laps: Option<u32>,
}

impl PickerStage {
    #[expect(
        clippy::too_many_arguments,
        reason = "one call site, and each is a separate fact of the screen: the \
                  model, its layout, its axis, its sources, the disc, the worker, \
                  the filter, the sheet and the title's own preview rules"
    )]
    pub(crate) fn new(
        model: Picker,
        layout: picker::Layout,
        livery_axis: LiveryAxis,
        sources: Vec<PreviewSource>,
        archives: oag_assets::Archives,
        distances: Option<Distances>,
        anisotropy: Anisotropy,
        base: Sheet,
        previews: Previews,
    ) -> Self {
        Self {
            model,
            layout,
            livery_axis,
            sources,
            archives,
            preview: None,
            built_for: None,
            distances,
            anisotropy,
            previews,
            slideshow: None,
            base,
            sheet: None,
            sheet_uploaded: false,
            laps: None,
        }
    }

    /// The sheet the renderer should draw this screen from, the first time
    /// it changes: the front end's own plus the selected circuit's stills.
    /// `None` once handed over, and on a screen with no stills of its own.
    pub(crate) fn take_sheet(&mut self) -> Option<&Sheet> {
        if self.sheet_uploaded {
            return None;
        }
        self.sheet_uploaded = true;
        self.sheet.as_ref()
    }

    /// Where an image sits in whichever sheet this screen draws from - its
    /// own extended one, or the front end's when it has none.
    pub(crate) fn placed(&self, src: &str) -> Option<Placed> {
        self.sheet.as_ref().unwrap_or(&self.base).get(src)
    }

    /// The stills on screen this tick, timed from the last selection change
    /// (see [`oag_ui_screens::picker::slideshow::Slideshow::at`]), faded in over
    /// [`oag_game::preview::CARD_FADE_SECONDS`] - see that constant's own doc
    /// for why this reads it rather than an attribute of its own.
    pub(crate) fn slideshow_draws(&self) -> Vec<Draw> {
        self.slideshow.as_ref().map_or_else(Vec::new, |show| {
            let seconds = self.model.since_selection();
            let alpha = (seconds / oag_game::preview::CARD_FADE_SECONDS).clamp(0.0, 1.0);
            show.draws(seconds, &|src| self.placed(src))
                .into_iter()
                .map(|draw| oag_game::preview::fade_draw(draw, alpha))
                .collect()
        })
    }

    /// The selected circuit's own `<Mode3D><Model>` pose, when its
    /// `screen.xml` authors one - see
    /// [`oag_game::preview::mode3d_view_projection`]. `None` on `Team
    /// Selection` (Pulse's craft `screen.xml` authors no `Mode3D`) and on any
    /// circuit missing one, so a caller falls back to [`oag_game::preview::orbit_for`].
    pub(crate) fn mode3d_model(&self) -> Option<&oag_ui::screen::Model> {
        self.slideshow.as_ref()?.model.as_ref()
    }

    /// Loads the selected entry's preview, in the selected livery, if it is
    /// not the one already built.
    pub(crate) fn refresh_preview(&mut self, gpu: &Gpu) {
        let index = self.model.index();
        let livery = match self.livery_axis {
            LiveryAxis::Skin => self.model.variant().map(|(id, _)| id.clone()),
            LiveryAxis::Variant => self.model.variant().map(|(id, _)| id.clone()),
        };
        let key = (index, livery);
        if self.built_for.as_ref() == Some(&key) {
            return;
        }
        let entry_changed = self
            .built_for
            .as_ref()
            .is_none_or(|(built, _)| *built != index);
        self.built_for = Some(key.clone());
        if entry_changed {
            self.load_slideshow(index);
        }
        // A title that previews with stills alone has no mesh to fail to
        // load, so there is nothing here to report as missing.
        let hull_only = self.model.kind() == picker::Kind::Ship && self.previews.ship_hull.is_some();
        let circuit_model =
            oag_game::preview::draws_circuit_model(self.previews.front_end, self.model.kind());
        self.preview = if self.previews.meshes || hull_only || circuit_model {
            match self.load_preview(gpu, index, key.1.as_deref(), self.livery_axis) {
                Ok(preview) => Some(preview),
                Err(error) => {
                    warn!("{error:#} - the preview draws nothing");
                    None
                }
            }
        } else {
            None
        };
    }

    /// Reads the selected entry's stills and puts them on a sheet of this
    /// screen's own, for the renderer to pick up on its next frame.
    ///
    /// **A craft entry as much as a circuit one**: Pure's craft preview is
    /// its team's own `screen.xml` chain, and Pulse's and HD's craft files
    /// author no stills, so asking costs an already-open archive read and
    /// returns an empty list there.
    fn load_slideshow(&mut self, index: usize) {
        // HD's track screen authors no hexagonal window: its `Emblem` and
        // `FlyByMovie` are the widgets that show a circuit.
        if self.layout.hd_track.is_some() {
            return;
        }
        let (location, zone) = match self.sources.get(index).cloned() {
            Some(PreviewSource::Track { location, zone, .. }) => (location, zone),
            Some(PreviewSource::Ship { location, .. }) => (location, false),
            None => return,
        };
        let mut report = Vec::new();
        let globals: Vec<(&str, &str)> = self
            .previews
            .globals
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        let loaded = oag_game::preview::slideshow(
            &mut self.archives,
            &location,
            zone,
            &globals,
            &self.previews.strings,
            &mut report,
        );
        for line in report {
            warn!("{line}");
        }
        match loaded {
            Ok((show, blobs)) => {
                let mut report = Vec::new();
                let sheet = self.base.extended(&blobs, &mut report);
                for line in report {
                    debug!("slideshow {line}");
                }
                debug!(
                    "slideshow {location}: {} state(s), {} still(s) on a {}x{} sheet",
                    show.states().len(),
                    blobs.len(),
                    sheet.width,
                    sheet.height
                );
                self.slideshow = Some(show);
                self.sheet = Some(sheet);
                self.sheet_uploaded = false;
            }
            Err(error) => {
                warn!("{error:#} - the hexagonal window draws nothing");
                self.slideshow = None;
            }
        }
    }

    fn load_preview(
        &mut self,
        gpu: &Gpu,
        index: usize,
        livery: Option<&str>,
        axis: LiveryAxis,
    ) -> Result<Preview> {
        let source = self
            .sources
            .get(index)
            .with_context(|| format!("entry {index} has no preview source"))?
            .clone();
        let name = source
            .entry_name(self.previews.ship_hull, self.previews.front_end)
            .with_context(|| format!("entry {index} has no circuit model on the disc"))?;
        let circuit_model =
            oag_game::preview::draws_circuit_model(self.previews.front_end, self.model.kind());
        // The variant row picks the directory the hull is read from; the
        // skin row picks a paint over it.
        let (skin, variant) = match axis {
            LiveryAxis::Skin => (livery, None),
            LiveryAxis::Variant => (None, livery.filter(|id| !id.is_empty())),
        };
        let variant_name = match (&source, variant) {
            (PreviewSource::Ship { location, join, .. }, Some(variant))
                if self.previews.ship_hull.is_some() =>
            {
                let located = oag_game::preview::variant::variant_location(*join, location, variant);
                PreviewSource::Ship {
                    location: located,
                    skins: Vec::new(),
                    join: None,
                }
                .entry_name(self.previews.ship_hull, self.previews.front_end)
            }
            _ => None,
        };
        let mut model = if circuit_model {
            oag_game::preview::psp2_scene::circuit_model(&mut self.archives, &name)
                .with_context(|| format!("{name} did not resolve as a preview mesh"))?
        } else {
            let entry = variant_name.as_deref().unwrap_or(&name);
            oag_game::preview::variant::model_or_default(&mut self.archives, entry, Some(&name))
                .with_context(|| format!("{entry} did not resolve as a preview mesh"))?
                .0
        };
        // The chosen paint over the hull's own texture slots - the same
        // swap a race makes (`oag_livery::ship_skin`), and the same
        // rule when it fails: the hull keeps its own paint, and the log
        // says so.
        if let (PreviewSource::Ship { skins, .. }, Some(skin)) =
            (&source, skin.filter(|s| !s.is_empty()))
            && let Some((_, entry)) = skins.iter().find(|(id, _)| id == skin)
        {
            for line in oag_game::preview::paint(&mut self.archives, entry, &name, &mut model) {
                debug!("preview {name}: {line}");
            }
        }
        if self.previews.ship_hull.is_some() && matches!(source, PreviewSource::Ship { .. }) {
            oag_game::preview::frame_hull(&mut model);
        }
        // The circuit model's own material: the ramp comes out of the model
        // and the preview recolours its vertices every frame.
        let ramp = if circuit_model {
            Some(
                oag_game::preview::track_model::Ramp::of(&mut self.archives, &name, &mut model)
                    .with_context(|| format!("{name}: reading its material"))?,
            )
        } else {
            None
        };
        debug!(
            "preview {name}: {} vertices, {} triangles",
            model.vertices.len(),
            model.indices.len() / 3
        );
        let preview = Preview::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            self.anisotropy,
            model,
        )
        .with_context(|| format!("building the preview for {name}"))?;
        Ok(match ramp {
            Some(ramp) => preview.with_ramp(ramp),
            None => preview,
        })
    }

    /// Copies whatever the distance worker has measured so far onto the
    /// panel's `Distance(m)` row, entry by entry. Cheap: one lock, one
    /// pass over the ids; called once a tick.
    pub(crate) fn refresh_info(&mut self) {
        let Some(distances) = &self.distances else {
            return;
        };
        let Ok(measured) = distances.lock() else {
            return;
        };
        let known: Vec<(usize, f32)> = self
            .model
            .entries()
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| measured.get(&entry.id).map(|length| (index, *length)))
            .collect();
        drop(measured);
        for (index, length) in known {
            if self.layout.hd_track.is_some() {
                let [track, race] = picker::hd::track::length_rows(length, self.laps);
                self.model.set_track_info(index, 0, track);
                self.model.set_track_info(index, 1, race);
            } else {
                self.model.set_track_info(index, 0, format!("{length:.0}"));
            }
        }
    }

    /// Draws the preview mesh over the finished frame: the circuit model on
    /// HD's own camera, otherwise the circuit's `<Mode3D>` camera or the orbit.
    pub(crate) fn draw_preview(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
        space: oag_display::space::Space,
    ) {
        let (orbit, seconds, rect) = (self.orbit(), self.model.seconds(), self.layout.preview);
        let hull_swap = self.previews.ship_hull.is_some()
            && self.model.kind() == picker::Kind::Ship
            && !self.hull_phase().visible;
        if hull_swap {
            return;
        }
        // `Track Creation`'s own `<Mode3D>` camera, when authored.
        let mode3d_model = self.mode3d_model().cloned();
        let track_model = self.layout.hd_track.as_ref().and_then(|screen| screen.model);
        let ship_model = self
            .layout
            .hd
            .as_ref()
            .and_then(|screen| screen.ship_model)
            .filter(|_| self.previews.ship_hull.is_some() && self.model.kind() == picker::Kind::Ship);
        let swap_scale = self.hull_phase().scale;
        let Some(preview) = self.preview.as_mut() else {
            return;
        };
        if let Some(widget) = track_model {
            preview.draw_track_model(
                &gpu.device,
                &gpu.queue,
                encoder,
                view,
                viewport,
                target_size,
                space,
                &widget,
                seconds,
            );
        } else if let Some(widget) = ship_model {
            let (view_projection, model) =
                oag_game::preview::ship_model::matrices(&widget, space, swap_scale);
            preview.draw_matrices(
                &gpu.device,
                &gpu.queue,
                encoder,
                view,
                viewport,
                target_size,
                space,
                view_projection,
                model,
                seconds,
            );
        } else {
            preview.draw_auto(
                &gpu.device,
                &gpu.queue,
                encoder,
                view,
                viewport,
                target_size,
                space,
                mode3d_model.as_ref(),
                rect,
                orbit,
                seconds,
            );
        }
    }

    /// The craft's swap state this tick on a title that draws the race hull -
    /// see [`oag_game::preview::hull_swap`].
    fn hull_phase(&self) -> oag_game::preview::hull_swap::Phase {
        oag_game::preview::hull_swap::phase(self.model.seconds(), self.model.since_selection())
    }

    /// How the preview is framed this tick - see [`oag_game::preview::orbit_for`].
    pub(crate) fn orbit(&self) -> Orbit {
        if self.previews.ship_hull.is_some() && self.model.kind() == picker::Kind::Ship {
            let mut orbit = oag_game::preview::hull_orbit();
            orbit.zoom /= self.hull_phase().scale;
            return orbit;
        }
        oag_game::preview::orbit_for(self.model.kind(), self.model.seconds())
    }
}

#[cfg(test)]
mod tests {
    use super::PreviewSource;

    /// Pulse's name, and no fallthrough to Pure's - Pure's `Team Selection`
    /// draws a `FEData.wad` sprite, not this hull. See [`PreviewSource::entry_name`].
    #[test]
    fn ship_names_pulses_front_end_hull() {
        let source = PreviewSource::Ship {
            location: r"Data\Ships\Feisar".to_string(),
            skins: Vec::new(),
            join: None,
        };
        assert_eq!(source.entry_name(None, None).as_deref(),
            Some(r"Data\Ships\Feisar\ship_FE.vex"));
    }

    /// HD names its race hull, not a `ship_FE.vex` that no HD archive carries.
    #[test]
    fn ship_names_the_titles_hull_file() {
        let source = PreviewSource::Ship {
            location: r"Data\Ships\Feisar_c1".to_string(),
            skins: Vec::new(),
            join: None,
        };
        assert_eq!(
            source.entry_name(Some("ship.vex"), None).as_deref(),
            Some(r"Data\Ships\Feisar_c1\ship.vex")
        );
    }

    /// `zone` steers the still chain, not the mesh name: a Zone run previews
    /// the same forward ribbon the ordinary run does.
    #[test]
    fn track_name_follows_reversed_only() {
        let forward = PreviewSource::Track {
            location: r"Data\Environments\01_Vineta_K".to_string(),
            reversed: false,
            zone: false,
        };
        assert_eq!(
            forward.entry_name(None, Some(oag_pulse::FRONT_END)).as_deref(),
            Some(r"Data\Environments\01_Vineta_K\FE\forward.vex")
        );
        let reversed = PreviewSource::Track {
            location: r"Data\Environments\01_Vineta_K".to_string(),
            reversed: true,
            zone: false,
        };
        assert_eq!(
            reversed.entry_name(None, Some(oag_pulse::FRONT_END)).as_deref(),
            Some(r"Data\Environments\01_Vineta_K\FE\reverse.vex")
        );
        let zone = PreviewSource::Track {
            location: r"Data\Environments\01_Vineta_K".to_string(),
            reversed: false,
            zone: true,
        };
        assert_eq!(
            zone.entry_name(None, Some(oag_pulse::FRONT_END)).as_deref(),
            Some(r"Data\Environments\01_Vineta_K\FE\forward.vex")
        );
    }

    /// HD names each circuit's own scene, and a circuit with no row has no
    /// model at all.
    #[test]
    fn hd_track_names_its_circuit_scene() {
        let source = |location: &str| PreviewSource::Track {
            location: location.to_string(),
            reversed: true,
            zone: false,
        };
        assert_eq!(
            source(r"Data\Environments\10_Sebenco_Climb")
                .entry_name(None, Some(oag_hd::frontend::FRONT_END))
                .as_deref(),
            Some(r"Data\Environments\10_Sebenco_Climb\FE\track06.vex")
        );
        assert_eq!(
            source(r"Data\Environments\99_Nowhere")
                .entry_name(None, Some(oag_hd::frontend::FRONT_END)),
            None
        );
    }
}
