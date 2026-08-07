//! GStreamer-backed platform-native H.264 decode for the PSP `.PMF` path.
//!
//! # Why GStreamer, not a hand-rolled decoder
//!
//! An earlier version of this decoder called `libva` (VA-API) directly: it
//! parsed SPS/PPS/slice headers itself and built the H.264 reference picture
//! list and every VA-API parameter buffer by hand. It ran without any
//! API-level error on real hardware and still produced wrong pixels - the
//! kind of bug a slice-level API gives no diagnostic for, because nothing in
//! the call chain reports "your field mapping is subtly wrong." See
//! [ADR-0017](../../../../docs/architecture/adr/0017-gstreamer-native-video.md)
//! for the full account. Maintaining a *correct* H.264 decoder - bitstream
//! parsing, DPB/reference-list bookkeeping, picture-order-count arithmetic,
//! scaling lists - is a much bigger and more error-prone undertaking than is
//! worth carrying here, and GStreamer already solved it. This module only
//! builds and drives a pipeline; GStreamer owns every part of actually
//! decoding H.264.
//!
//! # What this does
//!
//! `appsrc` (the whole demuxed elementary stream, pushed once) -> `h264parse`
//! -> `decodebin` (picks a hardware decoder if the system has one, or falls
//! back to `avdec_h264` from `gst-libav` - see ADR-0017 for why that software
//! fallback does not reopen ADR-0004/ADR-0008's rejection of a bundled H.264
//! decoder) -> `videoconvert` -> `appsink` capped to `video/x-raw,format=I420`.
//!
//! [`GstDecoder::open`] decodes every wanted frame eagerly and keeps the
//! result in memory; [`VideoDecoder::frame`] and [`VideoDecoder::rewind`]
//! just index into it. That costs more memory than a live streaming pull
//! would, and is affordable at cutscene scale - the PSP intro's 270 frames at
//! 480x272 I420 is about 53 MiB - see ADR-0017's consequences. It also means
//! there is no pipeline state to keep consistent across calls: once `open`
//! returns, GStreamer is out of the picture.
//!
//! [`VideoDecoder::frame`] still copies the requested frame's bytes into the
//! caller's buffer, the same as [`Av1CacheDecoder`](super::Av1CacheDecoder)
//! does - but where that copy *is* the AV1 decode, here it is pure overhead
//! on top of a frame this struct already holds. A 480x272 I420 frame is
//! ~196 KiB; at the 60 Hz a looping backdrop movie is polled, that is a
//! single-digit MiB/s memcpy, not worth reshaping [`VideoDecoder`]'s
//! out-parameter shape to avoid.

use anyhow::{Context, Result, bail};
use gstreamer::prelude::*;
use gstreamer::{self as gst};
use gstreamer_app as gst_app;
use gstreamer_video::{self as gst_video, VideoFrameExt};
use oag_formats::av1;

use super::{PixelFormat, VideoDecoder, VideoFrame};

/// One decoded frame, already de-strided into tightly packed I420.
#[derive(Debug, Clone)]
struct Decoded {
    bytes: Vec<u8>,
}

/// Plays a `.PMF`'s demuxed H.264 elementary stream through a GStreamer
/// pipeline instead of the AV1 cache. See the module documentation.
pub struct GstDecoder {
    frames: Vec<Decoded>,
    geometry: av1::Geometry,
}

impl std::fmt::Debug for GstDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GstDecoder")
            .field("frames", &self.frames.len())
            .field("geometry", &self.geometry)
            .finish()
    }
}

