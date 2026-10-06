//! Validates the cue-to-waveform rule against every sound bank on both Pulse
//! discs, and the claim that the PS2 ships the PSP's container unchanged.
//!
//! **`#[ignore]`d, never run in CI**: needs game content (`just test-data`);
//! see `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! # What these are for
//!
//! `docs/formats/psp-audio.md` split a bank into waveforms and read its name
//! table but had no rule joining them: a name resolved to a *cue*, whose `+0x08`
//! had been tried as a descriptor-section offset and failed on every bank.
//!
//! The rule is `first_command = *(u32 *)(cue + 0x08) / 8`; the evidence is that
//! each bank's cues then **partition its command table exactly** (no gap, no
//! overlap, ending on the last command), which a wrong base does not produce:
//! run over seven candidate readings, only this one passes.
//!
//! The second test is the independent half. A cue's name and its waveforms'
//! `+0x0e` flag word are written by different parts of the bank tool and
//! neither points at the other, so `~`-named cues looping and plain ones not is
//! a fact the arithmetic could not manufacture.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::sblk::{self, Bank};
use oag_formats::wad;

/// The PSP archives that hold sound banks.
const PSP_ARCHIVES: [&str; 2] = ["PSP_GAME/USRDIR/FE.wad", "PSP_GAME/USRDIR/Data.wad"];

/// The PS2 bulk archive. `WADSP.WAD`, its `FE.wad` counterpart, holds none.
const PS2_ARCHIVES: [&str; 1] = ["54748/WADS2.WAD"];

/// Pure's three archives. `FEData.wad` holds no bank.
const PURE_ARCHIVES: [&str; 3] = [
    "PSP_GAME/USRDIR/Data.wad",
    "PSP_GAME/USRDIR/FE.wad",
    "PSP_GAME/USRDIR/FEData.wad",
];

/// Banks Wipeout Pure carries, on each pressing.
const PURE_BANKS: usize = 29;

/// Banks the PSP disc carries. Fewer means the walk stopped finding them.
const PSP_BANKS: usize = 39;

/// Banks the PS2 disc carries.
const PS2_BANKS: usize = 44;

/// The candidate readings of a cue's `+0x08` as a command index. Kept in the
/// test because the *rejected* ones are the evidence: one of seven tried and the
/// only one that holds differs from a rule guessed and then checked.
type Base = fn(u32, u32, u32) -> Option<u32>;
const BASES: [(&str, Base); 7] = [
    ("raw index", |v, _cmd, _par| Some(v)),
    ("v / 8", |v, _cmd, _par| Some(v / 8)),
    ("(v - command_offset) / 8", |v, cmd, _par| {
        v.checked_sub(cmd).map(|d| d / 8)
    }),
    ("(v - parameter_offset) / 8", |v, _cmd, par| {
        v.checked_sub(par).map(|d| d / 8)
    }),
    ("(v & 0xffffff) / 8", |v, _cmd, _par| {
        Some((v & 0x00ff_ffff) / 8)
    }),
    ("v & 0xffffff", |v, _cmd, _par| Some(v & 0x00ff_ffff)),
    ("v - command_offset", |v, cmd, _par| v.checked_sub(cmd)),
];

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Every `SBlk` blob in one archive, decompressed, with its entry hash.
fn banks_in(disc: &mut DiscImage, archive_path: &str) -> Vec<(u32, Vec<u8>)> {
    let Some(archive) = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == archive_path)
        .cloned()
    else {
        panic!("{archive_path} present");
    };
    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = wad::Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, wad::Directory::directory_len(count))
        .expect("directory");
    let dir = wad::Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    let mut out = Vec::new();
    for entry in &dir.entries {
        if entry.size == 0 {
            continue;
        }
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let blob = match entry.compression {
            wad::Compression::None => raw,
            wad::Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            wad::Compression::Zlib => continue,
        };
        if sblk::looks_like_bank(&blob) {
            out.push((entry.name_hash, blob));
        }
    }
    out
}

