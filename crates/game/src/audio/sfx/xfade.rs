//! HD's engine note: a per-team crossfade of looping layers, driven by four
//! numbers the craft writes every tick.
//!
//! Wipeout HD has no `~ENGINE` cue and no held pitch law. What it has is
//! `xfship_<team>.xfx` ([`oag_formats::xfx`], `docs/formats/hd-xfx.md`): four
//! input channels, each smoothed by `XFadeSystem_UpdateChannels`, and nine
//! looping layers that each read one channel through a gain curve and a pitch
//! curve. Every layer names a cue in `shiphd.bnk`.
//!
//! # What is read and what is chosen
//!
//! **Read off the binary and the file** (`docs/ghidra/functions/ps3-hdfury-eu/xfade.md`):
//!
//! - the four channel inputs, `Ship_UpdateEngineCrossfade` (`0x000d5968`):
//!   channel 0 is `trunc(0.5 * speed_field + 5.0 * X)`, channel 3 is
//!   `5.12 * throttle` for the local player, all clamped to `0..=511`;
//! - the smoother, `XFadeSystem_UpdateChannels` (`0x00313d10`): the band is
//!   picked by the current value, the rate by the band and the direction, a
//!   rate of zero snaps, the step is `rate * milliseconds` (at most 5,000);
//! - the layer update, `XFadeSystem_UpdateLayers` (`0x00314b00`): gain is
//!   `curve[x] / 0x400`, the bend is `((curve[x] - 0x200) * 0x7fff) >> 9`
//!   clamped to `+-0x8000`, and the bend reaches the note through the cue
//!   descriptor's own bend range in semitones
//!   (`Scream_ComputeVoiceNote`, `0x0062e998`, the same law
//!   `super::layers` already plays on Pulse).
//!
//! **Measured live on RPCS3, 2026-10-05** (two boots, 126 hardware voices
//! read over the GDB stub, `docs/formats/hd-xfx.md` "Level"): the layer's
//! volume word is `curve[x] * craft_factor / 1024` and the voice's final
//! gain is `K * level * (word / 1024)^2` - **squared** - with `K = 0.295` on
//! every engine layer voice and `level` the cue-times-waveform volume
//! product [`Sound`] already carries. So `0x400` is unity, and a layer at
//! half volume is a quarter as loud, which is not Pulse's x^0.59 curve
//! ([`oag_audio::spatial::volume_curve`]) that this module used to apply.
//! [`ENGINE_BUS_RATIO`] carries the absolute scale against this port's other
//! cues; [`distance_factor`] is the per-craft factor.
//!
//! **Chosen, not measured**, no confidence attached:
//!
//! - `X` is held at [`X_REST`], the value both boots read on the grid.
//!   Live it ran 2.17 to 4.49 and tracked the mean hover-probe gap plus about
//!   1.12 (see the evidence page); this simulation does not keep a per-probe
//!   gap, so the live term is not reproduced. It moves channel 0 by at most
//!   about 12 of 511.
//! - Channels 1 and 2 are held at zero: they read zero in every live sample.
//! - [`distance_factor`] is a straight line fitted to eight readings, not the
//!   traced law (the writer of the slot word it is read from was not found).
//! - Channel 3 is written in every mode; the original skips it in the modes
//!   whose id is bit 6, 13, 14 or 21 of `0x206040` (Zone is 6) and this port has
//!   no map from those ids to its own modes.
//! - Opponents open a layer's voice only while it is audible. The original
//!   keeps every layer resident (62 hardware voices of 128 in one scan), so a
//!   race with an engine grows the mixer's pool to [`oag_audio::mixer::HD_VOICES`].
//! - A finished race releases every layer, as [`super::Engine`] does.

use std::collections::BTreeMap;
use std::sync::Arc;

use oag_assets::source::Archives;
use oag_audio::{Mixer, Play, Sound, VoiceId};
use oag_formats::sblk;
use oag_formats::xfx::Xfx;

use super::Cue;
use super::banks::load_named_cue;

/// Channel 0's term `X`, held. The two grid readings of the first boot were
/// 2.173 and 2.167 and of the second 2.156 and 2.160. Chosen, not measured:
/// see the module documentation.
pub const X_REST: f32 = 2.164;

