//! Scratch: the cameras of a scene and the camera's payload words.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage")?;
    let data = std::fs::read(&path)?;
    for c in oag_vex::camera::cameras(&data) {
        println!("{c:#?}");
    }
    let nodes = oag_vex::vex::nodes(&data).map_err(|e| format!("{e}"))?;
    for n in &nodes {
        if n.class_id == 0xf7 {
            let p = &data[n.payload()];
            for w in p.chunks(4) {
                print!("{:08x}({}) ", u32::from_le_bytes(w.try_into().unwrap()), f32::from_le_bytes(w.try_into().unwrap()));
            }
            println!();
        }
    }
    Ok(())
}
