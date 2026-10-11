//! The block codecs a UnityFS bundle uses: LZ4 / LZ4HC (one decoder, since the
//! HC variant only changes the encoder) and LZMA.

use crate::error::{Error, Result};

/// Decodes one raw LZ4 block to exactly `out_len` bytes.
///
/// `out_len` must already be bounded by the caller; this allocates it.
pub fn lz4_block(src: &[u8], out_len: usize) -> Result<Vec<u8>> {
    let bad = |reason: &str| Error::Decompress {
        codec: "lz4",
        reason: reason.to_string(),
    };
    // LZ4 cannot expand by more than 255:1, so a hostile declared size cannot
    // make this reserve more than the input could ever fill.
    if out_len > src.len().saturating_mul(255) {
        return Err(bad("declared size exceeds what the input can expand to"));
    }
    let mut out = Vec::with_capacity(out_len);
    let mut i = 0usize;
    while i < src.len() {
        let token = src[i];
        i += 1;
        let mut lit = usize::from(token >> 4);
        if lit == 15 {
            loop {
                let b = *src.get(i).ok_or_else(|| bad("literal length runs off"))?;
                i += 1;
                lit = lit
                    .checked_add(usize::from(b))
                    .ok_or_else(|| bad("overflow"))?;
                if b != 255 {
                    break;
                }
            }
        }
        let lit_end = i.checked_add(lit).ok_or_else(|| bad("overflow"))?;
        if lit_end > src.len() || out.len() + lit > out_len {
            return Err(bad("literal run past the end"));
        }
        out.extend_from_slice(&src[i..lit_end]);
        i = lit_end;
        if i == src.len() {
            break;
        }
        if i + 2 > src.len() {
            return Err(bad("offset runs off"));
        }
        let offset = usize::from(u16::from_le_bytes([src[i], src[i + 1]]));
        i += 2;
        if offset == 0 || offset > out.len() {
            return Err(bad("match offset outside the output"));
        }
        let mut mlen = usize::from(token & 15);
        if mlen == 15 {
            loop {
                let b = *src.get(i).ok_or_else(|| bad("match length runs off"))?;
                i += 1;
                mlen = mlen
                    .checked_add(usize::from(b))
                    .ok_or_else(|| bad("overflow"))?;
                if b != 255 {
                    break;
                }
            }
        }
        mlen += 4;
        if out.len() + mlen > out_len {
            return Err(bad("match past the end"));
        }
        let start = out.len() - offset;
        for k in 0..mlen {
            let b = out[start + k];
            out.push(b);
        }
    }
    if out.len() != out_len {
        return Err(bad("output shorter than declared"));
    }
    Ok(out)
}

/// The most dictionary memory an LZMA stream may ask for.
const LZMA_MEMLIMIT: usize = 256 << 20;

/// The most output reserved up front before data has actually arrived.
const RESERVE_CAP: usize = 64 << 20;

/// Decodes Unity's LZMA framing: five property bytes, then the stream, with the
/// unpacked size taken from the block table rather than a header field.
pub fn lzma_block(src: &[u8], out_len: usize) -> Result<Vec<u8>> {
    // LZMA has no fixed expansion bound, so reserve a modest amount and let the
    // vector grow as data actually arrives.
    let mut out = Vec::with_capacity(out_len.min(RESERVE_CAP));
    let options = lzma_rs_perf_exp::decompress::Options {
        unpacked_size: lzma_rs_perf_exp::decompress::UnpackedSize::UseProvided(Some(
            out_len as u64,
        )),
        memlimit: Some(LZMA_MEMLIMIT),
        allow_incomplete: false,
    };
    let mut input = src;
    lzma_rs_perf_exp::lzma_decompress_with_options(&mut input, &mut out, &options).map_err(
        |e| Error::Decompress {
            codec: "lzma",
            reason: e.to_string(),
        },
    )?;
    if out.len() != out_len {
        return Err(Error::Decompress {
            codec: "lzma",
            reason: format!("produced {} bytes, declared {out_len}", out.len()),
        });
    }
    Ok(out)
}

/// Decompresses `src` with the codec a block's flags name (the low six bits).
pub fn decompress(codec: u32, src: &[u8], out_len: usize) -> Result<Vec<u8>> {
    match codec {
        0 => {
            if src.len() != out_len {
                return Err(Error::Decompress {
                    codec: "none",
                    reason: "stored block length differs from its declared size".into(),
                });
            }
            Ok(src.to_vec())
        }
        1 => lzma_block(src, out_len),
        2 | 3 => lz4_block(src, out_len),
        other => Err(Error::UnsupportedCompression(other)),
    }
}
