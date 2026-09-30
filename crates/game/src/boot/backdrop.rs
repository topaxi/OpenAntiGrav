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
            Self::Fury(assets) => renderer.set_fury_backdrop(device, queue, &assets.clouds),
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
    /// the page's look fully eased in, sized to `viewport`. One frame, so the
    /// Fury trail the live picture accumulates is not here.
    #[must_use]
    pub fn still(
        &self,
        page: &str,
        viewport: (f32, f32, f32, f32),
        anim_seconds: Option<f32>,
        fury_path: Option<usize>,
    ) -> Option<Picture> {
        let (_, _, w, h) = viewport;
        match self {
            Self::Fury(assets) => {
                let mut model =
                    oag_ui::backdrop::Fury::new(assets.settings.clone(), assets.first, fury::SEED)?;
                if let Some(path) = fury_path {
                    model.force_path(path)?;
                }
                let frames = (anim_seconds.unwrap_or(0.0).max(0.0)
                    * oag_ui::backdrop::FRAMES_PER_SECOND) as u32;
                for _ in 0..frames {
                    model.tick();
                }
                Some(Picture::from(model.frame(
                    h,
                    w / h,
                    assets.tints.for_root(page == "main"),
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
    /// page showing is the menus' root, the disc's `Main Menu`.
    pub fn tick(&mut self, root: bool) {
        match self {
            Self::Fury { model, .. } => model.tick(),
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
