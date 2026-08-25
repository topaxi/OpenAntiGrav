//! `trackstartup.xml`: what a Wipeout HD circuit asks to be loaded with it.
//!
//! One per circuit, 16 on the disc, plain UTF-8 XML with no expansion or
//! escaping - so [`crate::fexml::parse`] reads it and this module is the schema
//! on top, the same split [`crate::handling`] uses.
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
//! # What is deliberately not done here
//!
//! **Nothing is placed.** An advert's chunks are node-local with a bias near
//! the origin, so the circuit must supply a transform, and **where that comes
//! from is unrecovered**: HD's circuits use 30 node classes and
//! `oag_formats::vex` already names all 30, so there is no dedicated slot
//! class; and searching every circuit for a node named exactly
//! `Billboard<digits>` finds one on `02_track` and one on `05_ubermall` and
//! nothing anywhere else, so the artists' names are not the convention either.
//!
//! Per `CLAUDE.md`, an asset whose **trigger** is unrecovered stays unwired
//! rather than placed on a guess: a hoarding put somewhere plausible is exactly
//! the kind of legible invention that survives review and removes the pressure
//! to find the real answer. This module parses and reports; a later change
//! places, once something says where.

use crate::fexml;

/// What a slot is filled with.
///
/// **A slot is either a model or a colour, and a reader that only knows about
/// models silently drops one in eight.** 104 of the disc's 118 `<Billboard>`
/// elements carry a `location`; the other 14 carry a `color` instead and no
/// model at all - `<Billboard num="1" type="landscape" color="red"/>`. What the
/// engine does with a colour is unrecovered; keeping it is what makes the
/// difference between "this circuit has 6 hoardings" and "this circuit has 8,
/// two of which are not models".
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
    /// Reads a whole `trackstartup.xml`.
    ///
    /// **Forgiving on purpose**, like the reader under it: an element this does
    /// not know is ignored and a `<Billboard>` missing `num` or `location` is
    /// dropped rather than failing the file, because the manifest is
    /// supplementary - a circuit whose adverts do not load still races. What
    /// *is* worth knowing is reported by the caller from the counts.
    #[must_use]
    pub fn parse(xml: &str) -> Self {
        let root = fexml::parse(xml);
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