impl GstDecoder {
    /// Decodes up to `wanted` frames from `video`, a demuxed Annex-B H.264
    /// elementary stream.
    ///
    /// `Ok(None)` means no usable pipeline could be built on this system -
    /// typically a GStreamer install missing `h264parse`, `decodebin`, or an
    /// H.264-capable decoder element - and the caller falls back to the AV1
    /// cache, the same as a missing `ffmpeg` does. `Err` means a pipeline was
    /// built but decoding the actual content failed.
    pub fn open(video: Vec<u8>, wanted: usize) -> Result<Option<Self>> {
        // Idempotent: safe to call again if some other part of the process
        // already initialised GStreamer.
        gst::init().context("initializing GStreamer")?;

        let Some((pipeline, appsrc, appsink)) = build_pipeline() else {
            return Ok(None);
        };

        let result = run(&pipeline, &appsrc, &appsink, video, wanted);
        // Always tear the pipeline down, decode succeeded or not - leaving
        // it in Playing would keep its threads alive for no reason.
        let _ = pipeline.set_state(gst::State::Null);
        let (frames, geometry) = result?;

        Ok(Some(Self { frames, geometry }))
    }
}

impl VideoDecoder for GstDecoder {
    fn label(&self) -> &'static str {
        "gstreamer"
    }

    fn geometry(&self) -> av1::Geometry {
        self.geometry
    }

    fn len(&self) -> usize {
        self.frames.len()
    }

    fn frame(&mut self, index: usize, out: &mut VideoFrame) -> Result<()> {
        let decoded = self
            .frames
            .get(index)
            .with_context(|| format!("frame {index} of {} is out of range", self.frames.len()))?;
        out.format = PixelFormat::I420;
        out.width = self.geometry.width;
        out.height = self.geometry.height;
        out.chroma_width = self.geometry.chroma_width;
        out.chroma_height = self.geometry.chroma_height;
        out.bytes.clear();
        out.bytes.extend_from_slice(&decoded.bytes);
        Ok(())
    }

    fn rewind(&mut self) {
        // Every frame is already decoded and held in `self.frames`; there is
        // no decoder state left to reset.
    }
}

/// Builds the `appsrc ! h264parse ! decodebin ! videoconvert ! appsink`
/// pipeline. `None` if any element in it is missing from the local GStreamer
/// install.
fn build_pipeline() -> Option<(gst::Pipeline, gst_app::AppSrc, gst_app::AppSink)> {
    let description = "appsrc name=src ! h264parse ! decodebin ! videoconvert ! video/x-raw,format=I420 ! appsink name=sink";
    let element = gst::parse::launch(description).ok()?;
    let pipeline = element.downcast::<gst::Pipeline>().ok()?;

    let appsrc = pipeline
        .by_name("src")?
        .downcast::<gst_app::AppSrc>()
        .ok()?;
    let appsink = pipeline
        .by_name("sink")?
        .downcast::<gst_app::AppSink>()
        .ok()?;
    Some((pipeline, appsrc, appsink))
}

/// Pushes `video` through `pipeline` and pulls up to `wanted` decoded I420
/// frames back out, along with the geometry GStreamer actually decoded.
fn run(
    pipeline: &gst::Pipeline,
    appsrc: &gst_app::AppSrc,
    appsink: &gst_app::AppSink,
    video: Vec<u8>,
    wanted: usize,
) -> Result<(Vec<Decoded>, av1::Geometry)> {
    appsrc.set_format(gst::Format::Bytes);
    // `Stream`, not `Seekable`: this is pushed once, start to finish, in
    // order - `Seekable` makes `appsrc` perform an initial seek on start
    // that this source doesn't support and always fails.
    appsrc.set_stream_type(gst_app::AppStreamType::Stream);
    appsrc.set_is_live(false);
    // Unbounded: the whole elementary stream is pushed in one buffer below,
    // not trickled in against `need-data`, so there is nothing to throttle.
    appsrc.set_max_bytes(0);
    appsrc.set_caps(Some(
        &gst::Caps::builder("video/x-h264")
            .field("stream-format", "byte-stream")
            .build(),
    ));
    // `sync=false`: an appsink honours buffer timestamps by default, which
    // paces pulls at 1x wall-clock playback speed. This decode has no
    // display to keep pace with, so that would just make every open() take
    // as long as the movie's own runtime.
    appsink.set_property("sync", false);

    appsrc
        .push_buffer(gst::Buffer::from_slice(video))
        .map_err(|e| anyhow::anyhow!("pushing the elementary stream into appsrc: {e:?}"))?;
    appsrc
        .end_of_stream()
        .map_err(|e| anyhow::anyhow!("signalling end-of-stream on appsrc: {e:?}"))?;

    if pipeline.set_state(gst::State::Playing).is_err() {
        bail!(
            "starting the decode pipeline: {}",
            bus_error(pipeline).unwrap_or_else(|| "no error was posted to the bus; rerun with \
                                     GST_DEBUG=3 to see why the state change failed"
                .to_string())
        );
    }

    let mut frames = Vec::new();
    let mut geometry = None;
    while frames.len() < wanted {
        let Ok(sample) = appsink.pull_sample() else {
            break; // EOS, or an error - the bus check below tells them apart.
        };
        let (decoded, sample_geometry) = decode_sample(&sample)?;
        geometry.get_or_insert(sample_geometry);
        frames.push(decoded);
    }

    if let Some(diagnostic) = bus_error(pipeline) {
        bail!("GStreamer pipeline error: {diagnostic}");
    }

    let geometry = geometry.with_context(|| "decoded no frames at all".to_string())?;
    Ok((frames, geometry))
}

