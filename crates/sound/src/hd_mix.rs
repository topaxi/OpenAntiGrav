//! HD's authored mix: a row of group volumes per game state, and the law that
//! turns a slider and a group into a gain.
//!
//! Everything HD plays goes through `Data\sound\GlobalAudioConfig.xml`'s
//! `ParameterMaps`: one `GroupVolumes` row (`music`, `user1` to `user12`) per
//! game state and channel layout, parsed at boot by `FUN_0030d0b0` into the
//! sound system at `0x00b6cc90 + 0x6a0 + 0xb8 * state`. The live group array
//! (`+0x308`, thirteen floats) chases the active state's row. Measured on RPCS3,
//! `docs/formats/hd-xfx.md` "The authored mix":
//!
//! - the **music** chain is `slider * group`, linear in both (halving either
//!   halves the dump's RMS: 0.48x and 0.52x);
//! - the **SFX** chain is `(slider * group)^2` per voice, folded into the
//!   hardware gain (the player's engine voices read `K = 0.295, 0.074, 0.018` at
//!   groups `0.68, 0.34, 0.17` with the slider at `0.8`);
//! - both sliders default to `80 %` (`default="80%"` in the options screen's
//!   XML), and the profile the original ran on read `0.8` for both.
//!
//! The port plays the **Stereo** row: the layout byte read on RPCS3 was `2`, the
//! code's stereo branch; a surround layout reads another row, differing only in
//! `user4` (`0.9` against `0.95`) in the rows measured.
//!
//! # What is chosen, not measured
//!
//! - [`SMOOTHING`]: the live array chases its target exponentially; one reading
//!   (a group falling from `1.1` to `0.001` in about 1.5 s) fits `0.025` per
//!   256-sample frame, and the XML's transition speeds are not applied beyond it.
//! - Which group a cue belongs to, other than the engine (`user7`, read off the
//!   player's voices) and the circuit's emitters (`user8`, the only audible group
//!   with the others zero): the `USER1..12` comment in the `DATA00` copy of the
//!   file is the only source, and it is not traced.

use oag_audio::{Bus, GROUPS, Mixer};

use super::Audio;
use super::library::Library;

/// The options screen's authored default for both sliders, `default="80%"`.
pub const SLIDER_DEFAULT: f32 = 0.8;

/// This port's own unity against HD's: a sound's `pan_volume_gain` is
/// `2 * a^2 * t^2` (`oag_audio::spatial::pan_volume_gain`) and HD's hardware
/// gain is `K * a^2 * t^2` with `K = (slider * group)^2`, so the bus divides
/// the two out.
pub const PSP_UNITY: f32 = 2.0;

/// The fraction of the way a live group moves to its target per 256-sample
/// frame. Chosen, not measured: see the module documentation.
pub const SMOOTHING: f32 = 0.025;

/// Frames of 256 samples in one 60 Hz tick at 48 kHz (`800 / 256`).
const FRAMES_PER_TICK: f32 = 3.125;

/// The game states `ParameterMaps` carries a row for, in the order the file
/// lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The front end.
    FrontEnd,
    /// The fly-over before the countdown.
    PreRace,
    /// The grid, until the start.
    Countdown,
    /// Racing.
    RaceNormal,
    /// Racing with the energy critical.
    CriticalEnergy,
    /// The player's craft is destroyed.
    PlayerDead,
    /// After the finish.
    PostRace,
    /// The results table.
    DisplayResults,
}

impl State {
    const ALL: [Self; 8] = [
        Self::FrontEnd,
        Self::PreRace,
        Self::Countdown,
        Self::RaceNormal,
        Self::CriticalEnergy,
        Self::PlayerDead,
        Self::PostRace,
        Self::DisplayResults,
    ];