/// Every bank on both discs, tagged with which disc it came from.
fn every_bank() -> Vec<(&'static str, u32, Vec<u8>)> {
    let mut out = Vec::new();
    if let Some(path) = image("pulse-psp-usa.chd") {
        let mut disc = DiscImage::open(&path).expect("open");
        for archive in PSP_ARCHIVES {
            out.extend(
                banks_in(&mut disc, archive)
                    .into_iter()
                    .map(|(hash, blob)| ("psp", hash, blob)),
            );
        }
    }
    if let Some(path) = image("pulse-ps2-eu.chd") {
        let mut disc = DiscImage::open(&path).expect("open");
        for archive in PS2_ARCHIVES {
            out.extend(
                banks_in(&mut disc, archive)
                    .into_iter()
                    .map(|(hash, blob)| ("ps2", hash, blob)),
            );
        }
    }
    out
}

/// The command table offset, header `+0x20`. `Bank` does not keep it; the
/// rejected candidates below need it.
fn command_offset(bank: &Bank<'_>) -> u32 {
    u32::from_le_bytes(bank.block[0x20..0x24].try_into().expect("in the header"))
}

#[test]
#[ignore = "needs a disc image"]
fn every_cue_owns_a_run_of_its_bank_command_table() {
    let blobs = every_bank();
    if blobs.is_empty() {
        return;
    }
    println!("banks          {}", blobs.len());

    // Leg one: of seven readings of `+0x08`, how many put every cue's run inside
    // the command table on every bank? Cues the runtime refuses to play are out
    // of scope for all seven equally (`Scream_StartSound` never reads their
    // `+0x08`), so excluding them favours none.
    let mut survivors = Vec::new();
    for (label, base) in BASES {
        let mut banks_ok = 0;
        let mut cues_ok = 0;
        let mut cues = 0;
        for (_, _, blob) in &blobs {
            let bank = Bank::parse(blob).expect("parse");
            let offset = command_offset(&bank);
            let mut all = true;
            for cue in bank.cues().into_iter().filter(sblk::Cue::plays) {
                cues += 1;
                let ok = base(cue.raw, offset, bank.parameter_offset).is_some_and(|start| {
                    start
                        .checked_add(cue.commands as u32)
                        .is_some_and(|end| end <= u32::from(bank.command_count))
                });
                cues_ok += usize::from(ok);
                all &= ok;
            }
            banks_ok += usize::from(all);
        }
        println!(
            "  {label:<26} banks {banks_ok:>3}/{:<3} cues {cues_ok:>5}/{cues}",
            blobs.len()
        );
        if cues_ok == cues {
            survivors.push(label);
        }
    }

    // Leg two: do the runs *tile* the table? The runtime's own gate
    // (`Scream_StartSound` refuses `+0x04 == 0`) excludes the empty cues, not a
    // threshold chosen to make this pass.
    let mut tiled = 0;
    let mut empty = 0;
    let mut empty_raw = Vec::new();
    let mut playable = 0;
    let mut failures = Vec::new();
    for (platform, hash, blob) in &blobs {
        let bank = Bank::parse(blob).expect("parse");
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for cue in bank.cues() {
            if cue.plays() {
                playable += 1;
                runs.push((cue.first_command, cue.commands));
            } else {
                empty += 1;
                if !empty_raw.contains(&cue.raw) {
                    empty_raw.push(cue.raw);
                }
            }
        }
        runs.sort_unstable();
        let mut at = 0;
        let mut exact = true;
        for &(start, count) in &runs {
            exact &= start == at;
            at = start + count;
        }
        exact &= at == usize::from(bank.command_count);
        tiled += usize::from(exact);
        if !exact {
            failures.push(format!("{platform} {hash:08x} {}", bank.name));
        }
    }
    println!("playable cues  {playable}");
    println!("empty cues     {empty}, every one storing +0x08 = {empty_raw:#010x?}");
    println!("tiles exactly  {tiled} of {}", blobs.len());

    assert!(
        blobs.len() >= PSP_BANKS,
        "only {} banks found, expected at least {PSP_BANKS}",
        blobs.len()
    );
    // Six of the seven readings have to die, or "the only one that works" is
    // not a claim this test supports.
    assert_eq!(
        survivors,
        vec!["v / 8", "(v & 0xffffff) / 8"],
        "the surviving readings changed"
    );
    // The two survivors are the same rule: no cue's `+0x08` has anything in its
    // top byte except the empty cues', which the count already excludes.
    assert_eq!(
        tiled,
        blobs.len(),
        "the cue runs do not tile the command table on {failures:?}"
    );
    // Every non-playing cue stores the same sentinel; a zero-count cue with a
    // real offset would mean the rule above hides something.
    assert_eq!(empty_raw, vec![0xffff_fff8], "an unexpected empty cue");
}

