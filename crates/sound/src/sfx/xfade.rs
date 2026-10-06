//! HD's engine note: a per-team crossfade of looping layers, driven by four
//! numbers the craft writes every tick.
//!
//! HD has no `~ENGINE` cue or held pitch law, but `xfship_<team>.xfx`
//! ([`oag_formats::xfx`], `docs/formats/hd-xfx.md`): four input channels, each
//! smoothed by `XFadeSystem_UpdateChannels`, and nine looping layers that each
//! read one channel through a gain curve and a pitch curve. Every layer names a
//! cue in `shiphd.bnk`.
//!
//! # What is read and what is chosen
//!
//! **Read off the binary and the file** (`docs/ghidra/functions/ps3-hdfury-eu/xfade.md`):
//!
//! - the channel inputs, `Ship_UpdateEngineCrossfade` (`0x000d5968`): channel 0
//!   is `trunc(0.5 * speed_field + 5.0 * X)`, channel 3 is `5.12 * throttle`
//!   for the local player, all clamped to `0..=511`;
//! - the smoother, `XFadeSystem_UpdateChannels` (`0x00313d10`): the band is
//!   picked by the current value, the rate by band and direction, a rate of
//!   zero snaps, the step is `rate * milliseconds` (at most 5,000);
//! - the layer update, `XFadeSystem_UpdateLayers` (`0x00314b00`): gain is
//!   `curve[x] / 0x400`, the bend is `((curve[x] - 0x200) * 0x7fff) >> 9`
//!   clamped to `+-0x8000`, reaching the note through the cue descriptor's bend
//!   range in semitones (`Scream_ComputeVoiceNote`, `0x0062e998`, the law
//!   `super::layers` plays on Pulse).
//!
//! **Measured live on RPCS3, 2026-10-05** (two boots, 126 hardware voices over
//! the GDB stub, `docs/formats/hd-xfx.md` "Level"): the layer's volume word is
//! `curve[x] * craft_factor / 1024` and the voice's final gain is
//! `K * level * (word / 1024)^2`, **squared**, with `K = 0.295` on every engine
//! layer voice and `level` the cue-times-waveform volume product [`Sound`]
//! carries. So `0x400` is unity and half volume is a quarter as loud, not
//! Pulse's x^0.59 curve ([`oag_audio::spatial::volume_curve`]). [`ENGINE_BUS_RATIO`]
//! carries the absolute scale against this port's other cues;
//! [`distance_factor`] is the per-craft factor.
//!
//! **Chosen, not measured**, no confidence attached:
//!
//! - `X` is held at [`X_REST`], the value both boots read on the grid. Live it
//!   ran 2.17 to 4.49, tracking the mean hover-probe gap plus about 1.12; this
//!   simulation keeps no per-probe gap. It moves channel 0 by at most about 12
//!   of 511.
//! - Channels 1 and 2 are held at zero: zero in every live sample.
//! - [`distance_factor`] is a line fitted to eight readings, not the traced law
//!   (the writer of its slot word was not found).
//! - Channel 3 is written in every mode; the original skips it in modes whose
//!   id is bit 6, 13, 14 or 21 of `0x206040` (Zone is 6), and this port has no
//!   map from those ids to its modes.
//! - Opponents open a layer's voice only while audible. The original keeps
//!   every layer resident (62 hardware voices of 128 in one scan), so a race
//!   with an engine grows the pool to [`oag_audio::mixer::HD_VOICES`].
//! - A finished race releases every layer, as [`super::Engine`] does.

use std::collections::BTreeMap;
use std::sync::Arc;

use oag_assets::source::Archives;
use oag_audio::{Bus, Mixer, Play, Sound, VoiceId};
use oag_formats::sblk;
use oag_formats::xfx::Xfx;

use super::Cue;
use super::banks::{load_cue_record, load_indexed_cue};

/// Channel 0's term `X`, held. The two grid readings of the first boot were
/// 2.173 and 2.167 and of the second 2.156 and 2.160. Chosen, not measured:
/// see the module documentation.
pub const X_REST: f32 = 2.164;

