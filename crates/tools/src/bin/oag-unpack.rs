//! Inspects and extracts the contents of PSP UMD and PS2 DVD disc images.
//!
//! ```text
//! oag-unpack info      <image>              platform, serial, volume descriptor
//! oag-unpack container <image>              CHD header and track metadata
//! oag-unpack list      <image> [pattern]    file listing
//! oag-unpack extract   <image> -o <dir>     extract files
//! oag-unpack hexdump   <image> <file>       dump bytes from a file on the disc
//! oag-unpack sniff     <image> [pattern]    magic + entropy triage
//! ```

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use oag_disc::{DiscImage, Entry};
use oag_formats::entropy;
use oag_tools::{glob, humanise};

/// How much of a file to read when sniffing it.
///
/// Enough for a header plus a representative entropy sample, without
/// decompressing a whole 100 MiB archive just to look at its first four bytes.
const SNIFF_BYTES: u64 = 64 * 1024;

#[derive(Parser, Debug)]
#[command(
    name = "oag-unpack",
    about = "Inspect and extract PSP UMD and PS2 DVD disc images",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Report the platform, serial and volume descriptor.
    Info {
        /// Disc image (`.chd` or `.iso`).
        image: PathBuf,
    },

    /// Report container-level facts, without mounting a filesystem.
    ///
    /// Use this when `info` fails: it says what the container claims to be,
    /// which is usually enough to explain the failure.
    Container {
        /// Disc image (`.chd` or `.iso`).
        image: PathBuf,
    },

    /// List the files on the disc.
    List {
        /// Disc image (`.chd` or `.iso`).
        image: PathBuf,
        /// Only list paths matching this glob, for example `*.wad`.
        pattern: Option<String>,
        /// Include directories in the listing.
        #[arg(long)]
        dirs: bool,
        /// Sort by size, largest first, instead of by path.
        #[arg(long)]
        by_size: bool,
    },

    /// Extract files to a directory.
    Extract {
        /// Disc image (`.chd` or `.iso`).
        image: PathBuf,
        /// Destination directory.
        #[arg(short, long)]
        out: PathBuf,
        /// Only extract paths matching this glob.
        pattern: Option<String>,
        /// Overwrite files that already exist.
        #[arg(long)]
        force: bool,
    },

    /// Hexdump the start of a file on the disc.
    Hexdump {
        /// Disc image (`.chd` or `.iso`).
        image: PathBuf,
        /// Path of the file on the disc.
        file: String,
        /// Byte offset to start at.
        #[arg(long, default_value_t = 0)]
        offset: u64,
        /// How many bytes to show.
        #[arg(long, default_value_t = 256)]
        length: u64,
    },

    /// Report magic bytes and entropy for each file, to triage unknown formats.
    Sniff {
        /// Disc image (`.chd` or `.iso`).
        image: PathBuf,
        /// Only sniff paths matching this glob.
        pattern: Option<String>,
        /// Group by extension and summarise, instead of listing every file.
        #[arg(long)]
        summary: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Info { image } => info(&image),
        Command::Container { image } => container(&image),
        Command::List {
            image,
            pattern,
            dirs,
            by_size,
        } => list(&image, pattern.as_deref(), dirs, by_size),
        Command::Extract {
            image,
            out,
            pattern,
            force,
        } => extract(&image, &out, pattern.as_deref(), force),
        Command::Hexdump {
            image,
            file,
            offset,
            length,
        } => hexdump(&image, &file, offset, length),
        Command::Sniff {
            image,
            pattern,
            summary,
        } => sniff(&image, pattern.as_deref(), summary),
    }
}

fn open(path: &Path) -> Result<DiscImage> {
    DiscImage::open(path).with_context(|| format!("opening {}", path.display()))
}

/// Selects the entries a subcommand should act on.
fn select(entries: &[Entry], pattern: Option<&str>, include_dirs: bool) -> Vec<Entry> {
    entries
        .iter()
        .filter(|e| include_dirs || !e.is_directory)
        .filter(|e| pattern.is_none_or(|p| glob::matches(p, &e.path)))
        .cloned()
        .collect()
}