/// Reads one `appsink` sample into a tightly packed I420 [`Decoded`] frame,
/// and the geometry its caps declare.
///
/// Uses `gstreamer_video`'s own stride-aware plane accessors rather than
/// indexing the raw buffer directly - GStreamer is free to pad each plane's
/// row to a wider stride than the picture width, and getting that wrong here
/// would be exactly the kind of silent-wrong-pixels bug this module exists to
/// avoid (see the module documentation).
fn decode_sample(sample: &gst::Sample) -> Result<(Decoded, av1::Geometry)> {
    let buffer = sample.buffer().context("sample carried no buffer")?;
    let caps = sample.caps().context("sample carried no caps")?;
    let info = gst_video::VideoInfo::from_caps(caps).context("reading video caps")?;
    let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
        .map_err(|_| anyhow::anyhow!("mapping the decoded frame"))?;

    let geometry = av1::Geometry::new(frame.width(), frame.height());
    // I420 planes in display order: Y, U, V.
    let mut bytes = Vec::with_capacity(frame_len(&geometry));
    for plane in 0..3 {
        let (plane_width, plane_height) = if plane == 0 {
            (geometry.width, geometry.height)
        } else {
            (geometry.chroma_width, geometry.chroma_height)
        };
        let stride = frame.plane_stride()[plane] as usize;
        let data = frame
            .plane_data(plane as u32)
            .map_err(|_| anyhow::anyhow!("reading plane {plane} of the decoded frame"))?;
        for row in 0..plane_height as usize {
            let start = row * stride;
            bytes.extend_from_slice(&data[start..start + plane_width as usize]);
        }
    }

    Ok((Decoded { bytes }, geometry))
}

fn frame_len(geometry: &av1::Geometry) -> usize {
    geometry.luma_len() + 2 * geometry.chroma_len()
}

/// The pipeline's most recent bus error, if any, as a human-readable string.
/// `None` when nothing has gone wrong - callers must not fold that into an
/// empty-but-present message, or a state-change failure with a slow-to-post
/// bus message reads as a blank, contentless error (this cost real time to
/// debug once already - see `HANDOVER.md`'s "Traps that are live").
fn bus_error(pipeline: &gst::Pipeline) -> Option<String> {
    let bus = pipeline.bus()?;
    // The pipeline posts its error message asynchronously, slightly after
    // set_state() itself returns Err - a non-blocking pop can race it and
    // find nothing yet, so wait briefly rather than giving up immediately.
    let msg =
        bus.timed_pop_filtered(gst::ClockTime::from_seconds(2), &[gst::MessageType::Error])?;
    let gst::MessageView::Error(err) = msg.view() else {
        return None;
    };
    Some(format!("{} ({:?})", err.error(), err.debug()))
}
