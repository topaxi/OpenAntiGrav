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
#[derive(Debug, Clone, PartialEq, Eq)]
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
        /// The craft file this entry previews instead of the title's own, where the race
        /// flies another hull: a Zone race's (`oag_game::preview::zone_hull_file`).
        hull_file: Option<String>,
        /// The whole entry, for a Zone hull kept outside the team's directory
        /// (`oag_game::preview::zone_race_hull_entry`), which beats the two above.
        zone_entry: Option<String>,
        /// The race hull this front-end hull's flaps are seated from
        /// (`oag_game::preview::race_hull_entry`); `None` where the title draws the race hull.
        race_hull: Option<String>,
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
            Self::Ship {
                zone_entry: Some(entry),
                ..
            } => Some(entry.clone()),
            Self::Ship {
                location,
                hull_file,
                ..
            } => Some(format!(
                r"{location}\{}",
                hull_file
                    .as_deref()
                    .or(ship_hull)
                    .unwrap_or("ship_FE.vex")
            )),
        }
    }
}

type Key = (usize, Option<String>);

/// What decides a selection's read: two reads with equal facts on one title and language read
/// the same bytes, which is what lets a screen adopt a read started before it opened.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReadFacts {
    key: Key,
    kind: picker::Kind,
    axis: LiveryAxis,
    source: Option<PreviewSource>,
    slideshow: bool,
    mesh: bool,
}

impl ReadFacts {
    /// The read of `key` on a screen of `model` and `layout`.
    fn of(
        key: Key,
        model: &Picker,
        layout: &picker::Layout,
        axis: LiveryAxis,
        sources: &[PreviewSource],
        previews: &Previews,
        entry_changed: bool,
    ) -> Self {
        let kind = model.kind();
        let hull_only = kind == picker::Kind::Ship && previews.ship_hull.is_some();
        let circuit_model = oag_game::preview::draws_circuit_model(previews.front_end, kind);
        let source = sources.get(key.0).cloned();
        Self {
            key,
            kind,
            axis,
            source,
            slideshow: entry_changed && layout.hd_track.is_none(),
            mesh: previews.meshes || hull_only || circuit_model,
        }
    }
}

/// What one selection's read needs, owned.
struct ReadJob {
    facts: ReadFacts,
    previews: Previews,
    archives: SharedArchives,
    /// The sheet to extend with the entry's stills, when [`ReadFacts::slideshow`] asks for them.
    base: Option<Sheet>,
}

enum ShowRead {
    /// The entry did not change, or the title draws no stills.
    Unchanged,
    Loaded(Box<Slideshow>, Box<Sheet>),
    Failed,
}

/// What a worker read for one selection.
struct Read {
    key: Key,
    show: ShowRead,
    preview: Option<Result<PreviewData>>,
}

impl ReadJob {
    /// The disc half of a selection: its stills (on a sheet) and its mesh, parsed.
    fn run(self) -> Read {
        let ReadJob { facts, previews, archives, base } = self;
        let ReadFacts { key, kind, axis, source, mesh, .. } = facts;
        let Some(source) = source else {
            warn!("entry {} has no preview source", key.0);
            return Read { key, show: ShowRead::Unchanged, preview: None };
        };
        let Ok(mut archives) = oag_thread::lock(&archives) else {
            warn!("the menu archives' lock is poisoned - the preview draws nothing");
            return Read { key, show: ShowRead::Failed, preview: None };
        };
        let show = match base {
            Some(base) => read_slideshow(&mut archives, &previews, &source, &base),
            None => ShowRead::Unchanged,
        };
        let preview = mesh.then(|| {
            read_preview(&mut archives, &previews, kind, &source, key.1.as_deref(), axis)
                .with_context(|| format!("entry {}", key.0))
        });
        Read { key, show, preview }
    }
}

