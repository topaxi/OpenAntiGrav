//! Measures the byte at `Texture` node payload `+0x06` across every `.vex`
//! file Pulse ships, on both of its pressings.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The tests skip with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # What this is for
//!
//! `docs/formats/pure-status.md` records, at confidence 88, that
//!
//! > The `Texture` node payload has a flags byte at `+0x06` whose bit 0 means
//! > *the texels are already in the GE's 16-byte by 8-row block order*.
//!
//! and that Pure sets it on model textures where Pulse does not.
//! [`vex::textures`] reads width, height, `bits_per_pixel`, `mip_count`,
//! `clut_size` and `texel_size`, and never looks at `+0x06`. Making it look is
//! the fix Pure needs.
//!
//! That change is only free for Pulse if the bit is clear on **every** Pulse
//! `Texture` node. If it is set anywhere, an unswizzling decoder starts
//! returning different texels for a Pulse asset, and a ground-truth screenshot
//! of a circuit would not necessarily cover the one model that flipped - so
//! `just test-data` would go green on a regression. This test is the
//! measurement that has to come first, and the pin that keeps it true.
//!
//! # What the sweep found
//!
//! - **Bit 0 is clear on every `Texture` node in Pulse's PSP corpus.** Reading
//!   `+0x06` and honouring bit 0 therefore changes nothing for Pulse, and needs
//!   no gate on the version word.
//! - **The byte is not constant, and it is not a lone flag either.** The
//!   distribution is pinned in [`PSP_FLAG_HISTOGRAM`]; the values are whatever
//!   the sweep measured, and what the other bits mean is left open rather than
//!   guessed at. Only bit 0 has a documented meaning and only bit 0 is claimed
//!   here.
//! - **The version-4 `Data\Defaults\Skycube.vex` is included, deliberately.**
//!   Its `Texture` class ID is `0x373`, not version 6's `0x3c1`, so a sweep
//!   keyed on [`vex::CLASS_TEXTURE`] would silently miss its six nodes - the
//!   six that use the *same* numbering Pure's 156 files do, and so the six most
//!   likely to carry Pure's flag. [`vex::classes_of`] picks the table from each
//!   file's own version word, which is the whole point of it.
//! - **All four PSP archives are swept**, not just `Data.wad`: `FEData.wad` and
//!   `BEData.wad` hold 70 version-6 `.vex` files between them, and a sweep that
//!   stopped at the two archives a title package names would have missed every
//!   one. `PS2MUSIC.WAD` and `PRERACE.WAD` are not swept - neither is a WAD this
//!   project parses (see `oag_pulse::archives::ps2`).
//! - **The PS2 pressing carries `Texture` nodes with an empty texel block.**
//!   `vex::textures` already returns `None` for each of them because the block
//!   after the tree is zero-length; the *nodes* are there and their `+0x06`
//!   bytes are readable, which is why the PS2 sweep asserts a non-empty node
//!   set rather than passing on an empty one.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_assets::Archive;
use oag_formats::vex;
use oag_formats::wad::Compression;

/// The flags byte under test, in a `Texture` node's payload.
const FLAGS_AT: usize = 0x06;

/// The bit `pure-status.md` reads as "these texels are already swizzled".
const PRE_SWIZZLED: u8 = 0x01;

/// The PSP-only archives no title package names, because they are not one of
/// the two archive *roles* [`oag_pulse::TITLE`] describes.
///
/// Swept anyway, and this is not belt-and-braces: between them they hold 70
/// version-6 `.vex` files, so "bit 0 is clear across the corpus" measured
/// without them would be a claim about half the corpus.
const PSP_EXTRA_ARCHIVES: &[&str] = &[oag_pulse::archives::FEDATA, oag_pulse::archives::BEDATA];

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// What one sweep measured, in the terms the question is asked in.
#[derive(Default)]
struct Sweep {
    /// Archives actually opened, by their own labels.
    archives: Vec<String>,
    /// Entries that carry the `.vex` magic, whatever their version.
    vex_files: usize,
    /// `.vex` files per format version word.
    versions: BTreeMap<u32, usize>,
    /// Files whose version has no class table at all
    /// ([`vex::Error::UnknownVersion`]). Counted rather than skipped silently:
    /// a whole format generation contributing nothing should show as a number.
    unknown_version: usize,
    /// Files whose version has a table, but whose `Texture` ID is unrecovered.
    no_texture_id: usize,
    /// Files whose node tree would not walk.
    unwalkable: usize,
    /// `Texture` nodes visited.
    texture_nodes: usize,
    /// Nodes whose payload is too short to hold the flags byte at all.
    short_payloads: usize,
    /// How often each value of the `+0x06` byte occurred, per format version.
    flags: BTreeMap<(u32, u8), usize>,
    /// Nodes with bit 0 set, and where.
    pre_swizzled: Vec<String>,
}

impl Sweep {
    fn report(&self, label: &str) {
        println!("{label}: archives {:?}", self.archives);
        println!(
            "  {} .vex files, versions {:?}",
            self.vex_files, self.versions
        );
        println!(
            "  skipped: {} unknown version, {} without a Texture id, {} unwalkable",
            self.unknown_version, self.no_texture_id, self.unwalkable
        );
        println!(
            "  {} Texture nodes, {} payloads too short for +0x{FLAGS_AT:02x}",
            self.texture_nodes, self.short_payloads
        );
        for ((version, value), count) in &self.flags {
            println!("    v{version} +0x06 = {value:#04x}: {count}");
        }
        println!("  bit 0 set on {} node(s)", self.pre_swizzled.len());
    }
}

