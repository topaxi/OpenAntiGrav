//! Which of a model's blended pipelines its draws can pick.
//!
//! `build` made every blended pipeline set whole: a two-sided and a culled twin of the
//! alpha-over, additive and unblended pipelines, of every authored blend state and of the
//! glow stamp, whatever the model's draws name. Each is a naga compile, and a hull, a plume or
//! a weapon halo names one class and one cull mode (`docs/architecture/load-time.md`, "Fourth
//! round"). A draw reaches a pipeline only through [`super::TransparentPipelines::select`],
//! [`super::Stamp::select`] or, for the overlay and shine passes, the draw's own `culled`
//! index into the alpha-over or additive pair; the model is immutable once built, so the
//! twins and classes its draws name are known here.

use crate::mesh::DrawCall;
use oag_vex::vex::BlendClass;

/// Which twin of a pipeline pair some draw picks: `[0]` two-sided, `[1]` culled.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Twins([bool; 2]);

impl Twins {
    pub(super) fn note(&mut self, draw: &DrawCall) {
        self.0[usize::from(draw.culled)] = true;
    }

    /// The twins the draws of `draws` pick.
    pub(super) fn of<'a>(draws: impl IntoIterator<Item = &'a DrawCall>) -> Self {
        let mut twins = Self::default();
        for draw in draws {
            twins.note(draw);
        }
        twins
    }

    pub(super) fn union(self, other: Self) -> Self {
        Self([self.0[0] || other.0[0], self.0[1] || other.0[1]])
    }

    /// The pair: each twin a draw picks is built by `make(culled)`. A twin no draw picks is the
    /// other twin's pipeline, and a pair no draw picks is `fallback()`'s, so the two slots of a
    /// pair always hold a pipeline of the same layout and targets.
    pub(super) fn pair(
        self,
        make: impl Fn(bool) -> wgpu::RenderPipeline,
        fallback: impl FnOnce() -> wgpu::RenderPipeline,
    ) -> [wgpu::RenderPipeline; 2] {
        match self.0 {
            [true, true] => [make(false), make(true)],
            [true, false] => {
                let pipeline = make(false);
                [pipeline.clone(), pipeline]
            }
            [false, true] => {
                let pipeline = make(true);
                [pipeline.clone(), pipeline]
            }
            [false, false] => {
                let pipeline = fallback();
                [pipeline.clone(), pipeline]
            }
        }
    }
}

/// The pipelines of one transparent list that its draws pick, by
/// [`super::TransparentPipelines::select`]'s own rule.
#[derive(Debug, Clone, Default)]
pub(super) struct Blended {
    pub(super) alpha_over: Twins,
    pub(super) additive: Twins,
    pub(super) unblended: Twins,
    pub(super) authored: Vec<((wgpu::BlendState, bool), Twins)>,
}

impl Blended {
    /// What the draws `draws` select. `authored` is every key the model's transparent list
    /// names, so a key none of `draws` picks keeps empty twins.
    pub(super) fn of<'a>(
        draws: impl IntoIterator<Item = &'a DrawCall>,
        authored: &[(wgpu::BlendState, bool)],
    ) -> Self {
        let mut reach = Self {
            authored: authored
                .iter()
                .map(|key| (*key, Twins::default()))
                .collect(),
            ..Self::default()
        };
        for draw in draws {
            let twins = match draw.blend_state {
                Some(state) => reach
                    .authored
                    .iter_mut()
                    .find(|(seen, _)| *seen == (state, draw.writes_depth))
                    .map_or(&mut reach.alpha_over, |(_, twins)| twins),
                None => match draw.blend {
                    Some(BlendClass::Additive) => &mut reach.additive,
                    Some(BlendClass::None) => &mut reach.unblended,
                    Some(BlendClass::AlphaOver) | None => &mut reach.alpha_over,
                },
            };
            twins.note(draw);
        }
        reach
    }

    pub(super) fn authored(&self, key: (wgpu::BlendState, bool)) -> Twins {
        self.authored
            .iter()
            .find(|(seen, _)| *seen == key)
            .map_or_else(Twins::default, |(_, twins)| *twins)
    }
}

/// The glow stamp pairs the transparent list picks, by [`super::Stamp::select`]'s rule.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Stamped {
    pub(super) alpha_over: Twins,
    pub(super) additive: Twins,
}

impl Stamped {
    pub(super) fn of<'a>(draws: impl IntoIterator<Item = &'a DrawCall>) -> Self {
        let mut reach = Self::default();
        for draw in draws {
            match draw.blend {
                Some(BlendClass::Additive) => reach.additive.note(draw),
                _ => reach.alpha_over.note(draw),
            }
        }
        reach
    }
}
