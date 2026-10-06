//! Which bytes of a file a parser actually reaches, and which it walks past.
//!
//! # Why this exists
//!
//! **A hand-written parser cannot fail on a field it does not know about.**
//! `rcsmodel` read the surface record embedded in a chunk header and returned;
//! the count at `+0x10` and offset table at `+0x18` naming the *other* surface
//! records were never read, so a quarter of the disc's chunks silently lost
//! their remaining geometry: 25,972 submeshes and 7.1 million triangles, 40% of
//! Wipeout HD's render geometry. Diagnostics that compared what was *drawn*
//! against what was *decided to draw* cannot surface an absence.
//!
//! A coverage sweep can: claim every range a parser reads and ask what is left.
//! An unclaimed table mid-file is a field nobody read.
//!
//! # What it is not
//!
//! **Not a correctness check, and a gap is not automatically a bug.** Real
//! files carry padding, alignment slack, lazily read string pools and sections
//! deliberately not decoded. The output is a list to *look at*, used as a
//! ratchet: record what is unclaimed today and fail when it grows. See
//! `crates/formats/tests/coverage_ground_truth.rs`.
//!
//! # Use
//!
//! ```
//! use oag_formats::coverage::Coverage;
//! let mut seen = Coverage::new(64);
//! seen.claim(0, 16, "header");
//! seen.claim(32, 32, "payload");
//! let gaps = seen.gaps(1);
//! assert_eq!(gaps.len(), 1);
//! assert_eq!((gaps[0].at, gaps[0].len), (16, 16));
//! assert_eq!((gaps[0].after, gaps[0].before), ("header", "payload"));
//! ```

/// A run of bytes no claim covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gap {
    /// Byte offset of the first unclaimed byte.
    pub at: usize,
    /// How many bytes.
    pub len: usize,
    /// What the parser read immediately before it, or `"start of file"`.
    /// What the parser read immediately before it, or `"start of file"`. The two
    /// names make a gap actionable: "384 bytes between the chunk header and its
    /// submesh descriptors" is a lead, "384 bytes at 0x11e5e0" a number.
    pub after: &'static str,
    /// What it read immediately after, or `"end of file"`.
    pub before: &'static str,
}

/// The ranges of one blob a parser reached.
///
/// Claims may overlap and may arrive in any order; [`Self::gaps`] sorts and
/// merges them. A claim that runs past the end is clamped rather than refused,
/// so a caller can hand over a computed length without checking it first.
#[derive(Debug, Clone, Default)]
pub struct Coverage {
    len: usize,
    claims: Vec<(usize, usize, &'static str)>,
}

impl Coverage {
    /// A sweep over a blob of `len` bytes, with nothing claimed yet.
    #[must_use]
    pub fn new(len: usize) -> Self {
        Self {
            len,
            claims: Vec::new(),
        }
    }

    /// Records that the parser reads `len` bytes at `at`, as `what`.
    ///
    /// A zero-length claim is dropped: kept, an empty table could name a gap it
    /// does not bound.
    pub fn claim(&mut self, at: usize, len: usize, what: &'static str) {
        if len == 0 || at >= self.len {
            return;
        }
        self.claims.push((at, len.min(self.len - at), what));
    }

    /// The blob's length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the blob is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// How many distinct bytes are claimed, counting an overlap once.
    #[must_use]
    pub fn claimed(&self) -> usize {
        self.len - self.gaps(1).iter().map(|g| g.len).sum::<usize>()
    }

    /// The share of the blob the parser reaches, `0.0` to `1.0`.
    #[must_use]
    pub fn fraction(&self) -> f64 {
        if self.len == 0 {
            return 1.0;
        }
        self.claimed() as f64 / self.len as f64
    }

    /// Every unclaimed run of at least `min` bytes, in file order.
    ///
    /// `min` is the slack to forgive: padding is a few bytes between records, a
    /// field nobody read is a table.
    #[must_use]
    pub fn gaps(&self, min: usize) -> Vec<Gap> {
        let mut claims = self.claims.clone();
        claims.sort_unstable_by_key(|(at, len, _)| (*at, *len));
        let mut gaps = Vec::new();
        let mut reached = 0usize;
        let mut previous = "start of file";
        for (at, len, what) in claims {
            if at > reached && at - reached >= min {
                gaps.push(Gap {
                    at: reached,
                    len: at - reached,
                    after: previous,
                    before: what,
                });
            }
            if at + len > reached {
                reached = at + len;
                previous = what;
            }
        }
        if self.len > reached && self.len - reached >= min {
            gaps.push(Gap {
                at: reached,
                len: self.len - reached,
                after: previous,
                before: "end of file",
            });
        }
        gaps
    }

    /// A one-line summary, and the largest few gaps.
    #[must_use]
    pub fn describe(&self, min: usize) -> String {
        let gaps = self.gaps(min);
        let unclaimed: usize = gaps.iter().map(|g| g.len).sum();
        let mut biggest = gaps.clone();
        biggest.sort_unstable_by_key(|g| std::cmp::Reverse(g.len));
        let mut out = format!(
            "{} of {} byte(s) read ({:.2}%), {} unclaimed run(s) of {min}+ bytes totalling {unclaimed}",
            self.claimed(),
            self.len,
            self.fraction() * 100.0,
            gaps.len(),
        );
        for gap in biggest.iter().take(5) {
            out.push_str(&format!(
                "\n  {:8} byte(s) at {:#010x}, between {} and {}",
                gap.len, gap.at, gap.after, gap.before,
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests;
