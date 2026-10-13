//! `scripts/psarc.py` (behind `just psarc`) and the Rust reader agree on every
//! archive of every family this project holds.
//!
//! **`#[ignore]`d and never run in CI.** They need the Omega, Vita and PS3
//! extractions and images, which this project does not ship.
//!
//! # Why this exists
//!
//! The script once mapped manifest line `n` to entry `n + 1`. That is true of
//! every PS3 and Vita archive and of 3 of Omega `data00`'s 10,926 entries, so on
//! the PS4 family `just psarc cat` handed back a 5,475-byte `<PI_Grid>` file as
//! `racebox_definition.xml` (the real one is 15,805 bytes) and a shader as the
//! English `entries.xml`. Nothing checked the two readers against each other,
//! and every claim made through the script on Omega was unchecked until it did.
//!
//! # What is asserted, per archive
//!
//! 1. `psarc.py list` names exactly the Rust reader's paths, in the same order,
//!    each with the size of the entry the Rust reader finds by digest. A size is
//!    what would have caught the bug on its first path, and the list comes from
//!    one script call, so it is cheap for the 46,005 Omega entries.
//! 2. `psarc.py verify` reports a bijection: every path matched to an entry and
//!    every entry to a path.
//! 3. `psarc.py cat` returns the Rust reader's bytes for a deterministic sample
//!    of paths spread across the archive, taken from the entries up to
//!    [`SAMPLE_CEILING`] bytes so a music bank does not dominate the run.
//!
//! Plus the two regressions by name: Omega's `racebox_definition.xml` and its
//! English `entries.xml`.
//!
//! Families covered: Omega base `data00`-`data04` and patch `data05`,
//! `data07`-`data09` (NUL-delimited manifest, digest-ordered entries), Wipeout
//! HD's seven PS3 archives read in place in the decrypted image, and Vita
//! `2048`'s five EU archives (newline-delimited, version 1.4).

use std::path::PathBuf;
use std::process::Command;

use oag_assets::psarc::Archive;

/// Entries larger than this are not sampled for the byte comparison; the list
/// comparison still covers their size.
const SAMPLE_CEILING: u64 = 4 << 20;

/// Paths compared byte for byte per archive.
const SAMPLES: usize = 5;

fn script() -> PathBuf {
    oag_testdata::repo_root().join("scripts/psarc.py")
}