fn info(path: &Path) -> Result<()> {
    let mut disc = open(path)?;

    let container = disc.container();
    let sectors = disc.sector_count();
    let pvd = disc
        .volume_descriptor()
        .context("reading volume descriptor")?;
    let title = disc.identify().context("identifying the title")?;
    let entries = disc.entries()?;

    let files = entries.iter().filter(|e| !e.is_directory).count();
    let dirs = entries.len() - files;
    let content: u64 = entries
        .iter()
        .filter(|e| !e.is_directory)
        .map(|e| e.size)
        .sum();

    println!("image          {}", path.display());
    println!("container      {container}");
    println!(
        "capacity       {} sectors, {}",
        sectors,
        humanise::bytes(u64::from(sectors) * 2048)
    );
    println!();
    println!("platform       {}", title.platform);
    println!(
        "serial         {}",
        title.serial.as_deref().unwrap_or("unknown")
    );
    println!(
        "boot           {}",
        title.boot_path.as_deref().unwrap_or("unknown")
    );
    if let Some(raw) = &title.raw {
        println!("identified by  {raw}");
    }
    println!();
    println!("volume id      {}", pvd.volume_id);
    println!("system id      {}", pvd.system_id);
    println!("publisher      {}", pvd.publisher_id);
    println!("application    {}", pvd.application_id);
    println!("created        {}", pvd.created);
    println!("block size     {} bytes", pvd.logical_block_size);
    println!("volume size    {} sectors", pvd.volume_space_size);
    println!();
    println!(
        "contents       {files} files in {dirs} directories, {}",
        humanise::bytes(content)
    );

    Ok(())
}

fn container(path: &Path) -> Result<()> {
    let info = oag_disc::chd_source::describe(path)
        .with_context(|| format!("reading the CHD header of {}", path.display()))?;

    println!("image          {}", path.display());
    println!("chd version    {}", info.version);
    println!("compressed     {}", info.compressed);
    println!(
        "hunks          {} x {} bytes",
        info.hunk_count, info.hunk_bytes
    );
    println!(
        "units          {} x {} bytes{}",
        info.unit_count,
        info.unit_bytes,
        match info.unit_bytes {
            2048 => "  (DVD / UMD user data)",
            2352 => "  (CD, no subcode)",
            2448 => "  (CD + 96-byte subcode)",
            _ => "",
        }
    );
    println!(
        "logical size   {} ({} bytes)",
        humanise::bytes(info.logical_bytes),
        info.logical_bytes
    );

    if info.metadata.is_empty() {
        println!("metadata       none");
    } else {
        println!("metadata       {} entries", info.metadata.len());
        for (tag, value) in &info.metadata {
            // Track entries repeat once per track and are long; show enough to
            // identify the track type without flooding the terminal.
            let shown: String = value.chars().take(100).collect();
            println!("  [{tag}] {shown}");
        }
    }

    Ok(())
}

fn list(path: &Path, pattern: Option<&str>, dirs: bool, by_size: bool) -> Result<()> {
    let mut disc = open(path)?;
    let mut selected = select(disc.entries()?, pattern, dirs);

    if by_size {
        selected.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
    } else {
        selected.sort_by(|a, b| a.path.cmp(&b.path));
    }

    println!("{:>12}  {:>8}  PATH", "SIZE", "LBA");
    for e in &selected {
        let size = if e.is_directory {
            "<dir>".to_string()
        } else {
            e.size.to_string()
        };
        println!("{size:>12}  {:>8}  {}", e.lba, e.path);
    }

    let total: u64 = selected
        .iter()
        .filter(|e| !e.is_directory)
        .map(|e| e.size)
        .sum();
    println!();
    println!("{} entries, {}", selected.len(), humanise::bytes(total));

    Ok(())
}

