//! Builds a race's EndRace screens on a worker while the race runs.
//!
//! Opening the disc image, reading `EndRace_Definition.xml`, uploading the sprite sheet and
//! building the trophy previews take tens of milliseconds on a desktop and several times that
//! on a handheld, and none of it depends on how the race went. Done on the frame that crosses
//! the line it was the finish-line hitch (`docs/rendering/frame-thread-cpu.md`, "The finish
//! frame"). Here it starts on the race's first frame and the finish only assembles the result.

use log::warn;

use crate::race_stage::endrace::EndRacePrep;
use crate::stage::Stage;

use super::super::Session;

/// The worker building a race's EndRace screens. Natively an [`oag_thread::Task`]; on the web
/// nothing is ever started (wgpu's web types are not `Send`), so the type is uninhabited there
/// and every `Option` of it is `None`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) type Prefetch = oag_thread::Task<Option<EndRacePrep>>;

/// See the native alias above.
#[cfg(target_arch = "wasm32")]
pub(crate) enum Prefetch {}

#[cfg(target_arch = "wasm32")]
impl Prefetch {
    pub(crate) fn is_finished(&self) -> bool {
        match *self {}
    }

    pub(crate) fn join(&mut self) -> Option<Result<Option<EndRacePrep>, oag_thread::Panicked>> {
        match *self {}
    }
}

/// What the build needs, all owned so the closure can leave the frame thread.
pub(crate) struct EndRacePrepArgs {
    source: String,
    dlc: Vec<std::path::PathBuf>,
    strings: oag_ui::language::StringTable,
    faces: oag_ui_screens::picker::FaceScales,
    grid: [f32; 2],
    globals: Vec<(String, String)>,
    base_sprites: oag_hud::sprite::Sheet,
    atlas: oag_ui::font::Atlas,
    title: &'static oag_title::Title,
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    anisotropy: oag_mesh::mesh_render::Anisotropy,
    with_trophies: bool,
}

impl EndRacePrepArgs {
    /// Opens the source, reads the screens and builds the GPU side. `None` after saying why:
    /// the caller then keeps the built-in results table for this race.
    pub(crate) fn run(self) -> Option<EndRacePrep> {
        let (packs, pure_packs, problems) = oag_source::dlc::packs_from_defaults(
            &self.dlc,
            &oag_source::cache::default_dlc_cache_dir(),
        );
        for problem in problems {
            warn!("{problem}");
        }
        let mut archives = match oag_source::title::open_source(&self.source, packs, pure_packs) {
            Ok(opened) => opened.archives,
            Err(error) => {
                warn!(
                    "cannot open {} for the EndRace screens: {error:#}",
                    self.source
                );
                return None;
            }
        };
        let global_refs: Vec<(&str, &str)> = self
            .globals
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        let screens = match oag_game::endrace::load(
            &mut archives,
            &self.strings,
            self.faces,
            self.grid,
            &self.base_sprites,
            &global_refs,
            self.title,
        ) {
            Ok(screens) => screens,
            Err(error) => {
                // Not "this source has no EndRace screens": the entry name asked for is
                // Pulse's own, and a title that authors its own elsewhere reaches here too.
                // Wipeout HD ships `/data/plugins/frontend/gui/endrace_definition.xml` in
                // five of its seven archives, in a widget vocabulary this build does not
                // read yet - see `oag_game::endrace::SCREEN_ENTRY`'s own doc.
                warn!(
                    "{error:#} - this title's EndRace screens are not read by this build; \
                     keeping the built-in results table"
                );
                return None;
            }
        };
        match EndRacePrep::build(
            &self.device,
            &self.queue,
            self.format,
            self.anisotropy,
            self.atlas,
            screens,
            self.with_trophies,
        ) {
            Ok(prep) => Some(prep),
            Err(error) => {
                warn!("cannot build the EndRace screens' own renderer: {error:#}");
                None
            }
        }
    }
}

impl Session {
    /// The inputs of [`EndRacePrepArgs::run`] for the race on screen, `None` when there is no
    /// shell to read them from (`--race`) or the title's flow is the touch one.
    pub(crate) fn endrace_prep_args(&self) -> Option<EndRacePrepArgs> {
        let Stage::Race(stage) = &self.stage else {
            return None;
        };
        let shell = self.shell.as_ref()?;
        let race_options = self.race_options.as_ref()?;
        if oag_game::endrace::dialect(shell.title) == Some(oag_title::EndRaceDialect::Touch) {
            return None;
        }
        Some(EndRacePrepArgs {
            source: race_options.source.clone(),
            dlc: race_options.dlc.clone(),
            strings: shell.strings.clone(),
            faces: oag_ui_screens::picker::FaceScales {
                default: shell.menu_font.as_ref().map_or(
                    oag_ui_screens::picker::FaceScales::default().default,
                    |menu| shell.font.line_height / menu.line_height,
                ),
                ..oag_ui_screens::picker::FaceScales::default()
            },
            grid: [shell.space.size.0, shell.space.size.1],
            globals: shell
                .globals
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            base_sprites: shell.sprites.clone(),
            atlas: shell
                .menu_font
                .clone()
                .unwrap_or_else(|| shell.font.clone()),
            title: shell.title,
            device: self.gpu.device.clone(),
            queue: self.gpu.queue.clone(),
            format: self.gpu.output.config.format,
            anisotropy: self.anisotropy,
            with_trophies: stage.campaign_cell.is_some(),
        })
    }

