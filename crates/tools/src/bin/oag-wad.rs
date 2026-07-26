//! Inspects and extracts WAD archives from Wipeout Pure and Pulse.
//!
//! ```text
//! oag-wad list    <archive> [--names <file>]   directory listing
//! oag-wad tags    <archive>                    inventory the blob type tags
//! oag-wad extract <archive> -o <dir>           extract every blob
//! oag-wad hash    <name>                       hash a name, for spot checks
//! ```
//!
//! `<archive>` is either a path to an extracted `.wad`, or `<image>:<path>` to
//! read one straight out of a disc image:
//!
//! ```sh
//! oag-wad list data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad
//! ```
//!
//! Reading from the image matters: `Data.wad` is 315 MiB, and extracting it
//! just to look at its directory is a waste of several minutes.
//!
//! # Names are hashes
//!
//! The directory stores only a CRC-32 of each name, so an archive cannot be
//! listed with names on its own. Pass `--names` a file of candidates, one per
//! line, and matching hashes are resolved. Candidates usually come from strings
//! in the game binary:
//!
//! ```sh
//! oag-wad list <archive> --names data/extracted/psp/boot-strings.names
//! ```
//!
//! Coverage varies: 52 of 54 entries in `BEData.wad` resolve this way, but only
//! 11 of 242 in `FEData.wad`, because most of those names are assembled at
//! runtime from format strings rather than stored whole.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

use oag_disc::DiscImage;
use oag_formats::texture::Texture;
use oag_formats::wad::{self, Blob, Compression, Directory};
use oag_formats::{fexml, lzss, png};
use oag_tools::humanise;

/// How much of a blob to read when only its type tag is wanted.
const TAG_PEEK_BYTES: u64 = 16;

#[derive(Parser, Debug)]
#[command(
    name = "oag-wad",
    about = "Inspect and extract Wipeout WAD archives",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// List the directory.
    List {
        /// A `.wad` path, or `<image>:<path-on-disc>`.
        archive: String,
        /// Sort by size, largest first, instead of by offset.
        #[arg(long)]
        by_size: bool,
        /// Show at most this many entries. 0 shows all.
        #[arg(long, default_value_t = 0)]
        limit: usize,
        /// A file of candidate names, one per line, used to resolve hashes.
        #[arg(long)]
        names: Option<PathBuf>,
    },

    /// Print the hash of a name, for spot checks.
    Hash {
        /// The entry name, for example `Data\\FE\\Images\\hex_bg.mip`.
        name: String,
    },

    /// Inventory the type tag at the start of each blob.
    Tags {
        /// A `.wad` path, or `<image>:<path-on-disc>`.
        archive: String,
    },

    /// Print one entry to stdout.
    Cat {
        /// A `.wad` path, or `<image>:<path-on-disc>`.
        archive: String,
        /// Entry index, or `0x`-prefixed name hash, or a name to hash.
        entry: String,
        /// Expand shortened front-end XML through its own dictionary.
        #[arg(long)]
        expand: bool,
    },

    /// Decompress every blob and check it against its declared size.
    ///
    /// Writes nothing. This is the round-trip test for the LZSS decoder.
    Verify {
        /// A `.wad` path, or `<image>:<path-on-disc>`.
        archive: String,
    },

    /// Extract every blob.
    Extract {
        /// A `.wad` path, or `<image>:<path-on-disc>`.
        archive: String,
        /// Destination directory.
        #[arg(short, long)]
        out: PathBuf,
        /// Also write a .png beside every blob that decodes as a texture.
        #[arg(long)]
        png: bool,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::List {
            archive,
            by_size,
            limit,
            names,
        } => list(&archive, by_size, limit, names.as_deref()),
        Command::Hash { name } => {
            println!("{:08x}  {name}", wad::hash_name(&name));
            Ok(())
        }
        Command::Tags { archive } => tags(&archive),
        Command::Verify { archive } => verify(&archive),
        Command::Cat {
            archive,
            entry,
            expand,
        } => cat(&archive, &entry, expand),
        Command::Extract { archive, out, png } => extract(&archive, &out, png),
    }
}

