use std::path::Path;
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut ar = oag_assets::psarc::Archive::open_file(Path::new(&args[0])).unwrap();
    for n in ["male01","male01_shadow","male02","male05","male06","female01","female06","female10_shadow"] {
        let name = format!("Data/crowd/Rigs/{n}.rcsmodel");
        let blob = ar.read_path(&name).unwrap();
        let m = oag_rcs::rcsmodel::psp2::parse(&blob).unwrap();
        for mt in &m.materials { println!("{n}: {} tex={:?} params={}", mt.name, mt.textures, mt.params.len()); }
    }
    let mut v: Vec<String> = ar.paths().iter().filter(|p| p.to_lowercase().contains("crowd")).cloned().collect();
    v.sort();
    for p in v.iter().filter(|p| !p.contains("Rigs/")) { println!("{p}"); }
}