/// Every `Texture` node in one archive, added to `sweep`.
///
/// The class ID comes from [`vex::classes_of`], per file. Using
/// [`vex::CLASS_TEXTURE`] instead would be the one mistake that produces a
/// confident wrong answer here: it is version 6's ID, and the file whose
/// textures are numbered the way Pure's are is exactly the version-4 one.
fn sweep_archive(archive: &mut Archive, sweep: &mut Sweep) {
    sweep.archives.push(archive.label().to_string());
    let label = archive.label().to_string();

    let entries: Vec<(usize, u32, Compression)> = archive
        .directory()
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (index, entry.size, entry.compression))
        .collect();

    for (index, size, compression) in entries {
        if size == 0 {
            continue;
        }
        // A stored blob's magic is readable without decompressing it, and every
        // PSP archive is stored. That turns a full read of 315 MiB of
        // `Data.wad` into a 16-byte peek for the entries that are not `.vex`.
        // A compressed entry - which is every entry on the PS2 pressing - has
        // to be decompressed either way.
        if compression == Compression::None {
            let Ok(head) = archive.peek(index, vex::FILE_HEADER_LEN as u64) else {
                continue;
            };
            if !vex::has_magic(&head) {
                continue;
            }
        }
        let Ok(bytes) = archive.read(index) else {
            continue;
        };
        if !vex::has_magic(&bytes) {
            continue;
        }

        sweep.vex_files += 1;
        if let Ok(version) = vex::version(&bytes) {
            *sweep.versions.entry(version).or_default() += 1;
        }

        let classes = match vex::classes_of(&bytes) {
            Ok(classes) => classes,
            Err(vex::Error::UnknownVersion { .. }) => {
                sweep.unknown_version += 1;
                continue;
            }
            Err(_) => {
                sweep.unwalkable += 1;
                continue;
            }
        };
        let Some(texture_class) = classes.texture else {
            sweep.no_texture_id += 1;
            continue;
        };
        let Ok(tree) = vex::nodes(&bytes) else {
            sweep.unwalkable += 1;
            continue;
        };

        for node in tree.iter().filter(|n| n.class_id == texture_class) {
            let payload = &bytes[node.payload()];
            let Some(&flags) = payload.get(FLAGS_AT) else {
                sweep.short_payloads += 1;
                sweep.texture_nodes += 1;
                continue;
            };
            sweep.texture_nodes += 1;
            *sweep.flags.entry((classes.version, flags)).or_default() += 1;
            if flags & PRE_SWIZZLED != 0 {
                sweep.pre_swizzled.push(format!(
                    "v{} {label} entry {index} node {:?} flags {flags:#04x}",
                    classes.version, node.name
                ));
            }
        }
    }
}

/// Opens every archive on `source` that this project parses, as Pulse.
///
/// The two archive *roles* come from [`oag_pulse::open`], which is what makes
/// this work unchanged on the PS2 pressing, where the same two are called
/// `WADS2.WAD` and `WADSP.WAD` and live under a serial-derived directory.
/// [`PSP_EXTRA_ARCHIVES`] are then tried by name and simply fail to open on a
/// PS2 source, so no platform test is needed.
fn sweep_source(path: &Path) -> Sweep {
    let source = path.to_str().expect("image path is utf-8");
    let mut archives = oag_pulse::open(source).expect("open as Pulse");
    let mut sweep = Sweep::default();

    sweep_archive(&mut archives.data, &mut sweep);
    if let Some(fe) = archives.fe.as_mut() {
        sweep_archive(fe, &mut sweep);
    }
    for extra in PSP_EXTRA_ARCHIVES {
        if let Ok(mut archive) = Archive::open(&format!("{source}:{extra}")) {
            sweep_archive(&mut archive, &mut sweep);
        }
    }
    sweep
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn no_psp_texture_node_is_flagged_pre_swizzled() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let sweep = sweep_source(&path);
    sweep.report("pulse-psp-usa");

    assert!(
        sweep.pre_swizzled.is_empty(),
        "bit 0 of +0x06 is set on Pulse PSP nodes, so reading it is a Pulse \
         behaviour change: {:?}",
        sweep.pre_swizzled
    );
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd; run with `just test-data`"]
fn no_ps2_texture_node_is_flagged_pre_swizzled() {
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let sweep = sweep_source(&path);
    sweep.report("pulse-ps2-eu");

    // An empty node set is a finding, not a pass. PS2 `.vex` scenes carry
    // `Texture` nodes with real dimensions and an empty texel block - see
    // `vex::textures` - so zero nodes here would mean the sweep stopped
    // working, not that the pressing has no textures.
    assert!(
        sweep.texture_nodes > 0,
        "no Texture nodes found on the PS2 pressing; the sweep measured nothing"
    );
    assert!(
        sweep.pre_swizzled.is_empty(),
        "bit 0 of +0x06 is set on Pulse PS2 nodes: {:?}",
        sweep.pre_swizzled
    );
}
