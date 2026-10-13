//! [`ZoneCircuit`]: where a title keeps the circuit a Zone race runs on.
//! Split out of `race.rs` under the 1,000-line rule; a move.

/// Where a title keeps the environment a Zone race flies through.
///
/// **Zone's whole look is authored, not computed.** A Zone circuit ships its own
/// meshes, its own textures, its own lights, its own `fogCube` and its own
/// `Skycube` - and the sky is the giveaway, because the twelve one-material
/// skies `docs/formats/skycube.md` counts across the PSP disc are exactly the
/// Zone variants against five or six materials for every race circuit. So there
/// is nothing to tint, desaturate or otherwise invent: loading the file the disc
/// authors *is* the aesthetic, per `CLAUDE.md`'s rule about not authoring a
/// stand-in for what the data already carries.
///
/// **Where that file lives is the part that diverges**, and unlike the two
/// fields beside it this one was measured on all three PSP/PS3 titles rather
/// than two:
///
/// | Title | Zone's environment | Measured on |
/// | --- | --- | --- |
/// | Pulse | `Data\Environments\16_Track\zone_track.vex` - the race circuit's own directory, one extra file | `pulse-psp-usa.chd`, `pulse-ps2-eu.chd` |
/// | Pure | `Data\Zone\01_Zone\track.vex` - four circuits of its own, declared `type="Zone"` | `pure-psp-eu.chd` |
/// | HD / Fury | `/data/environments/zone_1/track.vex` - four circuits of its own | `hdfury-ps3-eu-dec.iso` |
/// | 2048 | the race circuit itself, unchanged - no environment swap at all | played, see [`Self::SameCircuit`] |
///
/// Two shapes across the three disc-backed titles, so neither variant was
/// designed from one example and there was no third state for a title nobody
/// had looked at - which is what [ADR-0022] licenses and what the `Option`
/// fields this module's docs refuse do not have. 2048 later added a third
/// shape once it had a measurement of its own; see [`Self::SameCircuit`].
///
/// **The selector in the executable is unread.** `docs/formats/track.md` records
/// the binary's own `%s\%strack%s.vex` template, whose two `%s`s are exactly the
/// prefix and the `_reversed` suffix [`Self::Prefixed`] composes - but the
/// branch that puts `zone_` in the first one has not been found, the way
/// `Ship_LoadModel`'s `case 6` was found for the hull. Everything here is name
/// resolution against shipped archives, which is why the confidence is the 94 of
/// a direct probe and not higher.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneCircuit {
    /// A Zone race runs the *same* circuit, out of a second file in its
    /// directory whose name is the race one with this prefix on the front.
    ///
    /// Pulse, on both its platforms: `track.vex` beside `zone_track.vex`, and
    /// `track_reversed.vex` beside `zone_track_reversed.vex`. The prefix is
    /// carried rather than hardcoded because it is the first `%s` of the
    /// binary's own path template, and a title that spelled it differently would
    /// change this string and nothing else.
    ///
    /// **Not every circuit has one**, which is a load-correctness fact and not a
    /// menu one - see `oag_raceplay::catalogue::Track::available_in_zone`.
    Prefixed(&'static str),
    /// A Zone race runs a circuit of its own, sharing nothing with the race
    /// ones, and this is the one it opens when the caller names none.
    ///
    /// Pure and HD. There is deliberately **no mapping** from a race circuit to
    /// a Zone one here, because the discs author none: Pure ships eight race
    /// circuits and four Zone circuits, HD sixteen environments of which four
    /// are Zone. Asking "what is Vineta K's Zone variant" is a question with no
    /// answer in the data, so this variant answers the only question that does
    /// have one.
    ///
    /// **Every Zone circuit this title has, first entry first.** Not one
    /// string: a caller can still name any of the four directly - Pure's own
    /// four `type="Zone"` circuits, HD's `zone_1`..`zone_4` (declared
    /// `type="Race"` with a `zone="true"` flag instead, not `type="Zone"`;
    /// see [`Self::menu_tracks`]'s own docs for why the query has to match by
    /// name rather than by `type`) - and
    /// [`Self::variant_of`] needs the whole list to tell "the player picked a
    /// Zone circuit" from "the player picked a race circuit and this is Zone
    /// mode" - the two cases a menu that always names *something* cannot
    /// distinguish on its own. The first entry is what [`Self::default_track`]
    /// answers with, on the same "the disc's own ordering is the best
    /// available answer" terms as [`RaceDefaults::track`](super::RaceDefaults::track)'s own docs give.
    ///
    /// Never empty on a title actually wired up this way - see the
    /// `oag_pure`/`oag_hd`/`oag_2048` construction sites, each with its own
    /// evidence for what belongs in the list.
    ///
    /// **`also_race_circuits` says whether every ordinary race circuit is
    /// *also* offered in Zone mode, alongside this variant's own four -
    /// `false` on Pure, `true` on HD/Fury.** This is unverified against the
    /// executable, unlike everything else on this type: it rests on a
    /// play-based recollection (2026-08-28) that ordinary HD/Fury circuits -
    /// Anulpha Pass, Moa Therma among them - carry a "zone pendant" in the
    /// original menu, which is not something any disc data this codebase
    /// reads encodes. Two things do corroborate it. First, the four `zone="true"`
    /// entries are not generic placeholders: read against HD's own string
    /// table (`DATA06`'s `entries.xml`, the copy that names all 28) they are
    /// **Pro Tozo, Mallavol, Corridon 12 and Syncopia** - real, distinct
    /// track names, never declared with a reversed variant the way every one
    /// of the other twelve is, consistent with being Zone-exclusive content
    /// rather than the substrate a per-circuit Zone effect would be laid
    /// over. Second, HD ships a title-wide `zonemode.effectsettings`
    /// (`/data/environments/zonemode.effectsettings`, plus a
    /// `zonemodedlc3.effectsettings` revision) that nothing in this engine
    /// reads yet - the same "any circuit + a colour-grade effect" shape 2048
    /// confirmed for its own `SameCircuit`. Neither is a decompiled selector.
    Separate(&'static [&'static str], bool),
    /// A Zone race runs the exact circuit a race would have, and nothing is
    /// derived or substituted at all. 2048.
    ///
    /// The third shape, and not a variant of either of the other two: not
    /// [`Self::Prefixed`], because there is no second `.vex` beside the race
    /// one to derive a name for, and not [`Self::Separate`], because there is
    /// no dedicated Zone circuit to point at - 2048 runs Zone on whichever
    /// circuit the player already picked.
    ///
    /// **Source: played, not decompiled or name-probed.** This used to be
    /// [`Self::Separate`], pointed at `zone_1` of four Zone-named environments
    /// `dlc2.psarc` ships - `docs/formats/track.md` had already shown those
    /// four are Wipeout HD's own dedicated Zone circuits, reshipped verbatim
    /// splines and all, which read as the Pure/HD shape reused. Playing the
    /// game said otherwise: 2048's Zone mode ran on an ordinary circuit, not
    /// a separate environment. That agrees with a fact [`RaceDefaults::zone`](super::RaceDefaults::zone)
    /// already carried unread on this title - every circuit in the base
    /// package ships a `ZoneMode2048.effectSettings` beside its `track.vex` -
    /// which only makes sense if Zone runs on the circuit that file sits
    /// next to. The four ported `zone_N` environments are real disc content;
    /// they are simply not what this axis answers. Confidence 90, on the same
    /// bar the rest of `oag_2048::race` holds itself to for an unread
    /// executable: a direct behavioural observation, not yet corroborated
    /// against a decompiled selector or a read `effectSettings` format.
    SameCircuit,
}

