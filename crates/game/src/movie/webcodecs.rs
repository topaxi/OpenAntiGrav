//! A `.PMF`'s H.264, decoded by the browser's WebCodecs `VideoDecoder`.
//!
//! The web build has neither the AV1 cache nor `ffmpeg` to make one, but every
//! browser it runs in decodes H.264 itself. This hands the demuxed access units
//! ([`pmf::access_units`]) to `web/movie.js`, one chunk per picture in Annex B
//! form with no `description` (the first unit carries the parameter sets in
//! band), and reads the decoded frames back as planes, which go up through the
//! same `upload_frame` and `video.wesl` as natively.
//!
//! **Asynchronous, so polled.** A decode finishes on the browser's own threads
//! and its output arrives on the page's event loop between frames; nothing
//! here waits. [`VideoDecoder::frame`] answers [`Pending`] until the frame is
//! out, and the web's inline [`super::Feed`] asks again on its next poll.
//!
//! **Frames are counted, not timed.** The decoder outputs in display order, so
//! the n-th frame out since a reset is frame n; a chunk's timestamp is only
//! what the API requires. Every unit is submitted, past a capped extent too,
//! because a reordering decoder holds pictures back until later units arrive.
//!
//! The planes are copied rather than drawn through `copyExternalImageToTexture`
//! (which wgpu's web backend has, for a `VideoFrame`): that writes RGBA,
//! colour-converted by the browser, into a texture `video.wesl` does not read,
//! where the copy keeps one format and frames comparable byte for byte with
//! the native decode. A 480x272 frame is 196 KB.

use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};
use log::{debug, info};
use oag_video::{av1, pmf};
use wasm_bindgen::{JsCast, JsValue};

use super::{FrameStore, Pending, PixelFormat, VideoDecoder, VideoFrame};

/// Why a movie that is not a `.PMF` has no picture in a browser.
pub(super) const NO_DECODER: &str = "the web build has no decoder for this movie: WebCodecs \
     decodes no MPEG-2 (PS2) or Bink (HD), and a browser cannot run the ffmpeg transcode";

/// How many pictures may be inside the browser's decoder or on their way out of
/// it at once. Enough to cover a reordering decoder's delay; chosen, not
/// measured.
const IN_FLIGHT: u32 = 8;

/// A movie's decoder on the page, through `web/movie.js`.
#[derive(Debug)]
struct WebCodecs {
    key: String,
    codec: String,
    video: Vec<u8>,
    units: Vec<std::ops::Range<usize>>,
    geometry: av1::Geometry,
    /// Frames this movie plays, which may be fewer than `units`.
    len: usize,
    /// The page's handle, opened on first use (on the page's thread).
    handle: Option<u32>,
    submitted: usize,
    delivered: usize,
    flushed: bool,
}

/// A [`FrameStore`] over the browser's decoder, or why there is none.
pub(super) fn frame_store(
    video: &[u8],
    key: &str,
    width: u32,
    height: u32,
    wanted: usize,
) -> Result<FrameStore> {
    let codec = pmf::avc_codec(video).ok_or_else(|| anyhow!("{key} has no H.264 SPS"))?;
    if function("oagMovieOpen").is_none() {
        bail!("this page has no WebCodecs bridge (web/movie.js)");
    }
    let units = pmf::access_units(video);
    let len = wanted.min(units.len());
    if len == 0 {
        bail!("{key} holds no pictures");
    }
    let geometry = av1::Geometry::new(width, height);
    info!("movie: {key} ({codec}) decodes through the browser's WebCodecs");
    Ok(FrameStore {
        path: PathBuf::from(format!("{key}-webcodecs")),
        len,
        luma_len: geometry.luma_len(),
        chroma_len: geometry.chroma_len(),
        chroma_width: geometry.chroma_width,
        chroma_height: geometry.chroma_height,
        source: Box::new(WebCodecs {
            key: key.to_string(),
            codec,
            video: video.to_vec(),
            units,
            geometry,
            len,
            handle: None,
            submitted: 0,
            delivered: 0,
            flushed: false,
        }),
    })
}

impl WebCodecs {
    fn handle(&mut self) -> Result<u32> {
        if let Some(id) = self.handle {
            return Ok(id);
        }
        let opened = call(
            "oagMovieOpen",
            &[
                self.codec.as_str().into(),
                self.geometry.width.into(),
                self.geometry.height.into(),
            ],
        )?;
        let id = match (opened.as_f64(), opened.as_string()) {
            (Some(id), _) => id as u32,
            (None, Some(why)) => bail!("{}: {why}", self.key),
            (None, None) => bail!("{}: oagMovieOpen returned {opened:?}", self.key),
        };
        self.handle = Some(id);
        Ok(id)
    }

