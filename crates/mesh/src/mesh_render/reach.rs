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

    /// A model of two triangles: vertices 0 to 2 name no material row (a circuit's plain draws),
    /// 3 to 5 name one, so a prepassed build splits its transparent list into plain and rest.
    fn two_triangles() -> crate::mesh::Model {
        let mut model = triangle_model();
        let plain = model.vertices.clone();
        model
            .vertices
            .extend(plain.into_iter().map(|vertex| crate::mesh::GpuVertex {
                slots: vertex.slots | (1 << crate::mesh::slots::MATERIAL_SHIFT),
                ..vertex
            }));
        model.indices = vec![0, 1, 2, 3, 4, 5];
        model
    }

    fn build_circuit(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: &crate::mesh::Model,
    ) -> crate::mesh_render::Built {
        use crate::mesh_render as mr;
        let scope = mr::BuildCacheScope::open();
        let built = mr::build_with(
            device,
            queue,
            model,
            wgpu::TextureFormat::Rgba8Unorm,
            oag_gpu::formats::SceneStore::Computed,
            mr::Anisotropy::Off,
            1,
            mr::Depth::Scene,
            mr::TRANSPARENT_BLEND,
            mr::GlowMask::Stamped,
            mr::Velocity::Write,
            &mr::zone::StageArt::NONE,
            mr::ShadowMaps::NONE,
            mr::ShadowReceiver::Never,
            mr::Texcoords::Interleaved,
            true,
            false,
            false,
            false,
        )
        .expect("building the mesh pipeline");
        drop(scope);
        built
    }

    fn assert_all_distinct(what: &str, picked: &[&wgpu::RenderPipeline]) {
        for (at, one) in picked.iter().enumerate() {
            for (other_at, other) in picked.iter().enumerate().skip(at + 1) {
                assert_ne!(
                    one, other,
                    "{what}: picks {at} and {other_at} share one pipeline"
                );
            }
        }
    }

    /// Every draw of a built circuit-shaped model (prepass, glow stamps, a plain and a rest half,
    /// both cull modes, every blend class, an authored state, cutouts with and without a
    /// reference) is submitted through a pipeline built for it: no draw lands on a stand-in.
    #[test]
    fn every_draw_selects_a_pipeline_built_for_it() {
        let Some((device, queue)) = device() else {
            eprintln!("no GPU adapter: skipping");
            return;
        };
        let state = wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING;
        let mut model = two_triangles();
        model.per_batch_cull = true;
        let on = |draw: DrawCall, range: std::ops::Range<u32>| DrawCall { range, ..draw };
        model.draws = vec![on(draw(None, false), 0..3), on(draw(None, true), 0..3)];
        model.alpha_tested_draws = vec![
            DrawCall {
                alpha_test_ref: None,
                ..draw(None, false)
            },
            DrawCall {
                alpha_test_ref: Some(0.3),
                ..draw(None, true)
            },
        ];
        model.transparent_draws = Vec::new();
        for range in [0..3, 3..6] {
            for culled in [false, true] {
                for class in [
                    BlendClass::AlphaOver,
                    BlendClass::Additive,
                    BlendClass::None,
                ] {
                    model
                        .transparent_draws
                        .push(on(draw(Some(class), culled), range.clone()));
                }
                model.transparent_draws.push(on(
                    DrawCall {
                        blend_state: Some(state),
                        ..draw(None, culled)
                    },
                    range.clone(),
                ));
            }
        }
        let built = build_circuit(&device, &queue, &model);

        let plain = built
            .plain_blend
            .as_ref()
            .expect("a plain half of the list");
        let set = built.transparent_set();
        let picked: Vec<&wgpu::RenderPipeline> = model
            .transparent_draws
            .iter()
            .enumerate()
            .map(|(at, draw)| {
                if plain.takes(at) {
                    plain.pipelines().select(draw)
                } else {
                    set.select(draw)
                }
            })
            .collect();
        assert_eq!(picked.len(), 16);
        assert_all_distinct("blended draws, plain and rest", &picked);

        let stamp = built.stamp_pipeline.as_ref().expect("a stamping model");
        let stamps: Vec<&wgpu::RenderPipeline> = model
            .transparent_draws
            .iter()
            .filter(|draw| draw.range.start == 0)
            .filter(|draw| draw.blend_state.is_none() && draw.blend != Some(BlendClass::None))
            .map(|draw| stamp.select(draw))
            .collect();
        assert_eq!(stamps.len(), 4);
        assert_all_distinct("glow stamps", &stamps);

        let culled = built
            .culled
            .as_ref()
            .expect("the culling twin of the solid set");
        for (what, opaque, default, by_reference) in [
            (
                "two-sided",
                &built.pipeline,
                &built.alpha_test_pipeline,
                &built.cutout_pipelines,
            ),
            (
                "culled",
                &culled.pipeline,
                &culled.alpha_test_pipeline,
                &culled.cutout_pipelines,
            ),
        ] {
            let cutouts = crate::mesh_render::CutoutPipelines {
                default,
                by_reference,
            };
            let referenced = cutouts.select(&model.alpha_tested_draws[1]);
            let unreferenced = cutouts.select(&model.alpha_tested_draws[0]);
            assert_all_distinct(what, &[opaque, referenced, unreferenced]);
        }
        assert_ne!(
            built.alpha_test_pipeline, culled.alpha_test_pipeline,
            "the culling twin's cutout is its own"
        );
    }

    /// The overlay and shine passes index the alpha-over and additive pairs by any draw's
    /// `culled`, whatever its list: a model with only opaque draws, both cull modes, builds
    /// both twins of both.
    #[test]
    fn an_overlay_draws_every_list_through_a_pipeline_of_its_own_twin() {
        let Some((device, queue)) = device() else {
            eprintln!("no GPU adapter: skipping");
            return;
        };
        let mut model = two_triangles();
        model.draws = vec![draw(None, false), draw(None, true)];
        let built = build_circuit(&device, &queue, &model);
        assert_all_distinct(
            "overlay pairs",
            &[
                &built.blend_pipeline[0],
                &built.blend_pipeline[1],
                &built.additive_pipeline[0],
                &built.additive_pipeline[1],
            ],
        );
        assert!(
            built.stamp_pipeline.is_none(),
            "no transparent draw, nothing to stamp"
        );
    }
}
