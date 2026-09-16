//! The real oracle for `omega-ps4-eu`'s "which entries actually read their
//! real content" question, replacing [`psarc_sweep`]'s first-byte heuristic
//! with a per-extension magic check.
//!
//! # Why the digest cannot be the oracle
//!
//! The task this tool was written for asked for an MD5-of-content check.
//! That is not what this family's (or PS3's) digest field is:
//! `oag_formats::psarc::path_digest` is MD5 of the entry's own **path**,
//! uppercased - `docs/formats/psarc.md`'s "The check the confidence rests
//! on" section proves it at 11,664 of 11,664 on PS3, and this family's own
//! digest-based path matching (88 confidence) rests on the same field. A
//! digest match therefore proves an entry's *path*, not its *content* - it
//! is already checked, by construction, on every entry `Archive::paths`
//! lists. There is nothing left to verify on that axis.
//!
//! # Why first-byte-nonzero was misleading
//!
//! `psarc_sweep` (this crate's other example) checks only whether an
//! entry's first byte is nonzero, which `docs/formats/psarc.md`'s "Block
//! data location" section already uses to report "30-54% real" per
//! archive. That undercounts: several of this family's real, correctly
//! located `.gnf` entries store a long run of zero bytes before their pixel
//! payload (measured directly - see the module docs on the dedup case
//! below), so "first byte zero" catches real content and calls it fake.
//!
//! This tool checks the format's own magic instead, where one exists:
//! `.vex` (`VEXX` at `+0x0c`, `docs/formats/vex.md`), `.gnf` (`GNF ` at
//! `+0x00`, `docs/formats/README.md`'s Omega rows), and - **new**, the
//! `lane/omega-rcs` session that read the container underneath `.rcsmodel`/
//! `.rcsmaterial` - `ED AD 5C CA`/`E5 AD 5C CA` at `+0x00` on this PS4
//! family only (`docs/formats/rcsmodel.md`'s PS4 section; PS3 genuinely has
//! no magic, per `crates/rcs/src/rcsmodel.rs`'s own module docs, and that is
//! unchanged). Any extension without a known magic is still reported
//! "unvalidated" rather than forced into a bucket the format gives no way to
//! check.
//!
//! # The dedup finding
//!
//! Eleven different ship liveries' `ShieldHexagonal_ALPHA.gnf` on
//! `data03.psarc` share one identical `(offset, size)` pair - the packer
//! deduplicated byte-identical content rather than storing eleven copies.
//! Two adjacent, differently-named `Holographic_02_GLOW.gnf` entries at the
//! immediately preceding offset are genuinely real (`GNF ` at their very
//! first byte) - so a shared offset is not itself a sign of anything wrong.
//! The `ShieldHexagonal_ALPHA.gnf` group itself, though, is all zero for its
//! first 15,616 bytes, has no `GNF ` anywhere in its declared range, and its
//! nonzero middle (bytes 15,616-38,739) does not decode as anything -
//! genuinely missing content at a genuinely-correct-looking offset, not a
//! location bug. See `docs/formats/psarc.md`.
//!
//! `cargo run -p oag-assets --release --example psarc_oracle -- <path.psarc> [--verbose]`
//!
//! `--verbose` prints one line per entry that is not cleanly valid: index,
//! bucket, extension, size, offset, first_block, block_width, first 16
//! bytes, path.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Bucket {
    /// Magic checked and matched: this is the format it claims to be.
    Valid,
    /// The format has a known magic, and every byte in the declared range
    /// failed to show it - the manifest-path digest case, not a content
    /// digest: this is "genuinely stores nothing" as far as this tool can
    /// tell, either a real gap in the packed data or an intentionally empty
    /// stub for an unused variant.
    AllZero,
    /// Neither all-zero nor the expected magic: bytes are present but do not
    /// decode as the format the extension claims. The population this
    /// project has not explained yet.
    Garbage,
    /// The format (`.rcsmodel`) carries no magic to check by design, so
    /// "valid" cannot be asserted - only zero vs. nonzero.
    UnvalidatedNonZero,
    UnvalidatedAllZero,
    /// `Archive::read` itself failed.
    Error,
}

impl Bucket {
    fn label(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::AllZero => "all_zero",
            Self::Garbage => "garbage",
            Self::UnvalidatedNonZero => "unvalidated_nonzero",
            Self::UnvalidatedAllZero => "unvalidated_all_zero",
            Self::Error => "error",
        }
    }
}