/// How loud an engine layer plays against the same cue played as an ordinary
/// SCREAM voice: `0.2945 / 0.6377 = 0.462`.
///
/// Measured live on RPCS3 (`docs/formats/hd-xfx.md`, "Level"): every engine
/// layer voice's final hardware gain is `0.2945 * level * (v86 / 1024)^2`
/// (the same on two boots), and the nearest ordinary voices read in the first
/// boot's scan (four voices of two cues) are `0.6377 * level * (v86 / 1024)^2`,
/// where `level` is the cue/waveform volume product this port already folds
/// into [`Sound`]. Ordinary voices read `0.31`, `0.41` and `0.79` elsewhere, so
/// the denominator is the nearest reading and not a platform constant; the
/// ratio only places the engine against this port's other cues, whose own scale
/// is about 3x the original's (see the doc page).
pub const ENGINE_BUS_RATIO: f32 = 0.2945 / 0.6377;

/// The per-craft distance factor, `slot+4 / 1024` of the layer slots.
///
/// **Fitted, not traced**: the writer of that word was not found. Eight
/// craft read on the grid at 8.7 to 148 units from the listener gave 0.991,
/// 0.971, 0.938, 0.876, 0.831, 0.771, 0.720, 0.660; a straight line
/// `1.0626 - 0.0027 d` passes every point past 34 units within 0.006, and the
/// near craft reads 0.991 to 0.994 rather than 1. Assumes the camera and the
/// grid of this port's own spawn stand where the original's did.
#[must_use]
pub fn distance_factor(distance: f32) -> f32 {
    (1.0626 - 0.0027 * distance).clamp(0.0, 0.992)
}

/// Milliseconds the smoother may be advanced by in one step, `XFadeSystem_UpdateChannels`.
const MAX_STEP_MS: i64 = 5000;

/// The top of a channel and of both curves' index.
const TOP: i32 = 511;

/// A layer's gain at unity.
const GAIN_UNITY: f32 = 1024.0;

/// Where a layer's pitch curve is neutral.
const PITCH_NEUTRAL: i32 = 0x200;

/// Below this a layer is treated as silent and its voice is released.
const AUDIBLE: f32 = 1.0e-4;

/// One input channel's smoothing configuration, all in the file's own units.
#[derive(Debug, Clone, Copy)]
struct ChannelCfg {
    /// Band edges, `16.16`.
    edges: [i32; 4],
    /// Rising rates per band, `16.16` counts per millisecond.
    rise: [i32; 4],
    /// Falling-or-holding rates per band.
    fall: [i32; 4],
    scale: i32,
    bias: i32,
}

/// One layer as the table authors it.
#[derive(Debug, Clone)]
struct LayerCfg {
    name: String,
    channel: usize,
    gain: Vec<i16>,
    pitch: Vec<i16>,
}

/// One team's table, owned, so it outlives the file bytes it was read from.
#[derive(Debug, Clone)]
pub struct Table {
    channels: Vec<ChannelCfg>,
    layers: Vec<LayerCfg>,
}

impl Table {
    /// Copies a parsed file's channels and layers out of its bytes.
    #[must_use]
    pub fn from_xfx(xfx: &Xfx<'_>) -> Self {
        let channels = xfx
            .channels()
            .iter()
            .map(|c| ChannelCfg {
                edges: c.band_edges().map(|e| i32::from(e) << 16),
                rise: c.rise_rates(),
                fall: c.fall_rates(),
                scale: c.input_scale(),
                bias: i32::from(c.input_bias()) << 16,
            })
            .collect();
        let layers = xfx
            .layers()
            .iter()
            .map(|l| LayerCfg {
                name: l.name().to_string(),
                channel: usize::from(l.channel()),
                gain: l.gain().collect(),
                pitch: l.pitch().collect(),
            })
            .collect();
        Self { channels, layers }
    }

    /// The name of layer `n`, the cue it plays.
    #[must_use]
    pub fn layer_name(&self, n: usize) -> Option<&str> {
        self.layers.get(n).map(|l| l.name.as_str())
    }

    /// How many layers the team plays.
    #[must_use]
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }
}

/// One channel's running state: the smoothed value and the target it chases.
#[derive(Debug, Clone, Copy, Default)]
struct Smoother {
    /// `16.16`.
    value: i32,
    target: i32,
}

impl Smoother {
    /// `XFadeSystem_SetInput` (`0x00314618`): `value * scale + (bias << 16)`.
    fn set_input(&mut self, cfg: &ChannelCfg, input: i32) {
        self.target = input.wrapping_mul(cfg.scale).wrapping_add(cfg.bias);
    }

