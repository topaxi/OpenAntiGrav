//! A whole race, frame by frame, measured: `OAG_RENDER_BENCH_RACE=<stem>`.
//!
//! [`bench`](super::bench) re-draws **one** captured tick. This drives the race
//! from the start to the player's own last crossing, one simulation tick and
//! one drawn frame at a time, and records what each frame cost, so a lap that
//! is slow in one corner shows up where a single frame would never look.
//!
//! Per frame, kept apart because they answer different questions:
//!
//! - `tick_us`: the simulation step (and the audio cue drain that rides it, on
//!   the null backend).
//! - `record_us`, `submit_us`, `cpu_us` (their sum): CPU time to encode the
//!   scene and to finish and submit the encoder.
//! - `gpu_scene_us`, `gpu_blur_us`, `gpu_bloom_us`, `gpu_us`: the GPU's own
//!   timestamps around the race pass and the two post chains, read back late
//!   without waiting, so the pipeline is not drained to take them. Empty where
//!   the device has no timestamps or a chain did not run.
//! - `frame_us`: the wall time from the previous frame's start to this one's,
//!   with no idle between frames and at most [`IN_FLIGHT`] submissions
//!   outstanding. That is the throughput a frame budget is read against.
//! - `draws`, `culled`, `triangles`: what the scene submitted.
//!
//! **The scene only, no HUD**, which is what the single-frame bench measures.
//! **One view or a split** (`--players 2 --split-cut horizontal|vertical`):
//! a split is drawn by `Scene::render_viewports`, which records and submits
//! its own command buffers, so its `record_us` is the whole call (the submit
//! is inside it, `submit_us` is empty) and the GPU columns are one timestamped
//! scene pass per view (`gpu_view<N>_us`, `gpu_scene_us` their sum). It draws
//! no motion blur or bloom chain, so those columns stay empty. A multi-window
//! race is refused by name. The player's
//! craft is on `--autopilot` when the caller says so; the race ends on the
//! player's own crossing (not when the AI field is done), or at the tick cap
//! (`--ticks`, [`DEFAULT_TICK_CAP`] when zero), in which case the result says
//! `finished: false`.
//!
//! Writes `<stem>.csv` (every frame) and `<stem>.json` (the summary and
//! everything needed to reproduce the run). `scripts/bench-render.py` is the
//! matrix, the lock and the comparison; `docs/tools/render-bench.md` is the
//! guide.

use anyhow::{Context, Result, bail};
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::path::PathBuf;
use web_time::Instant;

use super::{CaptureOptions, HeldButtons, advance_one_tick, spectrum};
use oag_gpu::timing::{Half, PassTimer, Reading};
use oag_raceplay::{Race, Scene};
use stats::{Summary, json_string, summarise};

mod stats;

/// The environment variable that turns the bench on: the output path without
/// its extension.
pub(super) const ENV: &str = "OAG_RENDER_BENCH_RACE";

/// Submissions allowed to be outstanding on the GPU. Two is a double-buffered
/// window: the CPU records frame N while the GPU draws frame N-1, which is what
/// a vsynced window does. **Chosen, not measured.**
const IN_FLIGHT: usize = 2;

/// The tick cap when `--ticks` is zero: thirty minutes of racing. Chosen.
const DEFAULT_TICK_CAP: u32 = 60 * 60 * 30;

/// Frames left out of the summary (not the CSV): the first drawn frames pay
/// pipeline and buffer first use. Chosen.
const WARMUP_FRAMES: usize = 60;

/// Whether the environment asks for a race bench, and where it writes.
pub(super) fn requested() -> Option<PathBuf> {
    std::env::var_os(ENV)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Everything the loop draws with, borrowed from `capture`.
pub(super) struct Rig<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub adapter: wgpu::AdapterInfo,
    pub scene: &'a mut Scene,
    pub race: &'a mut Race,
    pub view: &'a wgpu::TextureView,
    pub viewport: (f32, f32, f32, f32),
    pub camera_jitter: Option<u32>,
    pub held: &'a mut HeldButtons,
    pub audio: &'a mut oag_sound::Audio,
    pub options: &'a CaptureOptions,
    pub title: &'static str,
}