#[test]
#[ignore = "needs a disc image"]
fn a_cue_name_agrees_with_the_flags_of_the_waveforms_it_reaches() {
    let blobs = every_bank();
    if blobs.is_empty() {
        return;
    }

    // `Sound_Play` strings the disassembler found on its own, and the bank each
    // has to be in. None of these was known from the bank side.
    // See `docs/ghidra/functions/psp-pulse-usa/{pads,contact-response,exhaust}.md`.
    const KNOWN: [(&str, &str, bool); 4] = [
        ("HUD", "SPEEDUPPAD", false),
        ("SHIP", "~ENGINE", true),
        ("SHIP", ".COLLISIONS", false),
        ("SHIP_ZM", "~ENGINE", true),
    ];

    let mut named = 0;
    let mut silent = 0;
    let (mut tilde_loop, mut tilde) = (0, 0);
    let (mut plain_loop, mut plain) = (0, 0);
    let mut found = Vec::new();

    for (platform, _, blob) in &blobs {
        let bank = Bank::parse(blob).expect("parse");
        for entry in bank.sound_names() {
            named += 1;
            let Some(cue) = bank.cue(entry.cue) else {
                panic!("{} names a cue that does not exist", bank.name);
            };
            let sounds = bank.cue_sounds(&cue);
            if sounds.is_empty() {
                silent += 1;
                continue;
            }
            let looping = sounds.iter().filter(|s| s.mode & 0x40 != 0).count();
            if entry.name.starts_with('~') {
                tilde += sounds.len();
                tilde_loop += looping;
            } else {
                plain += sounds.len();
                plain_loop += looping;
            }

            for (bank_name, cue_name, loops) in KNOWN {
                if bank.name != bank_name || entry.name != cue_name {
                    continue;
                }
                assert_eq!(
                    looping == sounds.len(),
                    loops,
                    "{platform} {bank_name}/{cue_name}: {looping} of {} waveforms loop",
                    sounds.len()
                );
                // Every waveform a cue names has to be inside the section, or
                // the arithmetic reached past the data it claims to index.
                for sound in &sounds {
                    assert!(
                        bank.waveform(sound).is_some(),
                        "{bank_name}/{cue_name} reaches outside the waveform section"
                    );
                }
                found.push(format!(
                    "{platform} {bank_name:<8} {cue_name:<12} {} waveform(s)",
                    sounds.len()
                ));
            }
        }
    }

    println!("named cues     {named}, reaching no waveform {silent}");
    println!("~name          {tilde_loop} of {tilde} waveforms loop");
    println!("plain name     {plain_loop} of {plain} waveforms loop");
    for line in &found {
        println!("  {line}");
    }

    // The asymmetry is one-way: a `~` name hands the caller a voice handle
    // (ADR-0018), which a one-shot like `~SPARKS` also wants, so `~` does not
    // *imply* looping. A plain name essentially never loops, which a wrong
    // cue-to-command mapping could not produce (it would scatter the flag at
    // the `~` rate everywhere).
    assert!(
        plain_loop * 100 < plain,
        "{plain_loop} of {plain} plainly named waveforms loop, which is not the shape expected"
    );
    assert!(
        tilde_loop * 4 > tilde,
        "only {tilde_loop} of {tilde} `~`-named waveforms loop"
    );
    // The whole list, not a count, because the *widths* are the finding:
    // `SPEEDUPPAD` is one sample, `.COLLISIONS` fifteen alternates, Zone's
    // engine nine where the ordinary one is a single loop. `HUD` appears twice
    // on the PSP (`FE.wad` and `Data.wad`, same entry hash).
    assert_eq!(
        found,
        [
            "psp HUD      SPEEDUPPAD   1 waveform(s)",
            "psp HUD      SPEEDUPPAD   1 waveform(s)",
            "psp SHIP     ~ENGINE      1 waveform(s)",
            "psp SHIP     .COLLISIONS  15 waveform(s)",
            "psp SHIP_ZM  ~ENGINE      9 waveform(s)",
            "ps2 HUD      SPEEDUPPAD   1 waveform(s)",
            "ps2 SHIP     ~ENGINE      1 waveform(s)",
            "ps2 SHIP     .COLLISIONS  15 waveform(s)",
            "ps2 SHIP_ZM  ~ENGINE      9 waveform(s)",
        ]
    );
}

