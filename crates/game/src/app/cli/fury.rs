//! Pinning the Fury menu backdrop on a `--menu-page` capture.

/// Pinning the Fury menu backdrop on a `--menu-page` capture.
#[derive(clap::Args, Debug)]
pub(crate) struct FuryArgs {
    /// Plays this static path of the Fury menu backdrop on a `--menu-page`
    /// capture instead of the picker's roll, `--anim-seconds` being the time
    /// into it. The original's own debug `Force Path`. This is how a capture
    /// is lined up with an RPCS3 frame whose path and clock were read off the
    /// clip (`scripts/hd-fury-backdrop-break.py`).
    #[arg(long, value_name = "INDEX")]
    pub(crate) fury_path: Option<usize>,

    /// States the Fury menu backdrop's screen tint on a `--menu-page` capture,
    /// one value for red, green and blue, instead of the skin's row for the
    /// page. `0` draws no cloud. This is what a pair script writes over the
    /// original's `default` row before a grab, because the cloud, its path
    /// and its music pulse are a new roll on every boot
    /// (`docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`, "Hidden for a
    /// compare shot").
    #[arg(long, value_name = "0..1")]
    pub(crate) fury_tint: Option<f32>,

    /// States the adapted luma the Fury front end's exposure is taken at on a
    /// `--menu-page` capture, in place of `FrontEndTone::adapted_fury`.
    /// With the backdrop's cloud hidden (`--fury-tint 0`) the original's
    /// exposure adapts to a darker page than the cloud-showing read the
    /// craft's light carries, and a pair records its own read.
    /// It sets the craft's light (`preview::model_light`) and pins the
    /// backdrop chain's adapted luma, which is what the cards' level follows.
    #[arg(long, value_name = "LUMA")]
    pub(crate) fury_adapted_luma: Option<f32>,
}
