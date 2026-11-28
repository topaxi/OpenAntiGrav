//! The disc chooser on screen: the cards or the list, and the renderer that
//! draws it.
//!
//! The one stage that runs before anything has been opened, which is what
//! decides everything about it. There is no disc, so there is no font, no
//! sprite sheet and no title skin - it draws with [`oag_ui::font::Atlas::build`]
//! and, until the cards' art has been read, [`oag_hud::sprite::Sheet::default`]:
//! the same disc-free pair the performance overlay is built from, in the 480x272
//! grid the renderer defaults to. The art is read on a worker
//! ([`crate::launcher::art`]) and swapped in when it lands.

use crate::launcher::{self, Launcher, art};
use crate::render::Renderer;
use oag_gameplay::input::Input;
use oag_hud::sprite::Sheet;
use oag_thread::Task;

use crate::app::gpu::Gpu;

/// What the window shows while a player is choosing which image to boot.
pub(crate) struct LauncherStage {
    pub(crate) renderer: Renderer,
    pub(crate) launcher: Launcher,
    /// The source the player settled on, waiting for the top of the next frame.
    ///
    /// Held for one frame rather than acted on where it is raised, for the
    /// reason `Session::frame`'s `Launch Game` check gives for its own
    /// ordering: the load that follows stalls the window for a second or so,
    /// and the frame that shows the row lighting up should be on screen before
    /// it does.
    pub(crate) picked: Option<String>,
    /// The cards' pictures, empty until read.
    sheet: Sheet,
    /// The read in flight.
    art: Option<Task<art::Loaded>>,
    /// The sources a read has been started for.
    requested: Vec<String>,
    /// [`Self::sheet`] has changed and the renderer has not been told.
    upload: bool,
}

impl LauncherStage {
    pub(crate) fn new(renderer: Renderer, launcher: Launcher) -> Self {
        Self {
            renderer,
            launcher,
            picked: None,
            sheet: Sheet::default(),
            art: None,
            requested: Vec::new(),
            upload: false,
        }
    }

    /// Whether a card's art is still being read.
    pub(crate) fn reading_art(&self) -> bool {
        self.art.is_some() || self.wants_art()
    }

    /// Cards, with a disc no read has covered: the first frame, a dropped disc,
    /// or one a key has just unlocked.
    fn wants_art(&self) -> bool {
        self.launcher.is_cards()
            && self
                .launcher
                .art_sources()
                .any(|source| !self.requested.iter().any(|seen| seen == source))
    }

    /// Takes a finished read, and starts one for whatever has none.
    pub(crate) fn poll_art(&mut self) {
        if let Some(task) = &mut self.art
            && task.is_finished()
        {
            match task.join() {
                Some(Ok(loaded)) => {
                    for (source, name) in loaded.pictures {
                        self.launcher.set_picture(&source, &name);
                    }
                    self.sheet = loaded.sheet;
                    self.upload = true;
                }
                _ => log::warn!("reading the launcher's art failed; the cards show names"),
            }
            self.art = None;
        }
        if self.art.is_none() && self.wants_art() {
            self.requested = self.launcher.art_sources().map(str::to_string).collect();
            let sources = self.requested.clone();
            self.art = Some(Task::spawn("launcher-art", move || art::load(&sources)));
        }
    }

    /// One tick: move the cursor, and take a confirmation if one arrives.
    pub(crate) fn update(&mut self, input: &mut Input, back: oag_gameplay::input::Button) {
        if self.picked.is_some() {
            return;
        }
        self.picked = self.launcher.update(input, back);
        if self.launcher.take_paste_request()
            && let Some(text) = crate::app::clipboard::read()
        {
            self.launcher.paste(&text);
        }
    }

    /// The same tick's pointer, on the same terms as [`Self::update`].
    pub(crate) fn pointer(&mut self, pointer: &oag_ui::pointer::Pointer) {
        if self.picked.is_some() {
            return;
        }
        self.picked = self.launcher.pointer(pointer);
    }

    /// Draws the chooser into `target`.
    pub(crate) fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
    ) {
        if std::mem::take(&mut self.upload) {
            self.renderer
                .set_sprites(&gpu.device, &gpu.queue, &self.sheet);
        }
        let list = launcher::draw(&self.launcher, &self.sheet);
        self.renderer.render(
            &gpu.device,
            &gpu.queue,
            encoder,
            target,
            &list,
            viewport,
            None,
        );
    }
}