/// How loud an engine layer plays against the same cue as an ordinary SCREAM
/// voice: `0.2945 / 0.6377 = 0.462`.
///
/// Measured live on RPCS3 (`docs/formats/hd-xfx.md`, "Level"): every engine
/// layer voice's final hardware gain is `0.2945 * level * (v86 / 1024)^2` (same
/// on two boots); the nearest ordinary voices in the first boot's scan (four
/// voices of two cues) are `0.6377 * level * (v86 / 1024)^2`. Ordinary voices
/// read `0.31`, `0.41` and `0.79` elsewhere, so the denominator is the nearest
/// reading, not a platform constant.
///
/// It is `0.68^2`: the player's `user7` group (`0.68` racing) squared against an
/// ordinary voice at group `1.0` (`GlobalAudioConfig.xml`, `hd-xfx.md` "The
/// authored mix"). A title with that mix plays the engine on its own group bus
/// and the group's law replaces this ratio; it stays for a title without one.
pub const ENGINE_BUS_RATIO: f32 = 0.2945 / 0.6377;

/// The per-craft distance factor, `slot+4 / 1024` of the layer slots.
///
/// **Fitted, not traced**: the writer of that word was not found. Eight craft
/// on the grid at 8.7 to 148 units from the listener gave 0.991, 0.971, 0.938,
/// 0.876, 0.831, 0.771, 0.720, 0.660; the line `1.0626 - 0.0027 d` passes every
/// point past 34 units within 0.006, and the near craft read 0.991 to 0.994, not
/// 1. Assumes this port's camera and spawn grid stand where the original's did.
#[must_use]
pub fn distance_factor(distance: f32) -> f32 {
    (1.0626 - 0.0027 * distance).clamp(0.0, 0.992)
}

/// Milliseconds the smoother may be advanced by in one step, `XFadeSystem_UpdateChannels`.
const MAX_STEP_MS: i64 = 5000;

/// The top of a channel and of both curves' index.
const TOP: i32 = 511;

/// Input channels a table can have: HD's four, the Vita's five.
const CHANNELS: usize = 5;

/// The ceiling the Vita's ship classes clamp every channel to: `0x43ff0000`,
/// 510.0, where HD's is [`TOP`].
const TOP_2048: i32 = 510;

/// The scale the Vita ship classes divide the pedal by before channel 4
/// (`DAT_8151fdd0`: 0.75 outside the Detonator mode, 0.7 in it).
const PEDAL_SCALE_2048: f32 = 0.75;

/// The speed-class factor channel 0 is multiplied by on the Vita ship classes,
/// held. The executable picks `1.25`, `1.15`, `0.95` or `0.8` by the race's
/// class word (`DAT_8153fd18` 1, 2, 3, 4) and leaves it untouched for any
/// other; this port does not carry that word to the sound frame, so the table
/// stands at neutral. **Chosen, not measured.**
const CLASS_FACTOR_2048: f32 = 1.0;

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
    /// The layer's kind byte: `0` plays a cue, `2` modulates another layer,
    /// `1` would start a stream (none is authored).
    kind: u8,
    /// The cue index the layer plays when [`Self::name`] is empty.
    cue: u16,
    /// For a kind-2 layer, the layer it modulates.
    link: i8,
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
                kind: l.kind(),
                cue: l.cue_index(),
                link: l.link(),
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

/// Which ship class writes a table's channels. The tables do not say: HD's
/// four-channel law and the Vita's five-channel law are different code on
/// different classes, and a Vita `HD`-era table (`xfship_assegai.xfx`) is read
/// by one and a `<team>2048` table by the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Law {
    /// `Ship_UpdateEngineCrossfade`, HD's, four channels written.
    Hd,
    /// The five Wipeout 2048 team classes (`FUN_812cd154` on the v1.04
    /// executable): channel 0 is half the speed field, channels 1 to 3 are
    /// zero, channel 4 follows the pedal.
    Vita2048,
}