/// Reads the entry's stills and puts them on a sheet of the screen's own - see
/// [`PickerStage::refresh_preview`].
///
/// **A craft entry as much as a circuit one**: Pure's craft preview is its team's own
/// `screen.xml` chain, and Pulse's and HD's craft files author no stills, so asking costs
/// an already-open archive read and returns an empty list there.
fn read_slideshow(
    archives: &mut oag_assets::Archives,
    previews: &Previews,
    source: &PreviewSource,
    base: &Sheet,
) -> ShowRead {
    let (location, zone) = match source {
        PreviewSource::Track { location, zone, .. } => (location.clone(), *zone),
        PreviewSource::Ship { location, .. } => (location.clone(), false),
    };
    let mut report = Vec::new();
    let globals: Vec<(&str, &str)> = previews
        .globals
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let loaded = oag_game::preview::slideshow(
        archives,
        &location,
        zone,
        &globals,
        &previews.strings,
        &mut report,
    );
    for line in report {
        warn!("{line}");
    }
    match loaded {
        Ok((show, blobs)) => {
            let mut report = Vec::new();
            let sheet = base.extended(&blobs, &mut report);
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
            ShowRead::Loaded(Box::new(show), Box::new(sheet))
        }
        Err(error) => {
            warn!("{error:#} - the hexagonal window draws nothing");
            ShowRead::Failed
        }
    }
}

/// A selection's read, on a worker where there is one.
enum Pending {
    #[cfg(not(target_arch = "wasm32"))]
    Task(oag_thread::Task<Read>),
    #[cfg(target_arch = "wasm32")]
    Done(Box<Read>),
}

impl Pending {
    fn start(job: ReadJob) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::Task(oag_thread::Task::spawn("preview-read", move || job.run()))
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self::Done(Box::new(job.run()))
        }
    }

    fn finished(&self) -> bool {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Task(task) => task.is_finished(),
            #[cfg(target_arch = "wasm32")]
            Self::Done(_) => true,
        }
    }

    fn join(self) -> Option<Read> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Task(mut task) => task.join().and_then(Result::ok),
            #[cfg(target_arch = "wasm32")]
            Self::Done(read) => Some(*read),
        }
    }
}

/// A screen's opening read, started on a worker before the screen opens, so that the frame that
/// opens the screen has nothing to wait for - see `session::opening_reads`. The screen adopts it
/// when it asks for the same read ([`ReadFacts`]) and reads for itself otherwise.
pub(crate) struct OpeningRead {
    facts: ReadFacts,
    archives: SharedArchives,
    pending: Pending,
}

impl OpeningRead {
    /// Starts the read of what a screen of `model` and `layout` opens on.
    pub(crate) fn start(
        model: &Picker,
        layout: &picker::Layout,
        axis: LiveryAxis,
        sources: &[PreviewSource],
        previews: &Previews,
        archives: &SharedArchives,
        base: &Sheet,
    ) -> Self {
        let key = (model.index(), model.variant().map(|(id, _)| id.clone()));
        let facts = ReadFacts::of(key, model, layout, axis, sources, previews, true);
        let job = ReadJob {
            base: facts.slideshow.then(|| base.clone()),
            facts: facts.clone(),
            previews: previews.clone(),
            archives: Arc::clone(archives),
        };
        Self { facts, archives: Arc::clone(archives), pending: Pending::start(job) }
    }

    /// Whether the worker has finished.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn landed(&self) -> bool {
        self.pending.finished()
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

/// One opened source the menus' screens share - see `session::menu_source`.
pub(crate) type SharedArchives = Arc<Mutex<oag_assets::Archives>>;

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
    /// The archives the previews are read out of: the menus' own, opened on a worker while
    /// they were up (`session::menu_source`), so neither this screen's opening nor a
    /// selection change opens the disc.
    archives: SharedArchives,
    /// The selected entry's preview, or `None` when its mesh did not load -
    /// which draws nothing there and says so in the log, per this project's
    /// rule for an asset that will not play.
    pub(crate) preview: Option<Preview>,
    /// Which entry and livery `preview` was built for, so a tick that
    /// changed nothing rebuilds nothing.
    built_for: Option<(usize, Option<String>)>,
    /// The latest entry and livery asked for; `built_for` catches up when the read lands.
    requested: Option<(usize, Option<String>)>,
    /// The worker reading `requested` (or the request before it) off the disc.
    pending: Option<Pending>,
    /// A read started before this screen opened, for its first request to adopt.
    opening: Option<OpeningRead>,
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
    /// The race's lap count on each circuit, in entry order, for HD's `RACE
    /// DISTANCE` row - the length times this. Set by the caller after
    /// [`Self::new`]; `None` when the race has no lap target, which is the
    /// row's `Infinity` glyph.
    pub(crate) laps: Vec<Option<u32>>,
    /// `(seat, seated)` when this is one seat's Ship Select on a split-screen
    /// race box, which reads that seat's device alone - see
    /// `crate::session::picker`'s `seat` module. `None` is every ordinary screen.
    pub(crate) seat: Option<(usize, usize)>,
    /// The title's depth fit as it plays across frames, where it has one: ticked
    /// every frame with whatever model is selected, so a step carries the last
    /// model's depth into the next - see [`oag_game::preview::ship_model::FitPlayer`].
    fit_motion: Option<oag_game::preview::ship_model::FitPlayer>,
}

