---
categories: [frontend, rendering]
---

# Zone flies its own environment now; the menu is the part still not mode-aware

2026-08-19, branch `worktree-zone-aesthetics`. **Zone's whole look is authored and was simply not being loaded** - a Zone circuit is a separate `.vex` with its own meshes, lights, `fogCube` and one-material `Skycube` (the twelve on the PSP disc that [skycube.md](../../docs/formats/skycube.md) counts are exactly the Zone variants), so nothing here tints or invents anything. `oag_title::ZoneCircuit` is the axis and it is one of the few **measured on all three titles**: Pulse keeps the file beside the race circuit under a `zone_` prefix, Pure declares four `type="Zone"` circuits under `Data\Zone\`, HD ships four `zone_N` environment directories - two shapes, three titles, so neither enum variant is designed from one example and there is no third "unknown" state. Evidence and confidences on [track.md](../../docs/formats/track.md) and [race-modes.md](../../docs/gameplay/race-modes.md); the sweep is `crates/game/tests/zone_ground_truth.rs`, six tests green across all four discs. **`availableInZone` is load-correctness, not menu data** - probed by name on all 24 Pulse entries and the attribute predicts the file **24 of 24** (16 present, 8 absent), and 22 of 32 on the PS2 disc; a circuit without it carries no Zone environment at all. **Three things worth knowing.** (1) **A real bug fell out of this**, in `oag_mesh::mesh::build_optional_class`: it returned an empty model for an *unrecovered class id* but **errored** for a class the file does not author, and `16_Track\zone_track.vex` authors no `Weapon Pad` - the disc agreeing that Zone has weapons off - so every Zone race failed to load with `decoded to no triangles` while the file parsed perfectly. Fixed by walking the tree for the class, so "authored but decodes to nothing" still fails loudly. Three doc comments had promised the empty-model behaviour for months; no shipped *race* track exercised it. (2) **The racing line is authored per environment, not shared** - it looked shared because `16_Track` splines identically in both (862 points either way): `10_Track` is 844 race / 848 zone and `26_Track` 847 / 852, so 2 of 16 move (the start line has since been derived from each file's own spline - `Course::START_LINE_ADVANCE`, 2026-09-16 - so nothing is inherited any more). The test asserts only that a minority move; an equality assertion was written, failed, and was right to delete. (3) **The menu still offers all 24 circuits in Zone, and the Zone HUD reads blank.** The track list is supplied once when the menus open and nothing re-supplies it when MODE changes, so picking a non-Zone circuit then Zone gives a load error naming `availableInZone` rather than a row that was never offered; `catalogue::tracks_of_kind` plus `Track::available_in_zone` are the two listings a mode-reactive filter would read, and **nothing dispatches between them on purpose** - which one applies is a title fact, so it lives in `ZoneCircuit` and not in a catalogue helper with no production caller. Separately, a Zone screenshot shows `Lap`/`Score`/`Zone` positioned correctly and **empty**, which reads as a regression and is the long-standing `IG_HUD_*` key-substitution gap ([hud.md](../../docs/ui/hud.md) scopes Zone out explicitly, [zone-mode.md](../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md) item 2 is the same hole from the RE side). The environment swap touches none of it. **Pure's Zone hull is no longer unrecovered**, and it closes an item `oag_pure::race`'s own module docs carried: Pure expresses the Zone craft as a **team**, `PI_Team name="Zone_01" type="Zone"`, whose `handlingstats.xml` opens `<Stats team="ZoneMode">` - the disc naming the mode in its own words. Recorded as `oag_pure::race::ZONE_TEAM` **with no reader**, deliberately: the craft axis has Pulse and Pure measured and HD unread, which is the two-of-three shape ADR-0022 does not license, where the circuit axis has three and no hole. Not to be confused with the `PI_Team name="Zone"` beside it, which is `type="Race"` - the unlockable livery. **The selector in the executable is unread** for all of this: the `%s\%strack%s.vex` template is recovered but the branch that puts `zone_` in its first `%s` - the analogue of `Ship_LoadModel`'s `case 6` for the hull - has not been found, which is what caps every claim here at 94 and is why **no `names.tsv` row landed**: everything recovered is shipped data, not a Ghidra rename.