impl ZoneCircuit {
    /// The circuit a Zone race opens when the caller named one.
    ///
    /// [`Self::Prefixed`] rewrites it, because on such a title the name the
    /// caller gave is a race circuit and its Zone twin is derivable.
    ///
    /// [`Self::Separate`] hands it back untouched **only when it already names
    /// one of this title's own Zone circuits** - checked case- and
    /// separator-insensitively, the same fold a PSARC container applies before
    /// it looks a path up, so `--track` and a menu-built entry name agree on
    /// what counts as a match. Anything else - which is what a menu always
    /// hands this when the player has not touched the Circuit row since
    /// picking Zone mode, because that row is filled with race circuits - falls
    /// back to the title's own default instead of being raced as though it
    /// were a Zone environment.
    ///
    /// **This used to hand back whatever it was given, unconditionally.** On a
    /// title with no per-circuit Zone mapping to derive, that read as "nothing
    /// to derive, so trust the caller" - but the caller is a menu that always
    /// names *some* circuit, Zone-shaped or not, and the untouched name it
    /// hands back most of the time is a race circuit's own environment. A Zone
    /// race on Wipeout HD or Wipeout Pure then flew the picked race circuit
    /// under Zone's rules rather than any Zone environment at all, and the
    /// load report said `racing {track} as named` - which reads as success.
    /// Trusting the caller was right; trusting it to have named a Zone circuit
    /// specifically was not, and this is the fix.
    ///
    /// **Idempotent under `Prefixed`**: a name already carrying the prefix comes
    /// back unchanged, so `--track ...\zone_track.vex --mode zone` asks for
    /// `zone_track.vex` rather than `zone_zone_track.vex`. `Separate` is
    /// idempotent too, for the same caller-facing reason: a name already in the
    /// list is recognised on the second pass exactly as on the first.
    ///
    /// **`also_race_circuits` skips the substitution check entirely** and
    /// hands the name back unconditionally, the same as `SameCircuit` - on
    /// a title wired this way, an ordinary race circuit is itself a
    /// legitimate Zone pick (see [`Self::Separate`]'s own docs), so there is
    /// nothing left to fall back from. Offering it in the menu and then
    /// substituting `tracks[0]` underneath it regardless would reproduce
    /// exactly the bug this method exists to close, one layer further down.
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
    /// `race_default` is [`RaceDefaults::track`](super::RaceDefaults::track), which is what a race would
    /// have opened. [`Self::Prefixed`] derives its Zone twin from it, so the
    /// title's opening circuit stays the opening circuit in both modes;
    /// [`Self::Separate`] ignores it and answers with its own first entry -
    /// even when `also_race_circuits` also offers every ordinary circuit in
    /// the menu, opening on the title's own dedicated Zone circuit first is
    /// still the best available answer with nothing named yet, on the same
    /// terms [`RaceDefaults::track`](super::RaceDefaults::track)'s own docs give.
    #[must_use]
    pub fn default_track(self, race_default: &str) -> String {
        match self {
            Self::Prefixed(_) => self.variant_of(race_default),
            Self::Separate(tracks, _) => tracks[0].to_string(),
            Self::SameCircuit => race_default.to_string(),
        }
    }

