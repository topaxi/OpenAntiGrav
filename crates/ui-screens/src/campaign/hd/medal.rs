//! The `Hexmedal_HD` atlas crop and the difficulty-rung text it sits beside -
//! split out of `hd.rs` under the 1,000-line rule
//! (`scripts/check-file-size.py`). `pub(super)`: every item here is an
//! internal helper `super::hd_cell_draw_list` calls, not part of this
//! crate's own public API.

use oag_tables::race_campaign::{Cell, Difficulty, Medal, Mode};

use oag_ui::frontend::{Draw, Placed};
use oag_ui::language::StringTable;
use oag_ui::screen::Image;

use super::super::draw::sprite_draw;

/// Where one medal tier's own static icon sits in `Hexmedal_HD.mip`/`.gtf` -
/// **measured off the disc's own authored crop, not eyeballed**:
/// `DATA06.PSARC`'s own copy of `CellMode_Definition.xml` (`data/fe/frontend/gui/cellmode_definition.xml`)
/// draws the same shared atlas a third way, on `Cell Selection`'s
/// `Target0/1/2 Medal` widgets, and *those* author the crop this build never
/// had for `Medal_{x}_{y}`: `width="60" height="60" u="0" v="0"` for
/// `Target0 Medal`, `v="61"` for `Target1`, `v="122"` for `Target2`,
/// `TxtrWidth`/`TxtrHeight` both `60` on all three. `Target0`/`Target1`/
/// `Target2` are already gold/silver/bronze elsewhere on this same screen
/// (`hd_cell_draw_list`'s own `"Target0" => hd_target_value(targets.gold, ..)`
/// arm and siblings, and `docs/ui/campaign-screens.md`'s note that this
/// widget family replaces the disc's own `IG_HUD_GOLD`/`SILVER`/`BRONZE`
/// labels), so the atlas reads **gold at the top, descending** - confidence
/// 90: a real widget's own authored attributes, for the identical texture,
/// on the same title's own screen family, not a decompile or a capture.
///
/// **`Target0/1/2 Medal` stopped being evidence-only and started being
/// drawn, 2026-09-27**, when `hd_cell_draw_list`'s own screen source switched
/// from `DATA02` to `DATA06` (see the module doc's "the winning archive"
/// section) - before that switch this doc's own numbers were read off
/// `DATA06` but the running screen still drew `DATA02`'s plain grey
/// `Target0/1/2 Image` arrow instead. `image_draw` draws the widget's own
/// authored crop directly now (`hd_medal_frame`'s numbers below still cover
/// `Medal_{x}_{y}`, the unauthored hex-grid badge, which is a different
/// widget on a different atlas placement rule).
///
/// **Cross-checked against the raw texel data independently**, and the two
/// disagree on which end is "top" until the discrepancy itself is
/// explained: decoding `data/fe/images/hexmedal_hd.gtf` directly
/// (`oag_texture::gtf::Gtf::parse`, `crates/texture/examples/scratch_hexmedal.rs`,
/// not committed) shows a 1024x256 DXT5 atlas whose *raster* row order is
/// the authored one upside down - the band this decode puts at
/// `y=196..256` (reaching the texture's own bottom edge exactly) is the one
/// whose mean RGB reads warm yellow-gold, matching `Target0`'s own gold at
/// `v=0`. That is consistent with a Y-flip between this decoder's raster
/// order and the GPU's own `V` convention (unmeasured *which* of the two is
/// "backwards" - nothing here decodes a second, independently-written GTF
/// reader to settle it) rather than two different textures: the row order,
/// the row height (60, matching `TxtrHeight`) and the per-row colour (one
/// warm, one neutral grey, one warm) all agree once the flip is accounted
/// for. The authored numbers below are what a caller already trusts
/// elsewhere in this same file (`Target0 Medal`'s own widget, wired as
/// ordinary XML), so they are used verbatim rather than the raster reading.
///
/// **Which column is "the" icon is not independently measured, and says
/// so** - `Target0 Medal`'s own `u="0"` picks the same first frame this
/// function already uses, but nothing pins whether the real screen holds
/// still on it or animates a spin through the rest of the row (the atlas
/// is far wider than one 60px frame). No earned-medal capture exists to
/// check either way - the same gap `medal_argb`'s own doc already records.
/// `u=0` is used because it is what the disc's own comparable widget uses,
/// not a guess.
///
/// **`difficulty` (`0` easy/novice .. `2` hard/elite, [`super::super::CellSelection::difficulty`]'s
/// own convention) selects one of three vertical blocks, not just a row.**
/// Measured 2026-09-28, against `DATA04.PSARC`'s own `1024x768` copy of this
/// texture (see [`oag_hd::campaign::PER_DIFFICULTY_MEDAL_ARCHIVE`]'s doc for
/// why that is the copy loaded): three `1024x256`-shaped bands, each holding
/// its own gold/silver/bronze row at the *same* `61`-pitch spacing the
/// authored `v=0/61/122` numbers above already give, separated by an
/// **unauthored** measured pitch of `183` (`60 + 1` gap, three rows, `3*61
/// = 183`) - read directly off the decoded atlas's own content bands
/// (alpha-per-row), not an authored crop; no widget on either archive's
/// `CellMode_Definition.xml` sources a `v` past `122`, so there is nothing
/// to cross-check this pitch against the way [`super`]'s own doc
/// cross-checks the `61` spacing. Confidence 60 on the pitch number itself,
/// still measured rather than guessed.
///
/// **Block-to-difficulty mapping is measured, 2026-09-28, confidence 90.**
/// The three blocks hold visibly different icon *shapes*, not only colour -
/// a plain, unadorned hex (bottom of the raster, nearest `v=0`), a
/// hook/"cane"-shaped emblem (middle), and a swirl/spiral emblem (top of the
/// raster, farthest from `v=0`), after the same raster/`V`-flip this file's
/// own crop-reconciliation paragraph above already establishes. An RPCS3
/// capture toggling `DifficultyButton` on `grid8_3_1` (`Cell Selection`,
/// `Fury`, `Race`/Talon's Junction/Venom, the first-reached cell on a
/// default walk) read the `Target0/1/2 Medal` row twice per rung across one
/// boot: `NOVICE` -> plain hex, `SKILLED` -> cane, `ELITE` -> swirl, paired
/// to each frame's own `AI DIFFICULTY (<rung>)` footer text rather than by
/// press count (`docs/reverse-engineering/rpcs3-capture.md`'s "Cell
/// Selection: DifficultyButton toggle, per-rung icon shape" section has the
/// full capture and this project's own matching render). That confirms,
/// rather than falsifies, the convergent-evidence reading this doc
/// previously reasoned to before any capture existed: `DATA02`'s shorter,
/// flat-schema copy (the one every pre-Fury cell effectively used before
/// per-difficulty targets existed) carries *only* the swirl shape, and
/// `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s
/// `SaveData_MigrateCellMedalsToHardElite` credits exactly that
/// pre-existing, single-tier medal at `HARD`/`ELITE` when migrating an old
/// save - the swirl icon and the hardest rung are the two things that
/// pre-existed the difficulty split, and the migration equates them. So
/// `easy/novice = plain hex = v-block 0`, `medium/skilled = cane =
/// v-block 1`, `hard/elite = swirl = v-block 2`, now a live-screen reading
/// rather than reasoned-to. Confidence 90, not higher: one boot, one cell,
/// one mode (`Race`) - not independently reproduced across a second boot or
/// a second mode family (e.g. an `Elimination`/`NitroBattle` cell's own
/// `Novice`/`Skilled`/`Elite`-named target triple).
pub(super) fn hd_medal_frame(medal: Medal, difficulty: Difficulty) -> [f32; 4] {
    const FRAME: f32 = 60.0;
    const BLOCK_PITCH: f32 = 183.0;
    let tier_v = match medal {
        Medal::Gold => 0.0,
        Medal::Silver => 61.0,
        Medal::Bronze => 122.0,
    };
    let block = match difficulty {
        Difficulty::Easy => 0.0,
        Difficulty::Medium => 1.0,
        Difficulty::Hard => 2.0,
    };
    [0.0, block * BLOCK_PITCH + tier_v, FRAME, FRAME]
}

