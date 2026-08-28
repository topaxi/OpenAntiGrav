//! `trackstartup.xml`: what a circuit asks to be loaded with it.
//!
//! On HD, one per circuit, 16 on the disc, plain UTF-8 XML with no expansion
//! or escaping. **Pulse ships the same file, per circuit, `fexml`-shortened**:
//! a `<code>` dictionary up top and single/double-letter element and
//! attribute names, the same compression its front-end screens use. Both
//! forms go through [`crate::fexml::text`] first, which expands the
//! shortened case and passes the plain one through unchanged (`is_fexml`
//! keys off the leading `<code`), so this module is the schema on top of
//! either, the same split [`crate::handling`] uses.
//!
//! ```xml
//! <TrackStartup>
//!   <LevelFx><UnderwaterSound type="1"/><WindSound type="1"/></LevelFx>
//!   <LoadSoundBank Filename="env9_talonsjunction.bnk"/>
//!   <Billboard num="1" type="landscape"
//!              location="Data\Billboards\HD_Adverts\Icaras\Looping_Background.vex"/>
//!   <!-- up to eight of these, each with a unique num -->
//! </TrackStartup>
//! ```
//!
//! # Why it is worth reading before anything can act on it
//!
//! **It is a list of things the disc says to load, and this project loads none
//! of them.** Measured over all 16 files: **118 `<Billboard>` entries, 104 of
//! them naming a model - 18 distinct ones, every one present on the disc** -
//! and the other 14 naming a colour instead (see [`Fill`]). Each
//! advert is a self-contained animated scene - around 41 nodes with a `World`, a
//! `Camera` and several `Anim Transform`s, 18 chunks, and its own scrolling
//! materials (`simpletextureandtexturealphauvoffsetscale`).
//!
//! So reading the manifest turns "we do not draw the hoardings" from a silence
//! into a line in the load report that names the eight files it wanted.
//!
//! # The slot numbers are not arbitrary
//!
//! **Slots 7 and 8 are reserved, and slot 8 is the start/finish line.** Over
//! all 16 circuits, slot 8 is `321Go_StartFinish.vex` on every one and that
//! model is in no other slot; slot 7 is `fx350.vex` on every one, likewise
//! exclusive. Slots 1 to 6 are the artist's - nine circuits share one default
//! six, the other seven customise, and four of those fill some with a colour
//! rather than a model.
//!
//! **A player's observation is what prompted the check and agrees with it**:
//! driving HD, slot 8's hoarding is consistently at the start/finish while the
//! rest are scattered through the circuit. The model's own name says the same,
//! and the two together are much stronger than either alone. What that does
//! *not* give is the transform - it says a slot number means a place, not
//! where the place is.
//!
//! # What the executable does with a slot, and what it still doesn't say
//!
//! [`docs/ghidra/functions/ps3-hdfury-eu/billboards.md`](../../../docs/ghidra/functions/ps3-hdfury-eu/billboards.md)
//! reads `TrackStartup_Load` and settles two things this module used to leave
//! open: `num` indexes a plain 9-entry array (`0` wasted, `1`-`8` real, no
//! name lookup - the two stray `Billboard<digits>` nodes are artist debris),
//! and a slot is **instantiated** rather than merely texturing geometry
//! already on screen. It also finds that the engine reads `Colour` (the UK
//! spelling, merged with `Color`) and a numeric `Glow`, and does *not* read
//! `type` at all - all four checked against the real 16 manifests, where
//! `Colour`/`Glow` never appear, so this reader is right to skip them. And it
//! finds a live surprise: slot 7 - authored `fx350.vex` on every circuit - is
//! silently replaced by a mode-specific gate model at runtime, regardless of
//! what the manifest says; slot 8 is not special-cased and keeps its own
//! `321Go_StartFinish.vex` as authored.
//!
//! **Nothing is placed here still.** An advert's chunks are node-local with a
//! bias near the origin, so the circuit must supply a transform, and **where
//! that comes from is unrecovered**: HD's circuits use 30 node classes and
//! `oag_formats::vex` already names all 30, so there is no dedicated slot
//! class; and searching every circuit for a node named exactly
//! `Billboard<digits>` finds one on `02_track` and one on `05_ubermall` and
//! nothing anywhere else, so the artists' names are not the convention either.
//! `GetBillboardMeshIdFromName`, on the same docs page, is the next lead: a
//! name-hash lookup against a per-track billboard list that resets a 4x4
//! matrix on every (re)bind - read, not yet chased.
//!
//! Per `CLAUDE.md`, an asset whose **trigger** is unrecovered stays unwired
//! rather than placed on a guess: a hoarding put somewhere plausible is exactly
//! the kind of legible invention that survives review and removes the pressure
//! to find the real answer. This module parses and reports; a later change
//! places, once something says where.
//!
//! # Cross-title: Pulse ships the same slots, and slot 7 is not reserved there
//!
//! Read directly off 11 of Pulse's 12 circuits (`26_Track` is a `Zone`
//! variant of `25_Track` and carries no `TrackStartup.xml` of its own, the
//! same way a `Zone` circuit skips the other per-circuit files this module's
//! sibling docs describe). **Slot 8 is `321Go_StartFinish.vex` on every one
//! that authors only one billboard 8** - the same finding as HD's, and the
//! same model name. `14_Track` authors *three* entries under `num="8"`
//! (`Checkered_StartFinish.vex`, `Final_Lap_StartFinish.vex`, then
//! `321Go_StartFinish.vex`) - [`TrackStartup::billboard`] returns the first,
//! so which one the original actually shows for a checkered flag or a final
//! lap, and whether `num` is really unique per Pulse file the way its HD
//! comment insists, is unread.
//!
//! **Slot 7 is not HD's `fx350.vex`.** It varies circuit to circuit - a
//! model (nine different adverts across the eleven circuits, `16_Track`
//! repeating slot 6's `auricom` advert) or a colour (`01`, `03`, `09`) - so
//! "slots 7 and 8 are reserved" is an HD-specific finding, not a title-wide
//! one; only slot 8 generalises.
//!
//! # `Data\Plugins\PI004\Definition.xml`: the catalogue a colour fill likely
//! draws from
//!
//! Pulse (and per its own `Language` survey, almost certainly Pure) ships a
//! plugin, `PI004`, that is a flat catalogue of every billboard advert on the
//! disc - 35 entries on Pulse, each a name (`Portrait2`, `Landscape14`, ...),
//! a `type` (`Portrait`/`Landscape`), a `location`, and a **space-separated
//! list of colours** the model supports (`Landscape19`'s malfunction advert
//! lists five: `white green blue red purple`). `docs/formats/pure-status.md`
//! independently found the same plugin id on Pure, described only as
//! "billboard placements" from its exclusion out of the language picker, not
//! read for content until now.
//!
//! That reframes [`Fill::Colour`]: a `TrackStartup.xml` colour is very
//! plausibly not a literal tint applied to a placeholder, but a **pool
//! selector** - "pick a catalogue entry whose colour list contains this
//! name" - since every colour a circuit authors (`red`, `blue`, `green`,
//! `grey`, `white`, `yellow`, `purple`, `orange`) appears in `PI004`'s own
//! tag lists and no circuit authors one that does not. **Not confirmed**:
//! which entry gets picked when several match, or whether this catalogue is
//! consulted for anything at all - no function has been traced reading it.
//!
//! # A cheap placement hypothesis, tried and it does not hold
//!
//! `16_Track`'s own art mesh embeds five numbered placeholder textures -
//! `billboard1/2/6/7/8.tga`, each an 8x8 icon of its own digit, painted on a
//! handful of quads scattered through the track (`oag-view --draws`; see
//! `docs/formats/vex.md`'s texture-stride section for how those were first
//! found). Slot numbers and texture numbers coinciding is tempting: bind
//! each numbered slot's advert to the placeholder quad wearing its number,
//! reusing the quad's own world transform in the same node tree.
//!
//! **The disc's own file rules that out for `16_Track`.** Its manifest
//! authors all eight slots, but only five numbers (`1`, `2`, `6`, `7`, `8`)
//! have a matching placeholder texture anywhere in the mesh; slots `3`, `4`
//! and `5` name a colour and a model respectively with no numbered quad to
//! bind to. A mechanism that cannot place three of the eight slots it is
//! supposed to explain is not the mechanism - this is the same shape of trap
//! HD's own `Billboard<digits>` node names turned out to be (see above), and
//! is recorded here rather than tried in the renderer for the same reason.
//! Whether the placeholder textures correlate with anything at all - a
//! different, unauthored slot count, a debug-only render path, nothing - is
//! unread. **This section is about reusing a placeholder quad's *transform*
//! for a new object, and stays correct**: the section below finds a real
//! mechanism, but it needs no new transform at all - the placeholder quad
//! never moves, only what texture its own draw call resolves to.
//!
//! # Settled, live, on Pulse: construction places nothing, but something else clearly overrides the disc's own texture
//!
//! 2026-08-28, a live PPSSPP debugger session
//! ([`docs/ghidra/functions/psp-pulse-usa/billboards.md`](../../../docs/ghidra/functions/psp-pulse-usa/billboards.md))
//! walked Pulse's own loader end to end, past a known `$gp`-relative
//! addressing trap that hid it from every static sweep: `World_LoadTrack`
//! builds the path, opens the file, and hands each `<Billboard>` to
//! `TrackStartup_Parse`, which reads the same six attributes this module
//! does and dispatches on `num` to one of two constructors - independently
//! confirming HD's `array[Num]`, no-name-lookup architecture on a second
//! title, and confirming live that `Color`/`Colour` merge into one field
//! here too. Both constructors funnel into a shared object builder that,
//! for a `.vex` (model) slot, writes a 4x4 transform into the new object -
//! **read live, it is the literal identity matrix, the same sixteen floats
//! for every billboard regardless of `num`.**
//!
//! **That was first read as "nothing overrides the disc's `billboard8.tga`
//! icon", and that reading was wrong - caught by the project owner, who
//! plays the original and said the start-line gantry never shows a
//! stretched digit there.** Checked directly: a live screenshot of the
//! original's start line shows a "GO" board, never a digit, while this
//! project's own renderer paints the literal icon. The identity-matrix
//! finding itself still stands; what does not stand is stopping at
//! construction and concluding nothing happens afterward. It does: `num`'s
//! three derived resource-tag IDs are lookup keys into a shared registry all
//! eight slots register into, and the two objects those keys resolve to for
//! slot 8 are read constantly during a real countdown, not just once at
//! construction - one of them carries a live per-frame animation-time
//! accumulator and four candidate display-state pointers, the right shape
//! for a multi-frame countdown display. The final texture bind was not
//! caught in the act, so this is strong circumstantial evidence, not an
//! instruction-level proof - see the doc page for the full chain.
//!
//! **A third pass found what actually plays there: a separate instantiated
//! mesh, not a retextured node 74.** A RAM scan during a live race found
//! `321Go_StartFinish.vex`'s own scene graph fully resident and parsed -
//! `world`/`camera1`/`start_lights`/`start_light_background` node names,
//! the same asset `TrackStartup.xml` names as slot 8's `location` on every
//! circuit that authors one. The disc's own numbered `billboard8.tga` quad
//! reads as authored track-mesh debris the real gantry covers, not what the
//! original draws. **One tension remains unresolved and is now the
//! priority**: the identity transform above and this asset being shared
//! across all 16 circuits mean something still has to move it to each
//! circuit's own gantry, and no writer for that transform has been found -
//! "instantiated" is settled, "drawn where the player sees it" is strong
//! inference from the asset's own node names, not caught directly.
//!
//! **One more live lead survives, not yet folded into the above.** The
//! colour path (`Billboard_CreateFromColour_q`) walks a per-track pool
//! before constructing anything, matching entries by type and colour and
//! consuming the match - confirmed *reached* during a real circuit load. Its
//! entries' own layout was not read; it may be a second instance of the same
//! registry-and-resolve pattern, for colour slots specifically.

