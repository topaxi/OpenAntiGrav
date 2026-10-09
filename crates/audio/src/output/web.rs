//! The browser's output: the page's `AudioWorklet` drains a [`SharedRing`]
//! that [`render::Ahead`] fills from a Web Worker.
//!
//! Three threads, as natively: the frame loop starts and stops voices under
//! the mixer lock (the page's thread, which spins on it rather than wait,
//! `oag_thread::lock`); the render-ahead loop mixes a chunk at a time into
//! the ring (a worker, `oag_thread::web`); and the audio thread copies out of
//! it (the worklet, `web/audio-worklet.js`). The worklet runs no Rust at all:
//! it reads the ring's header and samples by address in the module's shared
//! memory, never blocks, plays silence for whatever is missing and counts the
//! quantum ([`super::shared_ring::word`]).
//!
//! The page owns the `AudioContext` (`web/audio.js`): it is created here at
//! the context's own sample rate, which is the rate the mixer renders at, and
//! resumed on the player's first key press or click, the gesture a browser
//! asks for. See docs/tools/web.md, "Sound".

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use wasm_bindgen::{JsCast, JsValue};

use super::shared_ring::SharedRing;
use super::{MIN_BUFFER, Output, render};
use crate::mixer::Mixer;
use crate::output::Health;
use crate::spectrum::Spectrum;

/// Frames in one render quantum: what a worklet's `process` is handed per
/// call. 128 in every browser today; the `renderSizeHint` that would change
/// it is not asked for.
const QUANTUM_FRAMES: u64 = 128;

/// The ring the worklet drains.
#[derive(Debug)]
pub(crate) struct Stream {
    ring: Arc<SharedRing>,
}

impl Stream {
    /// Copies the worklet's counters into `health`, which reports them the
    /// way it reports a device's.
    pub(crate) fn observe(&self, health: &Health) {
        health.observe_worklet(
            u64::from(self.ring.quanta()),
            QUANTUM_FRAMES,
            u64::from(self.ring.underruns()),
        );
    }

    /// Suspends or resumes the page's `AudioContext`.
    pub(crate) fn set_paused(&self, paused: bool) {
        if let Err(why) = call("oagAudioSetPaused", &[JsValue::from_bool(paused)]) {
            log::warn!("audio: {why}");
        }
    }
}

/// Calls the page's `name` with `args`.
fn call(name: &str, args: &[JsValue]) -> Result<JsValue> {
    let global = js_sys::global();
    let function = js_sys::Reflect::get(&global, &JsValue::from_str(name))
        .ok()
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        .ok_or_else(|| anyhow!("the page offers no {name}"))?;
    let args: js_sys::Array = args.iter().collect();
    function
        .apply(&global, &args)
        .map_err(|why| anyhow!("{name} failed: {why:?}"))
}

/// Opens the page's output; see the module's own documentation.
pub(super) fn open(buffer: Duration) -> Result<Output> {
    let rate = call("oagAudioOpen", &[])?.as_f64().unwrap_or(0.0);
    if !(8_000.0..=384_000.0).contains(&rate) {
        bail!("the page turned sound off");
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "checked to be a sample rate above"
    )]
    let sample_rate = rate as u32;
    let buffer = buffer.max(MIN_BUFFER);
    let mixer = Arc::new(Mutex::new(Mixer::new(sample_rate)));
    let ring = Arc::new(SharedRing::new(render::ring_capacity(sample_rate, buffer)));
    let spectrum = Arc::new(Spectrum::new());
    let ahead = render::Ahead::spawn(
        Arc::clone(&mixer),
        Arc::clone(&ring),
        render::target_samples(sample_rate, buffer),
        sample_rate,
        Arc::clone(&spectrum),
    );
    #[expect(
        clippy::cast_possible_truncation,
        reason = "addresses and lengths in a wasm32 module's memory"
    )]
    let (header, samples, capacity) = (
        ring.header_addr() as u32,
        ring.samples_addr() as u32,
        ring.capacity() as u32,
    );
    call(
        "oagAudioConnect",
        &[
            wasm_bindgen::memory(),
            header.into(),
            samples.into(),
            capacity.into(),
        ],
    )?;
    Ok(Output {
        mixer,
        stream: None,
        ahead: Some(ahead),
        web: Some(Stream { ring }),
        sample_rate,
        device: Some("the page's AudioWorklet".to_string()),
        health: Arc::new(Health::default()),
        tap: None,
        spectrum,
    })
}
