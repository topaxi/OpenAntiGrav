//! The state blob as a plain-data struct: [`Blob`] and the [`blob!`](crate::blob)
//! macro.

/// A value laid out in the state blob: little-endian, in field order, no padding.
pub trait Blob: Sized {
    /// Its length in bytes.
    const BYTES: usize;

    /// Reads it from the front of `bytes`; missing bytes read as zero.
    fn from_bytes(bytes: &[u8]) -> Self;

    /// Writes it to the front of `bytes`; what does not fit is dropped.
    fn to_bytes(&self, bytes: &mut [u8]);
}

fn take<const N: usize>(bytes: &[u8]) -> [u8; N] {
    let mut out = [0; N];
    if let Some(head) = bytes.get(..N) {
        out.copy_from_slice(head);
    }
    out
}

fn put(bytes: &mut [u8], value: &[u8]) {
    if let Some(head) = bytes.get_mut(..value.len()) {
        head.copy_from_slice(value);
    }
}

macro_rules! number {
    ($($ty:ty),+) => {$(
        impl Blob for $ty {
            const BYTES: usize = size_of::<$ty>();

            fn from_bytes(bytes: &[u8]) -> Self {
                <$ty>::from_le_bytes(take(bytes))
            }

            fn to_bytes(&self, bytes: &mut [u8]) {
                put(bytes, &self.to_le_bytes());
            }
        }
    )+};
}

number!(u8, i8, u16, i16, u32, i32, u64, i64, f32);

impl Blob for bool {
    const BYTES: usize = 1;

    fn from_bytes(bytes: &[u8]) -> Self {
        u8::from_bytes(bytes) != 0
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        u8::from(*self).to_bytes(bytes);
    }
}

impl<T: Blob, const N: usize> Blob for [T; N] {
    const BYTES: usize = T::BYTES * N;

    fn from_bytes(bytes: &[u8]) -> Self {
        core::array::from_fn(|i| T::from_bytes(bytes.get(i * T::BYTES..).unwrap_or(&[])))
    }

    fn to_bytes(&self, bytes: &mut [u8]) {
        for (i, value) in self.iter().enumerate() {
            if let Some(rest) = bytes.get_mut(i * T::BYTES..) {
                value.to_bytes(rest);
            }
        }
    }
}

/// Defines a state blob struct and its [`Blob`] layout: each field in order,
/// little-endian, no padding. Fields are integers, `f32`, `bool`, fixed arrays of those
/// and other `blob!` structs. The struct takes the attributes written on it (derives
/// included) and nothing more; a `const` assert keeps it within `MAX_STATE_BYTES`.
///
/// ```
/// oag_ext::blob! {
///     /// One slot's record.
///     #[derive(Debug, Clone, Copy, PartialEq, Default)]
///     pub struct Record { pub spent: f32, pub burst: u16, pub boosting: bool }
/// }
/// assert_eq!(<Record as oag_ext::Blob>::BYTES, 7);
/// ```
#[macro_export]
macro_rules! blob {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $($(#[$field_meta:meta])* $field_vis:vis $field:ident : $ty:ty),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $name {
            $($(#[$field_meta])* $field_vis $field: $ty),*
        }

        impl $crate::Blob for $name {
            const BYTES: usize = 0 $(+ <$ty as $crate::Blob>::BYTES)*;

            fn from_bytes(bytes: &[u8]) -> Self {
                let mut at = 0_usize;
                $(
                    let $field = <$ty as $crate::Blob>::from_bytes(bytes.get(at..).unwrap_or(&[]));
                    at += <$ty as $crate::Blob>::BYTES;
                )*
                let _ = at;
                Self { $($field),* }
            }

            fn to_bytes(&self, bytes: &mut [u8]) {
                let mut at = 0_usize;
                $(
                    if let Some(rest) = bytes.get_mut(at..) {
                        <$ty as $crate::Blob>::to_bytes(&self.$field, rest);
                    }
                    at += <$ty as $crate::Blob>::BYTES;
                )*
                let _ = at;
            }
        }

        const _: () = assert!(
            <$name as $crate::Blob>::BYTES <= $crate::MAX_STATE_BYTES,
            "a state blob is at most MAX_STATE_BYTES long"
        );
    };
}