/// A WAD, either on the filesystem or inside a disc image.
///
/// Both are read lazily. The directory is small; the blobs are not, and on a
/// CHD every read costs decompression.
enum Source {
    File {
        path: PathBuf,
        file: std::fs::File,
        len: u64,
    },
    Disc {
        disc: Box<DiscImage>,
        entry: oag_disc::Entry,
        label: String,
    },
}

impl Source {
    /// Opens `spec`, which is either a path or `<image>:<path-on-disc>`.
    ///
    /// Splitting on the last colon rather than the first keeps Windows drive
    /// letters working.
    fn open(spec: &str) -> Result<Self> {
        if let Some((image, inner)) = split_disc_spec(spec) {
            let mut disc =
                DiscImage::open(image).with_context(|| format!("opening disc image {image}"))?;

            let entry = disc
                .entries()?
                .iter()
                .find(|e| !e.is_directory && e.path.eq_ignore_ascii_case(inner))
                .cloned()
                .with_context(|| format!("{inner} is not on {image}"))?;

            return Ok(Self::Disc {
                label: format!("{image}:{}", entry.path),
                entry,
                disc: Box::new(disc),
            });
        }

        let path = PathBuf::from(spec);
        let file =
            std::fs::File::open(&path).with_context(|| format!("opening {}", path.display()))?;
        let len = file.metadata()?.len();
        Ok(Self::File { path, file, len })
    }

    fn label(&self) -> String {
        match self {
            Self::File { path, .. } => path.display().to_string(),
            Self::Disc { label, .. } => label.clone(),
        }
    }

    fn len(&self) -> u64 {
        match self {
            Self::File { len, .. } => *len,
            Self::Disc { entry, .. } => entry.size,
        }
    }

    fn read(&mut self, offset: u64, len: u64) -> Result<Vec<u8>> {
        let len = len.min(self.len().saturating_sub(offset));
        match self {
            Self::File { file, .. } => {
                use std::io::{Read, Seek, SeekFrom};
                file.seek(SeekFrom::Start(offset))?;
                let mut buf = vec![0u8; len as usize];
                file.read_exact(&mut buf)?;
                Ok(buf)
            }
            Self::Disc { disc, entry, .. } => Ok(disc.read_entry_range(entry, offset, len)?),
        }
    }

    /// Reads the header, then exactly the directory. Two reads rather than
    /// slurping the archive.
    fn directory(&mut self) -> Result<Directory> {
        let header = self.read(0, wad::HEADER_LEN as u64)?;
        let count = Directory::peek_entry_count(&header)
            .map_err(|e| anyhow::anyhow!("{}: {e}", self.label()))?;

        let bytes = self.read(0, Directory::directory_len(count))?;
        Directory::parse(&bytes, Some(self.len()))
            .map_err(|e| anyhow::anyhow!("{}: {e}", self.label()))
    }
}

/// Splits `<image>:<path>`, or returns `None` for a plain path.
fn split_disc_spec(spec: &str) -> Option<(&str, &str)> {
    let (image, inner) = spec.rsplit_once(':')?;
    // A bare Windows drive letter is not a disc spec.
    if image.len() < 2 || inner.is_empty() {
        return None;
    }
    Some((image, inner))
}

