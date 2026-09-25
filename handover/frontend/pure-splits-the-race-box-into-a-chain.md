# Pure splits the race box into a chain, and 2048 has none at all

2026-09-05. The two titles whose race setup is *not* Pulse's, kept in one
thread because both findings are structural absences and both are cheap to act
on. The permanent write-up is
[`docs/formats/race-setup.md`](../../docs/formats/race-setup.md).

## Pure: same screen names, opposite shape

Pure keeps `Track Selection` and `Team Selection` but has **no `Racebox`, no
hex grid and no grid editor** - that branding postdates it. Where Pulse puts
five rows on one `Single Player` page, Pure walks a **linear chain of
one-choice-per-screen menus**:

```
Main Menu -> Single Player (mode only) -> Class Selection -> League | Tournament
          -> Track Selection -> Team Selection -> Launch Game
```

Zone short-circuits the whole thing: `Single Player -> Zone Track Selection ->
Launch Game`, with no class, no league and no team.

### Pure authors a fifth speed class, and an existing thread is asking

`Class Selection` offers **Vector, Venom, Flash, Rapier, Phantom** - five live
`<Menu name="Class">` entries, confidence 94.

`handover/gameplay/is-there-a-fifth-handling-class.md` is open on exactly this question,
and `docs/formats/handling-stats.md` records that three independently-recovered
Pulse subsystems all show four classes and nothing shipped for a fifth, while
noting "Wipeout HD's class ladder does begin at Vector".

**Pure authoring Vector is a data point for that thread and not an answer to
it.** Pure having five says nothing about whether Pulse has five, and the
handling-stats conclusion should not be edited on the strength of it. What it
does establish is that Vector is a real class in this lineage on a *PSP* title,
which is narrower than HD and closer to Pulse than the existing evidence got.

### The rest of Pure

- **The definition file is read, 2026-09-10**:
  `Data\Plugins\PI001\GUI\Selection_Definition.xml`, the same relative name
  Pulse uses for its own race box, resolves on `pure-psp-eu.chd` and holds
  every screen above. The "these files are unread" framing this thread opened
  with had never actually been checked against this exact path.
- **No AI difficulty anywhere in the front-end XML.** Confidence 94. So the
  "base settings like speed class, AI difficulty" shape is half true here.
- **No unlock machinery authored in the XML** - zero `<Unlock>`, zero `Grid=`,
  zero `GSDisableEntriesBitField` across all eleven files. `Show Unlocks`
  exists but is a post-race reward reveal, not a gate. **Gating still
  happens, in code**: a fresh profile's `Class Selection` visibly lists only
  Vector and Venom (PPSSPP capture, 2026-09-10).
- **The one authored preview is a 2D stat graph, and it previews the class** -
  four `<Image>` layers per class inside five `<Watch watch="Class">` blocks,
  all eleven `.mip`s resolved. This is the only flat-2D preview found in any
  title, and it is not a track or a ship.
- Its variant axis is a two-state `<MenuBitmap name="Livery">` toggle against
  Pulse's four-way `skin` cycler.
- `Track Selection` and `Team Selection` author an empty `<Menu allocate="16">`
  and a `<Viewport>` and nothing else **in the XML**. Code fills them with a
  real 3D preview each - confirmed live, not just inferred; see the Open item
  below.

## 2048: the flow does not exist

**2048 has no racebox and no track-selection screen.** Confidence 85. Across 27
`NEWGUI` files there is no `Track Creation`, no `<List name="Track">`, no
`<Model name="TrackModel">` and no `Single Player`-shaped page. The top-level
`Home` screen offers five `TouchButton`s and none of them is "race" - racing is
entered from the campaign event grid (`newFEshell`), drawn by code rather than
authored.

**That is a finding, not a gap.** 2048 replaced the race box with a campaign
grid. It still has a craft-selection screen (`team`, HD's screen renamed, with
a `TouchList` of skins), and the only authored track+class picker anywhere in
it is the crossplay lobby vote.

## Open

- **Pure's track and craft previews are confirmed real (PPSSPP capture,
  2026-09-10) and the track/ship pickers are implemented**, reusing
  `oag_ui::picker`/`oag_game::picker_stage` with `race_box: Some(...)` now set.
  **Both screens draw their entry list and their preview off the disc.** Pure
  previews with a pre-rendered still, not a mesh (RMSE-0 texture match,
  2026-09-10), named in full by the entry's own `screen.xml`.
  `<location>\Ship.vex` / `<location>\track.vex` were wired at confidence ~70
  on 2026-09-10 and removed the same day - they are the in-race hull and the
  full racing circuit. Still open: the stat bars authored in that same
  `screen.xml`, and `TeamSelection_ApplySelection`'s Phantom-model trigger
  (confidence 60, campaign-progress-shaped). Tracked in
  `handover/rendering/the-race-setup-previews-differ-by-title.md`.
- **Pure's screen ordering is capped at 88** - it rests on `<Redirect>` targets
  only and has never been confirmed against a capture.
- **Two of Pure's twelve `LoadXML` entries do not resolve**, and they are
  exactly the two carrying `localised="true"`. The reading that the loader
  rewrites a localised path before hashing is confidence 85; **the rewrite rule
  itself is unmeasured**, confidence under 50.
- `pure-psp-usa.chd` was never opened - every path resolved on the EU pressing,
  so the USA one would only re-confirm. Not a difference, just untested.
- **2048's `.pkg` is not readable by `oag-unpack`** (no ISO 9660 descriptor,
  which is expected); the decrypted `data.psarc` is the way in.
- Whether 2048's campaign grid is *reachable* as a race-setup path for this
  project at all is unexamined. It is code-drawn, so it is a much larger job
  than reading a definition file.

## Next Steps

1. **Add Pure's fifth class as a data point on
   `handover/gameplay/is-there-a-fifth-handling-class.md`** - one paragraph, citing
   Pure's `Class Selection` menu. Do not touch `handling-stats.md`'s
   conclusion. Ten minutes, and it is the highest-value item here because it
   feeds a thread someone is already on.
2. **Done, 2026-09-10**: captured Pure's `Team Selection` and `Track
   Selection` under PPSSPP. Settled that both previews are real; did not
   settle the ordering cap (a fresh-profile walk never needed to prove the
   chain order beyond what `<Redirect>` already states) or the exact file
   each preview loads.
3. Decide whether the localised-path rewrite is worth chasing. It blocks two
   files nobody currently needs, so probably not yet - but it will block the
   language work later, so note it there rather than losing it.
4. Leave 2048 alone for race setup. Its craft screen is the only part that maps
   onto this project's race box, and it is already reachable through the
   existing `remix` page's variant row.

## From the HANDOVER.md index (moved 2026-09-25)

**Pure authors a fifth speed class** (Vector), which is a data point for the open `is-there-a-fifth-handling-class` thread and explicitly not an answer to it - Pure having five says nothing about Pulse having five, and [handling-stats.md](../../docs/formats/handling-stats.md)'s conclusion should not be edited on the strength of it. Pure authors **no AI difficulty anywhere** and **zero unlock machinery** (no `<Unlock>`, no `Grid=`, no `GSDisableEntriesBitField` across all eleven files), and its one authored preview is a **2D stat graph of the speed class** - the only flat-2D preview found in any title. 2048 has no racebox and no track-select screen at all: racing is entered from a code-drawn campaign event grid, which is a property of the title rather than a gap in the measurement. Open: Pure's two previews, its ordering cap of 88 (redirect targets only, never captured), and a localised-path rewrite that hides two of its twelve definition files
