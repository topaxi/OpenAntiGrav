//! `trackstartup.xml`: what a circuit asks to be loaded with it.
//!
//! One per circuit (16 on HD, plain UTF-8; Pulse ships the same file
//! `fexml`-shortened). Both forms go through [`crate::fexml::text`], which
//! expands the shortened one and passes plain text through, so this module is
//! the schema on top of either, as with [`crate::handling`].
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
//! # Why read it
//!
//! It lists what the disc says to load, and this project loads none of it.
//! Over all 16 HD files: **118 `<Billboard>` entries, 104 naming a model (18
//! distinct, all on the disc)**, 14 a colour instead (see [`Fill`]). Each advert
//! is a self-contained animated scene (about 41 nodes, a `World`, a `Camera`,
//! several `Anim Transform`s, its own scrolling materials). Reading the manifest
//! turns "we do not draw the hoardings" into a load-report line naming the files
//! wanted.
//!
//! # Slot numbers
//!
//! **Slots 7 and 8 are reserved on HD, and slot 8 is the start/finish line.**
//! Over all 16 circuits slot 8 is `321Go_StartFinish.vex` and that model is in
//! no other slot; slot 7 is `fx350.vex`, likewise exclusive. Slots 1-6 are the
//! artist's (nine circuits share one default six, seven customise, four of those
//! fill some with a colour). A player's observation (slot 8's hoarding is at
//! the start/finish) prompted the check and agrees with it, as does the model's
//! name. That says a slot number means a place, not where it is.
//!
//! # What the executable does with a slot
//!
//! [`docs/ghidra/functions/ps3-hdfury-eu/billboards.md`](../../../docs/ghidra/functions/ps3-hdfury-eu/billboards.md)
//! reads `TrackStartup_Load`: `num` indexes a plain 9-entry array (`0` wasted,
//! `1`-`8` real, no name lookup; the two stray `Billboard<digits>` nodes are
//! artist debris) and a slot is **instantiated**, not merely texturing existing
//! geometry. The engine reads `Colour` (merged with `Color`) and a numeric
//! `Glow` and ignores `type`; `Colour`/`Glow` never appear in the 16 manifests,
//! so skipping them is right. **Slot 7 is silently replaced by a mode-specific
//! gate model at runtime**, whatever the manifest says; slot 8 keeps its
//! authored `321Go_StartFinish.vex`.
//!
//! **Nothing is placed yet, and where the transform comes from is
//! unrecovered.** An advert's chunks are node-local with a bias near the origin.
//! HD's circuits use 30 node classes, all already named by `oag_vex::vex`, so
//! there is no dedicated slot class; only `02_track` and `05_ubermall` carry a
//! node named `Billboard<digits>`, so the artists' names are not the convention.
//! Next lead, same page: `GetBillboardMeshIdFromName`, a name-hash lookup
//! against a per-track list that resets a 4x4 matrix on every (re)bind; read, not
//! chased. Per `CLAUDE.md` an asset whose trigger is unrecovered stays unwired:
//! this module parses and reports.
//!
//! # Cross-title: Pulse has the same slots; slot 7 is not reserved there
//!
//! Read off 11 of Pulse's 12 circuits (`26_Track`, a `Zone` variant of
//! `25_Track`, has no `TrackStartup.xml`). **Slot 8 is `321Go_StartFinish.vex`
//! on every one that authors one**, and **`num` is unique per manifest**.
//!
//! `14_Track` looked like an exception until 2026-09-05 (three `num="8"`
//! entries). It authors one: the other two lines are `#`-prefixed and sit
//! **after `</TrackStartup>`**, outside the document, which this module used to
//! walk into by descending from `#document`; [`schema_root`] is the fix. They
//! are dead: neither `Checkered_StartFinish.vex` nor
//! `Final_Lap_StartFinish.vex` resolves in Pulse's three WADs or `strings` of
//! `Data.wad`. They are fossils of an earlier three-file design, folded into
//! `321Go_StartFinish.vex`'s timeline (countdown, FINAL LAP and chequered boards
//! as three windows); see `docs/rendering/start-gantry.md`.
//!
//! **Slot 7 is not HD's `fx350.vex`.** It varies (nine different adverts across
//! the eleven circuits, or a colour on `01`, `03`, `09`), so "7 and 8 reserved"
//! is HD-specific; only slot 8 generalises.
//!
//! # `Data\Plugins\PI004\Definition.xml`: the catalogue a colour fill likely draws from
//!
//! Pulse (and almost certainly Pure, per `docs/formats/pure-status.md`, which
//! found the plugin id but did not read it) ships `PI004`, a flat catalogue of
//! every billboard advert: 35 entries on Pulse, each with a name
//! (`Portrait2`, `Landscape14`), a `type`, a `location` and a
//! **space-separated colour list** (`Landscape19` lists `white green blue red
//! purple`). So [`Fill::Colour`] is plausibly a **pool selector** ("pick an
//! entry whose list contains this name") rather than a tint: every colour a
//! circuit authors (`red`, `blue`, `green`, `grey`, `white`, `yellow`,
//! `purple`, `orange`) appears in `PI004`'s tags. **Not confirmed**: which
//! entry is picked when several match, or whether the catalogue is consulted at
//! all; no function reading it has been traced.
//!
//! # A placement hypothesis that does not hold
//!
//! `16_Track`'s art mesh embeds five numbered placeholder textures
//! (`billboard1/2/6/7/8.tga`, 8x8 digit icons on quads scattered through the
//! track; `oag-view --draws`, `docs/formats/vex.md`'s texture-stride section).
//! Binding each slot's advert to the quad wearing its number, reusing the
//! quad's world transform, is tempting. **The disc rules it out**: the manifest
//! authors all eight slots but only `1`, `2`, `6`, `7`, `8` have a quad, so it
//! cannot place three of them. Whether the textures correlate with anything is
//! unread. This concerns reusing a quad's *transform*; the section below finds a
//! real mechanism that needs none (the quad never moves, only the texture its
//! draw resolves to).
//!
//! # Live on Pulse: construction places nothing, something else overrides the texture
//!
//! 2026-08-28, a live PPSSPP session
//! ([`docs/ghidra/functions/psp-pulse-usa/billboards.md`](../../../docs/ghidra/functions/psp-pulse-usa/billboards.md))
//! walked Pulse's loader past a `$gp`-relative addressing trap that hid it from
//! static sweeps. `World_LoadTrack` hands each `<Billboard>` to
//! `TrackStartup_Parse`, which reads the same six attributes as this module and
//! dispatches on `num` to one of two constructors: HD's `array[Num]`,
//! no-name-lookup shape on a second title, with `Color`/`Colour` merged. Both
//! feed a shared builder that, for a `.vex` slot, writes a 4x4 transform into
//! the new object.
//!
//! **That transform is NOT the identity - corrected 2026-09-06, confidence 92.**
//! It is an identity rotation with translation `(x, -10.0, z, w)`, the same on
//! every call regardless of `num`, so still no per-circuit placement. The older
//! "literal identity, same sixteen floats" reading was wrong for one float:
//! `decompile_function` showed a VFPU `lv.q` as four scalar assignments and hid a
//! same-block `swc1` overwriting one lane. See that page and
//! `docs/ghidra/workflow.md`.
//!
//! **"Nothing overrides the disc's `billboard8.tga` icon" was wrong**, caught by
//! the project owner who plays the original: the start-line gantry never shows
//! a stretched digit, and a live screenshot shows a "GO" board while this
//! project's renderer painted the literal icon. After construction, `num`'s
//! three derived resource-tag IDs are lookup keys into a shared registry all
//! eight slots register into; the two objects slot 8 resolves to are read
//! constantly during a countdown, one with a per-frame animation-time
//! accumulator and four candidate display-state pointers. The final texture bind
//! was not caught, so this is strong circumstantial evidence, not proof.
//!
//! **A third pass found what plays there: a separate instantiated mesh.** A RAM
//! scan in a live race found `321Go_StartFinish.vex`'s scene graph resident and
//! parsed (`world`/`camera1`/`start_lights`/`start_light_background`), the asset
//! slot 8 names on every circuit. The numbered `billboard8.tga` quad is
//! track-mesh debris the real gantry covers. **Unresolved, and the priority**:
//! the transform above and an asset shared by all 16 circuits mean something
//! must move it to each circuit's gantry, and no writer for that has been
//! found. "Instantiated" is settled; "drawn where the player sees it" is
//! inference from node names.
//!
//! **One live lead remains.** The colour path (`Billboard_CreateFromColour_q`)
//! walks a per-track pool, matching by type and colour and consuming the match
//! (confirmed *reached* in a real circuit load); the entry layout was not read
//! and may be another registry-and-resolve instance.