/// `Medal_{x}_{y}`'s own draw: [`hd_medal_frame`]'s crop, drawn at its own
/// native 60x60 size, **at the widget's own authored `image.x`/`image.y`** -
/// unchanged from the pre-fix code, which already drew there (the size and
/// crop were the only things wrong - see the module's own bug writeup in
/// `docs/ui/campaign-screens.md`). **Deliberately not `hex_rect`-centred**,
/// an earlier draft of this fix tried: `CellMode_Definition.xml`'s own
/// `<Item>` grouping shows every `Medal_{x}_{y}` sits in its own item,
/// offset a constant `(+7, +2)` from `Bg_{x}_{y}`/`Outline_{x}_{y}`'s own
/// item at the same grid slot - checked across all seven columns
/// (`(7,2)`/`(67,36)`/`(127,2)`/`(187,36)`/`(247,2)`/`(307,36)`/`(367,2)`
/// against `Bg`'s own `(0,0)`/`(60,34)`/`(120,0)`/`(180,34)`/`(240,0)`/
/// `(300,34)`/`(360,0)`, the same `+7,+2` every time). That is the disc's
/// own registration between the medal layer and the hex layer, authored
/// once and not a per-column tune - a runtime centring formula would only
/// coincidentally reproduce it, and does not: it was tried, produced a
/// visibly-offset badge on every column, and was reverted in favour of
/// this, the simpler and disc-measured choice. **No tint**: unlike Pulse's
/// `hex_filled.mip` (a plain white hex [`super::super::draw::tinted_medal_draw`]
/// still colours with [`super::super::draw::medal_argb`] - that path is
/// untouched, and still correct for Pulse), HD's own atlas frame already
/// carries the tier's colour baked into its texels, so multiplying a flat
/// swatch over it a second time was the other half of the bug this
/// replaces.
pub(super) fn hd_tinted_medal_draw(
    image: &Image,
    placed: Placed,
    medal: Medal,
    difficulty: Difficulty,
) -> Draw {
    let [u, v, frame_width, frame_height] = hd_medal_frame(medal, difficulty);
    let cropped = Image {
        width: Some(frame_width),
        height: Some(frame_height),
        u: Some(u),
        v: Some(v),
        texture_width: Some(frame_width),
        texture_height: Some(frame_height),
        ..image.clone()
    };
    sprite_draw(&cropped, placed, image.x, image.y, 0xffff_ffff)
}