fn summarise(dir: &Directory, source: &Source) {
    println!("archive        {}", source.label());
    println!("version        {}", dir.version);
    println!("entries        {}", dir.entries.len());
    println!(
        "payload        {} of {}",
        humanise::bytes(dir.payload_len()),
        humanise::bytes(source.len())
    );

    let compressed = dir.compressed_entries();
    if compressed.is_empty() {
        println!("compression    none, every blob stored as-is");
    } else {
        let stored: u64 = compressed.iter().map(|(_, e)| u64::from(e.size)).sum();
        let expanded: u64 = compressed
            .iter()
            .map(|(_, e)| u64::from(e.size_uncompressed))
            .sum();
        println!(
            "compression    {} of {} entries, {} -> {} ({:.2}x)",
            compressed.len(),
            dir.entries.len(),
            humanise::bytes(stored),
            humanise::bytes(expanded),
            expanded as f64 / stored.max(1) as f64
        );
        println!("unpacked total {}", humanise::bytes(dir.uncompressed_len()));
    }

    // The offset chain is what pins down the field ordering, so a break is
    // worth surfacing rather than hiding.
    let breaks = dir.offset_chain_breaks();
    if breaks.is_empty() {
        println!("offset chain   consistent (64-byte aligned, no gaps)");
    } else {
        let shown: Vec<String> = breaks.iter().take(8).map(usize::to_string).collect();
        println!(
            "offset chain   {} break(s) at entries {}{}",
            breaks.len(),
            shown.join(", "),
            if breaks.len() > 8 { ", ..." } else { "" }
        );
    }
}

/// Hashes every line of a candidate-name file into a lookup table.
///
/// Names are not stored in the archive, so the only way to get a listing with
/// names is to hash names we already have and match. Candidates typically come
/// from strings in the game binary.
fn load_names(path: &Path) -> Result<BTreeMap<u32, String>> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;

    let mut map = BTreeMap::new();
    for line in text.lines() {
        let name = line.trim();
        if name.is_empty() || name.starts_with('#') {
            continue;
        }
        // First name wins, so an earlier, more canonical spelling is kept when
        // two candidates normalise to the same hash.
        map.entry(wad::hash_name(name))
            .or_insert_with(|| name.to_string());
    }
    Ok(map)
}

fn list(spec: &str, by_size: bool, limit: usize, names: Option<&Path>) -> Result<()> {
    let mut source = Source::open(spec)?;
    let dir = source.directory()?;

    let names = names.map(load_names).transpose()?.unwrap_or_default();

    summarise(&dir, &source);
    if !names.is_empty() {
        let known = dir
            .entries
            .iter()
            .filter(|e| names.contains_key(&e.name_hash))
            .count();
        println!(
            "names          {known} of {} resolved from {} candidate(s)",
            dir.entries.len(),
            names.len()
        );
    }
    println!();

    let mut rows: Vec<(usize, &wad::Entry)> = dir.entries.iter().enumerate().collect();
    if by_size {
        rows.sort_by_key(|(i, e)| (std::cmp::Reverse(e.size), *i));
    }
    if limit > 0 {
        rows.truncate(limit);
    }

    println!(
        "{:>6}  {:>10}  {:>12}  {:>12}  {:<5}  {:<8}  NAME",
        "#", "OFFSET", "SIZE", "UNCOMPRESSED", "COMP", "HASH"
    );
    for (i, e) in &rows {
        println!(
            "{i:>6}  {:>10}  {:>12}  {:>12}  {:<5}  {:08x}  {}",
            e.offset,
            e.size,
            e.size_uncompressed,
            e.compression.to_string(),
            e.name_hash,
            names.get(&e.name_hash).map_or("", String::as_str)
        );
    }

    if limit > 0 && dir.entries.len() > limit {
        println!(
            "\n({} more, use --limit 0 for all)",
            dir.entries.len() - limit
        );
    }

    Ok(())
}

