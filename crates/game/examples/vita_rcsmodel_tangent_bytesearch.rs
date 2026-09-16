//! Scratch probe: does `tangent` (declaration type `5`, 4 components,
//! `+0x10` on the common 28-byte stride) decode the same way `normal` did -
//! one signed byte per component, `byte/127.0`?
//!
//! # Why there is no HD twin oracle for this field
//!
//! `normal` and `Uv1` were both settled against Wipeout HD's own decoded
//! `.rcsmodel`/`Uv1` at index-exact vertices. `tangent` cannot use that
//! method: `handover/rendering/hd-needs-a-per-material-shader-path-and.md`
//! (not linked from here - see `docs/formats/2048-rcsmodel.md` instead)
//! records that HD's own renderer has no tangent frame either - grep
//! `oag_rcs::rcsmodel`/`oag_render::mesh::rcs` (the HD, not `psp2`, module)
//! turns up no decoded tangent to compare against, on either platform. So
//! this is an **internal-consistency** check, not a cross-title one: unit
//! length of the xyz part, orthogonality to the already-cracked `normal`,
//! and whether the byte this reading treats as unused is a `+-1` handedness
//! sign or genuine padding - each scored against deliberately-wrong controls,
//! since none of these three thresholds means anything without one.
//!
//! # What this sweeps
//!
//! Every byte assignment of the four raw bytes to `(x, y, z, w)`: which byte
//! is `w` (4 choices) x every ordering of the other three as `(x, y, z)` (6)
//! x two signedness conventions (`i8/127`, HD's unsigned-biased
//! `u8/127.5-1`) = 48 candidates, scored over every vertex in the base
//! package plus both DLC packs whose declared stride names a `tangent`
//! attribute.
//!
//! ```sh
//! cargo run -q --release -p oag-game --example vita_rcsmodel_tangent_bytesearch
//! ```

use std::collections::BTreeMap;

use oag_rcs::rcsmodel::psp2;