## Open

- The menu still offers all 24 circuits in Zone mode instead of filtering to the Zone-flagged ones - the track list is supplied once when menus open and isn't re-supplied when MODE changes.
- The executable's own selector - the branch that puts `zone_` into the `%s\%strack%s.vex` template - is unread, which caps every finding here at confidence 94.
- **Wipeout HD's own Zone counter has nothing to draw it with**, and that is a real gap rather than the item below re-surfacing: HD's `zone_hud.xml` authors no `"Zone"` text widget at all, only `ZonePlus0`-`ZonePlus10` (statically-labelled `"1"`-`"10"`) beside `ZonePlusLight0`-`10` image widgets - a light-up dial, not a number readout - plus `SpeedClass`/`NextSpeedClass` text widgets that likely pair with the `MR_*` speed-class voice lines in `speech_zone.bnk`. None of it is read on HD's own executable. See `docs/formats/psp-audio.md#speech_zonebnk-names-the-zone-announcer-one-ladder-per-title` and `docs/ui/hud.md`.

**2026-08-28: the item above it is fixed, and was stale by the time this paragraph was written rather than by anything in this session** - `"Zone"` and `"Score"` in `crate::hud::draw::text_for` (`crates/hud/src/draw.rs`) draw the counter and the score on Pulse, Pure and HD (HD's own layout does carry a `"Score"` widget, just no `"Zone"` one - see the new Open item). What was still genuinely missing was the **announcer**: `outcome.zone_advanced` was computed and never read. It is wired now, off each title's own `speech_zone.bnk` - a different milestone ladder per title, read straight off the shipped audio (`oag-wad sounds`) rather than assumed from Pulse - `oag_sound::sfx::Announcer`, `docs/formats/psp-audio.md`'s new section, and the correction to [zone-mode.md](../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md#the-ten-second-step). **Wipeout 2048 gets the announcer now too, off a Ghidra sweep rather than the shipped audio.** Its `data.psarc` is still not extracted in this tree, so `oag-wad sounds` cannot check it the way the other three were checked - but `data/audio/sound/speech_zone_NGP.bnk` and a fifteen-entry ladder matching HD's were read off the executable's own control flow: a real decompiled dispatch (`FUN_812b5890`/`FUN_812b0880`, both one-liners) picks between two live Zone speech banks by the selected circuit's pack id, and the base-package `DEFAULT_TRACK` reads as taking the `NGP` one. The milestone table's own reading function is still not found, despite every direct reader of the Zone speech handle being checked this pass - see [zone-audio.md](../../docs/ghidra/functions/vita-2048-eu-v104/zone-audio.md) for the full sweep and what would close it.

**2026-08-28: the first item above is fixed.** `oag_title::ZoneCircuit::menu_tracks` is the mode-reactive filter: it dispatches between filtering the race list by `available_in_zone` (`Prefixed`) and asking `catalogue::tracks_of_kind` for `type="Zone"` entries (`Separate`), or handing the race list back unfiltered (`SameCircuit`, 2048) - the same choice `crates/game/tests/zone_ground_truth.rs`'s `zone_circuits` helper used to make by hand, moved to the title axis and now exercised by that same ground-truth sweep instead of duplicated by it. Boot resolves the result once alongside the race list (`boot::Shell::zone_tracks`, off the new `boot::roster::load_zone_tracks`), and `Shell::tracks_for(mode)` in `crates/game/src/main/session.rs` is the one place the menu layer picks between the two lists - `open_menus`'s initial supply, the `LaunchRace` handler's stored-circuit lookup, and a new `Session::resupply_tracks_for_mode` wired off `apply_setting("race.mode")` so an already-open menu's CIRCUIT row updates the moment MODE changes rather than only on the next menu open. All 9 `zone_ground_truth` tests and the full `just` gate pass against the real Pulse/Pure/HD images. Confidence stays where the rest of this thread put it (94): nothing here reads the executable's own selector, which is the second Open item below.

**2026-08-28, later the same day: the fix above regressed HD specifically, and is now fixed again on better evidence.** `menu_tracks`'s `Separate` arm asked `catalogue::tracks_of_kind(xml, "Zone")` - correct for Pure, which does declare `type="Zone"`, but HD's own four (`25_Track`..`28_Track`, `Zone_1`..`Zone_4`) are `type="Race"` carrying a separate `zone="true"` flag instead (confirmed against `hdfury-ps3-eu-dec.iso`'s `definition.xml`: exactly four `zone="true"` attributes among 28 `PI_Track` entries, all and only the four Zone environments). A kind query answered empty on HD, so Zone mode's CIRCUIT row offered nothing at all - worse than the pre-fix "offers all 24," which is how the user caught it starting the very next session. Fixed by having `Separate` ask **by the name list it already carries** (`oag_raceplay::catalogue::tracks_named`, new) instead of by `type` - matches Pure's `type="Zone"` entries and HD's `zone="true"` ones the same way, since it never asks what `type` said. All 9 `zone_ground_truth` tests pass against the real discs, `hd_keeps_four_zone_environments_of_its_own` now also asserting the menu offers exactly the four `ZONE_TRACKS` entries. **Surfaced a bigger, still-open question while fixing this**: see `2048-and-hd-ship-an-unread-effectsettings-table.md`'s new Open item - a user's play recollection is that ordinary circuits (Anulpha Pass on HD, Moa Therma on Pulse) carry a "zone pendant" in the original menu, which may mean HD's real Zone mode is not `Separate`-only the way this fix assumes, but also lets a player pick an ordinary circuit dressed with the title-wide `zonemode.effectsettings` grade that thread already found and left unwired. Today's fix is the minimum correct read of what the disc's `PI_Track` data alone supports; it is not necessarily the whole mechanism.

**2026-08-28, same session, later still: the pendant question is implemented, on the user's explicit call to do so without the executable read first - and the "Moa Therma isn't on HD" claim two paragraphs up is wrong, corrected here.** Checking it required looking a `PI_Track` id up in the *right* string table, not guessing from its directory name: HD's own `03_Track` (a numerically-named, non-descriptive folder, unlike `15_Anulpha_Pass`) resolves to `MOA THERMA` against `DATA06`'s `entries.xml` - the copy that names all 28 ids, per `oag_game::language::CircuitNames`'s own docs. It is on HD/Fury after all, plainly missed by grepping the disc's folder names for a substring instead of reading the id through the string table the game itself uses. Two things corroborate the wider mechanism this session then implemented: first, the four `zone="true"` circuits are not generic placeholders either - read the same way, `25_Track`..`28_Track` are **Pro Tozo, Mallavol, Corridon 12 and Syncopia**, four real, distinct tracks that never carry a `reversed="true"` sibling the way all twelve ordinary circuits do, consistent with being Zone-exclusive rather than a substrate an effect would run over. Second, `zonemode.effectsettings` is real, title-wide, unread content already found on this thread's sibling. `oag_title::ZoneCircuit::Separate` now carries a second field, `also_race_circuits: bool` - `false` on Pure (still exactly its own four, verified), `true` on HD (unverified: `menu_tracks` appends every ordinary race circuit after the four zone-exclusive ones, and `variant_of` stops substituting entirely on this title, so picking Anulpha Pass in Zone mode now races Anulpha Pass itself rather than being silently rewritten to `Zone_1` - the load-time half of the fix, without which a widened menu would have quietly lied about what it would race, the exact bug class `variant_of`'s own fix closed for the narrow case). Ten `zone_ground_truth` tests and the full `just` gate pass against the real discs, including a new HD-specific load test proving the geometry is genuinely unsubstituted. **This is explicitly the unverified branch of the fork the paragraph above left open** - no executable selector was read, no `zonemode.effectsettings` was wired into rendering, and the confidence this carries is a play-based recollection plus two corroborating disc facts, not a decompiled dispatch. See `2048-and-hd-ship-an-unread-effectsettings-table.md`'s Open item for exactly what would raise it.

**2026-09-03: `also_race_circuits: true` is narrowed but still unverified - the front end's own layout rules out one whole shape, and the walk that would settle the rest is written and blocked on machine setup, not on this session.** Full account and the exact confidence in
[hd-frontend.md](../../docs/formats/hd-frontend.md#zone-is-a-mode-list-entry-not-a-separate-screen---and-the-walk-that-would-settle-whether-it-widens-is-blocked-on-hardware-access);
summary here. `racebox_definition.xml`'s `Single Player` screen puts `Zone` in
the same five-entry `Mode` list as `Arcade`/`Time Trial`/`Speed Lap`/`Tournament`,
and its `Redirect` sends every mode but `Tournament` to the identical `Track
Creation` screen - **no dedicated Zone circuit-select screen exists**, and the
shared `Track` list's own layout authors a `Padlock` and `ReverseIcon`s per row,
no Zone pendant/badge widget. That rules out a structurally separate,
narrower Zone screen (a real alternative this thread's `also_race_circuits`
guess had to leave open). It does **not** settle what the shared list's
*contents* are once `Mode` is `Zone` - the code that builds/filters that list
is unlocated (this Ghidra database names exactly one front-end function,
`FrontendRoot_Construct`), and the one existing capture of this carousel
(`hd-frontend.md`'s `DATA06` section) walked it without ever selecting `Zone`,
so it describes Arcade's list, not Zone's. **The decisive experiment is an
emulator capture, not a decompile, and it is already written**:
`scripts/rpcs3-drive.py browse --nav "Main Menu=right" --nav "Single
Player=down,down,down,down,cross" --screen "Track Creation" --button right
--steps 28` selects `Zone` (four `down`s off the list's `default="Arcade"`)
then walks the exact carousel already known how to read. **Blocked this
session on `/dev/uinput` permissions**, not a repository or code problem:
`just rpcs3-preflight` reports it mode `0600` group `root` with no udev rule,
and `sudo -n true` confirms no passwordless sudo, so the one-time fix
(`echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee
/etc/udev/rules.d/99-uinput.rules && sudo udevadm control --reload-rules &&
sudo udevadm trigger /dev/uinput`) needs the machine's own user to run it
once. After that, running the command above and reading `data/` for either
four rows or 28 closes this thread's central open question outright.

**2026-09-03, later the same day: asked directly, and the user's own
recollection of the real Zone-mode picker is the wide shape.** Offered the
udev fix above, the user did not take it up; asked instead what the Zone-mode
carousel showed on real hardware, their answer was **"the full track list,
with a few zone-only ones mixed in"** - exactly `also_race_circuits: true`'s
own shape (all 28 `PI_Track` entries, the four zone-exclusive ones among
them), not a narrow zone-only carousel. This is a second, independently-asked
play observation - the pendant recollection that started this fork was
volunteered while fixing something else; this one directly answers the
question the capture command above was written to settle. It is still a
recollection, not a capture or a decompile, so it does not move the
confidence number, but it is the second piece of play evidence pointing the
same way and there is now none pointing the other way.

## Next Steps

- ~~Wire `zonemode.effectsettings`/`zonemodedlc3.effectsettings` into rendering for the ordinary circuits now offered~~ **Done, landed on the sibling thread, not this one.** `2048-and-hd-ship-an-unread-effectsettings-table.md` records the render-application half landing 2026-08-30 ("a Zone race now grades its circuit off this table") and the HD trigger closing 2026-08-31 (`oag_hd::race::ZONE_STAGES`, "an HD Zone race escalates its colour grade"). That thread is now twenty-plus Ghidra passes deep into a single narrow shader-parameter question (which of two parallel publications a given material draw enters through) and blocked on vector-store write watchpoints in the patched RPCS3 build - not something to wander into from here.
- ~~Decompile HD's own Zone-mode circuit-select screen to either confirm or correct `also_race_circuits: true`~~ **Reframed, 2026-09-03: there is no dedicated screen to decompile** (see the new section above) - the open question is who populates the *shared* `Track` list's contents for `Mode=Zone`. **A first Ghidra dig happened this session and did not settle it** - see the new section below. The `rpcs3-drive.py browse` capture is still the fastest route to a decisive answer, once `/dev/uinput` is fixed on this machine; two independent play recollections already agree with the wide reading and none disagrees.
- The remaining two Open items above are both read-the-executable work - the `%s\%strack%s.vex` selector branch and HD's own Zone-counter widgets - not an engine change with a next step to name yet.

## 2026-09-03, a Ghidra dig into the front end: `TrackSelection_Screen` found and read, no mode filter located in it

First pass into this binary's front-end/GUI layer at all (previously one named function
total, `FrontendRoot_Construct`). Full evidence and three new names on
[`docs/ghidra/functions/ps3-hdfury-eu/track-selection-screen.md`](../../docs/ghidra/functions/ps3-hdfury-eu/track-selection-screen.md).

Found the C++ class implementing the `Track Creation` screen (`TrackSelection_Screen`,
via the same `.cpp`-filename-allocator-tag trick `memory.md`/`mode-manager.md` already
established) and read its constructor pair, static type-registrar, `OnConfirm` handler
(confirmed: persists the selected row into game state keyed by the literal string
`"Track"`), and what is very likely its `OnEnter`/show handler (`_q`-suffixed, 68 -
reached only via an indirect vtable slot, no direct caller to confirm *when* it fires).

**The `OnEnter` candidate builds a fixed 32-row visual widget pool and, separately, reads
an already-resolved item count off the Track list widget object - it does not compute or
filter that count itself, and no mode/Zone check appears anywhere in the population
loop.** The only Zone-adjacent code found anywhere in this class reads `g_GameState`'s
mode field for two purely cosmetic decisions (hiding one unidentified sub-widget for
Zone-Battle/Elimination specifically, and picking which record-panel labels to draw for
the focused row) - never for which tracks are listed. So: the actual population/binding
of the Track list's contents is **not in `TrackSelection_Screen`'s own code** - it is
either inside a generic, shared `List`-widget framework class (unexplored, no name or
string handle found yet - this would be a fresh, unbounded search, the framework layer
underneath every screen's `<List>`, not just this one), or primed by something upstream of
this screen's construction. `RaceBox_Screen` (the `Single Player` screen, where `Mode` is
actually chosen) was checked lightly and ruled out as the filter site too - its "confirm"
handler reads the final Mode/Track/laps selections to derive weapon-class/HUD flags for
race launch, not to populate a list.

**Net effect on confidence: unchanged (94), but the shape of the remaining uncertainty
narrowed.** Absence of a filter in the one class that plausibly would have carried it is
consistent with the wide `also_race_circuits: true` reading (nothing here would produce a
narrower Zone-only list), but it is not proof - the generic List-widget framework is still
unread and could yet carry mode-aware filtering the screen-level code never touches.

**A new, unrelated trap surfaced and is recorded in the scratchpad in full**: this
`ghidra-mcp` instance is shared across concurrent sessions, and `switch_program` sets a
global pointer any concurrent call can move - three renames without an explicit `program`
argument silently "succeeded" against a different, wrong program (`ps2-pulse-eu`, not
`ps3-hdfury-eu`) before this was caught by a follow-up `get_current_program_info` check
and redone correctly with `program` passed explicitly on every mutating call. All three
renames now verified landed on `ps3-hdfury-eu`.

Working notes with every address checked and every dead end: not committed
(untracked scratchpad, deleted along with this thread file when the work lands, per this
project's own convention for scratch-style working files) -
`hd-zone-tracklist-re-notes.md` in the worktree this session ran in, for
anyone picking this up in the same checkout.

**Next concrete lead**: find the generic `List`-widget framework class's own item-count /
data-binding code. No name or string handle for it is known yet - the search would start
from the vtable read technique this page's evidence used (`PTR_PTR_008b1ecc` and its
neighbouring shared slots, which are the *inherited*, not per-screen, methods this pass
deliberately skipped over), or from a fresh string search for whatever XML attribute name
racebox_definition.xml's `<List>` elements use to name their data source (not yet grepped
against the disc's own `racebox_definition.xml` for a `Data=`/`Source=`-shaped attribute).
Otherwise, the `rpcs3-drive.py browse` capture already written above remains the direct
route once the machine's `/dev/uinput` permission is fixed - still the faster and more
decisive path if a maintainer's session has that fixed before the next Ghidra pass.
