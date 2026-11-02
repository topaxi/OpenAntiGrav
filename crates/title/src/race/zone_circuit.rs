//! [`ZoneCircuit`]: where a title keeps the circuit a Zone race runs on.
//! Split out of `race.rs` under the 1,000-line rule; a move.

/// Where a title keeps the environment a Zone race flies through.
///
/// **Zone's look is authored, not computed.** A Zone circuit ships its own meshes,
/// textures, lights, `fogCube` and `Skycube`; the sky is the giveaway (the twelve
/// one-material skies `docs/formats/skycube.md` counts across the PSP disc are the
/// Zone variants, against five or six materials for every race circuit). So there
/// is nothing to tint or invent: loading the file the disc authors *is* the
/// aesthetic (CLAUDE.md).
///
/// **Where that file lives diverges**, measured on all three disc titles:
///
/// | Title | Zone's environment | Measured on |
/// | --- | --- | --- |
/// | Pulse | `Data\Environments\16_Track\zone_track.vex` - the race circuit's own directory, one extra file | `pulse-psp-usa.chd`, `pulse-ps2-eu.chd` |
/// | Pure | `Data\Zone\01_Zone\track.vex` - four circuits of its own, declared `type="Zone"` | `pure-psp-eu.chd` |
/// | HD / Fury | `/data/environments/zone_1/track.vex` - four circuits of its own | `hdfury-ps3-eu-dec.iso` |
/// | 2048 | the race circuit itself, unchanged - no environment swap at all | played, see [`Self::SameCircuit`] |
///
/// Two shapes across the disc-backed titles ([ADR-0022]); 2048 later added a third
/// ([`Self::SameCircuit`]).
///
/// **The selector in the executable is unread.** `docs/formats/track.md` records the
/// binary's `%s\%strack%s.vex` template, whose two `%s`s are the prefix and the
/// `_reversed` suffix [`Self::Prefixed`] composes, but the branch that puts `zone_`
/// in the first has not been found (as `Ship_LoadModel`'s `case 6` was for the
/// hull). Everything here is name resolution against shipped archives, hence the 94
/// of a direct probe.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneCircuit {
    /// A Zone race runs the *same* circuit, out of a second file in its directory
    /// whose name is the race one with this prefix on the front.
    ///
    /// Pulse, both platforms: `track.vex` beside `zone_track.vex`, and
    /// `track_reversed.vex` beside `zone_track_reversed.vex`. The prefix is the first
    /// `%s` of the binary's path template, carried so a title spelling it
    /// differently changes this string and nothing else.
    ///
    /// **Not every circuit has one**, a load-correctness fact, not a menu one
    /// (`oag_raceplay::catalogue::Track::available_in_zone`).
    Prefixed(&'static str),
    /// A Zone race runs a circuit of its own, sharing nothing with the race ones;
    /// this is the one it opens when the caller names none.
    ///
    /// Pure and HD. There is **no mapping** from a race circuit to a Zone one,
    /// because the discs author none (Pure: eight race circuits, four Zone; HD:
    /// sixteen environments, four Zone). "What is Vineta K's Zone variant" has no
    /// answer in the data.
    ///
    /// **Every Zone circuit this title has, first entry first.** A caller can still
    /// name any of them: Pure's four `type="Zone"` circuits, HD's `zone_1`..`zone_4`
    /// (declared `type="Race"` with a `zone="true"` flag; see [`Self::menu_tracks`]
    /// for why the query matches by name). [`Self::variant_of`] needs the whole list
    /// to tell "the player picked a Zone circuit" from "the player picked a race
    /// circuit and this is Zone mode", which a menu that always names *something*
    /// cannot distinguish. The first entry is what [`Self::default_track`] answers
    /// with, on the "disc's own ordering" terms of
    /// [`RaceDefaults::track`](super::RaceDefaults::track).
    ///
    /// Never empty on a wired title (see the `oag_pure`/`oag_hd`/`oag_2048`
    /// construction sites for the evidence).
    /// **`also_race_circuits` says whether every ordinary race circuit is *also*
    /// offered in Zone mode, alongside this variant's own four: `false` on Pure,
    /// `true` on HD/Fury.** Unverified against the executable: it rests on a
    /// play-based recollection that ordinary HD/Fury circuits (Anulpha Pass, Moa
    /// Therma among them) carry a "zone pendant" in the original menu, which no disc
    /// data encodes (`docs/formats/hd-frontend.md`, "Zone is a `Mode` list entry").
    /// Two things corroborate it. The four `zone="true"` entries are real, distinct
    /// names in HD's string table (`DATA06`'s `entries.xml`, the copy naming all
    /// 28): **Pro Tozo, Mallavol, Corridon 12 and Syncopia**, never declared with a
    /// reversed variant as every one of the other twelve is, consistent with
    /// Zone-exclusive content. And HD ships a title-wide `zonemode.effectsettings`
    /// grading any ordinary circuit raced in Zone (the "any circuit + a colour-grade
    /// effect" shape 2048 confirmed for `SameCircuit`), its four Zone circuits taking
    /// `zonemodedlc3.effectsettings` ([`crate::ZonePalette::TitleWide`]). Neither is
    /// a decompiled selector.
    Separate(&'static [&'static str], bool),
    /// A Zone race runs the exact circuit a race would have; nothing is derived or
    /// substituted. 2048.
    ///
    /// Neither [`Self::Prefixed`] (no second `.vex` beside the race one) nor
    /// [`Self::Separate`] (no dedicated Zone circuit): 2048 runs Zone on whichever
    /// circuit the player picked.
    ///
    /// **Source: played, not decompiled or name-probed.** It was [`Self::Separate`]
    /// pointed at `zone_1` of four Zone-named environments `dlc2.psarc` ships, which
    /// `docs/formats/track.md` showed are HD's dedicated Zone circuits reshipped; play
    /// showed 2048's Zone mode ran on an ordinary circuit (`docs/formats/2048-status.md`).
    /// That agrees with [`RaceDefaults::zone`](super::RaceDefaults::zone): every
    /// base-package circuit ships a `ZoneMode2048.effectSettings` beside its
    /// `track.vex`. The four ported `zone_N` environments are disc content this axis
    /// does not answer. Confidence 90: a direct behavioural observation, not
    /// corroborated against a decompiled selector or a read `effectSettings` format.
    SameCircuit,
}

