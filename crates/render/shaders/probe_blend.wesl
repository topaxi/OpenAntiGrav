// The hull environment-probe blend (`oag_render::probe_blend`): up to four decoded probes of
// the circuit weighted into one craft's two layers of the probe array, the arithmetic of
// `oag_vex::probes::mix` to the bit, so the array holds the bytes the CPU blend writes.
//
// `mix` is `acc += f32(byte) * scale` an image at a time, then `(acc + 0.5) as u8`, every
// step rounded as IEEE-754 rounds it. A shader's float arithmetic does not promise that: a
// driver may fuse a multiply into the add after it, and may reorder a chain of adds (Mesa
// does both to an operation that is not marked exact, and nothing here can mark one). A
// float version of this pass was 3 bytes off `mix` in one blend of eight on Intel's Vulkan
// driver. So the multiplies, the adds and the final truncation are all done on the floats'
// bits in integers, with the rounding by hand, and the result is the same on every adapter.

struct Job {
    // The four taps' slots in `sources`. A tap `mix` leaves out (weight zero) names any
    // slot and carries scale zero.
    slots: vec4<u32>,
    // The four taps' scales (weight over the weights' sum), as `f32` bits.
    scales: vec4<u32>,
    // `.x`: the craft, whose layers `2 * craft` and `2 * craft + 1` this job writes.
    craft: vec4<u32>,
}

struct Jobs {
    jobs: array<Job, 8>,
}

@group(0) @binding(0) var<uniform> jobs: Jobs;
// Decoded probes, two layers a slot in the array's own layout: the 64x32 irradiance map in
// the corner of layer `2 * slot`, the reflection map in layer `2 * slot + 1`.
@group(0) @binding(1) var sources: texture_2d_array<u32>;
@group(0) @binding(2) var blended: texture_storage_2d_array<rgba8unorm, write>;

// `f32(bytes) * scale` per channel as `f32` bits, correctly rounded (nearest, ties to even),
// `scale` a non-negative `f32` given as its bits.
//
// A byte times the scale's 24-bit significand fits 32 bits exactly; what is below the top
// 24 bits of that is rounded off by hand. A scale below the smallest normal number counts
// as zero: its products are under `1e-35`, which no sum of this blend can tell from zero
// once `0.5` is added.
fn products(bytes: vec3<u32>, scale: u32) -> vec3<u32> {
    let exponent = scale >> 23u;
    let wide = bytes * ((scale & 0x7fffffu) | 0x800000u);
    // The product's top bit is bit 23 to 31; a zero byte takes no shift and is masked below.
    let shift = vec3<u32>(8u) - min(countLeadingZeros(wide), vec3<u32>(8u));
    let kept = wide >> shift;
    let rest = wide - (kept << shift);
    let half = (vec3<u32>(1u) << shift) >> vec3<u32>(1u);
    let odd = (kept & vec3<u32>(1u)) == vec3<u32>(1u);
    let up = (rest > half) | ((rest == half) & (shift > vec3<u32>(0u)) & odd);
    // A significand rounded up to `2^24` carries into the exponent through the add.
    let significand = kept + select(vec3<u32>(0u), vec3<u32>(1u), up);
    let bits = ((vec3<u32>(exponent) + shift) << vec3<u32>(23u)) + (significand - vec3<u32>(0x800000u));
    let zero = (bytes == vec3<u32>(0u)) | vec3<bool>(exponent == 0u);
    return select(bits, vec3<u32>(0u), zero);
}

// `a + b` per channel as `f32` bits, correctly rounded, both zero or a normal positive number.
//
// The smaller significand is shifted down to the larger one's exponent with three bits kept
// below the last place (the third is set if anything nonzero fell off), which is all a
// round to nearest needs.
fn sums(a: vec3<u32>, b: vec3<u32>) -> vec3<u32> {
    let one = vec3<u32>(0x800000u);
    let mask = vec3<u32>(0x7fffffu);
    let high = max(a, b);
    let low = min(a, b);
    let exponent = high >> vec3<u32>(23u);
    let apart = min(exponent - (low >> vec3<u32>(23u)), vec3<u32>(31u));
    let small = ((low & mask) | one) << vec3<u32>(3u);
    let moved = small >> apart;
    let lost = vec3<u32>((moved << apart) != small);
    let total = (((high & mask) | one) << vec3<u32>(3u)) + (moved | lost);
    // A sum that reached the next power of two drops a place, keeping the bit it sheds.
    let carry = total >> vec3<u32>(27u);
    let placed = (total >> carry) | (total & carry);
    let kept = placed >> vec3<u32>(3u);
    let rest = placed & vec3<u32>(7u);
    let odd = (kept & vec3<u32>(1u)) == vec3<u32>(1u);
    let up = vec3<u32>((rest > vec3<u32>(4u)) | ((rest == vec3<u32>(4u)) & odd));
    let bits = ((exponent + carry) << vec3<u32>(23u)) + (kept + up - one);
    return select(bits, high, low == vec3<u32>(0u));
}

// `(value + 0.5) as u8` per channel: `0.5` added as `mix` adds it, then the whole part,
// which is at most 255.
fn rounded(value: vec3<u32>) -> vec3<u32> {
    let raised = sums(value, vec3<u32>(0x3f000000u));
    let exponent = raised >> vec3<u32>(23u);
    let significand = (raised & vec3<u32>(0x7fffffu)) | vec3<u32>(0x800000u);
    // `1.0` has exponent 127 and its whole part 23 bits up; from 256 on the answer is 255.
    let down = vec3<u32>(150u) - clamp(exponent, vec3<u32>(127u), vec3<u32>(135u));
    let whole = select(significand >> down, vec3<u32>(0u), exponent < vec3<u32>(127u));
    return min(whole, vec3<u32>(255u));
}

@compute @workgroup_size(8, 8, 1)
fn blend(@builtin(global_invocation_id) id: vec3<u32>) {
    let job = jobs.jobs[id.z / 2u];
    let which = id.z % 2u;
    // The irradiance map is the 64x32 corner of its layer; the rest is never sampled.
    if which == 0u && (id.x >= 64u || id.y >= 32u) {
        return;
    }
    let at = vec2<i32>(id.xy);
    // `mix`'s accumulator: zero, then each tap's product added in tap order.
    var sum = vec3<u32>(0u);
    for (var tap = 0u; tap < 4u; tap++) {
        let texel = textureLoad(sources, at, job.slots[tap] * 2u + which, 0).rgb;
        sum = sums(sum, products(texel, job.scales[tap]));
    }
    // A whole number of 255ths: the store rounds it back to the byte whatever it does with
    // the last bit of the division.
    textureStore(blended, at, job.craft.x * 2u + which, vec4<f32>(vec3<f32>(rounded(sum)) / 255.0, 1.0));
}