    fn tag(self) -> &'static str {
        match self {
            Self::FrontEnd => "ParameterMapFrontEnd",
            Self::PreRace => "ParameterMapPreRace",
            Self::Countdown => "ParameterMapCountdown",
            Self::RaceNormal => "ParameterMapRaceNormal",
            Self::CriticalEnergy => "ParameterMapRaceCriticalEnergy",
            Self::PlayerDead => "ParameterMapPlayerDead",
            Self::PostRace => "ParameterMapPostRace",
            Self::DisplayResults => "ParameterMapDisplayResults",
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// One row: the music group and the twelve user groups.
pub type Row = [f32; GROUPS + 1];

/// Every state's stereo row, as authored.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Maps {
    rows: [Option<Row>; State::ALL.len()],
}

impl Maps {
    /// Reads the stereo `GroupVolumes` of every state out of the file's text.
    /// Scans for tags rather than parsing XML (one authored document, attributes
    /// of one element). A state with no readable row stays `None` and the caller
    /// keeps its last.
    #[must_use]
    pub fn parse(xml: &str) -> Self {
        let mut maps = Self::default();
        for state in State::ALL {
            maps.rows[state.index()] = stereo_row(xml, state.tag());
        }
        maps
    }

    /// Reads and parses `Data\sound\GlobalAudioConfig.xml` out of `archives`;
    /// `None` when the title has no such entry (Pulse, Pure) or it holds no row,
    /// which keeps a title with no authored mix on its three buses.
    #[must_use]
    pub fn load(archives: &mut oag_assets::source::Archives) -> Option<Self> {
        let blob = archives
            .read_name(r"Data\sound\GlobalAudioConfig.xml")
            .ok()?;
        let maps = Self::parse(&String::from_utf8_lossy(&blob));
        maps.rows.iter().any(Option::is_some).then_some(maps)
    }

    /// The row a state plays, when the file authored one.
    #[must_use]
    pub fn row(&self, state: State) -> Option<&Row> {
        self.rows[state.index()].as_ref()
    }
}

/// The attribute `name` of the element opened at `at`, as a number. The file
/// writes some values with a trailing `f` (`"1.0f"`).
fn attribute(element: &str, name: &str) -> Option<f32> {
    let key = format!(" {name}=\"");
    let from = element.find(&key)? + key.len();
    let value = &element[from..from + element[from..].find('"')?];
    value.trim_end_matches('f').trim().parse().ok()
}

/// The first `GroupVolumes` after the state's `Stereo` tag.
fn stereo_row(xml: &str, tag: &str) -> Option<Row> {
    let open = xml.find(&format!("<{tag}"))?;
    let body = &xml[open..open + xml[open..].find(&format!("</{tag}>"))?];
    let stereo = &body[body.find("<Stereo>")?..];
    let from = stereo.find("<GroupVolumes")?;
    let element = &stereo[from..from + stereo[from..].find('>')?];
    let mut row = [0.0; GROUPS + 1];
    row[0] = attribute(element, "music")?;
    for (n, slot) in row.iter_mut().enumerate().skip(1) {
        *slot = attribute(element, &format!("user{n}"))?;
    }
    Some(row)
}

/// What a group does to a voice's gain once the slider is in: the SFX chain,
/// `(slider * group)^2`, over this port's own unity.
///
/// Measured per voice on RPCS3 (`K` of the player's engine layers at three
/// groups); the division by [`PSP_UNITY`] is the port's own scale.
#[must_use]
pub fn sfx_gain(slider: f32, group: f32) -> f32 {
    let k = slider * group;
    k * k / PSP_UNITY
}

/// The music chain, `slider * group`, linear in both.
#[must_use]
pub fn music_gain(slider: f32, group: f32) -> f32 {
    slider * group
}

/// What [`Audio`] keeps of HD's authored mix.
#[derive(Debug)]
pub struct Hd {
    /// The live groups, once a title with a mix has been read.
    pub(super) live: Option<Live>,
    /// The state the mix chases: set by the race each tick, `PreRace` from the
    /// moment a race loads (the fly-over has no race tick), and back to
    /// `FrontEnd` when the race's audio stops.
    pub(super) state: State,
    /// The music, effects and speech settings as gains, kept for the tick.
    pub(super) sliders: std::cell::Cell<[f32; 3]>,
}

impl Default for Hd {
    fn default() -> Self {
        Self {
            live: None,
            state: State::FrontEnd,
            sliders: std::cell::Cell::new([1.0; 3]),
        }
    }
}

/// The live group array chasing the active state's row.
#[derive(Debug, Clone)]
pub struct Live {
    maps: Maps,
    state: State,
    values: Row,
    settled: bool,
}

impl Live {
    /// A mix that starts in the front end.
    #[must_use]
    pub fn new(maps: Maps) -> Self {
        Self {
            maps,
            state: State::FrontEnd,
            values: [0.0; GROUPS + 1],
            settled: false,
        }
    }

