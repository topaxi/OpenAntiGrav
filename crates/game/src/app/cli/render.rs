//! Render settings for this run, over the saved profile.

use oag_mesh::mesh_render::Anisotropy;

use super::extra;

/// Render settings for this run, over the saved profile.
#[derive(clap::Args, Debug)]
pub(crate) struct RenderArgs {
    /// Anisotropic filtering level for track and ship textures: off, 2x, 4x,
    /// 8x or 16x.
    ///
    /// Overrides `[graphics] anisotropy` in the settings file
    /// (`settings::path`) for this run only; the file on disk is not changed.
    #[arg(long)]
    pub(crate) anisotropy: Option<Anisotropy>,

    /// How far out an authored `LodGroup` switches to its coarser tiers:
    /// `original`, `high` or `maximum`, overriding
    /// `[render_profiles.<title>] model_detail` for this run only (the file
    /// on disk is not changed). See `oag_mesh::mesh::ModelDetail`.
    #[arg(long)]
    pub(crate) lod: Option<oag_mesh::mesh::ModelDetail>,

    /// How far out a PSP `.vex` model keeps its finer texture levels:
    /// `original`, `high` or `maximum`, overriding
    /// `[render_profiles.<title>] texture_detail` for this run only (the file
    /// on disk is not changed). See `oag_mesh::mesh_render::TextureDetail`.
    #[arg(long)]
    pub(crate) texture_detail: Option<oag_mesh::mesh_render::TextureDetail>,

    /// Where the PSP engine ribbons draw against the circuit's translucent
    /// surfaces: `original` or `over-floors`, overriding
    /// `[render_profiles.<title>] trail_order` for this run only. See
    /// `oag_display::display::TrailOrder`.
    #[arg(long)]
    pub(crate) trail_order: Option<oag_display::display::TrailOrder>,

    /// How the circuit's blended list (trees, translucent floor patches, glass) is sorted:
    /// `original` or `depth`, overriding `[render_profiles.<title>] scenery_sort` for this
    /// run only. See `oag_display::display::ScenerySort`.
    #[arg(long)]
    pub(crate) scenery_sort: Option<oag_display::display::ScenerySort>,

    /// How long the PSP engine ribbons are: `original` or `hd`, overriding
    /// `[render_profiles.<title>] trail_length` for this run only. See
    /// `oag_display::display::TrailLength`.
    #[arg(long)]
    pub(crate) trail_length: Option<oag_display::display::TrailLength>,

    /// What a played weather adds: `original` or `enhanced`, overriding
    /// `[render_profiles.<title>] weather` for this run only. See
    /// `oag_display::display::Weather`.
    #[arg(long)]
    pub(crate) weather: Option<oag_display::display::Weather>,

    /// Whether the track's authored visibility set culls this run: `true` or
    /// `false`.
    ///
    /// Overrides `[graphics] pvs_culling` in the settings file
    /// (`settings::path`) for this run only; the file on disk is not changed.
    /// Here for the same reason `--lod` is, and for one more: **when geometry
    /// goes missing, the first question is whether the PVS took it**, and two
    /// captures differing only by this flag answer it without editing a
    /// settings file between them - which is how a comparison ends up
    /// differing by something else as well.
    #[arg(long)]
    pub(crate) pvs: Option<bool>,

    /// What resolves the frame onto the surface: off, fxaa, smaa, fsr1 or
    /// fsr3.
    ///
    /// Overrides `[render_profiles.<title>] reconstruction` for *every* title,
    /// for this run only; the file on disk is not changed. Every title rather
    /// than one, because no title is open yet this early to single one out -
    /// see `render_scale` in `app.rs`. Here for the same reason
    /// `--anisotropy` is, and for one more: two `--presented` captures
    /// differing only by this flag are how these get compared, and asking
    /// somebody to edit a settings file between them is how a comparison ends
    /// up differing by something else as well.
    #[arg(long)]
    pub(crate) reconstruction: Option<oag_display::display::Reconstruction>,

    /// What percentage of the displayed size the game is rendered at, 25 to
    /// 200.
    ///
    /// Overrides `[render_profiles.<title>] render_scale` for *every* title,
    /// for this run only - see `--upscaler`'s own doc for why every title
    /// rather than one. The companion to `--upscaler`: a resampler can only be
    /// judged at a scale where it has something to resample, and the two
    /// flags together are what let one command produce one image of a
    /// comparison.
    #[arg(long)]
    pub(crate) render_scale: Option<u32>,

    /// Offset the camera by a sub-pixel each frame, from a 16-phase Halton
    /// sequence.
    ///
    /// **This makes the picture worse, and is here anyway.** Camera jitter is
    /// the last renderer-side thing FSR 3.1 needs
    /// (`docs/overview/modern-features.md`), and nothing consumes it yet - with
    /// a spatial resolve or none, all it does is move every edge a fraction of
    /// a pixel a frame, which reads as a shimmer with no reconstruction behind
    /// it. It is built ahead of its consumer because the constraint it has to
    /// satisfy - not perturbing the culling frustum or the velocity buffer - is
    /// a property of *where* the offset is applied, and that is far cheaper to
    /// get right now than to retrofit under a temporal upscaler that is already
    /// ghosting.
    ///
    /// A flag rather than a `[graphics]` key deliberately: a settings key
    /// implies a choice a player should be making, and there is no version of
    /// this that a player wants until something reconstructs from it. The real
    /// gate, when it exists, is "is a temporal upscaler selected".
    ///
    /// Two `--screenshot` runs differing only by this differ by well under a
    /// pixel - invisible in a window, obvious in a byte diff, which is how to
    /// check it is live. Only the race's 3D scene moves: the HUD, the reticle
    /// and the menus are all drawn at presentation resolution and are not
    /// jittered. See `oag_post::jitter`.
    #[arg(long)]
    pub(crate) camera_jitter: bool,

