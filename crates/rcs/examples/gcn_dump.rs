//! Prints every GCN program in a file, or one, with its reflection.
//!
//! ```sh
//! cargo run -p oag-rcs --example gcn_dump -- <file>                  # a loose file (eboot.bin)
//! cargo run -p oag-rcs --example gcn_dump -- <archive.psarc> <entry>   # one archive entry
//! ... -- <file> [<entry>] --at 0x31e4                                 # only the Shdr at that byte
//! ... -- <file> [<entry>] --summary                                   # one line a program
//! ```
//!
//! See `docs/formats/gcn.md`.

use std::path::Path;

use oag_rcs::gcn::{self, Program};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().position(|a| a == name);
    let at = flag("--at").map(|k| {
        let v = args[k + 1].trim_start_matches("0x");
        usize::from_str_radix(v, 16).expect("--at takes a hex byte offset")
    });
    let summary = flag("--summary").is_some();
    let positional: Vec<&String> = args
        .iter()
        .enumerate()
        .filter(|(k, a)| !a.starts_with("--") && !(k > &0 && args[k - 1] == "--at"))
        .map(|(_, a)| a)
        .collect();
    let Some(path) = positional.first() else {
        eprintln!("usage: gcn_dump <file | archive.psarc entry> [--at 0xOFFSET] [--summary]");
        std::process::exit(2);
    };
    let bytes = match positional.get(1) {
        Some(entry) => oag_assets::psarc::Archive::open_file(Path::new(path.as_str()))
            .and_then(|mut a| a.read_path(entry))
            .unwrap_or_else(|e| panic!("{path}: {entry}: {e}")),
        None => std::fs::read(path.as_str()).unwrap_or_else(|e| panic!("{path}: {e}")),
    };
    for (tag, parsed) in gcn::scan(&bytes) {
        if at.is_some_and(|a| a != tag) {
            continue;
        }
        match parsed {
            Ok(p) if summary => println!("{}", line(&p)),
            Ok(p) => print_program(&p),
            Err(e) => println!("Shdr @{tag:#x}: refused: {e}"),
        }
    }
}

fn line(p: &Program<'_>) -> String {
    let names = |v: Vec<&str>| v.join(",");
    let refl = match &p.reflection {
        Ok(r) => format!(
            "cb [{}] samplers [{}] in [{}]",
            names(r.constants.iter().map(|c| c.name.as_str()).collect()),
            names(r.samplers.iter().map(|s| s.name.as_str()).collect()),
            names(r.inputs.iter().map(|a| a.name.as_str()).collect()),
        ),
        Err(e) => format!("reflection refused: {e}"),
    };
    format!(
        "Shdr @{:#x} {:?} {} instr, {refl}",
        p.at,
        p.stage,
        p.instructions.len()
    )
}

fn print_program(p: &Program<'_>) {
    println!("{}", line(p));
    if let Ok(r) = &p.reflection {
        for res in &r.resources {
            println!(
                "  resource slot {} {:#x} size {} {}",
                res.slot, res.kind, res.size, res.name
            );
            for c in &r.constants[res.members.clone()] {
                println!(
                    "    dw{:<3} {:<28} size {:<3} kind {:#x}",
                    c.offset / 4,
                    c.name,
                    c.size,
                    c.kind
                );
            }
        }
        for s in &r.samplers {
            println!("  sampler slot {} {}", s.slot, s.name);
        }
        for a in &r.inputs {
            println!("  input  {:#010x} {} {}", a.kind, a.name, a.semantic);
        }
        for a in &r.outputs {
            println!("  output {:#010x} {} {}", a.kind, a.name, a.semantic);
        }
    }
    match &p.vertex_inputs {
        Ok(inputs) => {
            for v in inputs {
                let name = p.input_in(v.vgpr).map_or("?", |(_, a)| a.name.as_str());
                println!(
                    "  fetch slot {} -> v{}..v{} {}",
                    v.semantic,
                    v.vgpr,
                    v.vgpr + v.size - 1,
                    name
                );
            }
        }
        Err(e) => println!("  vertex inputs refused: {e}"),
    }
    print!("{p}");
}