fn tags(spec: &str) -> Result<()> {
    let mut source = Source::open(spec)?;
    let dir = source.directory()?;

    summarise(&dir, &source);
    println!();

    let mut counts: BTreeMap<String, (usize, u64)> = BTreeMap::new();
    let mut empty = 0usize;

    for entry in &dir.entries {
        if entry.size == 0 {
            empty += 1;
            continue;
        }
        let head = source.read(u64::from(entry.offset), TAG_PEEK_BYTES)?;
        // A texture is identified by its size arithmetic, not by a magic, so
        // it needs the whole blob rather than a peek.
        let label = if entry.compression == Compression::None
            && Texture::looks_like_texture(
                &source.read(u64::from(entry.offset), u64::from(entry.size))?,
            ) {
            "texture (.mip)".to_string()
        } else {
            match Blob::peek(&head) {
                Some(blob) => blob.label(),
                None => "(too short)".to_string(),
            }
        };
        let slot = counts.entry(label).or_insert((0, 0));
        slot.0 += 1;
        slot.1 += u64::from(entry.size);
    }

    let mut rows: Vec<_> = counts.into_iter().collect();
    rows.sort_by_key(|(_, (_, bytes))| std::cmp::Reverse(*bytes));

    println!("{:<14}  {:>7}  {:>12}", "TAG", "COUNT", "BYTES");
    for (label, (count, bytes)) in &rows {
        println!("{label:<14}  {count:>7}  {:>12}", humanise::bytes(*bytes));
    }
    if empty > 0 {
        println!("{:<14}  {empty:>7}  {:>12}", "(zero-size)", "0 B");
    }

    println!("\n{} distinct tag(s)", rows.len());
    Ok(())
}

/// Decompresses a blob if it is stored compressed.
fn decode_blob(raw: &[u8], entry: &wad::Entry) -> Result<Vec<u8>> {
    match entry.compression {
        Compression::None => Ok(raw.to_vec()),
        Compression::Lzss => Ok(lzss::decompress(raw, entry.size_uncompressed as usize)?),
        // Present in the game but absent from every shipped archive, so there
        // is nothing to test an implementation against. Refuse rather than
        // ship an unverified decoder.
        Compression::Zlib => {
            anyhow::bail!("zlib-compressed entries are not supported yet (none were expected)")
        }
    }
}

/// Resolves an entry selector: an index, a `0x` hash, or a name to hash.
fn find_entry(dir: &Directory, selector: &str) -> Result<usize> {
    if let Some(hex) = selector.strip_prefix("0x") {
        let hash = u32::from_str_radix(hex, 16).context("parsing the hash")?;
        return dir
            .entries
            .iter()
            .position(|e| e.name_hash == hash)
            .with_context(|| format!("no entry with hash {hex}"));
    }

    // A bare integer is an index. Anything else is a name, which is the most
    // useful form now that the hash is known.
    if let Ok(index) = selector.parse::<usize>()
        && index < dir.entries.len()
    {
        return Ok(index);
    }

    let hash = wad::hash_name(selector);
    dir.entries
        .iter()
        .position(|e| e.name_hash == hash)
        .with_context(|| format!("no entry named {selector} (hash {hash:08x})"))
}

fn cat(spec: &str, selector: &str, want_expand: bool) -> Result<()> {
    use std::io::Write;

    let mut source = Source::open(spec)?;
    let dir = source.directory()?;
    let index = find_entry(&dir, selector)?;
    let entry = dir.entries[index];

    let raw = source.read(u64::from(entry.offset), u64::from(entry.size))?;
    let data = decode_blob(&raw, &entry)?;

    if want_expand && fexml::is_fexml(&data) {
        print!(
            "{}",
            fexml::expand(&data).context("expanding front-end XML")?
        );
        return Ok(());
    }

    std::io::stdout().write_all(&data)?;
    Ok(())
}

