//! Sharing `build`'s one WGSL module and its render pipelines across a race
//! scene's many drawables.
//!
//! `mesh_render::build` used to call `device.create_shader_module` and then
//! `device.create_render_pipeline` several times, once per drawable - once
//! per craft, per plume, per shield, per weapon model, some seventy calls in
//! one race scene. Naga reparses, revalidates and re-lowers the same
//! `mesh.wesl` text every time, and re-specialises an override-constant
//! pipeline from scratch, even between two drawables asking for the exact
//! same pipeline - eight identical hulls built eight identical pipelines.
//! That is the bulk of the 7.2-8.1 s `race::Scene::new` measures on a
//! desktop - see `docs/architecture/race-load-transition.md`.
//!
//! [`Scope`] opens a cache for the current thread around a whole scene's
//! build; [`shared_shader_module`] and [`cached_pipeline`] use it when one is
//! open and fall back to `build`'s old per-call behaviour otherwise, so a
//! caller that builds one model at a time - the asset viewer, a ground-truth
//! test - is unaffected: it still gets its own module and its own pipelines,
//! and pays nothing beyond the one `thread_local` read to check.

use std::cell::RefCell;
use std::collections::HashMap;

/// Everything that distinguishes one render pipeline `build` creates from
/// another, **except the shader module and the bind group layouts it was
/// built against**. Those two belong out of the key on purpose: wgpu
/// deduplicates a `BindGroupLayout` - and the `PipelineLayout` built from it -
/// by its descriptor's own content, the fact
/// [`super::material_bind_group_layout`]'s own doc comment already relies on
/// for the sun occlusion pass. Every `build()` call constructs `layout`,
/// `texture_layout`, `fog_layout` and `anim_layout` from the same fixed
/// descriptors regardless of the model it was called for, so a pipeline
/// cached from one call's layouts binds exactly as validly against a later
/// call's bind groups as one built fresh for it would have.
#[derive(PartialEq, Eq, Hash)]
struct Key {
    vertex_entry: &'static str,
    /// The vertex buffers' layouts - stride, step and attributes. Not
    /// optional: two pipelines differing only by where `texcoord` comes from
    /// (`super::Texcoords`) are otherwise one key, and the second caller got
    /// the first's pipeline reading a buffer it never bound.
    vertex_buffers: Vec<Option<(u64, wgpu::VertexStepMode, Vec<wgpu::VertexAttribute>)>>,
    fragment_entry: &'static str,
    targets: Vec<Option<wgpu::ColorTargetState>>,
    primitive: wgpu::PrimitiveState,
    depth_stencil: Option<wgpu::DepthStencilState>,
    multisample: wgpu::MultisampleState,
    /// `(name, value.to_bits())`: an override constant is an `f64`, which has
    /// no `Hash`, so this key carries its bits instead. Every value this
    /// crate feeds `PipelineCompilationOptions` is a plain literal read off a
    /// model, a light or a flame parameter - never accumulated - so two calls
    /// that agree on a name agree on its bits too.
    constants: Vec<(&'static str, u64)>,
}

impl Key {
    fn constants_of(constants: &[(&'static str, f64)]) -> Vec<(&'static str, u64)> {
        constants
            .iter()
            .map(|(name, value)| (*name, value.to_bits()))
            .collect()
    }
}

#[derive(Default)]
struct Cache {
    shader: Option<wgpu::ShaderModule>,
    pipelines: HashMap<Key, wgpu::RenderPipeline>,
    /// One upload per distinct decoded texture across the whole scene, keyed
    /// by the `Arc`'s address. The `Weak` is what makes the address a safe
    /// key: it keeps the allocation reserved after every strong owner lets go
    /// (`Model::release_texels` drops them one drawable at a time), so no
    /// other texture can be handed the same address while the entry lives.
    textures: HashMap<
        usize,
        (
            std::sync::Weak<crate::mesh::ModelTexture>,
            wgpu::TextureView,
        ),
    >,
    /// Pipeline constants every `build()` in the scope appends - see
    /// [`Scope::lit_by`].
    constants: Vec<(&'static str, f64)>,
    texture_calls: u32,
    texture_hits: u32,
    shader_calls: u32,
    shader_hits: u32,
    pipeline_calls: u32,
    pipeline_hits: u32,
}

thread_local! {
    static CACHE: RefCell<Option<Cache>> = const { RefCell::new(None) };
}

/// Opens the cache for the current thread until dropped.
///
/// **Every `build()` call this scope wraps must share one `device`** -
/// nothing here checks, because the one caller already holds that:
/// `race::Scene::new` opens the scope itself and builds every drawable from
/// its own `device`, so the windowed launch, the race-build worker and the
/// `--screenshot` capture all get it without asking. It used to be opened by
/// whoever called `Scene::new`, and the capture path never did: 9,859 pipelines
/// compiled for 53 distinct ones, about 7 MiB resident per drawable.
///
/// **Scopes do not nest.** The cache is one thread-local slot, so opening a
/// second scope replaces the first one's cache and dropping it empties the
/// slot, after which the outer scope's counters panic. Open it in one place.
#[derive(Debug)]
pub struct Scope {
    _private: (),
}

impl Scope {
    #[must_use]
    pub fn open() -> Self {
        CACHE.with(|cell| *cell.borrow_mut() = Some(Cache::default()));
        Self { _private: () }
    }

    /// Every pipeline built in this scope draws under `light`'s title rig, so
    /// the paths only another title's rig takes compile out of it.
    ///
    /// Only Omega's nova prelit curve so far: a light that is not nova is a
    /// title that never is, since nothing mid-race turns it on - a Zone grade
    /// rebuilds the light from this one and carries its flag, never sets it.
    /// A drawable built outside the scope keeps `mesh.wesl`'s live default and
    /// shades the same, only without the saving. Call it before the first
    /// `build()`: a pipeline built earlier is cached without the constant.
    pub fn lit_by(&self, light: &super::Light) {
        if light.nova <= 0.5 {
            CACHE.with(|cell| {
                if let Some(cache) = cell.borrow_mut().as_mut() {
                    cache.constants.push(("nova_prelit", 0.0));
                }
            });
        }
    }

    /// `(shader module: calls, reused)` over this scope's life so far.
    #[must_use]
    pub fn shader_counts(&self) -> (u32, u32) {
        with_open_cache(|cache| (cache.shader_calls, cache.shader_hits))
    }

    /// `(texture uploads: calls, reused)` over this scope's life so far.
    #[must_use]
    pub fn texture_counts(&self) -> (u32, u32) {
        with_open_cache(|cache| (cache.texture_calls, cache.texture_hits))
    }

    /// `(render pipeline: calls, reused, distinct pipelines built)` over this
    /// scope's life so far.
    #[must_use]
    pub fn pipeline_counts(&self) -> (u32, u32, usize) {
        with_open_cache(|cache| {
            (
                cache.pipeline_calls,
                cache.pipeline_hits,
                cache.pipelines.len(),
            )
        })
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        CACHE.with(|cell| *cell.borrow_mut() = None);
    }
}

fn with_open_cache<R>(read: impl FnOnce(&Cache) -> R) -> R {
    CACHE.with(|cell| {
        let cache = cell.borrow();
        read(
            cache
                .as_ref()
                .expect("Scope is open for the life of `self`"),
        )
    })
}

/// The constants [`Scope::lit_by`] set, for `build` to append; none with no
/// scope open.
pub(crate) fn scope_constants() -> Vec<(&'static str, f64)> {
    CACHE.with(|cell| {
        cell.borrow()
            .as_ref()
            .map_or_else(Vec::new, |cache| cache.constants.clone())
    })
}

/// `build`'s one shader module: shared for the life of an open [`Scope`],
/// freshly created and uncached with none open.
pub(crate) fn shared_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    CACHE.with(|cell| match cell.borrow_mut().as_mut() {
        None => create_shader_module(device),
        Some(cache) => {
            cache.shader_calls += 1;
            if let Some(shader) = &cache.shader {
                cache.shader_hits += 1;
                return shader.clone();
            }
            let shader = create_shader_module(device);
            cache.shader = Some(shader.clone());
            shader
        }
    })
}

fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("mesh"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!(concat!(env!("OUT_DIR"), "/mesh.wgsl")).into(),
        ),
    })
}

