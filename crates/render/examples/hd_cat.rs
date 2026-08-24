//! Scratch probe: write one archive entry to a file, so the microcode
//! disassembler (`scripts/ps3-microcode.py`) can be pointed at it.

use oag_assets::Container;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("usage: hd_cat <spec> <entry> <out>"))?;
    let entry = args.next().ok_or_else(|| anyhow::anyhow!("no entry"))?;
    let out = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("no output path"))?;
    let blob = Container::open(&spec)?.read_entry(&entry)?;
    std::fs::write(&out, &blob)?;
    println!("wrote {out} ({} bytes)", blob.len());
    Ok(())
}