    /// Changes the active state. A state with no authored row keeps the last.
    pub fn set_state(&mut self, state: State) {
        if self.maps.row(state).is_some() {
            self.state = state;
        }
    }

    /// Moves the live array one 60 Hz tick toward the active row. The first
    /// tick snaps, as a mixer opened mid-state has nothing to glide from.
    pub fn tick(&mut self) {
        let Some(target) = self.maps.row(self.state) else {
            return;
        };
        if !self.settled {
            self.values = *target;
            self.settled = true;
            return;
        }
        let k = 1.0 - (1.0 - SMOOTHING).powf(FRAMES_PER_TICK);
        for (value, goal) in self.values.iter_mut().zip(target) {
            *value += (goal - *value) * k;
        }
    }

    /// The live value of a group: `0` is music, `1..=12` the user groups.
    #[must_use]
    pub fn group(&self, n: usize) -> f32 {
        self.values.get(n).copied().unwrap_or(0.0)
    }

    /// Puts every bus on the live array: music linear, each group squared.
    /// Puts every bus on the live array: music linear, each group squared.
    ///
    /// `music_slider` and `sfx_slider` are the HD sliders in `0..=1`; the caller
    /// scales the port's percentages by [`SLIDER_DEFAULT`], so the port's default
    /// of `100 %` is the original's.
    ///
    /// The effects bus (every cue with no group) and speech bus (the announcer)
    /// play at group `1.0`: chosen, not measured. The ordinary voices in the
    /// first boot's scan had `K = 0.6377 = (0.8 * 1.0)^2`, but which cue sits in
    /// which group is not traced.
    pub fn apply(&self, mixer: &mut Mixer, music_slider: f32, sfx_slider: f32, speech: f32) {
        mixer.set_bus_gain(Bus::Music, music_gain(music_slider, self.group(0)));
        mixer.set_bus_gain(Bus::Sfx, sfx_gain(sfx_slider, 1.0));
        mixer.set_bus_gain(Bus::Speech, speech * sfx_gain(sfx_slider, 1.0));
        for n in 1..=GROUPS {
            mixer.set_bus_gain(Bus::Group(n as u8), sfx_gain(sfx_slider, self.group(n)));
        }
    }
}

#[cfg(test)]
mod tests;

impl Audio {
    /// A race has loaded: its fly-over runs on the `PreRace` row until the
    /// race's own tick takes over. A no-op where no mix was read.
    pub fn enter_race_mix(&mut self) {
        self.hd.state = State::PreRace;
    }

    /// Reads HD's authored mix off the booted source, once.
    ///
    /// A title with no `GlobalAudioConfig.xml` (Pulse, Pure) leaves
    /// [`Hd::live`] `None`, which keeps it on the three buses it had.
    pub(crate) fn load_hd_mix(&mut self, library: &dyn Library, source: &str) {
        if self.hd.live.is_some() {
            return;
        }
        let Some(mut opened) = library.open(source) else {
            return;
        };
        if let Some(maps) = Maps::load(&mut opened.archives) {
            self.hd.live = Some(Live::new(maps));
        }
    }

    /// Moves the live groups one tick toward `state`'s row and puts the buses
    /// on them. A no-op where no mix was read.
    pub(crate) fn hd_mix_tick(&mut self, state: State) {
        let [music, sfx, speech] = self.hd.sliders.get();
        let Some(live) = self.hd.live.as_mut() else {
            return;
        };
        live.set_state(state);
        live.tick();
        let live = live.clone();
        self.output.with_mixer(|mixer| {
            live.apply(mixer, music * SLIDER_DEFAULT, sfx * SLIDER_DEFAULT, speech)
        });
    }
}