    /// One `XFadeSystem_UpdateChannels` step for the channel, `ms` milliseconds on.
    ///
    /// The jitter the original also runs is zero on every shipped channel and
    /// is not modelled.
    fn step(&mut self, cfg: &ChannelCfg, ms: i64) {
        let cur = self.value;
        let band = band_of(cfg, cur);
        let rate = if self.target <= cur {
            cfg.fall[band]
        } else {
            cfg.rise[band]
        };
        if rate == 0 {
            self.value = self.target;
            return;
        }
        let ms = ms.clamp(0, MAX_STEP_MS);
        let step = i64::from(rate) * ms;
        let gap = i64::from(self.target) - i64::from(cur);
        self.value = if step < gap.abs() {
            if gap < 0 {
                cur - step as i32
            } else {
                cur + step as i32
            }
        } else {
            self.target
        };
    }
}

/// The band the smoothing reads its rate from, exactly as the decompile orders
/// it: below the first edge is band 0, below the second 1, below the third 2,
/// at or below the fourth 3, and above all of them back to 0.
fn band_of(cfg: &ChannelCfg, cur: i32) -> usize {
    if cur <= cfg.edges[0] {
        0
    } else if cur <= cfg.edges[1] {
        1
    } else if cur <= cfg.edges[2] {
        2
    } else if cur <= cfg.edges[3] {
        3
    } else {
        0
    }
}

/// What a layer sounds like at a smoothed channel value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    /// Linear gain on top of the cue's authored volume.
    pub gain: f32,
    /// The bend word the voice is given, `-0x8000..=0x7fff`.
    pub bend: i32,
}

fn level_at(layer: &LayerCfg, value: i32) -> Level {
    let x = (value >> 16).clamp(0, TOP) as usize;
    let gain = f32::from(layer.gain[x.min(layer.gain.len() - 1)]) / GAIN_UNITY;
    let pitch = i32::from(layer.pitch[x.min(layer.pitch.len() - 1)]);
    let bend = (((pitch - PITCH_NEUTRAL) * 0x7fff) >> 9).clamp(-0x8000, 0x8000);
    Level { gain, bend }
}

/// The playback-rate ratio a bend word gives a descriptor with these ranges.
///
/// Linear in semitones, down by `down` at `-0x8000` and up by `up` at
/// `+0x7fff`: the law `super::layers` plays Pulse's bends with.
#[must_use]
pub fn bend_ratio(bend: i32, down: i8, up: i8) -> f32 {
    let semitones = if bend < 0 {
        f32::from(down) * bend as f32 / 32768.0
    } else {
        f32::from(up) * bend as f32 / 32767.0
    };
    (semitones / 12.0).exp2()
}

/// One layer's decoded cue and the descriptor it was read from.
#[derive(Debug, Clone)]
struct LayerSound {
    sound: Arc<Sound>,
    down: i8,
    up: i8,
}

/// A team's table with its layers' sounds, shared by every craft of the team.
#[derive(Debug, Clone)]
pub struct Team {
    table: Table,
    /// One per table layer, [`None`] where the cue would not load or does not
    /// loop. A layer without a sound is silent and says so in the report.
    sounds: Vec<Option<LayerSound>>,
}

impl Team {
    /// How many layers have a sound to play.
    #[must_use]
    pub fn playable(&self) -> usize {
        self.sounds.iter().flatten().count()
    }

    /// The team's table.
    #[must_use]
    pub fn table(&self) -> &Table {
        &self.table
    }
}

/// The directory part of a bank entry, with its separator, or empty.
fn dir_of(entry: &str) -> &str {
    entry.rfind(['\\', '/']).map_or("", |at| &entry[..=at])
}

/// The disc's name for the team a grid slot flies.
///
/// Slot teams carry a Fury reskin suffix on some grids (`goteki_c1`); the table
/// is per team and not per reskin. The Detonator mode ship has its own table,
/// `det`.
#[must_use]
pub fn table_name(slot_team: &str) -> String {
    let lower = slot_team.to_lowercase();
    let base = lower
        .strip_suffix("_c1")
        .or_else(|| lower.strip_suffix("_n1"))
        .unwrap_or(&lower);
    match base {
        "detonator" => "det".to_string(),
        other => other.to_string(),
    }
}