/// A team's table with its layers' sounds, shared by every craft of the team.
#[derive(Debug, Clone)]
pub struct Team {
    law: Law,
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
/// Slot teams carry a Fury reskin suffix on some grids (`goteki_c1`) and, on
/// Wipeout 2048's, the livery directory (`Feisar2048\3`); the table is per team,
/// so both are dropped (`FUN_81263f6c` builds the file name from the team
/// record's name). The Detonator mode ship has its own table, `det`.
#[must_use]
pub fn table_name(slot_team: &str) -> String {
    let lower = slot_team.to_lowercase();
    let team = lower.split(['\\', '/']).next().unwrap_or(&lower);
    let base = team
        .strip_suffix("_c1")
        .or_else(|| team.strip_suffix("_n1"))
        .unwrap_or(team);
    match base {
        "detonator" => "det".to_string(),
        other => other.to_string(),
    }
}

/// The ship class a team's craft are, which picks the channel law: the five
/// Wipeout 2048 hulls write channel 4 ([`Law::Vita2048`]); every other table
/// (HD's, or an HD-era hull on a Vita grid) is written by the four-channel class.
fn law_of(name: &str) -> Law {
    if name.ends_with("2048") {
        Law::Vita2048
    } else {
        Law::Hd
    }
}

/// The table the executable falls back to when a team's own is not on the
/// disc (`FUN_81263f6c`), Zone or not.
const FALLBACK_TABLE: &str = "feisar";

/// Where a race's tables are read from.
#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    /// The ship bank the layers address.
    pub bank: &'a str,
    /// The infix a Zone race's table names carry, or empty (`ZONE_`).
    pub infix: &'a str,
}

