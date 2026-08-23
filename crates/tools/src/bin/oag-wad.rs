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

use oag_assets::Archive;
use oag_formats::texture::Texture;
use oag_formats::wad::{self, Blob, Compression, Directory};
use oag_formats::{fexml, lzss, png, ps2_texture, sblk};
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

    /// List the sound banks in an archive, and every cue each one names.
    ///
    /// A cue's waveform count is what `Bank::cue_sounds` resolves through the
    /// command table; `loop` is the descriptor's own `+0x40` flag. Seconds are
    /// at the rate `oag_game::audio::sfx` assumes, which is **not recovered** -
    /// see `docs/formats/psp-audio.md`.
    Sounds {
        /// A `.wad` path, or `<image>:<path-on-disc>`.
        archive: String,
        /// Only banks whose own name contains this, case-insensitively.
        #[arg(long)]
        bank: Option<String>,
        /// Only cues whose name contains this, case-insensitively.
        #[arg(long)]
        cue: Option<String>,
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
        Command::Sounds { archive, bank, cue } => sounds(&archive, bank.as_deref(), cue.as_deref()),
        Command::Extract { archive, out, png } => extract(&archive, &out, png),
    }
}

/// Lists every `SBlk` bank in an archive with the cues it names.
///
/// Reads every blob, because a bank may be compressed and the magic is inside
/// the payload. That is a whole-archive decompress on `WADS2.WAD`; it is the
/// same cost `verify` pays and the same cost the game pays at load.
fn sounds(spec: &str, bank_filter: Option<&str>, cue_filter: Option<&str>) -> Result<()> {
    let mut archive = Archive::open(spec)?;
    let entries = archive.directory().entries.clone();

    summarise(&archive);
    println!();

    let matches = |haystack: &str, needle: Option<&str>| {
        needle.is_none_or(|n| haystack.to_lowercase().contains(&n.to_lowercase()))
    };

    let mut banks = 0;
    let mut cues = 0;
    for (index, entry) in entries.iter().enumerate() {
        if entry.size == 0 {
            continue;
        }
        let blob = archive.read(index)?;
        if !sblk::looks_like_bank(&blob) {
            continue;
        }
        let parsed = sblk::Bank::parse(&blob)?;
        if !matches(&parsed.name, bank_filter) {
            continue;
        }
        banks += 1;
        println!(
            "#{index} {:08x}  {:<8}  {} cues, {} commands, {} waveforms, {}",
            entry.name_hash,
            if parsed.name.is_empty() {
                "(unnamed)"
            } else {
                &parsed.name
            },
            parsed.cue_count,
            parsed.command_count,
            parsed.waveform_count,
            humanise::bytes(parsed.waveforms.len() as u64),
        );
        let mut names = parsed.sound_names();
        names.sort_by(|a, b| a.name.cmp(&b.name));
        for name in names {
            if !matches(&name.name, cue_filter) {
                continue;
            }
            cues += 1;
            let Some(cue) = parsed.cue(name.cue) else {
                continue;
            };
            let waveforms = parsed.cue_sounds(&cue);
            let looping = waveforms.iter().filter(|s| s.mode & 0x40 != 0).count();
            let frames: usize = waveforms
                .iter()
                .map(|s| (s.length as usize / sblk::ADPCM_BLOCK_LEN) * sblk::ADPCM_BLOCK_SAMPLES)
                .sum();
            println!(
                "    {:<18} cue {:>3}  cmds {:>3}..{:<3} {:>2} waveform(s), {} looping, {:.2}s total",
                name.name,
                name.cue,
                cue.first_command,
                cue.first_command + cue.commands,
                waveforms.len(),
                looping,
                frames as f64 / f64::from(sblk::ASSUMED_SAMPLE_RATE),
            );
        }
    }
    println!();
    println!("{banks} bank(s), {cues} cue(s)");
    Ok(())
}

