//! Scratch tool: for every real entry (`size > 0`, by directory index - not
//! [`oag_assets::psarc::Archive::paths`], which only lists entries that also
//! resolved to a manifest path) in a `.psarc`, read it back through the
//! crate's own reader and report whether the entry's first byte is non-zero.
//!
//! First byte, not "any byte in the whole entry": that is what
//! `docs/formats/psarc.md`'s "Block data location" table itself measures
//! ("non-zero **at** `entry.offset`"), and it is not a stand-in for a
//! whole-buffer check. A first draft of this tool checked
//! `bytes.iter().any(|&b| b != 0)` and got a much higher, wrong-looking
//! "real" rate on the PS4 Omega Collection family: a large entry that is
//! genuinely garbage/zero at its header - the only place that matters, since
//! that is what a real reader checks first - can still carry a single stray
//! non-zero byte tens of kilobytes in, and "any non-zero byte" counts that as
//! real. Checking only the first byte reproduces the doc's own 54%/39%/30%
//! figures on `data00`/`data01`/`data03.psarc` almost exactly; see that
//! page's own account of the trap.
//!
//! `cargo run -p oag-assets --release --example psarc_sweep -- <path.psarc> [--verbose]`
//!
//! `--verbose` prints one line per real entry: index, real/zero/error, size,
//! offset, the first 16 bytes read, and the entry's path if one resolved.
//!
//! **Superseded by [`psarc_oracle`](../psarc_oracle.rs) for "is this entry's
//! content real" specifically.** This tool's own trap writeup above already
//! flags the position-based check as fragile; the failure it did not catch
//! is the opposite one - a real `.gnf` entry whose payload does not start at
//! byte zero (measured: several thousand bytes of legitimate leading zero
//! before real tiled-texture content) reads as "zero" here when it is not.
//! `psarc_oracle` checks the format's own magic instead of a byte position
//! and reports a three-way split (valid / all-zero / garbage) rather than
//! this tool's two. See `docs/formats/psarc.md`'s "Block data location"
//! section.

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: psarc_sweep <path> [--verbose]");
    let verbose = args.next().as_deref() == Some("--verbose");

    let mut archive =
        oag_assets::psarc::Archive::open(&path).unwrap_or_else(|e| panic!("open {path}: {e}"));

    // index -> path, for --verbose's per-entry dump.
    let mut path_of_index = std::collections::HashMap::new();
    for p in archive.paths().to_vec() {
        if let Some(i) = archive.index_of_path(&p) {
            path_of_index.insert(i, p);
        }
    }

    let entry_count = archive.directory().entries.len();
    let real_indices: Vec<usize> = (0..entry_count)
        .filter(|&i| archive.directory().entries[i].size > 0)
        .collect();
    let mut real = 0usize;
    let mut non_zero = 0usize;
    let mut errors = 0usize;

    for i in real_indices {
        match archive.read(i) {
            Ok(bytes) => {
                real += 1;
                let is_real = bytes.first().is_some_and(|&b| b != 0);
                if is_real {
                    non_zero += 1;
                }
                if verbose {
                    let entry = archive.directory().entries[i];
                    let head: Vec<String> =
                        bytes.iter().take(16).map(|b| format!("{b:02x}")).collect();
                    println!(
                        "{i}\t{}\t{}\t{}\t{}\t{}",
                        if is_real { "real" } else { "zero" },
                        entry.size,
                        entry.offset,
                        head.join(""),
                        path_of_index.get(&i).map_or("<no path>", |p| p.as_str()),
                    );
                }
            }
            Err(e) => {
                errors += 1;
                if verbose {
                    println!("{i}\terror\t{e}");
                }
            }
        }
    }

    println!(
        "{path}: {real} real entries (size > 0, by directory index), {non_zero} non-zero ({:.1}%), {errors} errors",
        100.0 * non_zero as f64 / real.max(1) as f64
    );
}
