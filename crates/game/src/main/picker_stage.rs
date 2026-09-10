//! The race box's selection screens, over the menu stage: the picker model,
//! the disc's layout for it, and the preview mesh for whatever is selected.
//!
//! Held on [`crate::menu_stage::MenuStage::picker`] rather than as a stage of
//! its own, the same way a modal prompt is: while one is open the menu behind
//! it is a picture, and closing it lands back on the page it opened from with
//! nothing rebuilt. See `oag_ui::picker` for the model and
//! `crate::session::picker` for the flow that opens and closes one.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use log::{info, warn};
use oag_game::preview::Preview;
use oag_render::camera::orbit::Orbit;
use oag_render::mesh_render::Anisotropy;
use oag_ui::picker::{self, Picker};

use crate::gpu::Gpu;

/// Where an entry's preview mesh is on the disc.
#[derive(Debug, Clone)]
pub(crate) enum PreviewSource {
    /// `<location>\FE\forward.vex` or `\reverse.vex` - the circuit's outline
    /// ribbon, which is what `Track Creation`'s info panel shows. See
    /// `docs/formats/race-setup.md`.
    Track { location: String, reversed: bool },
    /// `<location>\ship_FE.vex`, the team's front-end hull, and the skins it
    /// declares as `(name, archive entry)` - the paint the livery row cycles.
    Ship {
        location: String,
        skins: Vec<(String, String)>,
    },
}

impl PreviewSource {
    fn entry_name(&self) -> String {
        match self {
            Self::Track { location, reversed } => {
                let run = if *reversed { "reverse" } else { "forward" };
                format!(r"{location}\FE\{run}.vex")
            }
            Self::Ship { location, .. } => format!(r"{location}\ship_FE.vex"),
        }
    }
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
}

impl PickerStage {
    pub(crate) fn new(
        model: Picker,
        layout: picker::Layout,
        livery_axis: LiveryAxis,
        sources: Vec<PreviewSource>,
        archives: oag_assets::Archives,
        distances: Option<Distances>,
        anisotropy: Anisotropy,
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
        }
    }

    /// Loads the selected entry's preview, in the selected livery, if it is
    /// not the one already built.
    pub(crate) fn refresh_preview(&mut self, gpu: &Gpu) {
        let index = self.model.index();
        let livery = match self.livery_axis {
            LiveryAxis::Skin => self.model.variant().map(|(id, _)| id.clone()),
            LiveryAxis::Variant => None,
        };
        let key = (index, livery);
        if self.built_for.as_ref() == Some(&key) {
            return;
        }
        self.built_for = Some(key.clone());
        self.preview = match self.load_preview(gpu, index, key.1.as_deref()) {
            Ok(preview) => Some(preview),
            Err(error) => {
                warn!("{error:#} - the preview draws nothing");
                None
            }
        };
    }

    fn load_preview(&mut self, gpu: &Gpu, index: usize, skin: Option<&str>) -> Result<Preview> {
        let source = self
            .sources
            .get(index)
            .with_context(|| format!("entry {index} has no preview source"))?
            .clone();
        let name = source.entry_name();
        let blob = self
            .archives
            .read_name(&name)
            .with_context(|| format!("reading the preview mesh {name}"))?;
        let mut model = oag_render::mesh::build(&name, &blob)
            .with_context(|| format!("decoding the preview mesh {name}"))?;
        // The chosen paint over the hull's own texture slots - the same
        // swap a race makes (`crate::livery::ship_skin`), and the same
        // rule when it fails: the hull keeps its own paint, and the log
        // says so.
        if let (PreviewSource::Ship { skins, .. }, Some(skin)) =
            (&source, skin.filter(|s| !s.is_empty()))
            && let Some((_, entry)) = skins.iter().find(|(id, _)| id == skin)
        {
            match self
                .archives
                .read_name(entry)
                .map_err(anyhow::Error::from)
                .and_then(|blob| oag_texture::ship_skin::parse(&blob).map_err(anyhow::Error::from))
            {
                Ok(paint) => {
                    let applied = oag_render::mesh::ship_skin::apply(&mut model, &paint);
                    info!("preview {name}: skin {entry} over {applied} texture slot(s)");
                }
                Err(error) => warn!("{entry}: {error:#} - the preview keeps the hull's own paint"),
            }
        }
        info!(
            "preview {name}: {} vertices, {} triangles",
            model.vertices.len(),
            model.indices.len() / 3
        );
        Preview::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            self.anisotropy,
            model,
        )
        .with_context(|| format!("building the preview for {name}"))
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
            self.model.set_track_info(index, 0, format!("{length:.0}"));
        }
    }

    /// How the preview is framed this tick - see [`oag_game::preview::orbit_for`].
    pub(crate) fn orbit(&self) -> Orbit {
        oag_game::preview::orbit_for(self.model.kind(), self.model.seconds())
    }
}