impl ZoneCircuit {
    /// The circuit a Zone race opens when the caller named one.
    ///
    /// [`Self::Prefixed`] rewrites it: the caller's name is a race circuit and its
    /// Zone twin is derivable.
    ///
    /// [`Self::Separate`] hands it back untouched **only when it already names one of
    /// this title's own Zone circuits**, compared case- and separator-insensitively
    /// (the fold a PSARC applies before a lookup), so `--track` and a menu-built name
    /// agree. Anything else, which is what a menu hands over when the Circuit row was
    /// not touched since picking Zone (it is filled with race circuits), falls back
    /// to the title's default. Trusting the caller to have named a Zone circuit once
    /// flew the picked race circuit under Zone's rules on HD and Pure, with a load
    /// report line `racing {track} as named` that read as success.
    ///
    /// **Idempotent under `Prefixed`**: a name already carrying the prefix comes back
    /// unchanged (`zone_track.vex`, not `zone_zone_track.vex`); `Separate` too.
    ///
    /// **`also_race_circuits` skips the substitution check** and hands the name back,
    /// as `SameCircuit` does: an ordinary race circuit is itself a legitimate Zone
    /// pick, and substituting `tracks[0]` under a menu entry would reproduce the bug
    /// this method closes.
    #[must_use]
    pub fn variant_of(self, track: &str) -> String {
        match self {
            Self::Prefixed(prefix) => {
                let split = track.rfind(['\\', '/']).map_or(0, |i| i + 1);
                let (directory, file) = track.split_at(split);
                if file.starts_with(prefix) {
                    return track.to_string();
                }
                format!("{directory}{prefix}{file}")
            }
            Self::Separate(tracks, also_race_circuits) => {
                debug_assert!(
                    !tracks.is_empty(),
                    "a title wired up as Separate must name at least its own default"
                );
                if also_race_circuits || tracks.iter().any(|&own| paths_match(own, track)) {
                    track.to_string()
                } else {
                    tracks[0].to_string()
                }
            }
            Self::SameCircuit => track.to_string(),
        }
    }