/// Every team table a grid needs, loaded once per distinct team.
///
/// **Never fails**: a table that will not read, parse or resolve is a report
/// line and silence for that team, on [`super::Banks::load`]'s own terms.
pub(super) fn load(
    archives: &mut Archives,
    ship_bank: &str,
    slot_teams: &[String],
    report: &mut Vec<String>,
) -> BTreeMap<String, Arc<Team>> {
    let mut teams = BTreeMap::new();
    let blob = match archives.read_name(ship_bank) {
        Ok(blob) => blob,
        Err(e) => {
            report.push(format!("xfade: {ship_bank} not read: {e}"));
            return teams;
        }
    };
    let bank = match sblk::Bank::parse(&blob) {
        Ok(bank) => bank,
        Err(e) => {
            report.push(format!("xfade: {ship_bank} not parsed: {e}"));
            return teams;
        }
    };
    let dir = dir_of(ship_bank);
    for slot in slot_teams {
        let name = table_name(slot);
        if teams.contains_key(&name) {
            continue;
        }
        let entry = format!("{dir}xfship_{name}.xfx");
        let team = archives
            .read_name(&entry)
            .map_err(|e| e.to_string())
            .and_then(|blob| {
                let xfx = Xfx::parse(&blob).map_err(|e| e.to_string())?;
                Ok(Table::from_xfx(&xfx))
            })
            .map(|table| resolve(table, &bank, &entry, report));
        match team {
            Ok(team) => {
                report.push(format!(
                    "xfade: {entry} -> {} of {} layer(s) playable",
                    team.playable(),
                    team.table.layer_count()
                ));
                teams.insert(name, Arc::new(team));
            }
            Err(e) => report.push(format!("xfade: {entry} not loaded: {e}")),
        }
    }
    teams
}

/// Binds each layer's name to the bank's cue.
fn resolve(table: Table, bank: &sblk::Bank, entry: &str, report: &mut Vec<String>) -> Team {
    let sounds = table
        .layers
        .iter()
        .map(|layer| {
            let record = bank.cue_named(&layer.name)?;
            let descriptor = bank.cue_tree_sounds(&record).into_iter().next()?;
            let loaded = match load_named_cue(bank, &layer.name) {
                Ok((loaded, _)) => loaded,
                Err(e) => {
                    report.push(format!(
                        "xfade: {entry} layer {:?} not loaded: {e}",
                        layer.name
                    ));
                    return None;
                }
            };
            let (sound, looping) = loaded.waveforms.first()?.clone();
            if !looping {
                report.push(format!(
                    "xfade: {entry} layer {:?} is not marked looping; not held",
                    layer.name
                ));
                return None;
            }
            Some(LayerSound {
                sound,
                down: descriptor.bend_down,
                up: descriptor.bend_up,
            })
        })
        .collect();
    Team { table, sounds }
}

/// What the craft feeds the crossfade this tick.
#[derive(Debug, Clone, Copy)]
pub struct Inputs {
    /// `speed_field`: the craft's speed in HD's own unit, `km/h * 1.5`
    /// ([`oag_render::exhaust::hd::SPEED_FIELD_GAIN`]).
    pub speed_field: f32,
    /// The throttle, `0..=100`, for the local player only. Opponents never
    /// write channel 3.
    pub throttle: Option<f32>,
}

/// One craft's crossfaded engine: four smoothers and one voice per layer.
#[derive(Debug)]
pub struct Craft {
    team: Arc<Team>,
    smoothers: [Smoother; 4],
    voices: Vec<Option<VoiceId>>,
    doppler: oag_audio::Doppler,
    /// Milliseconds on the craft's own clock, for the smoother's step.
    clock_ms: i64,
    last_ms: Option<i64>,
    elapsed: f32,
    /// Whether the smoothers have been seeded from the first inputs.
    started: bool,
}

impl Craft {
    /// A craft playing `team`'s table.
    #[must_use]
    pub fn new(team: Arc<Team>) -> Self {
        let voices = vec![None; team.table.layers.len()];
        Self {
            team,
            smoothers: [Smoother::default(); 4],
            voices,
            doppler: oag_audio::Doppler::default(),
            clock_ms: 0,
            last_ms: None,
            elapsed: 0.0,
            started: false,
        }
    }

    /// The four channel inputs for `inputs`, in channel order, `None` for a
    /// channel this craft does not write.
    ///
    /// `Ship_UpdateEngineCrossfade`: `trunc(0.5 * speed_field + 5 * X)` and
    /// `trunc(5.12 * throttle)`, each clamped to `0..=511`. Channels 1 and 2
    /// are held at zero (chosen, not measured).
    #[must_use]
    pub fn channel_inputs(inputs: Inputs) -> [Option<i32>; 4] {
        let clamp = |v: f32| (v as i32).clamp(0, TOP);
        [
            Some(clamp(0.5 * inputs.speed_field + 5.0 * X_REST)),
            Some(0),
            Some(0),
            inputs.throttle.map(|t| clamp(5.12 * t)),
        ]
    }