    /// Draw every hull at its body's pose, without the title's drawn-hull shake
    /// (`oag_raceplay`'s `hull_shake`, Wipeout HD's `HullShake_Update`). For a still
    /// comparison against an original with its shake patched out
    /// (`scripts/rpcs3-hd-no-hull-shake.py`): a random shake cannot match frame for frame.
    #[arg(long)]
    pub(crate) no_hull_shake: bool,

    /// With `--race --screenshot`, draw the captured frame this far between
    /// the last two ticks, `0.0..=1.0`, as a window does between frames.
    /// `1.0`, the default, is the tick itself. A verification aid: a few
    /// captures at the same `--ticks` and rising alphas are the frames a
    /// 240 Hz window draws inside one tick. Refused without `--race`: the
    /// front end's own race capture draws each tick exactly and would drop it.
    /// See `oag_raceplay::between_ticks`.
    #[arg(long, default_value_t = 1.0, requires_all = ["screenshot", "race"])]
    pub(crate) draw_alpha: f32,

    /// How many samples the rasterizer takes: off or 4x.
    ///
    /// Overrides `[render_profiles.<title>] msaa` for *every* title, for this
    /// run only; the file on disk is not changed - see `--reconstruction`'s own
    /// doc for why every title rather than one. Here for the same reason
    /// `--reconstruction` is: two `--presented` captures differing only by this
    /// flag are how the levels get compared.
    #[arg(long)]
    pub(crate) msaa: Option<oag_display::display::Msaa>,

    /// `--motion-blur` and `--motion-blur-resolution` - see [`extra::BlurArgs`].
    #[command(flatten)]
    pub(crate) blur: extra::BlurArgs,

    /// What casts a shadow: off or blob.
    ///
    /// Overrides `[render_profiles.<title>] shadows` for *every* title, for
    /// this run only; the file on disk is not changed - see
    /// `--reconstruction`'s own doc for why every title rather than one. Here
    /// for the reason `--motion-blur` is, and for one more: the tier has no
    /// menu row yet, so until it has one this flag is the only way to select
    /// it, and two captures differing only by it are how it gets compared
    /// against itself off.
    #[arg(long)]
    pub(crate) shadows: Option<oag_display::display::Shadows>,

    /// How many texels the shadow maps get: original, high or ultra.
    ///
    /// Overrides `[render_profiles.<title>] shadow_resolution` for every title,
    /// for this run only. See docs/rendering/shadows.md, "Shadow resolution".
    #[arg(long)]
    pub(crate) shadow_resolution: Option<oag_display::display::ShadowResolution>,

    /// What Wipeout HD's scene target keeps of each pixel: original (the
    /// original's 8-bit sRGB target) or float.
    ///
    /// Overrides `[render_profiles.<title>] scene_precision` for every title,
    /// for this run only. See docs/rendering/hd-track-tone.md, "The scene target
    /// is 8-bit by default".
    #[arg(long)]
    pub(crate) scene_precision: Option<oag_display::display::ScenePrecision>,

    /// How much of the authored rig a circuit's plain surfaces compute: original (the
    /// original's shading) or simplified (no specular, normal map or SPU vertex lights).
    ///
    /// Overrides `[render_profiles.<title>] circuit_lighting` for every title, for this run
    /// only. See docs/rendering/hires-gpu-levers.md, "The circuit lighting rung".
    #[arg(long)]
    pub(crate) circuit_lighting: Option<oag_display::display::CircuitLighting>,

    /// How many texels the screen-space glass and sea target gets: original (the original's
    /// fixed size), half or quarter (that fraction of it on each axis, whatever the render
    /// scale).
    ///
    /// Overrides `[render_profiles.<title>] reflection_resolution` for every title, for this
    /// run only. See docs/rendering/water.md, "The reflection target's resolution".
    #[arg(long)]
    pub(crate) reflection_resolution: Option<oag_display::display::ReflectionResolution>,

    /// Which screen filter draws the finished frame: `off`, a built-in such
    /// as `psp-3000` or `crt-interlaced`, or the stem of a `.wgsl` file in
    /// `<config dir>/oag/shaders/`.
    ///
    /// Overrides `[render_profiles.<title>] screen_filter` for *every* title,
    /// for this run only, the way `--shadows` does. **Only visible under
    /// `--presented`** in a capture, for the reason the grade is: an ordinary
    /// `--screenshot` is the scene as drawn. See
    /// `docs/rendering/screen-filters.md`.
    #[arg(long, value_name = "ID")]
    pub(crate) screen_filter: Option<String>,
}
