//! The style's animated menu backdrop: one slot, because the two widgets are
//! never up together - `BackgroundAnim_Load` enables the scene widget only
//! when `FrontEnd_IsFuryStyle` is false, and the Fury style enables
//! `BackgroundAnimFury` instead.
//!
//! Everything a caller does with the slot is here - install it on a renderer,
//! start its per-stage state, tick it, ask it for a picture - so the four
//! places that used to spell the Fury case out (the live menu, the headless
//! capture, the session and the stage) name the slot and not its contents.

use std::sync::Arc;

use oag_tables::fexml;
use oag_ui::menu::Picture;
use oag_ui::scene_backdrop::Widget;

use super::fury::{self, FuryAssets};
use super::scene::{self, SceneAssets};
use crate::render::Renderer;

/// The backdrop a source's style draws.
#[derive(Debug)]
pub enum MenuBackdrop {
    /// The Fury style's point clouds.
    Fury(Arc<FuryAssets>),
    /// The HD style's filtered 3D scene.
    Scene(Box<SceneAssets>),
}

/// Reads whichever backdrop this source's style draws, or says why none.
///
/// `scene_enabled` is [`oag_title::BootProfile::menu_scene`]: the widget is in
/// every HD-lineage skin and only some titles draw it.
pub(super) fn load(
    archives: &mut oag_assets::Archives,
    (fury_style, scene_enabled): (bool, bool),
    skin_xml: Option<&str>,
    report: &mut Vec<String>,
) -> Option<Arc<MenuBackdrop>> {
    if fury_style {
        return fury::load(archives, true, skin_xml, report)
            .map(|assets| Arc::new(MenuBackdrop::Fury(assets)));
    }
    if !scene_enabled {
        return None;
    }
    let Some(widget) = skin_xml.and_then(|xml| Widget::read(&fexml::parse(xml))) else {
        report.push("menu backdrop: the skin authors no BackgroundAnim widget".to_string());
        return None;
    };
    scene::load(archives, widget, report)
        .map(|assets| Arc::new(MenuBackdrop::Scene(Box::new(assets))))
}

impl MenuBackdrop {
    /// Puts the backdrop's GPU side on `renderer`, once per menu stage, the
    /// way the sprite sheet is.
    pub fn install(&self, renderer: &mut Renderer, device: &wgpu::Device, queue: &wgpu::Queue) {
        match self {
            Self::Fury(assets) => renderer.set_fury_backdrop(
                device,
                queue,
                &assets.clouds,
                (assets.bloom, assets.model_bloom),
            ),
            Self::Scene(assets) => renderer.set_scene_backdrop(device, queue, assets.model.clone()),
        }
    }

    /// The per-stage state: the model that moves, fresh from its first frame.
    #[must_use]
    pub fn live(self: &Arc<Self>) -> Option<Live> {
        match &**self {
            Self::Fury(assets) => {
                oag_ui::backdrop::Fury::new(assets.settings.clone(), assets.first, fury::SEED).map(
                    |model| Live::Fury {
                        model: Box::new(model),
                        tints: assets.tints.clone(),
                    },
                )
            }
            Self::Scene(assets) => Some(Live::Scene {
                model: oag_ui::scene_backdrop::Scene::new(assets.widget.clone()),
                assets: Arc::clone(self),
            }),
        }
    }

    /// The picture of a captured page: the model run `anim_seconds` in, with
    /// the page's look fully eased in, sized to `viewport`. The Fury trail and
    /// glow are [`Self::settle`]'s: this is the one frame after them.
    #[must_use]
    pub fn still(
        &self,
        page: &str,
        viewport: (f32, f32, f32, f32),
        anim_seconds: Option<f32>,
        fury_path: Option<usize>,
        fury_tint: Option<f32>,
    ) -> Option<Picture> {
        let (_, _, w, h) = viewport;
        match self {
            Self::Fury(assets) => {
                let (mut model, frames) = assets.at_start(anim_seconds, fury_path)?;
                for _ in 0..frames {
                    model.tick(&oag_ui::backdrop::SILENCE);
                }
                Some(Picture::from(model.frame(
                    h,
                    w / h,
                    captured_tint(&assets.tints, page, fury_tint),
                )))
            }
            Self::Scene(assets) => {
                let scene = oag_ui::scene_backdrop::Scene::new(assets.widget.clone());
                let seconds = anim_seconds.unwrap_or(0.0);
                let camera = assets.view_projection(
                    oag_ui::scene_backdrop::loop_position(seconds, assets.widget.anim_length),
                    w / h,
                );
                Some(Picture::from(scene.settled(
                    seconds,
                    page == "main",
                    camera,
                )))
            }
        }
    }
}