impl PickerStage {
    #[expect(
        clippy::too_many_arguments,
        reason = "one call site, and each is a separate fact of the screen: the \
                  model, its layout, its axis, its sources, the disc, the worker, \
                  the filter, the sheet, the title's own preview rules and the read \
                  already under way"
    )]
    pub(crate) fn new(
        model: Picker,
        layout: picker::Layout,
        livery_axis: LiveryAxis,
        sources: Vec<PreviewSource>,
        archives: SharedArchives,
        distances: Option<Distances>,
        anisotropy: Anisotropy,
        base: Sheet,
        previews: Previews,
        opening: Option<OpeningRead>,
    ) -> Self {
        Self {
            model,
            layout,
            livery_axis,
            sources,
            archives,
            preview: None,
            built_for: None,
            requested: None,
            pending: None,
            opening,
            distances,
            anisotropy,
            previews,
            slideshow: None,
            base,
            sheet: None,
            sheet_uploaded: false,
            laps: Vec::new(),
            seat: None,
            fit_motion: None,
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
            show.draws_fading(
                seconds,
                oag_ui_screens::picker::slideshow::STEP_FADE_SECONDS,
                &|src| self.placed(src),
            )
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

    /// Asks for the selected entry's stills and preview mesh, in the selected livery, if they
    /// are not the ones already built or being read. The disc is read and parsed on a worker
    /// ([`Self::poll_preview`] lands the result), and the previous entry's picture stays up
    /// until then.
    pub(crate) fn refresh_preview(&mut self, gpu: &Gpu) {
        let index = self.model.index();
        let livery = self.model.variant().map(|(id, _)| id.clone());
        let key = (index, livery);
        if self.requested.as_ref() == Some(&key) {
            return;
        }
        self.requested = Some(key);
        self.poll_preview(gpu);
        // The screen's first picture has no earlier one to stay up: a screen that opens on
        // nothing where the original shows its stills is a hole, so the opening frame takes the
        // first read whole. `session::opening_reads` has it landed by then; a read it did not
        // foresee, or that is still running, is waited for here.
        if self.built_for.is_none()
            && let Some(pending) = self.pending.take()
        {
            if pending.finished() {
                debug!("{:?} opens on a read that has landed", self.model.kind());
            } else {
                debug!("{:?} opens waiting for its read", self.model.kind());
            }
            self.finish(gpu, pending);
        }
    }

    fn finish(&mut self, gpu: &Gpu, pending: Pending) {
        match pending.join() {
            Some(read) => self.land(gpu, read),
            None => {
                warn!("the preview worker panicked - the preview keeps its last picture");
                self.built_for = self.requested.clone();
            }
        }
    }

    /// Lands a finished read and starts the one for the latest request, if it differs.
    /// Called every tick and by [`Self::refresh_preview`].
    pub(crate) fn poll_preview(&mut self, gpu: &Gpu) {
        if let Some(pending) = &mut self.pending {
            if !pending.finished() {
                return;
            }
            let Some(pending) = self.pending.take() else {
                return;
            };
            self.finish(gpu, pending);
        }
        let Some(key) = self.requested.clone() else {
            return;
        };
        if self.built_for.as_ref() == Some(&key) {
            return;
        }
        let entry_changed = self
            .built_for
            .as_ref()
            .is_none_or(|(built, _)| *built != key.0);
        let facts = ReadFacts::of(
            key,
            &self.model,
            &self.layout,
            self.livery_axis,
            &self.sources,
            &self.previews,
            entry_changed,
        );
        self.pending = Some(match self.opening.take() {
            Some(opening)
                if opening.facts == facts && Arc::ptr_eq(&opening.archives, &self.archives) =>
            {
                opening.pending
            }
            _ => Pending::start(ReadJob {
                base: facts.slideshow.then(|| self.base.clone()),
                facts,
                previews: self.previews.clone(),
                archives: Arc::clone(&self.archives),
            }),
        });
    }

    fn land(&mut self, gpu: &Gpu, read: Read) {
        let Read { key, show, preview } = read;
        self.built_for = Some(key);
        match show {
            ShowRead::Unchanged => {}
            ShowRead::Loaded(show, sheet) => {
                self.slideshow = Some(*show);
                self.sheet = Some(*sheet);
                self.sheet_uploaded = false;
            }
            ShowRead::Failed => self.slideshow = None,
        }
        self.preview = match preview {
            None => None,
            Some(Err(error)) => {
                warn!("{error:#} - the preview draws nothing");
                None
            }
            Some(Ok(data)) => {
                let target = PreviewTarget {
                    device: &gpu.device,
                    queue: &gpu.queue,
                    format: gpu.output.config.format,
                    anisotropy: self.anisotropy,
                };
                match finish_preview(data, &target, &mut gpu.preview_pipelines.borrow_mut()) {
                    Ok(preview) => Some(preview),
                    Err(error) => {
                        warn!("{error:#} - the preview draws nothing");
                        None
                    }
                }
            }
        };
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
                let [track, race] = picker::hd::track::length_rows(length, self.laps.get(index).copied().flatten());
                self.model.set_track_info(index, 0, track);
                self.model.set_track_info(index, 1, race);
            } else {
                self.model.set_track_info(index, 0, picker::distance_text(length));
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
        (space, safe_area): (oag_display::space::Space, f32),
    ) {
        let (orbit, seconds, rect) = (self.orbit(), self.model.seconds(), self.layout.preview);
        let hull_swap = self.previews.ship_hull.is_some()
            && self.model.kind() == picker::Kind::Ship
            && !self.hull_phase().visible;
        if hull_swap {
            return;
        }
        // `Track Creation`'s own `<Mode3D>` camera, when authored.
        let mode3d_model = self
            .mode3d_model()
            .map(|model| oag_game::preview::placed_mode3d(model, self.model.kind(), space));
        let track_model = self.layout.hd_track.as_ref().and_then(|screen| screen.model);
        let ship_model = self
            .layout
            .hd
            .as_ref()
            .and_then(|screen| screen.ship_model)
            .filter(|_| self.previews.ship_hull.is_some() && self.model.kind() == picker::Kind::Ship);
        // The screen's own fit of the craft's depth, where the title has one and the
        // selected model's box is known.
        let frames_craft = self
            .previews
            .front_end
            .is_some_and(|front_end| front_end.ship_preview_fit.is_some());
        let (ship_fit, scale) = match self.ship_fit().zip(self.fit_motion) {
            Some(((fit, box_z), player)) => {
                let (framing, scale) = oag_game::preview::ship_model::placement(
                    frames_craft,
                    fit,
                    space.display_aspect,
                    box_z,
                    player.framing(),
                );
                (framing.map(|framing| (*fit, framing)), scale)
            }
            None => (None, 1.0),
        };
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
                oag_game::preview::ship_model::framed(
                    &widget,
                    ship_fit.as_ref().map(|(fit, framing)| (fit, *framing)),
                    space,
                    scale,
                    safe_area,
                );
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

    /// The title's depth fit and the box of the selected model, where the title
    /// has a fit and the box is known.
    fn ship_fit(&self) -> Option<(&oag_title::ShipFit, [f32; 2])> {
        if self.model.kind() != picker::Kind::Ship {
            return None;
        }
        let front_end = self.previews.front_end?;
        let fit = front_end
            .ship_preview_fit
            .as_ref()
            .or(front_end.ship_preview_settle.as_ref())?;
        let zone_hull = matches!(
            self.sources.get(self.model.index()),
            Some(PreviewSource::Ship {
                zone_entry: Some(_),
                ..
            })
        );
        let length = picker::hd::selected_length(&self.model);
        oag_game::preview::ship_model::box_z(fit, zone_hull, length).map(|box_z| (fit, box_z))
    }

    /// Advances the depth fit by the fixed ticks `dt` seconds make, one frame of
    /// the original's update each. Runs while the craft is hidden and on every
    /// step, as the original's does. `aspect` is the screen's own.
    pub(crate) fn tick_fit(&mut self, aspect: f32, dt: f32) {
        let Some((fit, box_z)) = self.ship_fit().map(|(fit, box_z)| (*fit, box_z)) else {
            return;
        };
        self.fit_motion
            .get_or_insert_with(|| oag_game::preview::ship_model::FitPlayer::open(&fit))
            .tick(&fit, aspect, box_z, dt);
    }

    /// The craft's swap state this tick on a title that draws the race hull -
    /// see [`oag_game::preview::hull_swap`].
    fn hull_phase(&self) -> oag_game::preview::hull_swap::Phase {
        oag_game::preview::hull_swap::phase(self.model.seconds(), self.model.since_selection())
    }

    /// How the preview is framed this tick - see [`oag_game::preview::orbit_for`].
    pub(crate) fn orbit(&self) -> Orbit {
        if self.previews.ship_hull.is_some() && self.model.kind() == picker::Kind::Ship {
            return oag_game::preview::hull_orbit();
        }
        oag_game::preview::orbit_for(self.model.kind(), self.model.seconds())
    }
}


/// Where [`build_preview`] builds: the device, the target the pass draws into, and the
/// sampling the menus use.
pub(crate) struct PreviewTarget<'a> {
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) anisotropy: Anisotropy,
}