    /// Advances the smoothers one tick, then writes every layer's gain and
    /// pitch to its voice, opening and releasing voices as layers become
    /// audible. `on` false releases everything.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        mixer: &mut Mixer,
        inputs: Inputs,
        on: bool,
        position: [f32; 3],
        listener: &oag_audio::Listener,
        doppler_enabled: bool,
        dt: f32,
    ) {
        if !on {
            self.stop(mixer);
            return;
        }
        self.elapsed += dt;
        self.clock_ms = (self.elapsed * 1000.0) as i64;
        let ms = self.clock_ms - self.last_ms.replace(self.clock_ms).unwrap_or(self.clock_ms);
        let wanted = Self::channel_inputs(inputs);
        for (n, input) in wanted.into_iter().enumerate() {
            let Some(cfg) = self.team.table.channels.get(n) else {
                continue;
            };
            if let Some(input) = input {
                self.smoothers[n].set_input(cfg, input);
            }
            // The first tick snaps: a voice opened at the grid starts where the
            // craft is, not where the table's zero is.
            if !self.started {
                self.smoothers[n].value = self.smoothers[n].target;
            } else {
                self.smoothers[n].step(cfg, ms);
            }
        }
        self.started = true;
        let table = &self.team.table;
        let mut distance = None;
        for (n, layer) in table.layers.iter().enumerate() {
            let Some(sound) = &self.team.sounds[n] else {
                continue;
            };
            let value = self.smoothers[layer.channel.min(3)].value;
            let level = level_at(layer, value);
            // The emitter's own radius gate and volume curve are Pulse's
            // (`Emitter::engine`, a 50 unit cull and x^0.59); HD's layer volume
            // is squared and its distance law is [`distance_factor`], so only
            // the pan and the distance are taken from the placement.
            let everywhere = oag_audio::Emitter {
                position,
                radius: f32::MAX,
                cone: None,
            };
            let placed = everywhere.place(listener, 1.0);
            let (gain, pan) = placed.map_or((0.0, None), |p| {
                let a = level.gain * distance_factor(p.distance);
                (ENGINE_BUS_RATIO * a * a, Some(p.pan))
            });
            if let Some(p) = placed {
                distance = Some(p.distance);
            }
            let audible = gain > AUDIBLE;
            let slot = &mut self.voices[n];
            match *slot {
                Some(id) if mixer.is_playing(id) => {
                    if audible {
                        mixer.set_gain(id, gain);
                        mixer.set_pan(id, pan);
                    } else {
                        mixer.stop(id);
                        *slot = None;
                    }
                }
                Some(_) => *slot = None,
                None => {}
            }
            if slot.is_none() && audible {
                *slot = mixer.play(Play {
                    gain,
                    pitch: bend_ratio(level.bend, sound.down, sound.up),
                    pan,
                    ..Play::looping(Arc::clone(&sound.sound), Cue::Engine.bus())
                });
            }
        }
        let doppler = match distance {
            Some(d) => self.doppler.ratio(d, dt, doppler_enabled),
            None => {
                self.doppler.reset();
                1.0
            }
        };
        for (n, layer) in table.layers.iter().enumerate() {
            let (Some(id), Some(sound)) = (self.voices[n], &self.team.sounds[n]) else {
                continue;
            };
            let level = level_at(layer, self.smoothers[layer.channel.min(3)].value);
            mixer.set_pitch(id, bend_ratio(level.bend, sound.down, sound.up) * doppler);
        }
    }

    /// The level layer `n` is at right now, for tests and reports.
    #[must_use]
    pub fn level(&self, n: usize) -> Option<Level> {
        let layer = self.team.table.layers.get(n)?;
        Some(level_at(layer, self.smoothers[layer.channel.min(3)].value))
    }

    /// Releases every layer's voice.
    pub fn stop(&mut self, mixer: &mut Mixer) {
        for slot in &mut self.voices {
            if let Some(id) = slot.take() {
                mixer.stop(id);
            }
        }
        self.last_ms = None;
        self.elapsed = 0.0;
        self.started = false;
        self.smoothers = [Smoother::default(); 4];
    }

    /// How many layer voices are open.
    #[must_use]
    pub fn open_voices(&self) -> usize {
        self.voices.iter().flatten().count()
    }
}

#[cfg(test)]
mod tests;