/// Wipeout Pure carries `SBlk` banks, which this project said for months it
/// did not.
///
/// # The claim that was wrong, and why it looked right
///
/// `pure-status.md` and `psp-audio.md` recorded, at confidence 85, that *"not
/// one entry in any of Pure's three archives begins with that magic"*. **True**,
/// and also of Pulse, whose 39 banks were already decoded: the magic sits at
/// `0x18` behind the container header and section table, so an offset-0 scan
/// finds nothing anywhere.
///
/// The first assertion is the *control* the original probe lacked (the offset-0
/// scan fails on a disc full of banks); the rest is the correction.
#[test]
#[ignore = "needs a disc image"]
fn wipeout_pure_carries_sblk_banks_after_all() {
    let mut ran = 0;
    for disc in ["pure-psp-usa.chd", "pure-psp-eu.chd"] {
        let Some(path) = image(disc) else {
            continue;
        };
        ran += 1;
        let mut image = DiscImage::open(&path).expect("open");
        let mut banks = 0;
        let mut spans = 0;
        let mut not_adpcm = 0;
        let mut names = 0;
        let mut at_offset_zero = 0;
        for archive_path in PURE_ARCHIVES {
            for (_, blob) in banks_in(&mut image, archive_path) {
                let bank = Bank::parse(&blob).expect("a Pure bank parses as a Pulse one");
                banks += 1;
                at_offset_zero += usize::from(blob.starts_with(sblk::MAGIC));
                names += bank.sound_names().len();
                for sound in bank.sounds() {
                    spans += 1;
                    not_adpcm += usize::from(!sound.is_adpcm());
                }
                assert_eq!(
                    bank.sound_names().len(),
                    usize::from(bank.cue_count),
                    "{}: names and cues disagree",
                    bank.name
                );
            }
        }
        println!("{disc}: {banks} banks, {names} names, {spans} spans");

        // The control: every bank just parsed, and none starts with the magic,
        // which is all the old probe measured.
        assert_eq!(
            at_offset_zero, 0,
            "a bank now begins with the magic, so the trap this records is gone"
        );
        assert_eq!(banks, PURE_BANKS, "Pure's bank count changed");
        // Pure is a PSP title, so it can carry nothing its hardware refuses.
        assert_eq!(not_adpcm, 0, "a Pure span is marked not-PS-ADPCM");
    }
    assert!(ran > 0, "no Pure disc image was present");
}

