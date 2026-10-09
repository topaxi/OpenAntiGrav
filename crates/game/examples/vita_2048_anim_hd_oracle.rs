//! Scratch: dump Wipeout HD's `Anim Transform` nodes and the `Mesh` nodes
//! beneath them for one circuit, as an oracle for 2048's re-export of the
//! same circuit (`.rcsskeleton` + `.rcsanimclip`).
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_2048_anim_hd_oracle -- \
//!     <track.vex> > <anim.tsv>
//! ```

use oag_vex::vex;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("a track.vex path");
    let data = std::fs::read(&path)?;
    let classes = vex::classes_of(&data)?;
    let nodes = vex::nodes(&data)?;
    let order = vex::byte_order(&data);
    let anims = vex::anim_transforms(&data, &nodes);
    let anchors = vex::anim_anchors(&data, &nodes);
    let world0 = vex::anchor_world(&data, &nodes, 0.0);
    let times = [
        0.0f32, 0.1, 0.3, 0.6, 1.0, 1.4, 2.5, 3.0, 3.9, 3.95, 4.1, 7.3, 24.9, 24.95,
    ];
    let worlds: Vec<Vec<Option<[f32; 16]>>> = times
        .iter()
        .map(|&t| vex::anchor_world(&data, &nodes, t))
        .collect();
    let world = vex::world_transforms(&data, &nodes);
    let mesh_class = classes.mesh.unwrap();
    let anim_class = classes.anim_transform.unwrap();
    let fmt16 = |m: &[f32; 16]| {
        m.iter()
            .map(|v| format!("{v}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    for (i, node) in nodes.iter().enumerate() {
        if node.class_id == anim_class {
            let Some(anim) = &anims[i] else {
                continue;
            };
            let w0 = world0[i].unwrap_or(world[i]);
            let name = node.name.clone().unwrap_or_default();
            // Full path name: ancestors' names joined with ':' (Maya namespaces are baked into names already).
            let parent_name = node
                .parent
                .and_then(|p| nodes[p].name.clone())
                .unwrap_or_default();
            println!(
                "ANIM\t{i}\t{name}\t{parent_name}\tparent={:?}\tflags={:#x}\tspk={}\tloop={}\tstep={}\ttq={:?}\ttb={:?}\tT={}\tR={}\tS={}\tworld0={}\tlocal0={}",
                node.parent,
                anim.flags,
                anim.seconds_per_key,
                anim.loop_seconds,
                anim.step,
                anim.translation_quantum,
                anim.translation_base,
                anim.translation.times.len(),
                anim.rotation.times.len(),
                anim.scale.times.len(),
                fmt16(&w0),
                fmt16(&anim.sample(0.0)),
            );
            let show = |label: &str, ch: &vex::AnimChannel| {
                let n = ch.times.len();
                let keys: Vec<String> = (0..n)
                    .map(|k| {
                        let v = ch.values[k];
                        let w = ch.w.get(k).copied();
                        format!(
                            "{}:({:.4},{:.4},{:.4}{})",
                            ch.times[k],
                            v[0],
                            v[1],
                            v[2],
                            w.map(|w| format!(",{w:.4}")).unwrap_or_default()
                        )
                    })
                    .collect();
                println!(
                    "  {label}\tlast_t={:?}\t{}",
                    ch.times.last(),
                    keys.join(" ")
                );
            };
            for (k, t) in times.iter().enumerate() {
                if let Some(w) = worlds[k][i] {
                    println!("  W\t{t}\t{}", fmt16(&w));
                }
            }
            show("T", &anim.translation);
            show("R", &anim.rotation);
            show("S", &anim.scale);
        } else if node.class_id == mesh_class && anchors[i].anchor.is_some() {
            let payload = &data[node.payload()];
            let hash = if payload.len() >= 0x34 {
                order.u32(payload, 0x30)
            } else {
                0
            };
            let read3 =
                |at: usize| -> [f32; 3] { std::array::from_fn(|k| order.f32(payload, at + k * 4)) };
            println!(
                "MESH\t{i}\t{}\tanchor={:?}\tparent={:?}\thash={hash:#010x}\tmin={:?}\tmax={:?}\tlocal={}\tworld={}",
                node.name.clone().unwrap_or_default(),
                anchors[i].anchor,
                node.parent,
                read3(0x10),
                read3(0x20),
                fmt16(&anchors[i].local),
                fmt16(&world[i]),
            );
        }
    }
    Ok(())
}