/// One frame's numbers. `NaN` is a reading that never came.
struct Frame {
    tick: u64,
    lap: u32,
    tick_us: f64,
    record_us: f64,
    submit_us: f64,
    frame_us: f64,
    /// One entry per timer: scene, blur, bloom for one view; one scene pass per
    /// view for a split.
    gpu: Vec<f64>,
    split: bool,
    draws: u32,
    culled: u32,
    triangles: u32,
}

impl Frame {
    fn cpu_us(&self) -> f64 {
        self.record_us
            + if self.submit_us.is_finite() {
                self.submit_us
            } else {
                0.0
            }
    }

    /// The scene passes: the one pass of a single view, the sum over a split's.
    fn gpu_scene_us(&self) -> f64 {
        if self.split {
            self.gpu_us()
        } else {
            self.gpu.first().copied().unwrap_or(f64::NAN)
        }
    }

    /// The blur and bloom chains: only a single view draws them.
    fn gpu_chain_us(&self, slot: usize) -> f64 {
        if self.split {
            f64::NAN
        } else {
            self.gpu.get(slot).copied().unwrap_or(f64::NAN)
        }
    }

    fn gpu_us(&self) -> f64 {
        let seen: Vec<f64> = self.gpu.iter().copied().filter(|v| v.is_finite()).collect();
        if seen.is_empty() {
            f64::NAN
        } else {
            seen.iter().sum()
        }
    }
}

fn load_average() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|text| {
            let mut fields = text.split_whitespace();
            Some(format!(
                "{} {} {}",
                fields.next()?,
                fields.next()?,
                fields.next()?
            ))
        })
        .unwrap_or_else(|| "unavailable".to_string())
}