    /// The circuit a Zone race opens when the caller named none.
    ///
    /// `race_default` is [`RaceDefaults::track`](super::RaceDefaults::track).
    /// [`Self::Prefixed`] derives its Zone twin from it; [`Self::Separate`] answers
    /// with its own first entry, even with `also_race_circuits`, the best available
    /// answer with nothing named.
    #[must_use]
    pub fn default_track(self, race_default: &str) -> String {
        match self {
            Self::Prefixed(_) => self.variant_of(race_default),
            Self::Separate(tracks, _) => tracks[0].to_string(),
            Self::SameCircuit => race_default.to_string(),
        }
    }

    /// Which circuits a menu's CIRCUIT row offers when Zone is the selected mode,
    /// given every circuit a normal race offers.
    ///
    /// **The dispatch this axis exists for**, taking closures so the definition does
    /// not parse itself. [`Self::Prefixed`] filters `race_tracks` to those flagged
    /// `availableInZone` (eight of Pulse's twenty-four lack the prefixed file);
    /// [`Self::Separate`] asks `by_own_names` for the tracks it names, since Pure and
    /// HD declare their four Zone circuits outside the race listing;
    /// [`Self::SameCircuit`] returns `race_tracks` unfiltered.
    ///
    /// **`by_own_names` asks by name, not by `type`.** A `of_kind("Zone")` query
    /// answered empty on HD/Fury, whose four are `type="Race"` with a `zone="true"`
    /// flag (all four and only the four pointing at `Zone_1`..`Zone_4`, measured on
    /// `hdfury-ps3-eu-dec.iso`). The name list this variant carries matches either
    /// declaration shape and cannot answer with a circuit the title does not name.
    ///
    /// **`also_race_circuits` appends every ordinary race circuit after this
    /// variant's own four**, unverified (see [`Self::Separate`]). A circuit already
    /// present under its own name (HD's four are also `type="Race"` entries) is not
    /// duplicated.
    ///
    /// Generic over `T` and closures because `oag-title` sits beneath `oag-game`
    /// (`docs/architecture/workspace-layout.md`) and must not reach up to
    /// `oag_raceplay::catalogue::Track`. `available_in_zone` and `by_own_names` are
    /// the two questions `crates/raceplay/tests/zone_ground_truth.rs` asked by hand per
    /// title; this puts the dispatch on the title axis rather than in each caller.
    #[must_use]
    pub fn menu_tracks<T: Clone + PartialEq>(
        self,
        race_tracks: &[T],
        available_in_zone: impl Fn(&T) -> bool,
        by_own_names: impl FnOnce(&[&str]) -> Vec<T>,
    ) -> Vec<T> {
        match self {
            Self::Prefixed(_) => race_tracks
                .iter()
                .filter(|track| available_in_zone(track))
                .cloned()
                .collect(),
            Self::Separate(tracks, also_race_circuits) => {
                let mut offered = by_own_names(tracks);
                if also_race_circuits {
                    let extra: Vec<T> = race_tracks
                        .iter()
                        .filter(|t| !offered.contains(t))
                        .cloned()
                        .collect();
                    offered.extend(extra);
                }
                offered
            }
            Self::SameCircuit => race_tracks.to_vec(),
        }
    }
}

/// Whether two entry names address the same archive entry.
///
/// Mirrors the fold [`oag_assets::psarc`](oag_assets)'s `normalise` applies (leading
/// `/` trimmed, `\` folded to `/`, ASCII-lowercased) so
/// [`ZoneCircuit::Separate`]'s membership check agrees with the archive. Repeated
/// rather than shared: `oag-title` does not depend on `oag-assets`.
fn paths_match(a: &str, b: &str) -> bool {
    fn normalise(path: &str) -> String {
        path.trim_start_matches('/')
            .replace('\\', "/")
            .to_ascii_lowercase()
    }
    normalise(a) == normalise(b)
}