/// Extensions this tool knows a magic for. `None` means "no magic exists for
/// this format" (measured, not merely undocumented). Absent from this table
/// entirely means "not checked yet".
///
/// `.rcsmodel`/`.rcsmaterial` carry **no magic on PS3** - `oag_rcs::rcsmodel`'s
/// own module docs measure that directly, and it still holds there. **On this
/// PS4 family they do**: every real (non-all-zero) sample carries a fixed
/// four-byte tag at `+0x00`, read little-endian as `0xCA5CADxx` ("cascaded").
/// **The low byte is not one-per-extension** - checked 2026-09-16 against
/// `.rcsskeleton` and `.rcsanimclip` too, both of which turned up in the same
/// census that first found `.rcsmodel`/`.rcsmaterial`: `.rcsmodel`,
/// `.rcsskeleton` and `.rcsanimclip` all share `0xCA5CADED`, and only
/// `.rcsmaterial` uses `0xCA5CADE5` - two values across four extensions, not
/// four. `.col` (also found by the same census) was checked and is **not** a
/// member: its one real sample opens `6B 64 74 72` instead. This is a
/// PS4-only finding (this tool only ever runs against this family's
/// archives) and does not change what `oag_rcs::rcsmodel::Model::parse`
/// (big-endian, PS3-shaped) accepts - see `docs/formats/rcsmodel.md`'s PS4
/// section. Confidence 85: consistent across every real sample checked
/// (77 of 77 `.rcsmodel` and 437 of 439 `.rcsmaterial` disc-wide, per that
/// page's own header-field table), but the container underneath the tag is
/// a different, unread layout rather than a byte-swap of PS3's.
fn expected_magic(ext: &str) -> Option<Option<(usize, &'static [u8])>> {
    match ext {
        "vex" => Some(Some((0x0c, b"VEXX"))),
        "gnf" => Some(Some((0x00, b"GNF "))),
        "rcsmodel" | "rcsskeleton" | "rcsanimclip" => Some(Some((0x00, &[0xed, 0xad, 0x5c, 0xca]))),
        "rcsmaterial" => Some(Some((0x00, &[0xe5, 0xad, 0x5c, 0xca]))),
        _ => None,
    }
}

fn classify(ext: &str, bytes: &[u8]) -> Bucket {
    let all_zero = bytes.iter().all(|&b| b == 0);
    match expected_magic(ext) {
        Some(Some((at, magic))) => {
            let matches = bytes.len() >= at + magic.len() && &bytes[at..at + magic.len()] == magic;
            if matches {
                Bucket::Valid
            } else if all_zero {
                Bucket::AllZero
            } else {
                Bucket::Garbage
            }
        }
        Some(None) | None => {
            if all_zero {
                Bucket::UnvalidatedAllZero
            } else {
                Bucket::UnvalidatedNonZero
            }
        }
    }
}

fn extension_of(path: &str) -> String {
    path.rsplit('.')
        .next()
        .filter(|_| path.contains('.'))
        .unwrap_or("<none>")
        .to_ascii_lowercase()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: psarc_oracle <path.psarc> [--verbose]");
    let verbose = args.next().as_deref() == Some("--verbose");

    let mut archive =
        oag_assets::psarc::Archive::open(&path).unwrap_or_else(|e| panic!("open {path}: {e}"));

    // path -> directory index, from the digest-matched set `Archive::paths`
    // already exposes - the only entries this tool has a name for.
    let mut path_of_index = std::collections::HashMap::new();
    for p in archive.paths().to_vec() {
        if let Some(i) = archive.index_of_path(&p) {
            path_of_index.insert(i, p);
        }
    }

    let mut by_ext_bucket: BTreeMap<(String, Bucket), usize> = BTreeMap::new();
    let mut total: BTreeMap<Bucket, usize> = BTreeMap::new();

    let mut indices: Vec<usize> = path_of_index.keys().copied().collect();
    indices.sort_unstable();

    for index in indices {
        let path = path_of_index[&index].clone();
        let ext = extension_of(&path);
        let entry = archive.directory().entries[index];
        let block_width = archive.directory().block_width;

        let bucket = match archive.read(index) {
            Ok(bytes) => classify(&ext, &bytes),
            Err(_) => Bucket::Error,
        };

        *by_ext_bucket.entry((ext.clone(), bucket)).or_insert(0) += 1;
        *total.entry(bucket).or_insert(0) += 1;

        let interesting = !matches!(bucket, Bucket::Valid | Bucket::UnvalidatedNonZero);
        if verbose && interesting {
            let bytes = archive.read(index).unwrap_or_default();
            let head: Vec<String> = bytes.iter().take(16).map(|b| format!("{b:02x}")).collect();
            println!(
                "{index}\t{}\t{ext}\t{}\t{}\t{}\t{}\t{}\t{path}",
                bucket.label(),
                entry.size,
                entry.offset,
                entry.first_block,
                block_width,
                head.join(""),
            );
        }
    }

    println!("\n{path}:");
    for (bucket, count) in &total {
        println!("  {}: {count}", bucket.label());
    }
    println!("  by extension:");
    for ((ext, bucket), count) in &by_ext_bucket {
        println!("    {ext}\t{}\t{count}", bucket.label());
    }
}