use crate::fexml;

/// What a slot is filled with.
///
/// **A slot is a model or a colour, and a reader that only knows models drops
/// one in eight**: 104 of 118 `<Billboard>`s carry a `location`, the other 14 a
/// `color` and no model (`<Billboard num="1" type="landscape" color="red"/>`).
/// What the engine does with a colour is unconfirmed; see this module's
/// `PI004` section (a pool selector, not proven).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fill {
    /// `location="Data\..."` as an archive entry name: backslashes to forward
    /// slashes and a leading slash, the spelling `oag_assets` matches (see
    /// `oag_formats::psarc`'s `normalise`).
    Model(String),
    /// `color="red"`, verbatim: a name, one of `red`, `blue`, `green`, `grey`,
    /// `white` on the disc. What they resolve to is unrecovered.
    Colour(String),
}

/// One `<Billboard>` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Billboard {
    /// The `num` attribute: 1 to 8, unique per file. Kept as read, not an index:
    /// what it addresses is not recovered here.
    pub num: u32,
    /// The `type` attribute: `"landscape"` on 115 of 118 slots, `"portrait"` on
    /// 3 (all colours). A string because a first pass that looked only at
    /// `location` slots missed the second value.
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
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackStartup {
    /// `<LoadSoundBank Filename="...">`, relative to the circuit's directory.
    pub sound_bank: Option<String>,
    /// `<LevelFx><UnderwaterSound type="N">`.
    pub underwater_sound: Option<u32>,
    /// `<LevelFx><WindSound type="N">`.
    pub wind_sound: Option<u32>,
    /// `<LevelFx><Weather>`, on the two circuits that author one.
    pub weather: Option<Weather>,
    /// Every `<Billboard>`, in document order.
    pub billboards: Vec<Billboard>,
}

