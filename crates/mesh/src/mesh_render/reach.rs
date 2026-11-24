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

use super::blend::Pick;
use crate::mesh::DrawCall;

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
            let twins = match Pick::of(draw) {
                Pick::Authored(key) => reach
                    .authored
                    .iter_mut()
                    .find(|(seen, _)| *seen == key)
                    .map_or(&mut reach.alpha_over, |(_, twins)| twins),
                Pick::Additive => &mut reach.additive,
                Pick::Unblended => &mut reach.unblended,
                Pick::AlphaOver => &mut reach.alpha_over,
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
            if super::Stamp::is_additive(draw) {
                reach.additive.note(draw);
            } else {
                reach.alpha_over.note(draw);
            }
        }
        reach
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh_render::pipeline_cache::tests::{build_once, device, triangle_model};
    use oag_vex::vex::BlendClass;

    fn draw(blend: Option<BlendClass>, culled: bool) -> DrawCall {
        DrawCall {
            blend,
            culled,
            ..triangle_model().draws.remove(0)
        }
    }

    #[test]
    fn twins_are_the_cull_modes_the_draws_name() {
        assert_eq!(Twins::of([]), Twins::default());
        assert_eq!(Twins::of([&draw(None, false)]).0, [true, false]);
        assert_eq!(Twins::of([&draw(None, true)]).0, [false, true]);
        let both = [draw(None, true), draw(None, false)];
        assert_eq!(Twins::of(&both).0, [true, true]);
        assert_eq!(
            Twins::of([&draw(None, true)]).union(Twins::of([&draw(None, false)])),
            Twins::of(&both)
        );
    }

    #[test]
    fn a_transparent_list_reaches_the_sets_its_draws_select() {
        let state = wgpu::BlendState::ALPHA_BLENDING;
        let authored = DrawCall {
            blend_state: Some(state),
            writes_depth: true,
            ..draw(Some(BlendClass::Additive), true)
        };
        let draws = [
            draw(Some(BlendClass::Additive), false),
            draw(Some(BlendClass::None), true),
            draw(None, false),
            authored,
        ];
        let reach = Blended::of(&draws, &[(state, true), (state, false)]);
        assert_eq!(reach.additive.0, [true, false]);
        assert_eq!(reach.unblended.0, [false, true]);
        assert_eq!(reach.alpha_over.0, [true, false]);
        assert_eq!(reach.authored((state, true)).0, [false, true]);
        assert_eq!(reach.authored((state, false)), Twins::default());
        let stamped = Stamped::of(&draws);
        assert_eq!(stamped.additive.0, [true, true]);
        assert_eq!(stamped.alpha_over.0, [true, true]);
    }

    #[test]
    fn a_model_with_no_transparent_draw_reaches_nothing() {
        let reach = Blended::of([], &[]);
        assert_eq!(reach.alpha_over, Twins::default());
        assert_eq!(reach.additive, Twins::default());
        assert_eq!(reach.unblended, Twins::default());
    }

    /// Every draw of a built model is submitted through a pipeline of its own class and
    /// cull mode: no draw lands on the stand-in a pair no draw picks holds.
    #[test]
    fn every_transparent_draw_selects_a_pipeline_built_for_it() {
        let Some((device, queue)) = device() else {
            eprintln!("no GPU adapter: skipping");
            return;
        };
        let state = wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING;
        let mut model = triangle_model();
        model.transparent_draws = vec![
            draw(Some(BlendClass::AlphaOver), false),
            draw(Some(BlendClass::Additive), true),
            draw(Some(BlendClass::None), false),
            draw(Some(BlendClass::None), true),
            DrawCall {
                blend_state: Some(state),
                ..draw(None, true)
            },
        ];
        let scope = crate::mesh_render::BuildCacheScope::open();
        let built = build_once(&device, &queue, &model);
        drop(scope);
        let set = built.transparent_set();
        let picked: Vec<&wgpu::RenderPipeline> = model
            .transparent_draws
            .iter()
            .map(|draw| set.select(draw))
            .collect();
        for (at, one) in picked.iter().enumerate() {
            for other in &picked[at + 1..] {
                assert_ne!(one, other, "two classes or twins share one pipeline");
            }
        }
    }
}