use crate::fexml;

/// What a slot is filled with.
///
/// **A slot is either a model or a colour, and a reader that only knows about
/// models silently drops one in eight.** 104 of the disc's 118 `<Billboard>`
/// elements carry a `location`; the other 14 carry a `color` instead and no
/// model at all - `<Billboard num="1" type="landscape" color="red"/>`. Keeping
/// it is what makes the difference between "this circuit has 6 hoardings" and
/// "this circuit has 8, two of which are not models".
///
/// What the engine does with a colour is still not confirmed, but Pulse's
/// `Data\Plugins\PI004\Definition.xml` is a real lead: see this module's
/// "cross-title" section. Every colour name any circuit authors also tags at
/// least one catalogue entry there, which reads as a pool selector rather
/// than a literal tint - not proven, since nothing has traced a function
/// reading either the manifest's colour or that catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fill {
    /// `location="Data\..."`, as an archive entry name.
    ///
    /// Stored with the file's own backslashes turned into forward slashes and
    /// a leading slash added, which is the spelling `oag_assets` matches on -
    /// see [`crate::psarc`]'s `normalise`.
    Model(String),
    /// `color="red"`, verbatim. A name rather than a value, and the disc uses
    /// five: `red`, `blue`, `green`, `grey`, `white`. What they resolve to is
    /// unrecovered.
    Colour(String),
}

