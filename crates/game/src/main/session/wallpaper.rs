//! A wallpaper's outputs as the session's windows: each output a desktop background
//! (`crate::wallpaper`) drawn like a further window of a `--windows` run, with a camera of its
//! own and nobody seated (`Race::watch_in_windows`). Outputs come and go while it runs.

use anyhow::{Context, Result};
use log::debug;

use oag_present::{perf, upscale};

use crate::gpu::Output;
use crate::stage::Stage;
use crate::windows::Screen;

use super::Session;

impl Session {
    /// Another output's background, after the ones already drawn.
    pub(crate) fn add_background(&mut self, output: Output) -> Result<()> {
        let framebuffer =
            upscale::Framebuffer::new(&self.gpu.device, output.config.format, self.framebuffer.allocation())
                .context("building a background's upscale pipeline")?;
        self.screens.push(Screen {
            output,
            framebuffer,
            acquire: perf::Meter::new(),
            present: perf::Meter::new(),
            rings: super::timing::Rings::new(&self.gpu.device, &self.gpu.queue),
            pointer: crate::pointer::Window::default(),
        });
        self.screens_opened = true;
        self.rewatch();
        Ok(())
    }

    /// Output `index` (0 is window 1) went away. Window 1 takes over the next output's surface
    /// when there is one; with none left it draws nothing until [`Self::replace_primary`].
    pub(crate) fn remove_background(&mut self, index: usize) {
        if index == 0 {
            if self.screens.is_empty() {
                self.gpu.output.lose();
            } else {
                std::mem::swap(&mut self.gpu.output, &mut self.screens[0].output);
                let (width, height) = self.gpu.output.size();
                self.resize(width, height);
                self.close_screen(0);
            }
        } else {
            self.close_screen(index - 1);
        }
        self.rewatch();
    }

    /// Window 1 on a new output, after every output had gone.
    pub(crate) fn replace_primary(&mut self, output: Output) {
        self.gpu.output = output;
        let (width, height) = self.gpu.output.size();
        self.resize(width, height);
        self.rewatch();
    }

    /// Output `index` (0 is window 1) was configured to `size` pixels.
    pub(crate) fn resize_background(&mut self, index: usize, size: winit::dpi::PhysicalSize<u32>) {
        if index == 0 {
            self.gpu.output.window.set_size(size);
            self.resize(size.width, size.height);
        } else if let Some(screen) = self.screens.get_mut(index - 1) {
            screen.output.window.set_size(size);
            self.resize_screen(index - 1, size);
        }
    }

    /// Deals the demo on screen across the outputs drawn now, a camera each. A demo still
    /// loading is dealt when its race is handed over (`Session::advance_race_build`).
    fn rewatch(&mut self) {
        let windows = 1 + self.screens.len();
        if let Stage::Race(stage) = &mut self.stage
            && stage.demo.is_some()
        {
            stage.race.watch_in_windows(windows);
            debug!("wallpaper: the demo is watched in {windows} output(s)");
        }
    }
}