/// Reads `source`'s preview mesh in `livery` and builds it, with `pipelines` kept for the next.
///
/// The picker calls it on the frame thread for each selection; the menus' prefetch
/// (`session::menu_source`) calls it on a worker for the entries a picker opens on, so the
/// first one compiles nothing on the frame thread.
#[expect(
    clippy::too_many_arguments,
    reason = "the screen's facts (archives, rules, kind), the entry's (source, livery, axis), \
              and where it is built"
)]
pub(crate) fn build_preview(
    archives: &mut oag_assets::Archives,
    previews: &Previews,
    kind: picker::Kind,
    source: &PreviewSource,
    livery: Option<&str>,
    axis: LiveryAxis,
    target: &PreviewTarget<'_>,
    pipelines: &mut oag_mesh::mesh_render::KeptBuildCache,
) -> Result<Preview> {
    let data = read_preview(archives, previews, kind, source, livery, axis)?;
    finish_preview(data, target, pipelines)
}

/// A preview mesh read and parsed off the disc, not yet on the GPU: everything
/// [`build_preview`] does that is not the upload, so it can run on a worker.
pub(crate) struct PreviewData {
    name: String,
    model: oag_mesh::mesh::Model,
    ramp: Option<oag_game::preview::track_model::Ramp>,
}