    /// Starts the worker that builds this race's EndRace screens, once per race, on the first
    /// frame the race runs. A no-op on every later frame, on a finished race and where there is
    /// nothing to build.
    pub(crate) fn start_endrace_prefetch(&mut self) {
        let Stage::Race(stage) = &self.stage else {
            return;
        };
        if stage.endrace_prefetch_started || stage.race.finished() {
            return;
        }
        // Not on the web: wgpu's web types belong to the page's thread and the race build runs
        // inline there too (docs/tools/web.md, "Threads"), so the finishing frame builds the
        // screens itself as it did before the prefetch.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let args = self.endrace_prep_args();
            let Stage::Race(stage) = &mut self.stage else {
                return;
            };
            stage.endrace_prefetch =
                args.map(|args| oag_thread::Task::spawn("endrace-prefetch", move || args.run()));
        }
        let Stage::Race(stage) = &mut self.stage else {
            return;
        };
        stage.endrace_prefetch_started = true;
    }

    /// Waits for the EndRace worker, if one runs, and drops what it built. A run that ends
    /// (`--drive`) calls this before the process exits: the worker holds clones of the device
    /// and queue, and a process that exits under it tears the Vulkan driver down mid-submit
    /// (a SIGSEGV on a loaded machine).
    pub(crate) fn finish_endrace_prefetch(&mut self) {
        if let Stage::Race(stage) = &mut self.stage
            && let Some(mut task) = stage.endrace_prefetch.take()
        {
            drop(task.join());
        }
    }

    /// The prepared screens: the worker's when it ran and succeeded, else built here. `None`
    /// when they cannot be built, which has been said already.
    pub(super) fn take_endrace_prep(&mut self) -> Option<EndRacePrep> {
        let task = match &mut self.stage {
            Stage::Race(stage) => stage.endrace_prefetch.take(),
            _ => None,
        };
        if let Some(mut task) = task {
            match task.join() {
                Some(Ok(prep)) => return prep,
                Some(Err(_)) => warn!("the EndRace prefetch panicked; building the screens now"),
                None => {}
            }
        }
        self.endrace_prep_args()?.run()
    }
}

#[cfg(test)]
mod tests {
    /// The body of `Session::build_endrace`: the one function that runs on the frame that
    /// crosses the line.
    fn build_endrace_body() -> &'static str {
        let source = include_str!("endrace.rs");
        let start = source
            .find("pub(crate) fn build_endrace(&mut self)")
            .expect("build_endrace is in endrace.rs");
        let end = source[start..]
            .find("pub(super) fn mark_endrace_unavailable")
            .expect("mark_endrace_unavailable follows it");
        &source[start..start + end]
    }

    /// The finish-line hitch was a disc open, an XML read, a sprite upload and a mesh build on
    /// the finishing frame. They live in [`super::EndRacePrepArgs::run`], which the worker
    /// calls; the finishing frame may only take the result (`take_endrace_prep`).
    #[test]
    fn the_finishing_frame_reads_no_disc_and_builds_no_gpu_resource() {
        let body = build_endrace_body();
        for forbidden in [
            "open_source(",
            "packs_from_defaults(",
            "endrace::load(",
            "Renderer::new(",
            "Preview::new(",
            "read_image(",
            "std::fs::",
        ] {
            assert!(
                !body.contains(forbidden),
                "build_endrace runs on the finishing frame and must not call {forbidden}"
            );
        }
        assert!(
            body.contains("take_endrace_prep()"),
            "the finishing frame takes the prefetched screens"
        );
    }

    /// The worker's side is where those calls went: if this fails the prefetch builds nothing.
    #[test]
    fn the_worker_does_the_disc_read_and_the_gpu_build() {
        let source = include_str!("endrace_prefetch.rs");
        let run = &source[source.find("pub(crate) fn run(self)").unwrap()..];
        for needed in ["open_source(", "endrace::load(", "EndRacePrep::build("] {
            assert!(run.contains(needed), "EndRacePrepArgs::run must call {needed}");
        }
    }
}
