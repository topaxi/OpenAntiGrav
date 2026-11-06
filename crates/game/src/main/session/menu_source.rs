//! The race box's disc reads, done on a worker while the menus are up.
//!
//! Track Select and Ship Select each opened the source on the frame that opened them (88 ms on
//! the Steam Deck, mostly the CHD's hunk map), Track Select a second time for its unlock gate,
//! and the first preview mesh compiled its pipelines there (63 ms, 0.7 s on a cold driver
//! cache). Here one worker, started on the first menu frame, opens the source once, reads the
//! gate and builds the previews the two screens open on, so their pipelines are compiled before
//! either screen is. The screens then share its archives. See
//! `docs/rendering/frame-thread-cpu.md` ("The race box's selection screens").
//!
//! Natively only: wgpu's web types are not `Send`, so the web build opens on demand as before.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use log::{debug, warn};
use oag_mesh::mesh_render::KeptBuildCache;
use oag_ui_screens::picker::Kind;

use crate::picker_stage::{LiveryAxis, PreviewSource, PreviewTarget, Previews, SharedArchives};
use crate::stage::Stage;

use super::Session;

/// Which source the menus' archives were opened from: a changed source or DLC set is another.
pub(super) type Key = (String, Vec<PathBuf>);

/// What the worker hands back.
pub(crate) struct Opened {
    key: Key,
    archives: SharedArchives,
    /// A second, owned open for the one screen that keeps its archives (the campaign's art
    /// player): taking it costs nothing, and a refill is opened on another worker.
    spare: Spare,
    /// `FEData.wad` of a PSP source, opened ahead for the campaign's per-cell skill read (it
    /// is no part of the mounted archives). `None` where the source has no such file.
    fedata: Option<Arc<Mutex<oag_assets::Archive>>>,
    gate: oag_game::unlock::Gate,
    /// The previews' pipelines, built on `device`.
    pipelines: KeptBuildCache,
    device: wgpu::Device,
}

/// The owned archives a campaign screen takes.
pub(crate) enum Spare {
    None,
    Ready(Box<oag_assets::Archives>),
    #[cfg(not(target_arch = "wasm32"))]
    Opening(oag_thread::Task<Option<oag_assets::Archives>>),
}

/// The menus' source: not started, being opened, or open.
#[derive(Default)]
pub(crate) enum MenuSource {
    #[default]
    Idle,
    #[cfg(not(target_arch = "wasm32"))]
    Opening(Key, oag_thread::Task<Option<Opened>>),
    Open(Box<Opened>),
    /// Opening failed for this key, which was said; the screens open on demand as before.
    Failed(Key),
}

/// One preview the worker builds to compile its pipelines.
struct Warm {
    kind: Kind,
    source: PreviewSource,
}

/// Everything the worker needs, owned.
#[cfg(not(target_arch = "wasm32"))]
struct Job {
    key: Key,
    title: &'static oag_title::Title,
    previews: Previews,
    warm: Vec<Warm>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    anisotropy: oag_mesh::mesh_render::Anisotropy,
}