    /// Feeds the decoder until enough is in flight, and flushes it once the
    /// last unit is in so a reordering decoder lets go of its tail.
    fn top_up(&mut self, id: u32) -> Result<()> {
        while self.submitted < self.units.len() && backlog(id)? < IN_FLIGHT {
            let unit = &self.video[self.units[self.submitted].clone()];
            let key = pmf::nal_units(unit).any(|nal| nal.first().is_some_and(|b| b & 0x1f == 5));
            // A copy, not a view: the module's memory is shared, and a chunk is
            // built from an ordinary buffer.
            let bytes = js_sys::Uint8Array::from(unit);
            call(
                "oagMovieDecode",
                &[
                    id.into(),
                    bytes.into(),
                    (self.submitted as u32).into(),
                    key.into(),
                ],
            )?;
            self.submitted += 1;
        }
        if self.submitted == self.units.len() && !self.flushed {
            call("oagMovieFlush", &[id.into()])?;
            self.flushed = true;
        }
        Ok(())
    }

    fn fill(&self, frame: &JsValue, out: &mut VideoFrame) -> Result<()> {
        let get = |name: &str| js_sys::Reflect::get(frame, &name.into()).unwrap_or(JsValue::NULL);
        let format = get("format").as_string().unwrap_or_default();
        let (width, height) = (
            get("width").as_f64().unwrap_or(0.0) as u32,
            get("height").as_f64().unwrap_or(0.0) as u32,
        );
        let g = self.geometry;
        if (width, height) != (g.width, g.height) {
            bail!(
                "{}: the browser decoded {width}x{height}, the movie declares {}x{}",
                self.key,
                g.width,
                g.height
            );
        }
        let bytes = get("bytes")
            .dyn_into::<js_sys::Uint8Array>()
            .map_err(|_| anyhow!("{}: a frame with no bytes", self.key))?
            .to_vec();
        let layout: Vec<(usize, usize)> = js_sys::Array::from(&get("layout"))
            .iter()
            .map(|plane| {
                let at = |name: &str| {
                    js_sys::Reflect::get(&plane, &name.into())
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as usize
                };
                (at("offset"), at("stride"))
            })
            .collect();
        let plane = |index: usize, row_len: usize, rows: u32, out: &mut Vec<u8>| -> Result<()> {
            let (offset, stride) = *layout
                .get(index)
                .ok_or_else(|| anyhow!("{}: plane {index} missing", self.key))?;
            for row in 0..rows as usize {
                let at = offset + row * stride;
                out.extend_from_slice(
                    bytes
                        .get(at..at + row_len)
                        .ok_or_else(|| anyhow!("{}: plane {index} is short", self.key))?,
                );
            }
            Ok(())
        };

        out.format = PixelFormat::I420;
        (out.width, out.height) = (g.width, g.height);
        (out.chroma_width, out.chroma_height) = (g.chroma_width, g.chroma_height);
        out.bytes.clear();
        let (cw, ch) = (g.chroma_width as usize, g.chroma_height);
        if let Some(red_first) = match format.as_str() {
            "RGBX" | "RGBA" => Some(true),
            "BGRX" | "BGRA" => Some(false),
            _ => None,
        } {
            let mut packed = Vec::with_capacity(g.luma_len() * 4);
            plane(0, g.width as usize * 4, g.height, &mut packed)?;
            to_i420(&packed, g, red_first, &mut out.bytes);
            return Ok(());
        }
        plane(0, g.width as usize, g.height, &mut out.bytes)?;
        match format.as_str() {
            "I420" => {
                plane(1, cw, ch, &mut out.bytes)?;
                plane(2, cw, ch, &mut out.bytes)?;
            }
            // Interleaved chroma, which a hardware decoder commonly hands out:
            // split into the two planes `video.wesl` reads.
            "NV12" => {
                let mut uv = Vec::with_capacity(cw * 2 * ch as usize);
                plane(1, cw * 2, ch, &mut uv)?;
                out.bytes.extend(uv.iter().step_by(2));
                out.bytes.extend(uv.iter().skip(1).step_by(2));
            }
            other => bail!(
                "{}: the browser decoded {other:?}, not I420, NV12 or packed RGB",
                self.key
            ),
        }
        Ok(())
    }
}

