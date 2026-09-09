//! A HUD layout that is more than one file.
//!
//! # Why this exists
//!
//! Pulse authors each mode's HUD as **one** self-contained file: five layouts,
//! five archive entries, and [`Layout::from_xml`] reads any of them on its own.
//! HD/Fury authors the same dialect the opposite way round - a mode's root file
//! is a shell that names a dozen fragments, and the widgets are all in the
//! fragments. `Data\XML\Elimination_HUD.xml` is 1,856 bytes and holds **three**
//! widget elements, of which two are the `ForwardHUD`/`ReverseHUD` groups that
//! wrap everything else. Composed, it is **15 files and 204 widgets**.
//!
//! Read the shell alone and a HUD comes out with two empty rectangles in it, and
//! nothing reports a loss, because nothing was dropped: the widgets were never
//! in the file. That is the failure this module removes.
//!
//! # The include
//!
//! ```xml
//! <LoadXML>
//!   <Values SrcRel="HUD_colours.xml" DirectEmbed="true"></Values>
//! </LoadXML>
//! ```
//!
//! `SrcRel` is resolved **against the including file's own directory**, and that
//! is what makes the three HUD skins distinct rather than aliases: `/data/xml/
//! elimination_hud.xml` and `/data/xml/wo3_hud/elimination_hud.xml` both name
//! `HUD_elim_line.xml`, and they mean two different files. Resolving from a
//! fixed root instead loads the default skin's fragments for all three, which
//! looks like a working HUD and is the wrong one.
//!
//! `Src` is the same thing with a whole path in it, taken as-is.
//! [`oag_assets::psarc::Archive::read_path`] already matches case, backslashes
//! and a leading `/` loosely, so neither spelling needs normalising here.
//!
//! # Two attributes this cannot learn anything about
//!
//! Counted over every `<LoadXML>` in every HUD file the disc ships, 2026-08-17:
//! **720 of 720** carry `SrcRel`, **720 of 720** carry `DirectEmbed="true"`,
//! and **none** carries `Src`. So the shipped data is unanimous on both, which
//! means neither can be read for what it selects - `DirectEmbed` is not
//! consulted here, and the `Src` branch is exercised by this module's own tests
//! and by nothing on the disc. Both are worth knowing as **untested paths**
//! rather than as decisions.

use std::collections::BTreeSet;

use oag_tables::fexml::{self, Node};

use super::Layout;

/// A layout and the account of how it was assembled.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Composed {
    /// The layout, with every fragment's widgets in it.
    pub layout: Layout,
    /// Every file read, the root first and then in include order.
    pub files: Vec<String>,
    /// Includes that named a file the reader could not produce.
    ///
    /// Surfaced rather than swallowed for the same reason [`Layout::skipped`]
    /// is: a fragment that silently fails to load takes its widgets with it and
    /// leaves a HUD that looks deliberate.
    pub missing: Vec<String>,
}

/// Reads a root layout and splices in everything it includes.
///
/// `read` is handed a path exactly as the XML spells it and returns the file's
/// bytes, or `None` when it has no such entry. It is called **once per include
/// occurrence**, not once per distinct path: a file two fragments both name is
/// read twice and spliced twice, and appears twice in [`Composed::files`].
/// Nothing on the disc does that often enough to be worth a cache.
///
/// Returns `None` only when the **root** cannot be read or is not XML at all -
/// a missing fragment is a [`Composed::missing`] entry, not a failure, because
/// most of a HUD is better than none of one.
///
/// # Cycles
///
/// An include whose target is already on the stack above it is dropped and
/// recorded in [`Composed::missing`]. Nothing on either disc does this; the
/// guard is here because a splice with no cycle check is an infinite loop
/// rather than a wrong answer.
pub fn compose<R>(root: &str, mut read: R) -> Option<Composed>
where
    R: FnMut(&str) -> Option<Vec<u8>>,
{
    let mut out = Composed::default();
    let mut tree = parse_file(root, &mut read)?;
    out.files.push(root.to_string());

    let mut stack = BTreeSet::new();
    stack.insert(key(root));
    splice(&mut tree, root, &mut stack, &mut read, &mut out);

    out.layout = Layout::from_tree(&tree);
    Some(out)
}