    /// Which circuits a menu's CIRCUIT row should offer when Zone is the
    /// selected mode, given every circuit a normal race offers.
    ///
    /// **The dispatch this axis exists for, and the reason it takes closures
    /// instead of a definition to parse itself.** [`Self::Prefixed`] filters
    /// `race_tracks` down to the ones flagged `availableInZone`, because on
    /// such a title the Zone circuits *are* the race ones, just missing the
    /// prefixed file for eight of Pulse's twenty-four; [`Self::Separate`]
    /// asks `by_own_names` for the tracks it names itself, because Pure and
    /// HD each declare their own four Zone circuits outside the race listing
    /// entirely; [`Self::SameCircuit`] hands `race_tracks` back unfiltered,
    /// the same list a race offers, because 2048 runs Zone on whichever
    /// circuit is already picked.
    ///
    /// **`by_own_names` asks by name, not by `type`.** It used to ask
    /// `of_kind("Zone")` instead - which read every declared Zone circuit as
    /// `type="Zone"`, true of Pure but not of HD/Fury, whose own four are
    /// `type="Race"` carrying a separate `zone="true"` flag (measured
    /// 2026-08-28 against `hdfury-ps3-eu-dec.iso`, all four and only the four
    /// pointing at `Zone_1`..`Zone_4`). A kind query answered empty on HD, so
    /// Zone mode's CIRCUIT row offered nothing there. Asking by the name list
    /// this variant already carries sidesteps the question: it does not care
    /// what `type` said, matching either title's declaration shape the same
    /// way, and it cannot answer with a circuit the title itself does not
    /// name as one of its own.
    ///
    /// **`also_race_circuits` appends every ordinary race circuit after this
    /// variant's own four, own circuits first** - unverified, see
    /// [`Self::Separate`]'s own docs for the evidence and its limits. A race
    /// circuit already present under its own name (true of HD's four Zone
    /// circuits, which are also `type="Race"` entries in `race_tracks`) is
    /// not duplicated.
    ///
    /// Generic over `T` and closures rather than naming
    /// `oag_raceplay::catalogue::Track` and calling `oag_raceplay::catalogue`
    /// directly: `oag-title` sits beneath `oag-game` in the dependency graph
    /// (`docs/architecture/workspace-layout.md`) and must not reach up to it.
    /// `available_in_zone` and `by_own_names` are the two questions
    /// `crates/raceplay/tests/zone_ground_truth.rs` asked by hand, once per
    /// title, before this method existed; this is that dispatch moved to the
    /// one place that has to agree with itself everywhere it is asked - the
    /// title axis, not a caller (a ground-truth test, a menu, `--menu-page`)
    /// repeating the match.
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
/// Mirrors the fold [`oag_assets::psarc`](oag_assets)'s own `normalise`
/// applies before it looks a path up - leading `/` trimmed, `\` folded to `/`,
/// ASCII-lowercased - so [`ZoneCircuit::Separate`]'s membership check agrees
/// with the archive about which spellings name the same file. Repeated here
/// rather than shared: `oag-title` describes a title and does not depend on
/// `oag-assets`, which reads one.
fn paths_match(a: &str, b: &str) -> bool {
    fn normalise(path: &str) -> String {
        path.trim_start_matches('/')
            .replace('\\', "/")
            .to_ascii_lowercase()
    }
    normalise(a) == normalise(b)
}