/// `Target Title`'s own text - `DATA06`'s single shared header replacing
/// `DATA02`'s per-target `IG_HUD_GOLD`/`SILVER`/`BRONZE` labels (see
/// [`hd_medal_frame`]'s own doc). **Composed, not read verbatim from the
/// widget's own `idstring="IG_HUD_TARGET"`** - three live RPCS3 frames on
/// this exact widget (the Fury race and speed-lap frames, plus the `Elimination` frame)
/// show text the bare idstring alone cannot produce:
///
/// | Cell | Mode | Frame reads |
/// | --- | --- | --- |
/// | `grid8_3_1` | `Race` | `"TARGET (NOVICE)"` |
/// | `grid8_4_2` | `Speed Lap` | `"TARGET LAP TIME (NOVICE)"` |
/// | `grid8_3_2` | `Elimination` | `"TARGET 200 (NOVICE)"` |
///
/// Three parts, each independently evidenced:
/// - **The base label is mode-dependent.** `Race`'s own is `IG_HUD_TARGET`
///   (`"TARGET"`) unchanged; `Speed Lap`'s is a different idstring entirely,
///   `FE_TLTIME` (`"TARGET LAP TIME"`,
///   the `DATA06` English string entries). `Time
///   Trial`'s own case is inferred, not captured: `FE_TARGTIME` (`"TARGET
///   TIME"`) is the only other `FE_TARG*`/`FE_TL*` idstring on the disc,
///   named for exactly the mode this function has no frame for -
///   confidence 60. Every other mode falls back to the widget's own
///   `IG_HUD_TARGET`, measured for `Race`, chosen by elimination for
///   `Zone`/`Tournament`/`Head2Head`/`Mode::Other`.
/// - **A number is inserted for `Elimination` and `NitroBattle` alike -
///   `"the elimination family"`.** `grid8_3_2`'s own `NitroElimNovice="200"`
///   (`grid_08.xml`) matches the captured `200` exactly; `grid8_3_1`/`grid8_4_2`
///   (`Race`/`Speed Lap`) both author the same field too, always dummy `1`s,
///   and neither frame shows a number - confirming the insert is mode-gated,
///   not "whenever the field parses". `NitroBattle` (`grid8_2_1`) has no
///   frame of its own, but is included on stronger evidence than a guess:
///   `campaign_grids_ground_truth.rs`'s own
///   `eliminationfamily_cells_carry_a_real_nitro_triple_and_a_dummy_flat_one`
///   ground-truths, against the real disc, that `Elimination` and
///   `Mode::Other("NitroBattle")` are exactly the two modes whose
///   `nitro_elimination_targets` is real (non-`(1, 1, 1)`) while
///   `difficulty_targets` is the dummy flat `1`/`2`/`3` - the same pairing
///   this function's own condition below tests for. `Detonator`
///   (`Mode::Other("Detonator")`) is the disc's own mirror image of that
///   (real `difficulty_targets`, dummy nitro triple) and is correctly left
///   out by the same test.
/// - **The `(DIFFICULTY)` suffix is unconditional across all three
///   captures.** Gated here on the cell carrying a difficulty rung at all
///   (`difficulty_targets`/`nitro_elimination_targets` `Some`) since every
///   captured cell is Fury's own (`grid8`, always per-difficulty) - a
///   base-`Wipeout HD` cell reached through this project's own `DATA02`
///   grid precedence parses neither field, so this degrades to no suffix
///   there rather than a guessed difficulty name. Not independently
///   captured either way for a base-HD cell.
pub(super) fn hd_target_title(
    cell: &Cell,
    strings: &StringTable,
    difficulty: Difficulty,
) -> String {
    let base = match cell.mode {
        Mode::TimeTrial => strings.get_or_id("FE_TARGTIME"),
        Mode::SpeedLap => strings.get_or_id("FE_TLTIME"),
        _ => strings.get_or_id("IG_HUD_TARGET"),
    };
    let mut title = base.to_string();
    let is_elimination_family = matches!(cell.mode, Mode::Elimination)
        || matches!(&cell.mode, Mode::Other(name) if name == "NitroBattle");
    if is_elimination_family
        && let Some(target) = cell.nitro_elimination_target_for_difficulty(difficulty)
    {
        title.push(' ');
        title.push_str(&target.to_string());
    }
    if cell.difficulty_targets.is_some() || cell.nitro_elimination_targets.is_some() {
        title.push_str(" (");
        title.push_str(strings.get_or_id(hd_difficulty_id(difficulty)));
        title.push(')');
    }
    title
}

/// `Easy`/`Medium`/`Hard`'s own idstring names, `0` through `2` - the same
/// three ids this disc's own difficulty rungs resolve
/// (the `DATA06` English string entries: `<entry
/// id="Easy" string="NOVICE">`, `id="Medium" string="SKILLED">`, `id="Hard"
/// string="ELITE">`), this title's own words for the same rung
/// (`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s own
/// `Novice`/`Skilled`/`Elite` measurement, the same one
/// [`oag_tables::race_campaign::Cell::nitro_elimination_target_for_difficulty`]'s
/// doc already cites). Confidence 85: the id spelling is a direct disc
/// read; matching it to [`Difficulty`]'s own rung order reuses the same one
/// `Cell::targets_for_difficulty` already assumes, not independently
/// re-verified here.
pub(super) fn hd_difficulty_id(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Easy => "Easy",
        Difficulty::Medium => "Medium",
        Difficulty::Hard => "Hard",
    }
}