/// One file's tree, or `None` when it cannot be read.
fn parse_file<R>(path: &str, read: &mut R) -> Option<Node>
where
    R: FnMut(&str) -> Option<Vec<u8>>,
{
    let blob = read(path)?;
    // `fexml::text` decides shortened-versus-plain from the blob's own first
    // bytes. Both dialects ship - Pulse's HUD layouts are shortened and HD's are
    // not - so calling `expand` unconditionally here would reject every HD file
    // with "no `<code>` dictionary element". See `docs/ui/hud.md`.
    let xml = fexml::text(&blob).ok()?;
    Some(fexml::parse(&xml))
}

/// Replaces every `<LoadXML>` under `node` with the children of what it names.
fn splice<R>(
    node: &mut Node,
    here: &str,
    stack: &mut BTreeSet<String>,
    read: &mut R,
    out: &mut Composed,
) where
    R: FnMut(&str) -> Option<Vec<u8>>,
{
    let mut replaced: Vec<Node> = Vec::with_capacity(node.children.len());
    for mut child in std::mem::take(&mut node.children) {
        if !child.name.eq_ignore_ascii_case("LoadXML") {
            splice(&mut child, here, stack, read, out);
            replaced.push(child);
            continue;
        }

        let Some(target) = target_of(&child, here) else {
            out.missing
                .push(format!("{here}: a <LoadXML> naming nothing"));
            continue;
        };
        if !stack.insert(key(&target)) {
            out.missing
                .push(format!("{here}: {target} would close a cycle"));
            continue;
        }
        match parse_file(&target, read) {
            Some(mut sub) => {
                out.files.push(target.clone());
                splice(&mut sub, &target, stack, read, out);
                // The included file's root is synthetic - `fexml::parse` wraps a
                // document in one - so its *children* are what belongs here, in
                // the include's place, keeping paint order.
                replaced.append(&mut sub.children);
            }
            None => out.missing.push(target.clone()),
        }
        stack.remove(&key(&target));
    }
    node.children = replaced;
}

/// The path a `<LoadXML>` names, resolved against the file it appears in.
fn target_of(node: &Node, here: &str) -> Option<String> {
    if let Some(src) = node.value("Src") {
        return Some(src.trim().to_string());
    }
    Some(join(here, node.value("SrcRel")?.trim()))
}

/// `rel` read from the directory `here` sits in, with `.` and `..` applied.
///
/// **`..` ships**, which is the only reason this is not string concatenation:
/// `Data\XML\Duel_HUD\Duel_HUD.xml` reaches four of its fragments as
/// `SrcRel="..\HUD_colours.xml"` and the rest by bare name, so a reader that
/// pastes the two together asks the archive for
/// `/data/xml/duel_hud/..\hud_colours.xml` and is told there is no such entry.
/// The four it loses are the colour table, the countdown, the info text and the
/// time-difference readout - a quarter of that mode's HUD, absent with an
/// explanation that reads like a missing file.
///
/// Both separators are split on and the result is joined with `/`, since every
/// path in this format's containers is matched with separators and case folded
/// anyway - see [`oag_assets::psarc::Archive::read_path`].
fn join(here: &str, rel: &str) -> String {
    let mut parts: Vec<&str> = here.split(['/', '\\']).collect();
    // The last element of `here` is the including file itself, not a directory.
    parts.pop();
    for part in rel.split(['/', '\\']) {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// One spelling of a path, for the cycle guard only.
fn key(path: &str) -> String {
    path.to_ascii_lowercase().replace('\\', "/")
}

#[cfg(test)]
mod tests;