impl VideoDecoder for WebCodecs {
    fn label(&self) -> &'static str {
        "webcodecs"
    }

    fn geometry(&self) -> av1::Geometry {
        self.geometry
    }

    fn len(&self) -> usize {
        self.len
    }

    fn frame(&mut self, index: usize, out: &mut VideoFrame) -> Result<()> {
        let id = self.handle()?;
        if index < self.delivered {
            self.rewind();
        }
        loop {
            if let Some(why) = call("oagMovieError", &[id.into()])?.as_string() {
                bail!("{}: {why}", self.key);
            }
            self.top_up(id)?;
            let frame = call("oagMovieTake", &[id.into()])?;
            if frame.is_null() || frame.is_undefined() {
                if self.flushed && backlog(id)? == 0 {
                    bail!(
                        "{}: the browser decoded {} of {} frames",
                        self.key,
                        self.delivered,
                        self.len
                    );
                }
                return Err(Pending.into());
            }
            let n = self.delivered;
            self.delivered += 1;
            if n == index {
                self.fill(&frame, out)?;
                if index == 30 {
                    // FNV-1a over the luma plane, for comparing a frame with the
                    // native decode's ("Movies" in docs/tools/web.md).
                    let luma = &out.bytes[..self.geometry.luma_len()];
                    let hash = luma.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, &b| {
                        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
                    });
                    debug!("movie: {} frame 30 luma fnv1a {hash:016x}", self.key);
                }
                return Ok(());
            }
        }
    }

    fn rewind(&mut self) {
        if let Some(id) = self.handle {
            debug!(
                "movie: {} rewound after {} frame(s)",
                self.key, self.delivered
            );
            let _ = call("oagMovieReset", &[id.into()]);
        }
        (self.submitted, self.delivered, self.flushed) = (0, 0, false);
    }
}

impl Drop for WebCodecs {
    fn drop(&mut self) {
        if let Some(id) = self.handle {
            let _ = call("oagMovieClose", &[id.into()]);
        }
    }
}

/// Packed RGB back into BT.601 limited-range I420, the inverse of the matrix
/// `video.wesl` draws with, for a browser that hands out only RGB frames
/// (Firefox 155 gives `BGRX`, software decode or not). The browser already
/// converted the picture once, so this is close to the native frame, not equal
/// to it: chroma is the 2x2 average of what the browser upsampled.
fn to_i420(packed: &[u8], g: av1::Geometry, red_first: bool, out: &mut Vec<u8>) {
    let (w, h) = (g.width as usize, g.height as usize);
    let rgb = |x: usize, y: usize| {
        let p = &packed[(y.min(h - 1) * w + x.min(w - 1)) * 4..][..3];
        let (r, b) = if red_first {
            (p[0], p[2])
        } else {
            (p[2], p[0])
        };
        (f32::from(r), f32::from(p[1]), f32::from(b))
    };
    let byte = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    for y in 0..h {
        for x in 0..w {
            let (r, gr, b) = rgb(x, y);
            out.push(byte(16.0 + 0.256_788 * r + 0.504_129 * gr + 0.097_906 * b));
        }
    }
    let mut cb = Vec::with_capacity(g.chroma_len());
    let mut cr = Vec::with_capacity(g.chroma_len());
    for cy in 0..g.chroma_height as usize {
        for cx in 0..g.chroma_width as usize {
            let (mut r, mut gr, mut b) = (0.0, 0.0, 0.0);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (pr, pg, pb) = rgb(cx * 2 + dx, cy * 2 + dy);
                (r, gr, b) = (r + pr / 4.0, gr + pg / 4.0, b + pb / 4.0);
            }
            cb.push(byte(128.0 - 0.148_223 * r - 0.290_993 * gr + 0.439_216 * b));
            cr.push(byte(128.0 + 0.439_216 * r - 0.367_788 * gr - 0.071_427 * b));
        }
    }
    out.extend(cb);
    out.extend(cr);
}

fn backlog(id: u32) -> Result<u32> {
    Ok(call("oagMovieBacklog", &[id.into()])?
        .as_f64()
        .unwrap_or(0.0) as u32)
}

fn function(name: &str) -> Option<js_sys::Function> {
    js_sys::Reflect::get(&js_sys::global(), &name.into())
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()
}

/// Calls the page's global function `name` (`web/movie.js`).
fn call(name: &str, args: &[JsValue]) -> Result<JsValue> {
    let function = function(name).ok_or_else(|| anyhow!("the page has no {name}"))?;
    let array = args.iter().collect::<js_sys::Array>();
    function
        .apply(&js_sys::global(), &array)
        .map_err(|why| anyhow!("{name}: {why:?}"))
}
