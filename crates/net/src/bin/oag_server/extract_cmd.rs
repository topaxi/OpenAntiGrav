//! `oag-server extract`: keep only what a server's races read of a disc or folder.
//!
//! ```sh
//! oag-server extract --image data/images/pulse-psp-eu.chd --image data/extracted/vita \
//!     --out data/server
//! # writes data/server/pulse-psp-eu/ and data/server/vita/, each opened by `--image` of
//! # `oag-server lobby` and `oag-server race` like the source it came from
//! ```
//!
//! Output is the source's own content, sliced: never commit it, never ship it
//! (`docs/overview/legal.md`). It lives under `data/`, which is ignored.

use std::path::PathBuf;

use anyhow::{Result, anyhow};

#[derive(clap::Args, Debug)]
pub struct Args {
    /// A disc image or unpacked folder to extract from, repeatable; its title is read from it.
    #[arg(long = "image", required = true)]
    images: Vec<String>,
    /// The folder the extractions are written under, one subfolder per image.
    #[arg(long, default_value = "data/server")]
    out: PathBuf,
    /// Print one line per race loaded instead of one per image.
    #[arg(long)]
    verbose: bool,
    #[command(flatten)]
    log: oag_log::tool::LogArgs,
}

pub fn run(args: &Args) -> Result<()> {
    oag_log::tool::start("oag-server-extract", "warn,oag=info", &args.log);
    let mut failed = false;
    for source in &args.images {
        let stem = std::path::Path::new(source)
            .file_stem()
            .or_else(|| std::path::Path::new(source).file_name())
            .map_or_else(
                || "source".to_string(),
                |s| s.to_string_lossy().into_owned(),
            );
        let out = args.out.join(&stem);
        println!("extracting {source} to {}", out.display());
        let report = oag_net::extract::extract(
            source,
            &out,
            oag_net::BUILD_VERSION,
            &oag_net::extract::plan,
            |line| {
                if args.verbose || !line.starts_with("  ") {
                    println!("{line}");
                }
            },
        )?;
        let kept: u64 = report.manifest.files.iter().map(|f| f.kept).sum();
        let whole: u64 = report.manifest.files.iter().map(|f| f.len).sum();
        let size: u64 = report
            .manifest
            .files
            .iter()
            .map(|f| std::fs::metadata(out.join(&f.path)).map_or(0, |m| m.len()))
            .sum();
        println!(
            "{}: {} races loaded, {} refused by the source itself, {} files, {} MiB on disk \
             ({} MiB of {} MiB original data kept)",
            report.manifest.title,
            report.loaded,
            report.refused.len(),
            report.manifest.files.len(),
            size >> 20,
            kept >> 20,
            whole >> 20
        );
        for refused in &report.refused {
            println!(
                "  refused by the source: {}: {}",
                refused.label,
                refused.result.as_ref().unwrap_err()
            );
        }
        for gap in &report.gaps {
            println!("  GAP {gap}");
            failed = true;
        }
    }
    if failed {
        return Err(anyhow!(
            "the extraction does not reproduce every race the source loads"
        ));
    }
    Ok(())
}