/// The disc half of [`build_preview`].
pub(crate) fn read_preview(
    archives: &mut oag_assets::Archives,
    previews: &Previews,
    kind: picker::Kind,
    source: &PreviewSource,
    livery: Option<&str>,
    axis: LiveryAxis,
) -> Result<PreviewData> {
    let source = source.clone();
    let name = source
        .entry_name(previews.ship_hull, previews.front_end)
        .context("no circuit model on the disc")?;
    let circuit_model =
        oag_game::preview::draws_circuit_model(previews.front_end, kind);
    // The variant row picks the directory the hull is read from; the
    // skin row picks a paint over it.
    let (skin, variant) = match axis {
        LiveryAxis::Skin => (livery, None),
        LiveryAxis::Variant => (None, livery.filter(|id| !id.is_empty())),
    };
    let variant_name = match (&source, variant) {
        (PreviewSource::Ship { location, join, .. }, Some(variant))
            if previews.ship_hull.is_some() =>
        {
            let located = oag_game::preview::variant::variant_location(*join, location, variant);
            PreviewSource::Ship {
                location: located,
                skins: Vec::new(),
                join: None,
                hull_file: None,
                zone_entry: None,
                race_hull: None,
            }
            .entry_name(previews.ship_hull, previews.front_end)
        }
        _ => None,
    };
    let mut model = if circuit_model {
        oag_game::preview::psp2_scene::model(archives, &name)
            .with_context(|| format!("{name} did not resolve as a preview mesh"))?
    } else {
        let entry = variant_name.as_deref().unwrap_or(&name);
        let race_hull = match &source {
            PreviewSource::Ship { race_hull, .. } => race_hull.as_deref(),
            PreviewSource::Track { .. } => None,
        };
        oag_game::preview::variant::model_or_default(
            archives,
            entry,
            Some(&name),
            race_hull,
        )
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
        for line in oag_game::preview::paint(archives, entry, &name, &mut model) {
            debug!("preview {name}: {line}");
        }
    }
    if previews.ship_hull.is_some() && matches!(source, PreviewSource::Ship { .. }) {
        oag_game::preview::frame_hull(&mut model);
    }
    // The circuit model's own material: the ramp comes out of the model
    // and the preview recolours its vertices every frame.
    let ramp = if circuit_model {
        Some(
            oag_game::preview::track_model::Ramp::of(archives, &name, &mut model)
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
    Ok(PreviewData { name, model, ramp })
}

/// The GPU half of [`build_preview`], on the thread that owns `pipelines`.
pub(crate) fn finish_preview(
    data: PreviewData,
    target: &PreviewTarget<'_>,
    pipelines: &mut oag_mesh::mesh_render::KeptBuildCache,
) -> Result<Preview> {
    let PreviewData { name, model, ramp } = data;
    let preview = pipelines.build(|| {
        Preview::new(
            target.device,
            target.queue,
            target.format,
            target.anisotropy,
            model,
        )
    })
    .with_context(|| format!("building the preview for {name}"))?;
    Ok(match ramp {
        Some(ramp) => preview.with_ramp(ramp),
        None => preview,
    })
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
            hull_file: None,
            zone_entry: None,
            race_hull: None,
        };
        assert_eq!(source.entry_name(None, None).as_deref(),
            Some(r"Data\Ships\Feisar\ship_FE.vex"));
    }

    /// A Zone race previews the Zone hull's front-end twin, not the race hull's.
    #[test]
    fn a_zone_race_previews_the_zone_hull() {
        let zone = oag_game::preview::zone_hull_file(oag_pulse::TITLE, oag_race::Mode::Zone);
        let source = PreviewSource::Ship {
            location: r"Data\Ships\Feisar".to_string(),
            skins: Vec::new(),
            join: None,
            hull_file: zone,
            zone_entry: None,
            race_hull: None,
        };
        assert_eq!(
            source.entry_name(None, None).as_deref(),
            Some(r"Data\Ships\Feisar\Zone_FE.vex")
        );
        assert_eq!(
            oag_game::preview::zone_hull_file(oag_pulse::TITLE, oag_race::Mode::TimeTrial),
            None
        );
        assert_eq!(
            oag_game::preview::zone_hull_file(oag_hd::TITLE, oag_race::Mode::Zone),
            None,
            "HD's Zone craft is a ship of its own, not a file in the team's directory"
        );
    }

    /// HD names its race hull, not a `ship_FE.vex` that no HD archive carries.
    #[test]
    fn ship_names_the_titles_hull_file() {
        let source = PreviewSource::Ship {
            location: r"Data\Ships\Feisar_c1".to_string(),
            skins: Vec::new(),
            join: None,
            hull_file: None,
            zone_entry: None,
            race_hull: None,
        };
        assert_eq!(
            source.entry_name(Some("ship.vex"), None).as_deref(),
            Some(r"Data\Ships\Feisar_c1\ship.vex")
        );
    }

    /// A Zone race on HD or Omega previews the hull the race flies, kept outside the team's own
    /// directory, whichever team is picked: not the team's race hull. Fails on a screen that
    /// names `<team>\ship.vex` in Zone.
    #[test]
    fn a_zone_race_previews_the_shared_hull_where_the_title_keeps_it_outside_the_team() {
        for title in [oag_hd::TITLE, oag_omega::TITLE] {
            let entry = |team: &str, mode| oag_game::preview::zone_race_hull_entry(title, team, mode);
            let zone = entry("Assegai", oag_race::Mode::Zone).expect("a Zone hull entry");
            assert_eq!(entry("Feisar", oag_race::Mode::Zone), Some(zone.clone()), "{}", title.name);
            assert_eq!(entry("Assegai", oag_race::Mode::TimeTrial), None, "{}", title.name);
            let source = PreviewSource::Ship {
                location: r"Data\Ships\Assegai".to_string(),
                skins: Vec::new(),
                join: None,
                hull_file: None,
                zone_entry: Some(zone.clone()),
                race_hull: None,
            };
            assert_eq!(
                source.entry_name(Some("ship.vex"), None).as_deref(),
                Some(zone.as_str())
            );
        }
        // Pulse's Zone hull is a file in the team's own directory, and Pure draws no mesh.
        for title in [oag_pulse::TITLE, oag_pure::TITLE] {
            assert_eq!(
                oag_game::preview::zone_race_hull_entry(title, "Feisar", oag_race::Mode::Zone),
                None,
                "{}",
                title.name
            );
        }
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