/// Opens the key's source with its DLC packs.
#[cfg(not(target_arch = "wasm32"))]
fn open_key((source, dlc): &Key) -> Option<oag_assets::Archives> {
    let (packs, pure_packs, problems) =
        oag_source::dlc::packs_from_defaults(dlc, &oag_source::cache::default_dlc_cache_dir());
    for problem in problems {
        warn!("{problem}");
    }
    match oag_source::title::open_source(source, packs, pure_packs) {
        Ok(opened) => Some(opened.archives),
        Err(error) => {
            warn!("cannot open {source} for the menus: {error:#}");
            None
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Job {
    fn run(self) -> Option<Opened> {
        let started = web_time::Instant::now();
        let mut archives = open_key(&self.key)?;
        let gate = oag_game::unlock::Gate::read(self.title, &mut archives);
        // Before the previews' pipelines (0.8 s on a cold driver cache): the campaign screens
        // take this one, and a player reaches them sooner than a preview.
        let spare = open_key(&self.key).map_or(Spare::None, |archives| Spare::Ready(Box::new(archives)));
        let fedata =
            oag_assets::Archive::open(&super::campaign::fedata_spec(&self.key.0))
                .ok()
                .map(|archive| Arc::new(Mutex::new(archive)));
        let opened_in = started.elapsed();
        let mut pipelines = KeptBuildCache::default();
        let target = PreviewTarget {
            device: &self.device,
            queue: &self.queue,
            format: self.format,
            anisotropy: self.anisotropy,
        };
        for warm in &self.warm {
            if oag_thread::gpu_workers::stopping() {
                return None;
            }
            if let Err(error) = crate::picker_stage::build_preview(
                &mut archives,
                &self.previews,
                warm.kind,
                &warm.source,
                None,
                LiveryAxis::Variant,
                &target,
                &mut pipelines,
            ) {
                // The screen reports its own previews; this one only compiles ahead.
                debug!("menu source: no {:?} preview to build ahead: {error:#}", warm.kind);
            }
        }
        debug!(
            "menu source: opened in {:.0} ms, {} preview pipeline(s) built in {:.0} ms more",
            opened_in.as_secs_f64() * 1e3,
            pipelines.pipelines(),
            (started.elapsed() - opened_in).as_secs_f64() * 1e3
        );
        Some(Opened {
            key: self.key,
            archives: Arc::new(Mutex::new(archives)),
            spare,
            fedata,
            gate,
            pipelines,
            device: self.device,
        })
    }
}

impl Session {
    /// The source the menus would open now, `None` with no source chosen.
    pub(super) fn menu_source_key(&self) -> Option<Key> {
        let options = self.race_options.as_ref()?;
        Some((options.source.clone(), options.dlc.clone()))
    }

    /// Starts the worker on a menu frame, once per source. A no-op on every later frame.
    pub(crate) fn start_menu_source(&mut self) {
        if !matches!(self.stage, Stage::Menu(_)) {
            return;
        }
        let Some(key) = self.menu_source_key() else {
            return;
        };
        let current = match &self.menu_source {
            MenuSource::Idle => None,
            #[cfg(not(target_arch = "wasm32"))]
            MenuSource::Opening(opening, _) => Some(opening),
            MenuSource::Open(opened) => Some(&opened.key),
            MenuSource::Failed(failed) => Some(failed),
        };
        if current == Some(&key) {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(shell) = self.shell.as_ref() else {
                return;
            };
            let Some(previews) = self.picker_previews() else {
                return;
            };
            let job = Job {
                key: key.clone(),
                title: shell.title,
                previews,
                warm: self.warm_previews(),
                device: self.gpu.device.clone(),
                queue: self.gpu.queue.clone(),
                format: self.gpu.output.config.format,
                anisotropy: self.anisotropy,
            };
            let guard = oag_thread::gpu_workers::Guard::enter("menu-source");
            let task = oag_thread::Task::spawn("menu-source", move || {
                guard.hold_to_thread_end();
                job.run()
            });
            self.menu_source = MenuSource::Opening(key, task);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.menu_source = MenuSource::Failed(key);
        }
    }

    /// The menus' opened source for the current options, waiting for the worker if it is still
    /// running. `None` where there is none (the web, a failed open, a changed source): the
    /// caller opens its own as it always did.
    pub(crate) fn menu_source(&mut self) -> Option<(SharedArchives, oag_game::unlock::Gate)> {
        let key = self.menu_source_key()?;
        #[cfg(not(target_arch = "wasm32"))]
        if let MenuSource::Opening(opening, _) = &self.menu_source
            && opening == &key
        {
            let MenuSource::Opening(opening, mut task) = std::mem::take(&mut self.menu_source)
            else {
                unreachable!("matched just above");
            };
            let waited = web_time::Instant::now();
            let finished = task.is_finished();
            self.menu_source = match task.join() {
                Some(Ok(Some(mut opened))) => {
                    if !finished {
                        debug!(
                            "menu source: waited {:.0} ms for the worker",
                            waited.elapsed().as_secs_f64() * 1e3
                        );
                    }
                    self.adopt_preview_pipelines(&mut opened);
                    MenuSource::Open(Box::new(opened))
                }
                Some(Err(_)) => {
                    warn!("the menu source worker panicked; the screens open the disc themselves");
                    MenuSource::Failed(opening)
                }
                _ => MenuSource::Failed(opening),
            };
        }
        match &self.menu_source {
            MenuSource::Open(opened) if opened.key == key => {
                Some((Arc::clone(&opened.archives), opened.gate.clone()))
            }
            _ => None,
        }
    }

    /// [`Self::menu_source`] that never waits: `None` while the worker is still running.
    pub(super) fn landed_menu_source(&mut self) -> Option<(SharedArchives, oag_game::unlock::Gate)> {
        #[cfg(not(target_arch = "wasm32"))]
        if matches!(&self.menu_source, MenuSource::Opening(_, task) if !task.is_finished()) {
            return None;
        }
        self.menu_source()
    }

    /// Owned archives for a screen that keeps them, from the worker's spare when the menus'
    /// source is open for the current options (a refill is opened on a worker at once), else
    /// `None`: the caller opens its own.
    pub(crate) fn take_menu_archives(&mut self) -> Option<oag_assets::Archives> {
        self.menu_source()?;
        let MenuSource::Open(opened) = &mut self.menu_source else {
            return None;
        };
        let taken = match std::mem::replace(&mut opened.spare, Spare::None) {
            Spare::Ready(archives) => Some(*archives),
            #[cfg(not(target_arch = "wasm32"))]
            Spare::Opening(mut task) => task.join().and_then(Result::ok).flatten(),
            Spare::None => None,
        };
        #[cfg(not(target_arch = "wasm32"))]
        if taken.is_some() {
            let key = opened.key.clone();
            opened.spare = Spare::Opening(oag_thread::Task::spawn("menu-spare", move || {
                open_key(&key)
            }));
        }
        taken
    }

    /// What a campaign cell's launch reads off the disc, from the menus' source when it is
    /// open: `FEData.wad` and the shared archives. Both `None` where there is no such source.
    pub(crate) fn menu_campaign_reads(
        &mut self,
    ) -> (Option<Arc<Mutex<oag_assets::Archive>>>, Option<SharedArchives>) {
        let Some((archives, _)) = self.menu_source() else {
            return (None, None);
        };
        let fedata = match &self.menu_source {
            MenuSource::Open(opened) => opened.fedata.clone(),
            _ => None,
        };
        (fedata, Some(archives))
    }

    /// Runs `read` on the menus' shared archives (opened on the worker), or on a fresh open
    /// where there are none (the web, a changed source).
    pub(crate) fn with_menu_archives<R>(
        &mut self,
        what: &str,
        read: impl FnOnce(&mut oag_assets::Archives) -> R,
    ) -> Option<R> {
        if let Some((archives, _)) = self.menu_source() {
            let mut archives = archives.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            return Some(read(&mut archives));
        }
        let mut archives = self.open_title_archives(what)?;
        Some(read(&mut archives))
    }

    /// The worker's pipelines become the previews' own, where they were built for this device
    /// and the frame thread has not built its own meanwhile.
    fn adopt_preview_pipelines(&self, opened: &mut Opened) {
        let mut kept = self.gpu.preview_pipelines.borrow_mut();
        if opened.device == self.gpu.device && kept.pipelines() == 0 {
            *kept = std::mem::take(&mut opened.pipelines);
        }
    }

    /// The selection screens' preview rules, off the shell. `None` with no shell.
    pub(crate) fn picker_previews(&self) -> Option<Previews> {
        let shell = self.shell.as_ref()?;
        let front_end = shell.title.front_end;
        Some(Previews {
            meshes: front_end.is_some_and(|front_end| front_end.preview_meshes),
            ship_hull: front_end.and_then(|front_end| front_end.ship_preview_hull),
            front_end,
            globals: shell.globals.clone(),
            strings: shell.strings.clone(),
        })
    }

    /// The circuit and the team the two screens would open on now: the previews whose
    /// pipelines are worth compiling ahead. Another entry of the same screen draws through the
    /// same pipelines.
    fn warm_previews(&self) -> Vec<Warm> {
        let Some(shell) = self.shell.as_ref() else {
            return Vec::new();
        };
        let title = shell.title;
        let mode = self.race_mode();
        let draws = |kind| {
            oag_game::preview::draws_mesh(title, kind)
                || (kind == Kind::Ship
                    && title
                        .front_end
                        .is_some_and(|front_end| front_end.ship_preview_hull.is_some()))
        };
        let mut warm = Vec::new();
        let tracks = shell.tracks_for(mode);
        let track = tracks
            .iter()
            .find(|(track, _)| track.id == self.settings.race.track)
            .or_else(|| tracks.first());
        if let Some((track, _)) = track
            && draws(Kind::Track)
        {
            warm.push(Warm {
                kind: Kind::Track,
                source: PreviewSource::Track {
                    location: track.location.clone(),
                    reversed: track.reversed,
                    zone: mode == oag_race::Mode::Zone,
                },
            });
        }
        // Every team, not only the current one: each hull may need a pipeline the others do not,
        // and a pipeline first built on the frame thread is a 30 to 50 ms Down step on the Deck.
        if draws(Kind::Ship) {
            for choice in &shell.teams {
                let details = shell
                    .team_details
                    .iter()
                    .find(|team| team.id == choice.value);
                warm.push(Warm {
                    kind: Kind::Ship,
                    source: super::picker::ship_source(
                        title,
                        details,
                        &choice.value,
                        mode,
                        Vec::new(),
                    ),
                });
            }
        }
        warm
    }
}
