//! The loading screen's wave, as Pulse authored it.
//!
//! Every number here was read out of `Loading_DrawWave` (`0x0890a8e4`) or the
//! `.rodata` beside it, and is documented at
//! `docs/ghidra/functions/psp-pulse-usa/loading-screen.md`. A table in the
//! [ADR-0022] sense: the algorithm that consumes them is
//! [`oag_render::loading`] and stays there, because it is a mechanism rather
//! than a fact about a release.
//!
//! **The reference space these are written in does not live here.**
//! `oag_render::loading::REF_WIDTH`/`REF_HEIGHT` are the PSP's 480x272, which is
//! a console fact under [ADR-0004] rather than a title one, and the renderer's
//! own deliberate divergence: it computes in that space and maps to the target
//! framebuffer, where the PS2 port kept the PSP's numbers on a 640-wide screen
//! and drew the wave in the wrong place. Copying 480x272 into a title package
//! would plant a second copy of a leak the split is trying to close.
//!
//! [ADR-0004]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0004-asset-pipeline.md
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// Pulse's loading screen: the wave, and no backdrop.
///
/// **No backdrop is a measurement, not a gap.** The original's screen draws the
/// wave over a cleared frame - `Loading_DrawWave` (`0x0890a8e4`) is the whole of
/// what it puts on screen besides the tips - and the only asset it touches is
/// [`GLOW_STRIP_ENTRY`], which is what makes it drawable before the thing being
/// loaded exists. Nothing on the disc is a full-screen loading still.
///
/// No caption either: the word over the wave is this build's own heading, and
/// no `<Text idstring=...>` on the original's screen has been located to
/// replace it with. See [`oag_title::Loading::caption`].
pub static LOADING: oag_title::Loading = oag_title::Loading {
    wave: Some(oag_title::loading::Wave {
        tips: TIPS_ENTRY,
        glow_strip: GLOW_STRIP_ENTRY,
    }),
    // Pulse has no illustrated features and no caption id: its screen is the
    // wave with a tip over it, and the word above them is this build's own
    // heading. See `oag_title::Loading`.
    features: &[],
    chrome: &[],
    palette: None,
    caption: None,
    deck: None,
    labels: None,
};

/// The disc entry holding the **26** `<PI_LoadingScreen>` tips.
///
/// `Data.wad` entry **1126** on the USA pressing and **1125** on the EU one -
/// which is exactly why it is reached by name rather than by index. The count
/// read 30 and the index was written as if both pressings shared it until
/// 2026-08-09; both are measured now.
pub const TIPS_ENTRY: &str = r"Data\Plugins\loading\Definition.xml";

/// The 32x32 cyan strip every column of the wave samples.
///
/// `Data.wad` entry 68, name hash `d857f34b`. The only asset the original's
/// loading screen touches, which is what makes the screen drawable while the
/// thing being loaded does not exist yet.
pub const GLOW_STRIP_ENTRY: &str = r"Data\Defaults\Loading\LoadingPulseOverlay.mip";

/// Columns across the strip.
///
/// The original steps screen X by 2 from 0 to 480. This is the random walk's
/// step count as much as it is a geometry figure - see the module doc comment
/// on why it does not scale with resolution.
pub const COLUMNS: usize = 240;

/// Bands drawn per column: the two oscillator layers, and the slew-limited
/// blend of them.
pub const BANDS: usize = 3;

/// Where the strip sits, in reference pixels from the top.
pub const BASELINE_Y: f32 = 220.0;

/// Quad height, and the glow strip's own size.
pub const STRIP_SIZE: f32 = 32.0;

/// The heartbeat.
///
/// 24 floats straight out of `.rodata` at `0x08a88034`, byte-identical in the
/// EU and PS2 builds. Two peaks of 99 with a shallow trough between them, a
/// decay tail, then six frames of silence. At the loading thread's 30 Hz that
/// is one beat every 0.8 s, and it is why the game is called Pulse.
pub const ENVELOPE: [f32; 24] = [
    0.0, 0.0, 10.0, 40.0, 70.0, 99.0, 70.0, 40.0, 10.0, 40.0, 70.0, 99.0, 70.0, 40.0, 30.0, 20.0,
    10.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

/// The envelope's own maximum, which is what it is divided by.
pub const ENVELOPE_PEAK: f32 = 99.0;

/// Amplitude floor. The wave idles at a tenth rather than going fully flat.
pub const ENVELOPE_FLOOR: f32 = 0.1;

/// Tracking rates for the two oscillator layers, from `g_loading_wave_rates`.
pub const LAYER_RATES: [f32; 2] = [0.1, 0.05];

/// Per-column damping applied to each layer's energy.
pub const ENERGY_DAMPING: f32 = 0.985;

/// Full width of the per-column random impulse: uniform in `+/-7.5`.
pub const ENERGY_IMPULSE: f32 = 15.0;

/// Most the slew-limited band moves in one column.
pub const SLEW_STEP: f32 = 0.5;

/// How far the slew-limited band must lag before it moves at all.
pub const SLEW_DEADBAND: f32 = 4.0;

/// Weights blending the two layers into the third band.
pub const BLEND_WEIGHTS: [f32; 2] = [0.7, 0.3];

/// Low bound of the amplitude ramp across X, in reference pixels.
pub const AMPLITUDE_RAMP: (i32, i32) = (30, 286);

/// Low and high bounds of the alpha ramp across X, in reference pixels.
///
/// Tints `0xff000000` to `0xff808080` in the original.
pub const ALPHA_RAMP: (i32, i32) = (10, 350);