/// `+0x0e`'s `0x80` marks a waveform that is **not** PS-ADPCM, so a PSP or PS2
/// build, which refuses one, ships none. The polarity was documented backwards
/// until 2026-08-23; see `docs/formats/psp-audio.md`.
#[test]
#[ignore = "needs a disc image"]
fn no_psp_or_ps2_waveform_is_marked_as_not_adpcm() {
    let mut rows = Vec::new();
    for (disc, archives) in [
        ("pulse-psp-usa.chd", &PSP_ARCHIVES[..]),
        ("pulse-ps2-eu.chd", &PS2_ARCHIVES[..]),
        ("pure-psp-usa.chd", &PURE_ARCHIVES[..]),
        ("pure-psp-eu.chd", &PURE_ARCHIVES[..]),
    ] {
        let Some(path) = image(disc) else {
            continue;
        };
        let mut image = DiscImage::open(&path).expect("open");
        let (mut spans, mut flagged) = (0usize, 0usize);
        for archive_path in archives {
            for (_, blob) in banks_in(&mut image, archive_path) {
                for sound in Bank::parse(&blob).expect("parse").sounds() {
                    spans += 1;
                    flagged += usize::from(!sound.is_adpcm());
                }
            }
        }
        println!("{disc}: {flagged} of {spans} spans are marked not-PS-ADPCM");
        rows.push((disc, spans, flagged));
    }
    assert!(!rows.is_empty(), "no disc image was present");
    for (disc, spans, flagged) in rows {
        assert!(spans > 100, "{disc}: only {spans} spans found");
        // A build whose only response to the bit is to print "THIS SYSTEM ONLY
        // SUPPORTS ADPCM VOICE DATA" cannot ship a waveform that sets it.
        assert_eq!(flagged, 0, "{disc} carries {flagged} non-ADPCM spans");
    }
}

#[test]
#[ignore = "needs a disc image"]
fn the_ps2_disc_carries_the_same_container_unchanged() {
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let mut banks = 0;
    let mut blocks = 0;
    let mut in_spec = 0;
    let mut named = 0;
    let mut names = 0;
    let mut tiles = 0;
    for archive in PS2_ARCHIVES {
        for (_, blob) in banks_in(&mut disc, archive) {
            // Byte-for-byte the PSP reader (no endian switch, no relaxed
            // framing): `Bank::parse` refuses anything whose sections do not
            // close exactly, so parsing at all is the result.
            let bank = Bank::parse(&blob).expect("a PS2 bank parses as a PSP one");
            banks += 1;
            blocks += bank.adpcm_blocks();
            for block in bank.waveforms.as_chunks::<{ sblk::ADPCM_BLOCK_LEN }>().0 {
                in_spec += usize::from(sblk::adpcm_block_is_in_spec(block));
            }
            named += usize::from(!bank.name.is_empty());
            let table = bank.sound_names();
            names += table.len();
            assert_eq!(
                table.len(),
                usize::from(bank.cue_count),
                "{}: {} names for {} cues",
                bank.name,
                table.len(),
                bank.cue_count
            );

            // The spans still tile the waveform section: the PS2 payload is the
            // same data, not merely the same framing.
            let mut spans: Vec<(u32, u32)> =
                bank.sounds().iter().map(|s| (s.offset, s.length)).collect();
            spans.sort_unstable();
            spans.dedup();
            let mut at = 0;
            let mut exact = true;
            for (offset, length) in spans {
                exact &= offset == at;
                at = offset + length;
            }
            tiles += usize::from(exact && at as usize == bank.waveforms.len());
        }
    }
    println!("ps2 banks      {banks}");
    println!("adpcm blocks   {blocks}, in spec {in_spec}");
    println!("self-named     {named}");
    println!("names          {names}");
    println!("spans tile     {tiles} of {banks}");

    assert_eq!(banks, PS2_BANKS, "the PS2 bank count changed");
    assert!(
        in_spec * 1000 >= blocks * 999,
        "{in_spec} of {blocks} blocks have an in-spec predictor and shift"
    );
    assert_eq!(tiles, banks, "a PS2 bank's waveform spans do not tile it");
}