fn extract(path: &Path, out: &Path, pattern: Option<&str>, force: bool) -> Result<()> {
    let mut disc = open(path)?;
    let selected = select(disc.entries()?, pattern, false);

    if selected.is_empty() {
        println!("nothing to extract");
        return Ok(());
    }

    std::fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;

    let mut written = 0u64;
    let mut skipped = 0usize;

    for entry in &selected {
        let dest = safe_join(out, &entry.path)?;

        if dest.exists() && !force {
            skipped += 1;
            continue;
        }

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }

        let data = disc
            .read_entry(entry)
            .with_context(|| format!("reading {}", entry.path))?;

        std::fs::write(&dest, &data).with_context(|| format!("writing {}", dest.display()))?;
        written += data.len() as u64;

        println!("{:>12}  {}", data.len(), entry.path);
    }

    println!();
    println!(
        "extracted {} files ({}) to {}",
        selected.len() - skipped,
        humanise::bytes(written),
        out.display()
    );
    if skipped > 0 {
        println!("skipped {skipped} existing files (use --force to overwrite)");
    }

    Ok(())
}

/// Joins a disc path onto an output directory, refusing to escape it.
///
/// Disc paths come from a file we did not author. A record naming `../../etc`
/// must not be able to write outside the destination.
///
/// # The colon matters on Windows
///
/// `PathBuf::push` is platform-aware, and on Windows a component that looks like
/// a drive-relative path (`C:evil`) or a stream name (`file:stream`) is not
/// treated as an ordinary name: pushing `C:evil` **replaces** everything
/// accumulated so far. `decode_identifier` passes any ASCII byte through, so a
/// crafted ISO can produce exactly that. Rejecting the colon outright is cheap
/// and no legitimate ISO 9660 identifier contains one.
fn safe_join(root: &Path, disc_path: &str) -> Result<PathBuf> {
    let mut dest = root.to_path_buf();

    for component in disc_path.split('/') {
        anyhow::ensure!(
            !component.is_empty() && component != "." && component != "..",
            "refusing to extract {disc_path}: path component {component:?} escapes the output directory",
        );
        anyhow::ensure!(
            !component.contains('\\') && !component.contains('\0') && !component.contains(':'),
            "refusing to extract {disc_path}: illegal character in path component {component:?}",
        );
        dest.push(component);
    }

    Ok(dest)
}

fn hexdump(path: &Path, file: &str, offset: u64, length: u64) -> Result<()> {
    let mut disc = open(path)?;

    let entry = disc
        .entries()?
        .iter()
        .find(|e| !e.is_directory && e.path.eq_ignore_ascii_case(file))
        .cloned()
        .with_context(|| format!("{file} is not on this disc"))?;

    anyhow::ensure!(
        offset < entry.size,
        "offset {offset} is past the end of {file} ({} bytes)",
        entry.size
    );

    let window = disc.read_entry_range(&entry, offset, length)?;

    println!(
        "{}  offset {offset}, {} of {} bytes",
        entry.path,
        window.len(),
        entry.size
    );
    println!();

    for (row, chunk) in window.chunks(16).enumerate() {
        let addr = offset + (row * 16) as u64;
        print!("{addr:08x}  ");
        for i in 0..16 {
            match chunk.get(i) {
                Some(b) => print!("{b:02x} "),
                None => print!("   "),
            }
            if i == 7 {
                print!(" ");
            }
        }
        let text: String = chunk
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        println!(" |{text}|");
    }

    Ok(())
}