const PACKAGES: [&str; 3] = [
    "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

/// One vertex this probe can test: the raw 4 tangent bytes, and the already-
/// decoded normal at the same vertex (via [`psp2::unpack_normal`], fixed
/// offset, confidence 96).
struct Sample {
    raw: [u8; 4],
    normal: [f32; 3],
    /// Whether this vertex's `Uv1`, decoded through the same per-stride
    /// declaration this probe uses for `tangent`'s own offset, comes out
    /// finite. A proxy for "this chunk's stride really does share the
    /// layout `find_by_stride` picked for it" - `SubMesh::non_finite_
    /// texcoords`'s own doc records ~21% of the corpus's texcoords failing
    /// this exact check because two chunks can share a stride while packing
    /// different attributes into it. If `tangent`'s own candidate scores
    /// meaningfully better restricted to `clean` vertices, that trap is
    /// part of why the unrestricted score looks weaker than `normal`'s did.
    clean: bool,
}

fn collect() -> anyhow::Result<Vec<Sample>> {
    let mut out = Vec::new();
    for package in PACKAGES {
        let Ok(mut archive) = oag_assets::psarc::Archive::open(package) else {
            continue;
        };
        let mut entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
            .cloned()
            .collect();
        entries.sort();
        for entry in entries {
            let Ok(blob) = archive.read_path(&entry) else {
                continue;
            };
            let Ok(model) = psp2::parse(&blob) else {
                continue;
            };
            let Some(&gpu) = model.sections.get(1) else {
                continue;
            };
            let cpu = model.sections[0];
            let declarations = psp2::vertex_decl::find_by_stride(&blob[cpu.at..cpu.at + cpu.len]);

            for submesh in &model.submeshes {
                if submesh.normals.is_empty() {
                    continue;
                }
                let Some(decl) = declarations.get(&submesh.stride) else {
                    continue;
                };
                let Some(tangent) = decl.attribute(psp2::vertex_decl::TANGENT_HASH) else {
                    continue;
                };
                if tangent.components != 4 || tangent.gxm_type != 5 {
                    continue;
                }
                let off = usize::from(tangent.offset);
                if submesh.stride < off + 4 {
                    continue;
                }
                let uv1_off = decl
                    .diffuse_texcoord()
                    .filter(|a| submesh.stride >= usize::from(a.offset) + 4)
                    .map(|a| usize::from(a.offset));
                let base = cpu.at + submesh.record;
                let vertex_ptr = u32::from_le_bytes(
                    blob[base + psp2::VERTEX_POINTER..][..4].try_into().unwrap(),
                );
                let vertex_at = gpu.at + vertex_ptr as usize;
                for (v, normal) in submesh.normals.iter().enumerate() {
                    let at = vertex_at + v * submesh.stride + off;
                    if at + 4 > blob.len() {
                        continue;
                    }
                    let clean = uv1_off.is_some_and(|uv_off| {
                        let uv_at = vertex_at + v * submesh.stride + uv_off;
                        blob.get(uv_at..uv_at + 4).is_some_and(|b| {
                            let uv = psp2::unpack_texcoord(b.try_into().unwrap());
                            uv[0].is_finite() && uv[1].is_finite()
                        })
                    });
                    out.push(Sample {
                        raw: blob[at..at + 4].try_into().unwrap(),
                        normal: *normal,
                        clean,
                    });
                }
            }
        }
    }
    Ok(out)
}

fn permutations3<T: Copy>(items: [T; 3]) -> [[T; 3]; 6] {
    let [a, b, c] = items;
    [
        [a, b, c],
        [a, c, b],
        [b, a, c],
        [b, c, a],
        [c, a, b],
        [c, b, a],
    ]
}

struct Score {
    label: String,
    n: usize,
    unit_within_10pct: usize,
    mean_len: f64,
    mean_abs_dot: f64,
    orth_within_20deg: usize,
}

fn score(label: String, samples: &[Sample], decode: impl Fn(&[u8; 4]) -> [f32; 3]) -> Score {
    score_filtered(label, samples, decode, |_| true)
}

fn score_filtered(
    label: String,
    samples: &[Sample],
    decode: impl Fn(&[u8; 4]) -> [f32; 3],
    keep: impl Fn(&Sample) -> bool,
) -> Score {
    let samples: Vec<&Sample> = samples.iter().filter(|s| keep(s)).collect();
    let mut len_sum = 0f64;
    let mut unit_within_10pct = 0usize;
    let mut abs_dot_sum = 0f64;
    let mut orth_within_20deg = 0usize;
    for s in &samples {
        let t = decode(&s.raw);
        let len: f32 = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
        len_sum += f64::from(len);
        if (len - 1.0).abs() < 0.1 {
            unit_within_10pct += 1;
        }
        let unit = if len > 1e-6 {
            [t[0] / len, t[1] / len, t[2] / len]
        } else {
            t
        };
        let dot: f32 = (0..3).map(|a| unit[a] * s.normal[a]).sum();
        abs_dot_sum += f64::from(dot.abs());
        // Orthogonal (90 deg) within 20 degrees either side.
        if dot.abs() < 0.342 {
            orth_within_20deg += 1;
        }
    }
    Score {
        label,
        n: samples.len(),
        unit_within_10pct,
        mean_len: len_sum / samples.len() as f64,
        mean_abs_dot: abs_dot_sum / samples.len() as f64,
        orth_within_20deg,
    }
}

fn print_score(s: &Score) {
    println!(
        "  unit {:5.1}%  mean_len {:.3}  mean|dot(n)| {:.3}  orth-ish {:5.1}%  n={}  {}",
        100.0 * s.unit_within_10pct as f64 / s.n.max(1) as f64,
        s.mean_len,
        s.mean_abs_dot,
        100.0 * s.orth_within_20deg as f64 / s.n.max(1) as f64,
        s.n,
        s.label
    );
}

fn main() -> anyhow::Result<()> {
    let samples = collect()?;
    println!(
        "{} vertices with a declared tangent attribute (type 5, 4 components) and a decoded normal",
        samples.len()
    );
    if samples.is_empty() {
        return Ok(());
    }

    let mut scores = Vec::new();
    for w_byte in 0..4usize {
        let rest: Vec<usize> = (0..4).filter(|&i| i != w_byte).collect();
        let rest: [usize; 3] = [rest[0], rest[1], rest[2]];
        for order in permutations3(rest) {
            for signed in [true, false] {
                let label = format!(
                    "byte[{}]=x byte[{}]=y byte[{}]=z ({}), w=byte[{w_byte}]",
                    order[0],
                    order[1],
                    order[2],
                    if signed { "i8/127" } else { "u8/127.5-1" }
                );
                let decode = move |raw: &[u8; 4]| -> [f32; 3] {
                    std::array::from_fn(|a| {
                        let b = raw[order[a]];
                        if signed {
                            f32::from(b as i8) / 127.0
                        } else {
                            f32::from(b) / 127.5 - 1.0
                        }
                    })
                };
                scores.push(score(label, &samples, decode));
            }
        }
    }

    // A control every one of the 48 real candidates should beat: three raw
    // bytes read straight as if they always summed to a unit vector, with no
    // signed conversion at all (values 0..255 divided by 255) - not a
    // plausible encoding, just a floor for what "no signal" looks like.
    let control = score(
        "CONTROL: byte[0..3]/255, no sign".to_string(),
        &samples,
        |raw| std::array::from_fn(|a| f32::from(raw[a]) / 255.0),
    );

    scores.sort_by_key(|s| std::cmp::Reverse(s.unit_within_10pct));
    println!("\n=== ranked by unit-length survival (|len - 1| < 10%) ===");
    for s in scores.iter().take(10) {
        println!(
            "  unit {:5.1}%  mean_len {:.3}  mean|dot(n)| {:.3}  orth-ish {:5.1}%  n={}  {}",
            100.0 * s.unit_within_10pct as f64 / s.n as f64,
            s.mean_len,
            s.mean_abs_dot,
            100.0 * s.orth_within_20deg as f64 / s.n as f64,
            s.n,
            s.label
        );
    }
    println!(
        "  CONTROL: unit {:5.1}%  mean_len {:.3}  mean|dot(n)| {:.3}  orth-ish {:5.1}%  n={}",
        100.0 * control.unit_within_10pct as f64 / control.n as f64,
        control.mean_len,
        control.mean_abs_dot,
        100.0 * control.orth_within_20deg as f64 / control.n as f64,
        control.n,
    );

    // The winning candidate, restricted to vertices whose Uv1 (decoded
    // through the same per-stride declaration `tangent`'s own offset came
    // from) is finite - a proxy for "this chunk's declaration really is the
    // one this stride was authored with", the same ~21%-contamination trap
    // `SubMesh::non_finite_texcoords` already documents. If the winning
    // candidate's numbers improve substantially on this subset, that trap
    // - not the byte encoding - is most of why the unrestricted score looked
    // weaker than `normal`'s did.
    let clean_n = samples.iter().filter(|s| s.clean).count();
    println!(
        "\n=== same winning candidate (byte[0]=x byte[1]=y byte[2]=z i8/127, w=byte[3]), \
         split by whether this vertex's Uv1 decoded finite ({clean_n}/{} clean) ===",
        samples.len()
    );
    let winning =
        |raw: &[u8; 4]| -> [f32; 3] { std::array::from_fn(|a| f32::from(raw[a] as i8) / 127.0) };
    print_score(&score_filtered(
        "clean (Uv1 finite)".to_string(),
        &samples,
        winning,
        |s| s.clean,
    ));
    print_score(&score_filtered(
        "dirty (Uv1 non-finite or no Uv1 for this stride)".to_string(),
        &samples,
        winning,
        |s| !s.clean,
    ));

    // The winning candidate's own length and |dot| distributions, bucketed -
    // is "mean_len 0.805" a smooth spread around that value, or two
    // populations (some genuinely unit, some near zero or otherwise off)
    // averaging out to it? Matters for whether "58% pass a 10% band" is the
    // right way to read this at all.
    println!("\n=== winning candidate: length and |dot(normal)| histograms ===");
    let mut len_hist = [0usize; 15]; // 0.0-0.1, 0.1-0.2, ..., 1.4+
    let mut dot_hist = [0usize; 11]; // 0.0-0.1, ..., 1.0
    for s in &samples {
        let t = winning(&s.raw);
        let len = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
        len_hist[((len * 10.0) as usize).min(14)] += 1;
        let unit = if len > 1e-6 {
            [t[0] / len, t[1] / len, t[2] / len]
        } else {
            t
        };
        let dot: f32 = (0..3).map(|a| unit[a] * s.normal[a]).sum::<f32>().abs();
        dot_hist[((dot * 10.0) as usize).min(10)] += 1;
    }
    println!("  len buckets (0.1 wide, last is 1.4+): {len_hist:?}");
    println!("  |dot| buckets (0.1 wide, last is 1.0): {dot_hist:?}");

    // Restricted to vertices whose length already passes the 10% unit band -
    // if orthogonality improves sharply there, "unit and orthogonal" are one
    // population and the rest is a different (or wrongly declared) one.
    print_score(&score_filtered(
        "winning candidate, restricted to |len-1|<10%".to_string(),
        &samples,
        winning,
        |s| {
            let t = winning(&s.raw);
            let len = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
            (len - 1.0).abs() < 0.1
        },
    ));

    // Which byte position, if any, looks like a +-1 (i.e. +-127 raw) sign
    // rather than genuine padding or a fourth used magnitude - histogram all
    // four positions independently of any winning candidate above.
    println!("\n=== per-byte-position histogram (top values, i8 reading), all vertices ===");
    for pos in 0..4usize {
        print_byte_histogram(&samples, pos, |_| true);
    }
    println!(
        "\n=== byte[3] only, restricted to the same |len-1|<10% subset above \
         (does w look more like a clean +-1 sign once xyz is well-formed?) ==="
    );
    print_byte_histogram(&samples, 3, |s| {
        let t = winning(&s.raw);
        let len = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
        (len - 1.0).abs() < 0.1
    });

    Ok(())
}

fn print_byte_histogram(samples: &[Sample], pos: usize, keep: impl Fn(&Sample) -> bool) {
    let mut hist: BTreeMap<i8, usize> = BTreeMap::new();
    let mut n = 0usize;
    for s in samples.iter().filter(|s| keep(s)) {
        *hist.entry(s.raw[pos] as i8).or_default() += 1;
        n += 1;
    }
    let mut hist: Vec<(i8, usize)> = hist.into_iter().collect();
    hist.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    let exactly_127_or_neg128 = hist
        .iter()
        .filter(|&&(v, _)| v == 127 || v == -128 || v == -127)
        .map(|&(_, count)| count)
        .sum::<usize>();
    println!(
        "  byte[{pos}]: {:.1}% at exactly +-127/-128, top values {:?} (n={n})",
        100.0 * exactly_127_or_neg128 as f64 / n.max(1) as f64,
        &hist[..hist.len().min(6)]
    );
}