/// One of `build`'s render pipelines: shared for the life of an open
/// [`Scope`] with every earlier call whose descriptor-affecting state was the
/// same, freshly created and uncached with none open. `create` runs only on a
/// cache miss.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cached_pipeline(
    vertex_entry: &'static str,
    vertex_buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
    fragment_entry: &'static str,
    targets: &[Option<wgpu::ColorTargetState>],
    primitive: wgpu::PrimitiveState,
    depth_stencil: Option<wgpu::DepthStencilState>,
    multisample: wgpu::MultisampleState,
    constants: &[(&'static str, f64)],
    create: impl FnOnce() -> wgpu::RenderPipeline,
) -> wgpu::RenderPipeline {
    CACHE.with(|cell| match cell.borrow_mut().as_mut() {
        None => create(),
        Some(cache) => {
            let key = Key {
                vertex_entry,
                vertex_buffers: vertex_buffers
                    .iter()
                    .map(|layout| {
                        layout
                            .as_ref()
                            .map(|l| (l.array_stride, l.step_mode, l.attributes.to_vec()))
                    })
                    .collect(),
                fragment_entry,
                targets: targets.to_vec(),
                primitive,
                depth_stencil,
                multisample,
                constants: Key::constants_of(constants),
            };
            cache.pipeline_calls += 1;
            if let Some(pipeline) = cache.pipelines.get(&key) {
                cache.pipeline_hits += 1;
                return pipeline.clone();
            }
            let pipeline = create();
            cache.pipelines.insert(key, pipeline.clone());
            pipeline
        }
    })
}

/// One texture's view: shared for the life of an open [`Scope`] with every
/// earlier call that named the same decoded texture, uploaded afresh with none
/// open. `upload` runs only on a miss.
///
/// **This is what stops eight craft of three teams uploading eight copies of
/// three liveries.** `mesh_render::build` already uploads once per texture
/// within a model; the scene builds a model per craft, and a team's craft share
/// their decoded textures by `Arc` but not their GPU copies.
pub(crate) fn cached_texture_view(
    texture: &std::sync::Arc<crate::mesh::ModelTexture>,
    upload: impl FnOnce() -> Option<wgpu::TextureView>,
) -> Option<wgpu::TextureView> {
    CACHE.with(|cell| match cell.borrow_mut().as_mut() {
        None => upload(),
        Some(cache) => {
            let key = std::sync::Arc::as_ptr(texture) as usize;
            cache.texture_calls += 1;
            if let Some((_, view)) = cache.textures.get(&key) {
                cache.texture_hits += 1;
                return Some(view.clone());
            }
            // A texture the device cannot hold is not remembered: asking again
            // is arithmetic, not an upload.
            let view = upload()?;
            cache
                .textures
                .insert(key, (std::sync::Arc::downgrade(texture), view.clone()));
            Some(view)
        }
    })
}

#[cfg(test)]
mod tests;