fn sniff(path: &Path, pattern: Option<&str>, summary: bool) -> Result<()> {
    let mut disc = open(path)?;
    let selected = select(disc.entries()?, pattern, false);

    struct Sniffed {
        entry: Entry,
        signature: String,
        unknown: bool,
        entropy: f64,
    }

    let mut results = Vec::with_capacity(selected.len());
    for entry in selected {
        let head = disc
            .read_entry_head(&entry, SNIFF_BYTES)
            .with_context(|| format!("reading {}", entry.path))?;

        let signature = oag_formats::identify(&head);
        results.push(Sniffed {
            entropy: entropy::shannon(&head),
            signature: signature.to_string(),
            unknown: signature.is_unknown(),
            entry,
        });
    }

    if summary {
        print_sniff_summary(
            &results
                .iter()
                .map(|s| (&s.entry, s.entropy, s.unknown))
                .collect::<Vec<_>>(),
        );
        return Ok(());
    }

    println!(
        "{:>12}  {:>7}  {:<11}  {:<28}  PATH",
        "SIZE", "ENTROPY", "DENSITY", "SIGNATURE"
    );
    for s in &results {
        println!(
            "{:>12}  {:>7.3}  {:<11}  {:<28}  {}",
            s.entry.size,
            s.entropy,
            entropy::classify(s.entropy).to_string(),
            s.signature,
            s.entry.path
        );
    }

    let unknown = results.iter().filter(|s| s.unknown).count();
    println!();
    println!(
        "{} files, {} with no recognised signature (these are the reverse-engineering targets)",
        results.len(),
        unknown
    );

    std::io::stdout().flush()?;
    Ok(())
}

/// Groups by extension, which is the fastest way to see which unknown format
/// accounts for most of the disc and therefore deserves attention first.
fn print_sniff_summary(results: &[(&Entry, f64, bool)]) {
    struct Group {
        count: usize,
        bytes: u64,
        entropy_sum: f64,
        unknown: usize,
    }

    let mut groups: BTreeMap<String, Group> = BTreeMap::new();

    for (entry, entropy, unknown) in results {
        let key = entry.extension().unwrap_or_else(|| "(none)".to_string());
        let g = groups.entry(key).or_insert(Group {
            count: 0,
            bytes: 0,
            entropy_sum: 0.0,
            unknown: 0,
        });
        g.count += 1;
        g.bytes += entry.size;
        g.entropy_sum += entropy;
        g.unknown += usize::from(*unknown);
    }

    let mut rows: Vec<_> = groups.into_iter().collect();
    rows.sort_by_key(|(_, g)| std::cmp::Reverse(g.bytes));

    println!(
        "{:<10}  {:>6}  {:>12}  {:>9}  {:>9}",
        "EXT", "COUNT", "BYTES", "MEAN ENT", "UNKNOWN"
    );
    for (ext, g) in &rows {
        println!(
            "{:<10}  {:>6}  {:>12}  {:>9.3}  {:>9}",
            ext,
            g.count,
            humanise::bytes(g.bytes),
            g.entropy_sum / g.count as f64,
            g.unknown
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The only thing between a hostile ISO and arbitrary filesystem writes.
    #[test]
    fn safe_join_accepts_ordinary_disc_paths() {
        let root = Path::new("/out");
        assert_eq!(
            safe_join(root, "PSP_GAME/SYSDIR/BOOT.BIN").unwrap(),
            Path::new("/out/PSP_GAME/SYSDIR/BOOT.BIN")
        );
        assert_eq!(
            safe_join(root, "FILE.EXT").unwrap(),
            Path::new("/out/FILE.EXT")
        );
    }

    #[test]
    fn safe_join_refuses_traversal() {
        let root = Path::new("/out");
        for path in [
            "../etc/passwd",
            "a/../../etc/passwd",
            "a/./b",
            "/absolute",
            "a//b",
            "",
            "..",
        ] {
            assert!(
                safe_join(root, path).is_err(),
                "{path:?} should not be joinable"
            );
        }
    }

    /// `PathBuf::push` treats these as drive-relative or stream names on
    /// Windows, where pushing one discards the accumulated path entirely. The
    /// test asserts the rejection on every platform, because the ISO does not
    /// know which one it will be read on.
    #[test]
    fn safe_join_refuses_windows_path_prefixes() {
        let root = Path::new("/out");
        for path in ["C:evil", "C:/evil", "dir/C:evil", "file:stream"] {
            assert!(
                safe_join(root, path).is_err(),
                "{path:?} escapes on Windows and must be refused everywhere"
            );
        }
    }

    #[test]
    fn safe_join_refuses_separators_and_nul_inside_a_component() {
        let root = Path::new("/out");
        assert!(safe_join(root, "a\\b").is_err());
        assert!(safe_join(root, "a\0b").is_err());
    }
}