/// Runs the race to its end, draws every frame, and writes the result files.
///
/// # Errors
///
/// A split or multi-window race, or a result file that cannot be written.
pub(super) fn run(stem: &std::path::Path, rig: Rig<'_>) -> Result<()> {
    let Rig {
        device,
        queue,
        adapter,
        scene,
        race,
        view,
        viewport,
        camera_jitter,
        held,
        audio,
        options,
        title,
    } = rig;
    if race.windowed() {
        bail!("{ENV}: a multi-window race is not benched here, it has one window");
    }
    let split = race.viewport_count() > 1;
    let views = if split { race.views_drawn() } else { 1 };
    let cap = if options.ticks > 0 {
        options.ticks
    } else {
        DEFAULT_TICK_CAP
    };
    let slots = if split { views } else { 3 };
    let mut timers: Vec<Option<PassTimer>> =
        (0..slots).map(|_| PassTimer::new(device, queue)).collect();
    let timed = timers.iter().all(Option::is_some);
    let mut gpu: Vec<Vec<f64>> = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut in_flight: VecDeque<wgpu::SubmissionIndex> = VecDeque::new();
    let load_start = load_average();
    let begun = Instant::now();
    let mut previous: Option<Instant> = None;
    let mut finished = false;
    let mut tick = 0u32;
    while tick < cap {
        if in_flight.len() >= IN_FLIGHT
            && let Some(index) = in_flight.pop_front()
        {
            let _ = device.poll(wgpu::PollType::Wait {
                submission_index: Some(index),
                timeout: None,
            });
        }
        let start = Instant::now();
        let frame_us = previous.map_or(f64::NAN, |p| (start - p).as_secs_f64() * 1e6);
        previous = Some(start);

        advance_one_tick(race, held, audio, options, tick);
        tick += 1;
        let tick_us = start.elapsed().as_secs_f64() * 1e6;
        if race.finished() {
            // The player's crossing ends the bench; the field running on
            // under the end-race screens is not what a lap costs.
            finished = true;
            break;
        }

        let index = frames.len() as u64;
        collect(device, &mut timers, &mut gpu);
        gpu.push(vec![f64::NAN; slots]);
        for timer in timers.iter_mut().flatten() {
            timer.begin(index);
        }
        if scene.sync_zone_grade(race) {
            scene.rebind_zone_art(device, queue);
        }
        let spectrum = spectrum::zone_spectrum(options, audio);
        let record = Instant::now();
        let stats;
        let mut submit_us = f64::NAN;
        let mut encoder = None;
        if split {
            // Records and submits inside the call; the empty submission below
            // only names the point the frame-in-flight window waits on.
            stats = scene.render_viewports(
                device,
                queue,
                view,
                race,
                viewport,
                options.fov,
                options.frustum_culling,
                options.pvs_culling,
                options.anim_seconds,
                options.shadows,
                &spectrum,
                oag_raceplay::WindowEffects {
                    motion_blur: options.motion_blur,
                    camera_jitter,
                    ..Default::default()
                },
                &mut timers,
            );
        } else {
            let mut frame_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("race bench frame"),
                });
            let [scene_timer, blur_timer, bloom_timer] = &timers[..] else {
                unreachable!("three timers for one view");
            };
            stats = scene.render(
                device,
                queue,
                &mut frame_encoder,
                view,
                race,
                viewport,
                options.fov,
                options.frustum_culling,
                options.pvs_culling,
                options.anim_seconds,
                options.motion_blur,
                options.shadows,
                camera_jitter,
                &spectrum,
                scene_timer.as_ref().and_then(PassTimer::writes),
                blur_timer.as_ref().and_then(|t| {
                    Some(oag_post::motion_blur::ChainTimestamps {
                        begin: Some(t.half_writes(Half::Begin)?),
                        end: Some(t.half_writes(Half::End)?),
                    })
                }),
                bloom_timer.as_ref().and_then(|t| {
                    Some(oag_post::hd_bloom::ChainTimestamps {
                        begin: t.half_writes(Half::Begin)?,
                        end: t.half_writes(Half::End)?,
                    })
                }),
            );
            // A claim the chain did not write into is given back, never resolved:
            // four of those end every reading for the run (`PassTimer::abandon`).
            for (timer, encoded) in
                timers
                    .iter_mut()
                    .zip([true, stats.blur_encoded, stats.hd_bloom_encoded])
            {
                if let Some(timer) = timer {
                    if encoded {
                        timer.resolve(&mut frame_encoder);
                    } else {
                        timer.abandon();
                    }
                }
            }
            encoder = Some(frame_encoder);
        }
        let recorded = record.elapsed().as_secs_f64() * 1e6;
        if let Some(encoder) = encoder {
            let submit = Instant::now();
            in_flight.push_back(queue.submit(Some(encoder.finish())));
            submit_us = submit.elapsed().as_secs_f64() * 1e6;
        } else {
            in_flight.push_back(queue.submit(std::iter::empty()));
        }
        frames.push(Frame {
            tick: race.sim.world.tick,
            lap: race.sim.world.race[0].lap,
            tick_us,
            record_us: recorded,
            submit_us,
            frame_us,
            gpu: Vec::new(),
            split,
            draws: stats.draws_submitted,
            culled: stats.draws_culled,
            triangles: stats.triangles,
        });
    }
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    // The tail of the readings, which land after the last frame is submitted.
    for _ in 0..4 {
        collect(device, &mut timers, &mut gpu);
    }
    for (frame, readings) in frames.iter_mut().zip(&gpu) {
        frame.gpu.clone_from(readings);
    }
    let wall = begun.elapsed().as_secs_f64();
    let load_end = load_average();

    let csv = csv(&frames);
    let facts = Facts {
        title,
        adapter: &adapter,
        timestamps: timed,
        finished,
        wall,
        load_start,
        load_end,
        cap,
        views,
        race,
        options,
    };
    let json = json(&frames, &facts);
    let beside = |extension: &str| {
        let mut name = stem.as_os_str().to_owned();
        name.push(".");
        name.push(extension);
        PathBuf::from(name)
    };
    let csv_path = beside("csv");
    let json_path = beside("json");
    if let Some(parent) = json_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&csv_path, csv).with_context(|| format!("writing {}", csv_path.display()))?;
    std::fs::write(&json_path, json).with_context(|| format!("writing {}", json_path.display()))?;
    println!(
        "race bench: {} frame(s) over {} tick(s), finished {finished}, state hash {:016x}, wrote {}",
        frames.len(),
        race.sim.world.tick,
        race.sim.state_hash(),
        json_path.display()
    );
    Ok(())
}