/// Every team table a grid needs, loaded once per distinct team.
///
/// Never fails: a table that will not read, parse or resolve is a report line
/// and silence for that team ([`super::Banks::load`]). A team whose file is not
/// on the disc plays `xfship_feisar.xfx`, as the original does.
pub(super) fn load(
    archives: &mut Archives,
    source: Source<'_>,
    slot_teams: &[String],
    report: &mut Vec<String>,
) -> BTreeMap<String, Arc<Team>> {
    let Source {
        bank: ship_bank,
        infix,
    } = source;
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
    let read = |archives: &mut Archives, entry: &str| {
        archives
            .read_name(entry)
            .map_err(|e| e.to_string())
            .and_then(|blob| {
                let xfx = Xfx::parse(&blob).map_err(|e| e.to_string())?;
                Ok(Table::from_xfx(&xfx))
            })
    };
    for slot in slot_teams {
        let name = table_name(slot);
        if teams.contains_key(&name) {
            continue;
        }
        let mut entry = format!("{dir}xfship_{infix}{name}.xfx");
        let mut table = read(archives, &entry);
        if table.is_err() && name != FALLBACK_TABLE {
            let own = std::mem::replace(&mut entry, format!("{dir}xfship_{FALLBACK_TABLE}.xfx"));
            report.push(format!(
                "xfade: {own} not on the disc; the original plays {entry}"
            ));
            table = read(archives, &entry);
        }
        match table.map(|table| resolve(table, &bank, &entry, law_of(&name), report)) {
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

/// Binds each layer to the bank's cue: by name where it has one, by cue index
/// where the name is empty (`FUN_8125d4b6`).
///
/// Only kind-0 layers play a cue. A kind-2 layer modulates another and a kind-1
/// would start a stream; neither has a sound of its own.
fn resolve(
    table: Table,
    bank: &sblk::Bank,
    entry: &str,
    law: Law,
    report: &mut Vec<String>,
) -> Team {
    let sounds = table
        .layers
        .iter()
        .map(|layer| {
            let label = if layer.name.is_empty() {
                format!("cue {}", layer.cue)
            } else {
                format!("{:?}", layer.name)
            };
            match layer.kind {
                0 => {}
                2 => return None,
                kind => {
                    report.push(format!(
                        "xfade: {entry} layer {label} is kind {kind}; not played"
                    ));
                    return None;
                }
            }
            let (loaded, record) = if layer.name.is_empty() {
                match load_indexed_cue(bank, layer.cue) {
                    Ok(found) => found,
                    Err(e) => {
                        report.push(format!("xfade: {entry} layer {label} not loaded: {e}"));
                        return None;
                    }
                }
            } else {
                let Some(record) = bank.cue_named_or_hashed(&layer.name) else {
                    report.push(format!(
                        "xfade: {entry} layer {label} names no cue in {}",
                        bank.name
                    ));
                    return None;
                };
                match load_cue_record(bank, &record, &layer.name) {
                    Ok((loaded, _)) => (loaded, record),
                    Err(e) => {
                        report.push(format!("xfade: {entry} layer {label} not loaded: {e}"));
                        return None;
                    }
                }
            };
            let descriptor = bank.cue_tree_sounds(&record).into_iter().next()?;
            let (sound, looping) = loaded.waveforms.first()?.clone();
            if !looping {
                report.push(format!(
                    "xfade: {entry} layer {label} is not marked looping; not held"
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
    Team { law, table, sounds }
}

/// What the craft feeds the crossfade this tick.
#[derive(Debug, Clone, Copy)]
pub struct Inputs {
    /// `speed_field`: the craft's speed in HD's own unit, `km/h * 1.5`
    /// ([`oag_fx::exhaust::hd::SPEED_FIELD_GAIN`]).
    pub speed_field: f32,
    /// The throttle, `0..=100`, for the local player only. Opponents never
    /// write channel 3.
    pub throttle: Option<f32>,
}

/// One craft's crossfaded engine: four smoothers and one voice per layer.
#[derive(Debug)]
pub struct Craft {
    team: Arc<Team>,
    smoothers: [Smoother; CHANNELS],
    /// Per layer, what a kind-2 layer last wrote into it: the gain it
    /// multiplies by and the bend it adds. Neutral until a modulator runs.
    modulation: Vec<(f32, i32)>,
    voices: Vec<Option<VoiceId>>,
    doppler: oag_audio::Doppler,
    clock_ms: i64,
    last_ms: Option<i64>,
    elapsed: f32,
    started: bool,
    /// The authored volume group the layers play on, when the title has an
    /// authored mix (`user7`, [`crate::hd_mix`]). `None` plays on the
    /// effects bus at [`ENGINE_BUS_RATIO`].
    bus: Option<Bus>,
}

impl Craft {
    /// A craft playing `team`'s table.
    #[must_use]
    pub fn new(team: Arc<Team>) -> Self {
        let voices = vec![None; team.table.layers.len()];
        Self {
            modulation: vec![(1.0, 0); team.table.layers.len()],
            team,
            smoothers: [Smoother::default(); CHANNELS],
            voices,
            doppler: oag_audio::Doppler::default(),
            clock_ms: 0,
            last_ms: None,
            elapsed: 0.0,
            started: false,
            bus: None,
        }
    }

    /// Plays the layers on `bus`, an authored group. The group's own law
    /// ([`crate::hd_mix::sfx_gain`]) then carries the level
    /// [`ENGINE_BUS_RATIO`] stood in for.
    #[must_use]
    pub fn on_bus(mut self, bus: Option<Bus>) -> Self {
        self.bus = bus;
        self
    }

    /// The channel inputs for `inputs` under `law`, in channel order, `None` for
    /// a channel this craft does not write.
    ///
    /// [`Law::Hd`], `Ship_UpdateEngineCrossfade`: `trunc(0.5 * speed_field + 5 *
    /// X)` and `trunc(5.12 * throttle)`, each clamped to `0..=511`; channels 1
    /// and 2 held at zero (chosen, not measured); no channel 4.
    ///
    /// [`Law::Vita2048`], `FUN_812cd154`: `trunc(0.5 * speed_field * class)`
    /// clamped to `0..=510` on channel 0, zero on channels 1 to 3, as read.
    /// Channel 4 is `70 * pedal / 0.75` in the original, `pedal` being a ship
    /// field (`+0x6098`) whose writer was not found; this port feeds it the
    /// throttle (`0.7 * throttle / 0.75` of `0..=100`) and nothing for an
    /// opponent: chosen, not measured.
    #[must_use]
    pub fn channel_inputs(inputs: Inputs, law: Law) -> [Option<i32>; CHANNELS] {
        match law {
            Law::Hd => {
                let clamp = |v: f32| (v as i32).clamp(0, TOP);
                [
                    Some(clamp(0.5 * inputs.speed_field + 5.0 * X_REST)),
                    Some(0),
                    Some(0),
                    inputs.throttle.map(|t| clamp(5.12 * t)),
                    None,
                ]
            }
            Law::Vita2048 => {
                let clamp = |v: f32| (v as i32).clamp(0, TOP_2048);
                [
                    Some(clamp(0.5 * inputs.speed_field * CLASS_FACTOR_2048)),
                    Some(0),
                    Some(0),
                    Some(0),
                    inputs
                        .throttle
                        .map(|t| clamp(70.0 * (t / 100.0) / PEDAL_SCALE_2048)),
                ]
            }
        }
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
        let wanted = Self::channel_inputs(inputs, self.team.law);
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
            if layer.kind == 2 {
                // A modulator plays nothing: its curves become the linked
                // layer's gain multiplier and bend offset (`FUN_8125d61e`),
                // read by that layer on the next tick it runs.
                let level = level_at(layer, self.smoothers[layer.channel.min(CHANNELS - 1)].value);
                if let Some(slot) = usize::try_from(layer.link)
                    .ok()
                    .and_then(|t| self.modulation.get_mut(t))
                {
                    *slot = (level.gain, level.bend);
                }
                continue;
            }
            let Some(sound) = &self.team.sounds[n] else {
                continue;
            };
            let level = self.level_of(n);
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
                (
                    self.bus.map_or(ENGINE_BUS_RATIO, |_| 1.0) * a * a,
                    Some(p.pan),
                )
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
                    ..Play::looping(
                        Arc::clone(&sound.sound),
                        self.bus.unwrap_or_else(|| Cue::Engine.bus()),
                    )
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
        for n in 0..table.layers.len() {
            let (Some(id), Some(sound)) = (self.voices[n], &self.team.sounds[n]) else {
                continue;
            };
            let level = self.level_of(n);
            mixer.set_pitch(id, bend_ratio(level.bend, sound.down, sound.up) * doppler);
        }
    }

    /// Layer `n`'s level at its channel's smoothed value, with what a modulator
    /// last wrote into it: gain times the modulator's, bend plus the modulator's,
    /// clamped as the layer's own is.
    ///
    /// The product is chosen, not measured: `FUN_8125d61e` writes the
    /// modulator's gain (itself a product with its two slot floats) into the
    /// target's gain slot, and this takes that to be the curve value alone.
    fn level_of(&self, n: usize) -> Level {
        let layer = &self.team.table.layers[n];
        let base = level_at(layer, self.smoothers[layer.channel.min(CHANNELS - 1)].value);
        let (gain, bend) = self.modulation[n];
        Level {
            gain: base.gain * gain,
            bend: (base.bend + bend).clamp(-0x8000, 0x8000),
        }
    }

    /// The level layer `n` is at right now, for tests and reports.
    #[must_use]
    pub fn level(&self, n: usize) -> Option<Level> {
        (n < self.team.table.layers.len()).then(|| self.level_of(n))
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
        self.smoothers = [Smoother::default(); CHANNELS];
        self.modulation.fill((1.0, 0));
    }

    /// How many layer voices are open.
    #[must_use]
    pub fn open_voices(&self) -> usize {
        self.voices.iter().flatten().count()
    }
}

#[cfg(test)]
mod tests;