/// One `<Billboard>` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Billboard {
    /// The `num` attribute: 1 to 8, and unique within a file per its own
    /// comment. Kept as read rather than as an index, because nothing here has
    /// recovered what it addresses.
    pub num: u32,
    /// The `type` attribute: `"landscape"` on 115 of the disc's 118 slots and
    /// `"portrait"` on 3. A string rather than an enum because that second
    /// value is exactly what a first pass missed - it only looked at slots
    /// carrying a `location`, and all three `portrait` slots are colours.
    pub kind: String,
    /// What fills the slot.
    pub fill: Fill,
}

impl Billboard {
    /// The archive entry this slot names, or `None` for a colour.
    #[must_use]
    pub fn location(&self) -> Option<&str> {
        match &self.fill {
            Fill::Model(path) => Some(path),
            Fill::Colour(_) => None,
        }
    }
}

/// A circuit's startup manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrackStartup {
    /// `<LoadSoundBank Filename="...">`, relative to the circuit's directory.
    pub sound_bank: Option<String>,
    /// `<LevelFx><UnderwaterSound type="N">`.
    pub underwater_sound: Option<u32>,
    /// `<LevelFx><WindSound type="N">`.
    pub wind_sound: Option<u32>,
    /// Every `<Billboard>`, in document order.
    pub billboards: Vec<Billboard>,
}