/// Files the GPU readings that have landed under the frames they name.
fn collect(device: &wgpu::Device, timers: &mut [Option<PassTimer>], gpu: &mut [Vec<f64>]) {
    for (slot, timer) in timers.iter_mut().enumerate() {
        let Some(timer) = timer else { continue };
        let mut readings: [Option<Reading>; 4] = [None; 4];
        timer.drain(device, &mut readings);
        for reading in readings.into_iter().flatten() {
            if let Some(cell) = usize::try_from(reading.frame)
                .ok()
                .and_then(|f| gpu.get_mut(f))
                .and_then(|entry| entry.get_mut(slot))
            {
                *cell = f64::from(reading.seconds) * 1e6;
            }
        }
    }
}

fn csv(frames: &[Frame]) -> String {
    let views = frames
        .first()
        .filter(|f| f.split)
        .map_or(0, |f| f.gpu.len());
    let mut out = String::from(
        "frame,tick,lap,tick_us,record_us,submit_us,cpu_us,frame_us,gpu_scene_us,gpu_blur_us,gpu_bloom_us,gpu_us,draws,culled,triangles",
    );
    for view in 0..views {
        let _ = write!(out, ",gpu_view{view}_us");
    }
    out.push('\n');
    let cell = |v: f64| {
        if v.is_finite() {
            format!("{v:.1}")
        } else {
            String::new()
        }
    };
    for (index, f) in frames.iter().enumerate() {
        let _ = write!(
            out,
            "{index},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            f.tick,
            f.lap,
            cell(f.tick_us),
            cell(f.record_us),
            cell(f.submit_us),
            cell(f.cpu_us()),
            cell(f.frame_us),
            cell(f.gpu_scene_us()),
            cell(f.gpu_chain_us(1)),
            cell(f.gpu_chain_us(2)),
            cell(f.gpu_us()),
            f.draws,
            f.culled,
            f.triangles
        );
        if f.split {
            for value in &f.gpu {
                let _ = write!(out, ",{}", cell(*value));
            }
        }
        out.push('\n');
    }
    out
}

struct Facts<'a> {
    title: &'static str,
    adapter: &'a wgpu::AdapterInfo,
    timestamps: bool,
    finished: bool,
    wall: f64,
    load_start: String,
    load_end: String,
    cap: u32,
    views: usize,
    race: &'a Race,
    options: &'a CaptureOptions,
}

fn summary_line(values: impl Iterator<Item = f64>) -> String {
    let values: Vec<f64> = values.collect();
    summarise(&values)
        .as_ref()
        .map_or_else(|| "null".to_string(), Summary::json)
}

/// The metrics a summary reports: name and how to read it off a frame.
type Read = fn(&Frame) -> f64;

const METRICS: [(&str, Read); 11] = [
    ("tick_us", |f| f.tick_us),
    ("record_us", |f| f.record_us),
    ("submit_us", |f| f.submit_us),
    ("cpu_us", Frame::cpu_us),
    ("frame_us", |f| f.frame_us),
    ("gpu_scene_us", Frame::gpu_scene_us),
    ("gpu_blur_us", |f| f.gpu_chain_us(1)),
    ("gpu_bloom_us", |f| f.gpu_chain_us(2)),
    ("gpu_us", Frame::gpu_us),
    ("draws", |f| f64::from(f.draws)),
    ("triangles", |f| f64::from(f.triangles)),
];