impl TrackStartup {
    /// Reads a whole `trackstartup.xml`, HD's plain form or Pulse's shortened one.
    ///
    /// **Forgiving on purpose**: an unknown element is ignored and a
    /// `<Billboard>` missing `num` or `location` is dropped, because the manifest
    /// is supplementary (a circuit whose adverts do not load still races); the
    /// caller reports from the counts. A shortened file with an empty `<code>`
    /// dictionary parses as raw text for the same reason.
    ///
    /// **Scoped to the `<TrackStartup>` element**, which stops `14_Track`'s two
    /// commented-out slot-8 lines being read; see [`schema_root`].
    #[must_use]
    pub fn parse(xml: &str) -> Self {
        let expanded = fexml::text(xml.as_bytes());
        let document = fexml::parse(expanded.as_deref().unwrap_or(xml));
        let root = schema_root(&document);
        let mut out = Self::default();
        for element in descendants(root) {
            match element.name.to_ascii_lowercase().as_str() {
                "loadsoundbank" => {
                    out.sound_bank = element.attr("filename").map(str::to_string);
                }
                "underwatersound" => out.underwater_sound = element.attr("type").and_then(number),
                "windsound" => out.wind_sound = element.attr("type").and_then(number),
                "weather" => out.weather = Some(Weather::from_element(element)),
                "billboard" => {
                    let Some(num) = element.attr("num").and_then(number) else {
                        continue;
                    };
                    // A location wins where both are present: it names something loadable.
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

/// The `<TrackStartup>` element itself, or the whole document if none is named.
///
/// A Pulse manifest has two top-level elements (the `<code>` dictionary, then
/// the root), and `14_Track` has two `#`-prefixed `<Billboard>` lines *after*
/// the root's close tag, naming files not on the disc; reading them made it look
/// like three slot 8s. Content outside the root is not part of the document and
/// the original's walker never sees it. The fallback keeps the forgiving
/// contract for a file whose dictionary failed to expand.
fn schema_root(document: &fexml::Node) -> &fexml::Node {
    descendants(document)
        .into_iter()
        .find(|node| node.name.eq_ignore_ascii_case("TrackStartup"))
        .unwrap_or(document)
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

mod weather;
pub use weather::{Weather, effect_name};

#[cfg(test)]
mod tests;
