//! What `MenuStage` draws over the menus rather than as one of their pages,
//! split out of `menu_stage.rs` under the 1,000-line rule: the pause tint,
//! Wipeout Pure's SINGLE PLAYER screens and the split-screen join screen.

use anyhow::Result;
use oag_ui::menu;

use super::{MenuStage, footer};
use crate::gpu::Gpu;

/// The pause overlay's tint, over a parked race's picture.
///
/// **Chosen, not authored.** Neither PSP title's own front-end XML defines a
/// pause screen to read one off: `Skin.xml`'s own `LoadXML` list is
/// exhaustively 22 files - see
/// [fe-menu-definitions.md](../../../../../docs/formats/fe-menu-definitions.md) -
/// and none of them is a `Pause` definition. The one HUD string that names
/// pausing, `IG_PAUSE_QUIT`, is drawn by nothing:
/// [hud.md](../../../../../docs/ui/hud.md) records "There is no pause" against
/// it, because leaving a race used to drop the `World` outright. So there is
/// no colour or alpha on the disc for this to recover, and this one is
/// this project's own rather than a reading of one.
pub(super) const PAUSE_OVERLAY: [f32; 4] = [0.0, 0.0, 0.0, 0.55];

impl MenuStage {
    /// Wipeout Pure's SINGLE PLAYER screens, drawn as the title's own menu pages
    /// are: on the frame the page behind would carry (its background and marks,
    /// the `Confirm` legend and ticker), with the page's rows replaced by the
    /// screen's (`oag_ui_screens::pure_chain::draw_list`).
    #[expect(
        clippy::too_many_arguments,
        reason = "the same facts `MenuStage::render` already has in hand"
    )]
    pub(super) fn render_pure_chain(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        shown: Option<menu::Picture>,
        frozen_race: bool,
        ticker_tips: &[String],
    ) -> Result<()> {
        let Some(open) = self.pure_chain.as_mut() else {
            return Ok(());
        };
        // The picture beside the rows lives on a sheet of its own, the front
        // end's plus the textures shown so far.
        if let Some(sheet) = open.art_mut().take_sheet() {
            self.renderer.set_sprites(&gpu.device, &gpu.queue, sheet);
        }
        let art = open.art().draws();
        let mut layers = oag_ui_screens::pure_chain::draw_list(
            open.choice(),
            &self.skin,
            &self.frame,
            if frozen_race { None } else { shown },
            frozen_race,
        );
        layers.body.extend(art);
        let footer = self
            .ticker
            .is_some()
            .then(|| self.footer_overlay(Vec::new(), ticker_tips));
        let (list, ticker_clip) = footer::append(layers.flatten(), footer);
        self.renderer.render_with(
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            &gpu.device,
            &gpu.queue,
            encoder,
            view,
            &list,
            viewport,
            footer::resolve_clip(None, ticker_clip, frozen_race),
        );
        Ok(())
    }

    /// The split-screen join screen, drawn as Wipeout Pure's SINGLE PLAYER
    /// screens are: on the frame the page behind it would carry, with the
    /// page's rows replaced by the screen's (`oag_ui_screens::join::draw_list`).
    #[expect(
        clippy::too_many_arguments,
        reason = "the same facts `MenuStage::render` already has in hand"
    )]
    pub(super) fn render_join(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        shown: Option<menu::Picture>,
        frozen_race: bool,
        ticker_tips: &[String],
    ) -> Result<()> {
        let Some(open) = self.join.as_ref() else {
            return Ok(());
        };
        let layers = oag_ui_screens::join::draw_list(
            open.screen(),
            &self.skin,
            &self.frame,
            if frozen_race { None } else { shown },
            frozen_race,
        );
        let footer = self
            .ticker
            .is_some()
            .then(|| self.footer_overlay(Vec::new(), ticker_tips));
        let (list, ticker_clip) = footer::append(layers.flatten(), footer);
        self.renderer.render_with(
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            &gpu.device,
            &gpu.queue,
            encoder,
            view,
            &list,
            viewport,
            footer::resolve_clip(None, ticker_clip, frozen_race),
        );
        Ok(())
    }
}