fn summarise(archive: &Archive) {
    let dir = archive.directory();
    println!("archive        {}", archive.label());
    println!("version        {}", dir.version);
    println!("entries        {}", dir.entries.len());
    println!(
        "payload        {} of {}",
        humanise::bytes(dir.payload_len()),
        humanise::bytes(archive.len())
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
    let archive = Archive::open(spec)?;
    let dir = archive.directory();

    let names = names.map(load_names).transpose()?.unwrap_or_default();

    summarise(&archive);
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
    let mut archive = Archive::open(spec)?;
    let entries = archive.directory().entries.clone();

    summarise(&archive);
    println!();

    let mut counts: BTreeMap<String, (usize, u64)> = BTreeMap::new();
    let mut empty = 0usize;

    for (index, entry) in entries.iter().enumerate() {
        if entry.size == 0 {
            empty += 1;
            continue;
        }
        // Raw, not decoded: the tag lives at the start of the stored bytes.
        // A texture is identified by its size arithmetic against the stored
        // (uncompressed, for every archive seen) length, which needs the
        // whole blob; a compressed entry never gets that read, so `tags`
        // stays a directory-speed scan even on `WADS2.WAD`'s 360 MB.
        let head = archive.peek_raw(index, TAG_PEEK_BYTES)?;
        let label = if entry.compression == Compression::None
            && Texture::looks_like_texture(&archive.read_raw(index)?)
        {
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

    let mut archive = Archive::open(spec)?;
    let index = find_entry(archive.directory(), selector)?;
    let data = archive
        .read(index)
        .with_context(|| format!("reading entry {index}"))?;

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
    let mut archive = Archive::open(spec)?;
    let entries = archive.directory().entries.clone();

    summarise(&archive);
    println!();

    let mut by_compression: BTreeMap<String, (usize, u64)> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut leftovers: BTreeMap<usize, usize> = BTreeMap::new();

    for (i, entry) in entries.iter().enumerate() {
        let raw = archive
            .read_raw(i)
            .with_context(|| format!("reading entry {i}"))?;

        // For a compressed entry, check where the *reader* stopped as well as how
        // much came out. The decoder stops once it has produced the declared
        // length, so a size match is a tautology; unread input at the end is not.
        // A wrong bit or field order usually still terminates, just not on the
        // last byte of the stream.
        // Where the *reader* stopped, as well as how much came out. The decoder
        // stops once it has produced the declared length, so a size match is a
        // tautology; how much input it needed to get there is not. Every shipped
        // stream ends with one or two bytes left, which is the encoder's final
        // bit-buffer flush: see `lzss::MAX_TRAILING_BYTES`.
        if entry.compression == Compression::Lzss
            && let Ok(decoded) = lzss::decompress_reporting(&raw, entry.size_uncompressed as usize)
        {
            *leftovers.entry(decoded.leftover).or_insert(0) += 1;
            if decoded.leftover > lzss::MAX_TRAILING_BYTES {
                failures.push(format!(
                    "entry {i} ({:08x}): stopped {} bytes before the end of a {}-byte stream",
                    entry.name_hash,
                    decoded.leftover,
                    raw.len()
                ));
            }
        }

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

    if !leftovers.is_empty() {
        println!(
            "{:>8}  {:>7}   trailing bytes never read",
            "LEFTOVER", "COUNT"
        );
        for (leftover, count) in &leftovers {
            println!("{leftover:>8}  {count:>7}");
        }
        println!();
    }

    println!("{:<8}  {:>7}  {:>12}", "STORED", "COUNT", "BYTES OUT");
    for (kind, (count, bytes)) in &by_compression {
        println!("{kind:<8}  {count:>7}  {:>12}", humanise::bytes(*bytes));
    }

    println!();
    if failures.is_empty() {
        let compressed = entries.iter().filter(|e| e.is_compressed()).count();
        println!(
            "OK: all {} entries decoded to their declared size, and all {compressed} \n\
             compressed streams were read to within {} bytes of their end",
            entries.len(),
            lzss::MAX_TRAILING_BYTES
        );
        Ok(())
    } else {
        for f in failures.iter().take(20) {
            println!("  {f}");
        }
        if failures.len() > 20 {
            println!("  ... and {} more", failures.len() - 20);
        }
        bail!("{} of {} entries failed", failures.len(), entries.len())
    }
}

fn extract(spec: &str, out: &Path, want_png: bool) -> Result<()> {
    let mut archive = Archive::open(spec)?;
    let entries = archive.directory().entries.clone();

    if entries.is_empty() {
        bail!("archive has no entries");
    }

    std::fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;

    let mut written = 0u64;
    let mut textures = 0usize;
    let mut swizzled_looking = 0usize;

    for (i, entry) in entries.iter().enumerate() {
        let data = archive
            .read(i)
            .with_context(|| format!("reading entry {i}"))?;

        // Named by index and hash, because the real names are not in the
        // archive. The index keeps directory order visible; the hash is what
        // the game actually looks entries up by, so it is the stable
        // identifier.
        let texture = Texture::parse(&data).ok();
        // The PS2's textures are a different format entirely - a GS upload
        // packet, see docs/formats/ps2-texture.md - but they are recognised the
        // same way, by their own size arithmetic, so trying both cannot
        // misclassify.
        let ps2 = if texture.is_none() {
            ps2_texture::parse(&data).ok()
        } else {
            None
        };
        let tag = match (&texture, &ps2, Blob::peek(&data).and_then(|b| b.tag)) {
            // A texture has no magic, so it is only recognisable by its size
            // arithmetic. Checking it first stops the first two bytes of a
            // width being mistaken for a type tag.
            (Some(_), _, _) => "mip".to_string(),
            (None, Some(_), _) => "gstex".to_string(),
            (None, None, Some(tag)) => tag.to_ascii_lowercase(),
            (None, None, None) => "bin".to_string(),
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

        if let (true, Some(tex)) = (want_png, ps2) {
            textures += 1;
            let image =
                png::encode_rgba(u32::from(tex.width), u32::from(tex.height), &tex.to_rgba());
            let png_name = format!("{i:05}_{:08x}.png", entry.name_hash);
            std::fs::write(out.join(&png_name), &image)
                .with_context(|| format!("writing {png_name}"))?;
        }
    }

    println!(
        "extracted {} entries ({}) to {}",
        entries.len(),
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