impl MenuBackdrop {
    /// Draws the frames leading up to [`Self::still`]'s, a still of a backdrop
    /// that feeds back having no other way to come by its history. The scene
    /// has none. `layer` is the page's own models, drawn into each of those frames
    /// (`Renderer::settle_fury_backdrop`).
    pub fn settle(
        &self,
        renderer: &mut Renderer,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        page: &str,
        viewport: (f32, f32, f32, f32),
        (anim_seconds, fury_path, fury_tint): (Option<f32>, Option<usize>, Option<f32>),
        layer: Option<crate::render::DrawSceneLayer<'_>>,
    ) {
        let Self::Fury(assets) = self else {
            return;
        };
        let Some((mut model, frames)) = assets.at_start(anim_seconds, fury_path) else {
            return;
        };
        let (_, _, w, h) = viewport;
        let tint = captured_tint(&assets.tints, page, fury_tint);
        let history = frames.min(oag_ui::backdrop::FRAMES_TO_SETTLE);
        for _ in 0..frames - history {
            model.tick(&oag_ui::backdrop::SILENCE);
        }
        let leading_up = (0..history).map(|_| {
            let frame = model.frame(h, w / h, tint);
            model.tick(&oag_ui::backdrop::SILENCE);
            frame
        });
        renderer.settle_fury_backdrop(device, queue, leading_up, (w as u32, h as u32), layer);
    }
}

/// The tint a captured page's cloud is drawn under: the skin's row for the
/// page, or the one value a capture states for red, green and blue
/// (`--fury-tint`). The row's alpha stays, as a write of three floats over the
/// original's row leaves its fourth.
fn captured_tint(tints: &oag_ui::backdrop::Tints, page: &str, stated: Option<f32>) -> [f32; 4] {
    let authored = tints.for_root(page == "main");
    stated.map_or(authored, |value| [value, value, value, authored[3]])
}

/// A stage's running backdrop.
#[derive(Debug)]
pub enum Live {
    /// The Fury point-cloud model and the skin's tints.
    Fury {
        model: Box<oag_ui::backdrop::Fury>,
        tints: oag_ui::backdrop::Tints,
    },
    /// The scene's clock and eased look.
    Scene {
        model: oag_ui::scene_backdrop::Scene,
        assets: Arc<MenuBackdrop>,
    },
}

impl Live {
    /// One frame on, off the same fixed step the movie is: the original counts
    /// its clip up once per rendered frame at sixty. `root` is whether the
    /// page showing is the menus' root, the disc's `Main Menu`. `bands` is the
    /// sound system's equaliser this frame, which only the Fury widget reads.
    pub fn tick(&mut self, root: bool, bands: &[f32; oag_ui::backdrop::BANDS]) {
        match self {
            Self::Fury { model, .. } => model.tick(bands),
            Self::Scene { model, .. } => model.tick(root),
        }
    }

    /// This frame's picture, sized to `viewport` (the Fury sprite size and
    /// colours scale with the picture's line count) for the page showing.
    #[must_use]
    pub fn picture(&self, viewport: (f32, f32, f32, f32), root: bool) -> Picture {
        let (_, _, w, h) = viewport;
        match self {
            Self::Fury { model, tints } => {
                Picture::from(model.frame(h, w / h, tints.for_root(root)))
            }
            Self::Scene { model, assets } => {
                let MenuBackdrop::Scene(assets) = &**assets else {
                    unreachable!("a scene's state is built from a scene's assets");
                };
                let camera = assets.view_projection(model.seconds(), w / h);
                Picture::from(model.frame(camera))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::captured_tint;

    fn tints() -> oag_ui::backdrop::Tints {
        let skin = r#"<Skin><BackgroundAnimFury name="bgAnimFury">
            <ScreenSetting name="default" tint="0xFF202020" equaliser="0"/>
            <ScreenSetting name="Main Menu" tint="0xFFFFFFFF" equaliser="0"/>
        </BackgroundAnimFury></Skin>"#;
        oag_ui::backdrop::Tints::read(&oag_tables::fexml::parse(skin)).expect("the widget")
    }

    /// A capture draws a page under the skin's row unless it states a tint, and
    /// a stated `0` is the hidden cloud of a compare shot on every page.
    #[test]
    fn a_stated_tint_replaces_the_rows_colour_and_keeps_its_alpha() {
        let grey = 0x20 as f32 / 255.0;
        assert_eq!(
            captured_tint(&tints(), "track-select", None),
            [grey, grey, grey, 1.0]
        );
        assert_eq!(captured_tint(&tints(), "main", None), [1.0; 4]);
        for page in ["track-select", "ship-select", "main"] {
            assert_eq!(
                captured_tint(&tints(), page, Some(0.0)),
                [0.0, 0.0, 0.0, 1.0]
            );
        }
        assert_eq!(
            captured_tint(&tints(), "ship-select", Some(0.5)),
            [0.5, 0.5, 0.5, 1.0]
        );
    }
}
