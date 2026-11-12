//! Checks, lists and packs mods.
//!
//! ```text
//! oag-mod check <mod>                 open a folder, .zip or .oagmod and check its manifest
//! oag-mod list  [<dir>...]            the mods a search finds (the default search path when none)
//! oag-mod pack  <folder> [-o <file>]  write the folder as a .oagmod: zstd for text, stored for PNG and OGG
//! oag-mod init-pack <folder>... [--repository <url>]
//!                                     write an oag-mod.toml into each PPSSPP texture pack folder that has none
//! ```
//!
//! `just pack-mod <folder>` runs `pack`; `just fetch-texture-packs` runs `init-pack`. The format is `docs/tools/mods.md`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Checks, lists and packs mods")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Open a mod and check its manifest.
    Check {
        /// A folder, `.zip` or `.oagmod`.
        path: PathBuf,
    },
    /// List the mods a search finds.
    List {
        /// Directories to search; the default search path when none is given.
        dirs: Vec<PathBuf>,
    },
    /// Write an `oag-mod.toml` into each PPSSPP texture pack folder that has none, from the
    /// manifest the game would otherwise recognise. An existing manifest is left as it is.
    InitPack {
        /// Folders holding a `textures.ini`.
        #[arg(required = true)]
        folders: Vec<PathBuf>,
        /// Where the pack was fetched from: written as `repository`, and its owner as the author.
        #[arg(long)]
        repository: Option<String>,
    },
    /// Write a mod folder as a `.oagmod` (zip with zstd entries).
    Pack {
        /// The mod folder.
        folder: PathBuf,
        /// The output file; `<folder name>-<version>.oagmod` beside the folder when omitted.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Check { path } => {
            let module = oag_mod::Mod::open(&path).with_context(|| path.display().to_string())?;
            println!("{}", module.describe());
            println!("  files: {}", module.names().join(", "));
        }
        Command::List { dirs } => {
            let bases = if dirs.is_empty() {
                oag_mod::mods_search_path()
            } else {
                dirs
            };
            let game = semver::Version::parse(env!("CARGO_PKG_VERSION"))?;
            let loaded = oag_mod::load(&bases, &game);
            for module in &loaded.mods {
                println!("{}", module.describe());
            }
            for problem in &loaded.failed {
                println!("left out: {problem}");
            }
            println!(
                "{} mod(s) in {}",
                loaded.mods.len(),
                bases
                    .iter()
                    .map(|b| b.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        Command::InitPack {
            folders,
            repository,
        } => {
            for folder in &folders {
                let written = oag_mod::texture_pack::write_manifest(
                    folder,
                    repository.as_deref(),
                    &|serial| oag_source::title::by_serial(serial).map(|title| title.slug()),
                )
                .with_context(|| folder.display().to_string())?;
                match written {
                    oag_mod::texture_pack::Written::Wrote(path) => {
                        println!("wrote {}", path.display());
                    }
                    oag_mod::texture_pack::Written::Kept(path) => {
                        println!("kept {} (already there)", path.display());
                    }
                }
            }
        }
        Command::Pack { folder, out } => {
            let module =
                oag_mod::Mod::open(&folder).with_context(|| folder.display().to_string())?;
            let out = out.unwrap_or_else(|| {
                folder
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .join(format!(
                        "{}-{}.oagmod",
                        module.manifest.id, module.manifest.version
                    ))
            });
            let packed = oag_mod::pack::pack(&folder, &out)?;
            println!(
                "{}: {} zstd, {} stored, {} -> {} bytes",
                out.display(),
                packed.zstd,
                packed.stored,
                packed.bytes_in,
                packed.bytes_out
            );
        }
    }
    Ok(())
}