fn json(frames: &[Frame], facts: &Facts<'_>) -> String {
    let counted = &frames[WARMUP_FRAMES.min(frames.len())..];
    let world = &facts.race.sim.world;
    let o = facts.options;
    let a = facts.adapter;
    let settings = [
        ("size", format!("{}x{}", o.size.0, o.size.1)),
        ("msaa", format!("{:?}", o.msaa)),
        ("shadows", format!("{:?}", o.shadows)),
        ("traced_quality", o.traced_quality.name().to_string()),
        ("motion_blur", format!("{:?}", o.motion_blur)),
        (
            "motion_blur_resolution",
            o.motion_blur_resolution.name().to_string(),
        ),
        ("anisotropy", format!("{:?}", o.anisotropy)),
        ("model_detail", format!("{:?}", o.model_detail)),
        ("texture_detail", format!("{:?}", o.texture_detail)),
        ("trail_order", format!("{:?}", o.trail_order)),
        ("frustum_culling", format!("{:?}", o.frustum_culling)),
        ("pvs_culling", format!("{:?}", o.pvs_culling)),
        ("camera_view", format!("{:?}", o.camera_view)),
        ("fov", format!("{:?}", o.fov)),
        ("presented", format!("{}", o.presented.is_some())),
        ("autopilot", format!("{}", o.autopilot)),
    ];
    let split_settings = [
        ("views", facts.views.to_string()),
        ("split_cut", o.cut.name().to_string()),
    ];
    let settings: Vec<(&str, String)> = settings
        .into_iter()
        .chain(split_settings.into_iter().filter(|_| facts.views > 1))
        .collect();
    let mut out = String::from("{\n");
    let mut put = |key: &str, value: String| {
        let _ = writeln!(out, "  {}: {value},", json_string(key));
    };
    put("format", "1".to_string());
    put("title", json_string(facts.title));
    put("git_hash", json_string(oag_net::GIT_HASH));
    put(
        "build",
        json_string(&format!(
            "{}, perf-probe {}",
            if cfg!(debug_assertions) {
                "debug assertions on"
            } else {
                "release"
            },
            cfg!(feature = "perf-probe")
        )),
    );
    put(
        "adapter",
        format!(
            "{{\"name\": {}, \"type\": {}, \"backend\": {}, \"driver\": {}, \"driver_info\": {}, \"gpu_timestamps\": {}}}",
            json_string(&a.name),
            json_string(&format!("{:?}", a.device_type)),
            json_string(&format!("{:?}", a.backend)),
            json_string(&a.driver),
            json_string(&a.driver_info),
            facts.timestamps
        ),
    );
    put(
        "settings",
        format!(
            "{{{}}}",
            settings
                .iter()
                .map(|(k, v)| format!("{}: {}", json_string(k), json_string(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    put("mode", json_string(world.mode().name()));
    put(
        "laps_target",
        world
            .laps_target()
            .map_or_else(|| "null".to_string(), |n| n.to_string()),
    );
    put("people", o.players.to_string());
    put("tick_cap", facts.cap.to_string());
    put("finished", facts.finished.to_string());
    put("final_tick", world.tick.to_string());
    put(
        "final_state_hash",
        json_string(&format!("{:016x}", facts.race.sim.state_hash())),
    );
    put("frames", frames.len().to_string());
    put("warmup_frames_excluded", WARMUP_FRAMES.to_string());
    put("wall_seconds", format!("{:.3}", facts.wall));
    put("load_average_start", json_string(&facts.load_start));
    put("load_average_end", json_string(&facts.load_end));
    let metrics = METRICS
        .iter()
        .map(|(name, read)| {
            format!(
                "    {}: {}",
                json_string(name),
                summary_line(counted.iter().map(read))
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let view_metrics: String = (0..facts.views)
        .filter(|_| facts.views > 1)
        .map(|view| {
            format!(
                ",\n    {}: {}",
                json_string(&format!("gpu_view{view}_us")),
                summary_line(
                    counted
                        .iter()
                        .map(|f| f.gpu.get(view).copied().unwrap_or(f64::NAN))
                )
            )
        })
        .collect();
    put("metrics", format!("{{\n{metrics}{view_metrics}\n  }}"));
    let last_lap = frames.iter().map(|f| f.lap).max().unwrap_or(0);
    let laps = (1..=last_lap)
        .map(|lap| {
            let of = |read: Read| summary_line(counted.iter().filter(|f| f.lap == lap).map(read));
            format!(
                "    {{\"lap\": {lap}, \"frame_us\": {}, \"cpu_us\": {}, \"gpu_us\": {}}}",
                of(|f| f.frame_us),
                of(Frame::cpu_us),
                of(Frame::gpu_us)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    // Last key: no trailing comma.
    let _ = write!(out, "  \"per_lap\": [\n{laps}\n  ]\n}}\n");
    out
}