fn run(args: &[&str]) -> Vec<u8> {
    let out = Command::new("python3")
        .arg(script())
        .args(args)
        .output()
        .expect("python3 runs");
    assert!(
        out.status.success(),
        "psarc.py {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

/// `(path, size)` per line of `psarc.py list`, in the script's order. A line is
/// `{size:12}  {archive}  {path}`.
fn script_list(spec: &str) -> Vec<(String, u64)> {
    String::from_utf8(run(&["list", spec]))
        .expect("the listing is UTF-8")
        .lines()
        .map(|line| {
            let (size, rest) = line.trim_start().split_once("  ").expect("size column");
            let (_archive, path) = rest.split_once("  ").expect("archive column");
            (path.to_string(), size.parse().expect("a size"))
        })
        .collect()
}

fn reader_list(archive: &Archive) -> Vec<(String, u64)> {
    archive
        .paths()
        .iter()
        .map(|path| {
            let index = archive.index_of_path(path).expect("a listed path resolves");
            (path.clone(), archive.directory().entries[index].size)
        })
        .collect()
}

/// Spread over the archive, rather than the first few entries, which are the
/// ones a positional misreading gets right by luck.
fn sample(listing: &[(String, u64)]) -> Vec<&str> {
    let mut picked: Vec<&str> = Vec::new();
    for k in 0..SAMPLES {
        let start = k * listing.len() / SAMPLES;
        let hit = listing[start..]
            .iter()
            .chain(&listing[..start])
            .find(|(path, size)| {
                (1..=SAMPLE_CEILING).contains(size) && !picked.contains(&path.as_str())
            });
        if let Some((path, _)) = hit {
            picked.push(path);
        }
    }
    picked
}

/// One archive, whole claim. `spec` is whatever both readers accept:
/// a loose `.psarc` path or `<image>:<path on disc>`.
fn agree(spec: &str) -> usize {
    let mut archive = Archive::open(spec).unwrap_or_else(|e| panic!("open {spec}: {e}"));
    let rust = reader_list(&archive);
    let script = script_list(spec);

    assert_eq!(
        script.len(),
        rust.len(),
        "{spec}: entries the script and the reader list"
    );
    let wrong: Vec<_> = script
        .iter()
        .zip(&rust)
        .filter(|(a, b)| a != b)
        .take(5)
        .collect();
    assert!(
        wrong.is_empty(),
        "{spec}: first lines where script and reader differ: {wrong:?}"
    );

    let verify = String::from_utf8(run(&["verify", spec])).expect("UTF-8");
    assert!(
        verify.contains("1 of 1 archives are a bijection"),
        "{spec}: {verify}"
    );

    let samples = sample(&rust);
    assert!(
        !samples.is_empty(),
        "{spec}: nothing small enough to sample"
    );
    for path in samples {
        let theirs = run(&["cat", spec, path]);
        let ours = archive.read_path(path).expect("the reader reads it");
        assert!(
            theirs == ours,
            "{spec}: {path}: script returned {} bytes, the reader {}",
            theirs.len(),
            ours.len()
        );
    }
    rust.len()
}

fn exact(relative: &str) -> Option<PathBuf> {
    oag_testdata::exact(relative)
}

fn loose(relative: &str) -> Option<String> {
    exact(relative).map(|p: PathBuf| p.display().to_string())
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu{,-patch}/uroot"]
fn the_script_reads_omegas_nine_archives_as_the_reader_does() {
    let mut entries = 0;
    let mut archives = 0;
    let base = ["data00", "data01", "data02", "data03", "data04"].map(|n| ("omega-eu", n));
    let patch = ["data05", "data07", "data08", "data09"].map(|n| ("omega-eu-patch", n));
    for (dir, name) in base.into_iter().chain(patch) {
        let Some(spec) = loose(&format!("data/extracted/ps4/{dir}/uroot/{name}.psarc")) else {
            return;
        };
        entries += agree(&spec);
        archives += 1;
    }
    assert_eq!(archives, 9, "five base and four patch archives");
    assert_eq!(entries, 46_005, "docs/formats/psarc.md's bijection total");
}

/// The two reads that were wrong, by name. Both are in the patch's `data09`,
/// the only Omega archive with a complete front end.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu-patch/uroot/data09.psarc"]
fn the_two_reads_the_old_script_got_wrong_are_right() {
    let Some(spec) = loose("data/extracted/ps4/omega-eu-patch/uroot/data09.psarc") else {
        return;
    };

    let racebox = run(&[
        "cat",
        &spec,
        "/data/plugins/frontend/gui/racebox_definition.xml",
    ]);
    assert_eq!(racebox.len(), 15_805);
    let text = String::from_utf8_lossy(&racebox);
    for list in ["NumberOfPlayers", "SplitScreen"] {
        assert!(text.contains(list), "racebox_definition.xml authors {list}");
    }
    assert!(
        !text.contains("<PI_Grid"),
        "not the 5,475-byte grid file the positional lookup returned"
    );

    let strings = run(&["cat", &spec, "/data/plugins/languages/english/entries.xml"]);
    assert_eq!(strings.len(), 212_564);
    assert!(
        strings.starts_with(b"<StringTable>"),
        "the English string table, not a shader blob"
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_script_reads_hds_seven_archives_as_the_reader_does() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let entries: usize = (0..7)
        .map(|n| {
            agree(&format!(
                "{}:PS3_GAME/USRDIR/DATA0{n}.PSARC",
                image.display()
            ))
        })
        .sum();
    assert_eq!(entries, 11_664, "docs/formats/psarc.md's census");
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_script_reads_vitas_2048_archives_as_the_reader_does() {
    let mut entries = 0;
    let mut archives = 0;
    for relative in [
        "base/PSP2/data.psarc",
        "dlc1/PSP2/dlc1.psarc",
        "dlc2/PSP2/dlc2.psarc",
        "patch-v104/PSP2/data1.psarc",
        "patch-v104/PSP2/data2.psarc",
    ] {
        let Some(spec) = loose(&format!("data/extracted/vita/PCSF00007/{relative}")) else {
            return;
        };
        entries += agree(&spec);
        archives += 1;
    }
    assert_eq!(archives, 5);
    assert_eq!(entries, 18_430 + 1_903 + 1_721 + 854 + 1_526);
}