fn verify(spec: &str) -> Result<()> {
    let mut source = Source::open(spec)?;
    let dir = source.directory()?;

    summarise(&dir, &source);
    println!();

    let mut by_compression: BTreeMap<String, (usize, u64)> = BTreeMap::new();
    let mut failures = Vec::new();

    for (i, entry) in dir.entries.iter().enumerate() {
        let raw = source
            .read(u64::from(entry.offset), u64::from(entry.size))
            .with_context(|| format!("reading entry {i}"))?;

        match decode_blob(&raw, entry) {
            Ok(data) if data.len() as u32 == entry.size_uncompressed => {
                let slot = by_compression
                    .entry(entry.compression.to_string())
                    .or_insert((0, 0));
                slot.0 += 1;
                slot.1 += data.len() as u64;
            }
            // A length mismatch means the decoder is wrong even though it did
            // not error, which is the failure mode worth catching.
            Ok(data) => failures.push(format!(
                "entry {i} ({:08x}): produced {} bytes, declared {}",
                entry.name_hash,
                data.len(),
                entry.size_uncompressed
            )),
            Err(e) => failures.push(format!("entry {i} ({:08x}): {e:#}", entry.name_hash)),
        }
    }

    println!("{:<8}  {:>7}  {:>12}", "STORED", "COUNT", "BYTES OUT");
    for (kind, (count, bytes)) in &by_compression {
        println!("{kind:<8}  {count:>7}  {:>12}", humanise::bytes(*bytes));
    }

    println!();
    if failures.is_empty() {
        println!(
            "OK: all {} entries decoded to their declared size",
            dir.entries.len()
        );
        Ok(())
    } else {
        for f in failures.iter().take(20) {
            println!("  {f}");
        }
        if failures.len() > 20 {
            println!("  ... and {} more", failures.len() - 20);
        }
        bail!("{} of {} entries failed", failures.len(), dir.entries.len())
    }
}

fn extract(spec: &str, out: &Path, want_png: bool) -> Result<()> {
    let mut source = Source::open(spec)?;
    let dir = source.directory()?;

    if dir.entries.is_empty() {
        bail!("archive has no entries");
    }

    std::fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;

    let mut written = 0u64;
    let mut textures = 0usize;
    let mut swizzled_looking = 0usize;

    for (i, entry) in dir.entries.iter().enumerate() {
        let raw = source
            .read(u64::from(entry.offset), u64::from(entry.size))
            .with_context(|| format!("reading entry {i}"))?;

        let data = decode_blob(&raw, entry).with_context(|| format!("decoding entry {i}"))?;

        // Named by index and hash, because the real names are not in the
        // archive. The index keeps directory order visible; the hash is what
        // the game actually looks entries up by, so it is the stable
        // identifier.
        let texture = Texture::parse(&data).ok();
        let tag = match (&texture, Blob::peek(&data).and_then(|b| b.tag)) {
            // A texture has no magic, so it is only recognisable by its size
            // arithmetic. Checking it first stops the first two bytes of a
            // width being mistaken for a type tag.
            (Some(_), _) => "mip".to_string(),
            (None, Some(tag)) => tag.to_ascii_lowercase(),
            (None, None) => "bin".to_string(),
        };
        let name = format!("{i:05}_{:08x}.{tag}", entry.name_hash);

        std::fs::write(out.join(&name), &data).with_context(|| format!("writing {name}"))?;
        written += data.len() as u64;

        if let (true, Some(tex)) = (want_png, texture) {
            textures += 1;
            if tex.looks_swizzled() {
                swizzled_looking += 1;
            }
            let image =
                png::encode_rgba(u32::from(tex.width), u32::from(tex.height), &tex.to_rgba());
            let png_name = format!("{i:05}_{:08x}.png", entry.name_hash);
            std::fs::write(out.join(&png_name), &image)
                .with_context(|| format!("writing {png_name}"))?;
        }
    }

    println!(
        "extracted {} entries ({}) to {}",
        dir.entries.len(),
        humanise::bytes(written),
        out.display()
    );
    if want_png {
        println!("{textures} decoded as textures and were written as PNG");
        if swizzled_looking > 0 {
            // A hint that pixel data needs unswizzling, not a determination.
            println!("{swizzled_looking} of those look swizzled (see docs/formats/psp-texture.md)");
        }
    }
    Ok(())
}