impl TrackStartup {
    /// Reads a whole `trackstartup.xml`, HD's plain form or Pulse's
    /// `fexml`-shortened one.
    ///
    /// **Forgiving on purpose**, like the reader under it: an element this does
    /// not know is ignored and a `<Billboard>` missing `num` or `location` is
    /// dropped rather than failing the file, because the manifest is
    /// supplementary - a circuit whose adverts do not load still races. What
    /// *is* worth knowing is reported by the caller from the counts.
    ///
    /// A shortened file whose `<code>` dictionary is somehow empty falls back
    /// to parsing the raw text rather than returning nothing - the same
    /// "still races" reasoning one level up, since a body of single-letter
    /// tags this module's schema does not recognise costs nothing beyond the
    /// billboards it cannot name.
    #[must_use]
    pub fn parse(xml: &str) -> Self {
        let expanded = fexml::text(xml.as_bytes());
        let root = fexml::parse(expanded.as_deref().unwrap_or(xml));
        let mut out = Self::default();
        for element in descendants(&root) {
            match element.name.to_ascii_lowercase().as_str() {
                "loadsoundbank" => {
                    out.sound_bank = element.attr("filename").map(str::to_string);
                }
                "underwatersound" => out.underwater_sound = element.attr("type").and_then(number),
                "windsound" => out.wind_sound = element.attr("type").and_then(number),
                "billboard" => {
                    let Some(num) = element.attr("num").and_then(number) else {
                        continue;
                    };
                    // A location wins where both are somehow present: it is the
                    // one that names something loadable.
                    let Some(fill) = element
                        .attr("location")
                        .map(|l| Fill::Model(entry_name(l)))
                        .or_else(|| element.attr("color").map(|c| Fill::Colour(c.to_string())))
                    else {
                        continue;
                    };
                    out.billboards.push(Billboard {
                        num,
                        kind: element.attr("type").unwrap_or_default().to_string(),
                        fill,
                    });
                }
                _ => {}
            }
        }
        out
    }

    /// The billboard with this `num`, if the file names one.
    #[must_use]
    pub fn billboard(&self, num: u32) -> Option<&Billboard> {
        self.billboards.iter().find(|b| b.num == num)
    }
}

/// Every element under `node`, itself included, depth-first.
fn descendants(node: &fexml::Node) -> Vec<&fexml::Node> {
    let mut out = vec![node];
    let mut at = 0;
    while at < out.len() {
        let here = out[at];
        at += 1;
        out.extend(here.children.iter());
    }
    out
}

/// A decimal attribute, or `None` for one that is not.
fn number(text: &str) -> Option<u32> {
    text.trim().parse().ok()
}

/// The manifest's `Data\A\B.vex` as the `/data/a/b.vex` an archive matches.
fn entry_name(location: &str) -> String {
    format!(
        "/{}",
        location.trim_start_matches(['/', '\\']).replace('\\', "/")
    )
}

#[cfg(test)]
mod tests;
