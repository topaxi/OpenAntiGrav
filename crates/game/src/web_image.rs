//! The picked image read a slice at a time, for the web build (`web.rs`).
//!
//! The page hands over a reader object, `{ size, read(offset, length) }`,
//! whose `read` returns that many bytes of the picked `File` synchronously
//! (a synchronous `XMLHttpRequest` on a blob URL of `file.slice(..)`, in
//! `web/main.js`). [`Sliced`] puts a block cache in front of it and registers
//! as an [`oag_disc::mount::Blob`], so the disc readers above never learn the
//! image is not in memory. Images of any size open this way; the cache is the
//! only part held.
//!
//! A race load reads on a Web Worker (`oag_raceplay::web_thread`), where the
//! page's reader object does not exist: `web/worker.js` puts its own there as
//! `oagImageReader`, a `FileReaderSync` over the same `File`, and each thread
//! picks up its realm's reader on first use. The cache is shared; the page's
//! thread never waits on it (see [`Sliced::read_at`]). See docs/tools/web.md.

use std::io;
use std::sync::Mutex;

use wasm_bindgen::{JsCast, JsValue};

/// One cached block: 1 MiB, the unit a miss fetches. Chosen, not measured
/// against anything but the fetch cost below: one fetch costs about 1 ms
/// whatever its size up to a few MiB in Chromium 153, and copies at about
/// 110 MB/s.
const BLOCK: u64 = 1 << 20;
/// Blocks kept, least recently used dropped first: 64 MiB of the module's
/// memory. Chosen, not measured.
const BLOCKS: usize = 64;
/// A read at least this long skips the cache and is fetched whole.
const DIRECT: usize = 4 << 20;

/// The page's reader behind a block cache.
pub(crate) struct Sliced {
    size: u64,
    cache: Mutex<Cache>,
}

thread_local! {
    /// This thread's reader object and its `read`. JS handles may not cross
    /// threads, and a [`Blob`](oag_disc::mount::Blob) must be `Send + Sync`,
    /// so they live here and [`Sliced`] holds only plain data: the page's,
    /// given to [`Sliced::new`], or a worker's `oagImageReader`, found on its
    /// first read.
    static READER: std::cell::RefCell<Option<(JsValue, js_sys::Function)>> =
        const { std::cell::RefCell::new(None) };
}

#[derive(Default)]
struct Cache {
    /// `(block index, bytes, last use)`.
    blocks: Vec<(u64, Box<[u8]>, u64)>,
    clock: u64,
    fetches: u64,
    fetched: u64,
}

impl Sliced {
    /// Wraps the page's reader object.
    ///
    /// # Errors
    /// The object has no numeric `size` or no `read` function.
    pub(crate) fn new(reader: JsValue) -> Result<Self, JsValue> {
        let size = js_sys::Reflect::get(&reader, &JsValue::from_str("size"))?
            .as_f64()
            .ok_or_else(|| JsValue::from_str("image reader has no size"))?;
        let read = js_sys::Reflect::get(&reader, &JsValue::from_str("read"))?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| JsValue::from_str("image reader has no read function"))?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let size = size as u64;
        READER.with_borrow_mut(|slot| *slot = Some((reader, read)));
        Ok(Self {
            size,
            cache: Mutex::new(Cache::default()),
        })
    }

    /// This realm's reader, from `globalThis.oagImageReader` (a worker's).
    fn realm_reader() -> Result<(JsValue, js_sys::Function), JsValue> {
        let reader = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("oagImageReader"))?;
        let read = js_sys::Reflect::get(&reader, &JsValue::from_str("read"))?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| JsValue::from_str("no image reader on this thread"))?;
        Ok((reader, read))
    }

    /// Fetches `[offset, offset + into.len())` from the page into `into`.
    fn fetch(cache: &mut Cache, offset: u64, into: &mut [u8]) -> io::Result<()> {
        Self::fetch_uncounted(offset, into)?;
        cache.fetches += 1;
        cache.fetched += into.len() as u64;
        if cache.fetches.is_power_of_two() || cache.fetches.is_multiple_of(1024) {
            log::info!(
                "web: image read {} times, {} MiB fetched",
                cache.fetches,
                cache.fetched >> 20
            );
        }
        Ok(())
    }

    /// [`Self::fetch`] without the cache's counters.
    fn fetch_uncounted(offset: u64, into: &mut [u8]) -> io::Result<()> {
        READER.with_borrow_mut(|reader| {
            if reader.is_none() {
                *reader = Self::realm_reader().ok();
            }
        });
        #[allow(clippy::cast_precision_loss)]
        let bytes = READER
            .with_borrow(|reader| {
                let (this, read) = reader
                    .as_ref()
                    .ok_or_else(|| JsValue::from_str("no image reader on this thread"))?;
                read.call2(
                    this,
                    &JsValue::from_f64(offset as f64),
                    &JsValue::from_f64(into.len() as f64),
                )
            })
            .map_err(|why| io::Error::other(format!("image read failed: {why:?}")))?
            .dyn_into::<js_sys::Uint8Array>()
            .map_err(|_| io::Error::other("image read returned no bytes"))?;
        if bytes.length() as usize != into.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "image read at {offset} returned {} of {} bytes",
                    bytes.length(),
                    into.len()
                ),
            ));
        }
        bytes.copy_to(into);
        log::debug!("web: image fetch at {offset}, {} bytes", into.len());
        Ok(())
    }

    /// The cached block `index`, fetched on a miss.
    fn block<'a>(&self, cache: &'a mut Cache, index: u64) -> io::Result<&'a [u8]> {
        cache.clock += 1;
        let now = cache.clock;
        if let Some(at) = cache.blocks.iter().position(|(i, _, _)| *i == index) {
            cache.blocks[at].2 = now;
            return Ok(&cache.blocks[at].1);
        }
        let start = index * BLOCK;
        let len = usize::try_from(BLOCK.min(self.size - start)).unwrap_or(usize::MAX);
        let mut bytes = vec![0; len].into_boxed_slice();
        Self::fetch(cache, start, &mut bytes)?;
        if cache.blocks.len() >= BLOCKS
            && let Some(oldest) = (0..cache.blocks.len()).min_by_key(|&i| cache.blocks[i].2)
        {
            cache.blocks.swap_remove(oldest);
        }
        cache.blocks.push((index, bytes, now));
        Ok(&cache.blocks.last().expect("just pushed").1)
    }
}

impl oag_disc::mount::Blob for Sliced {
    fn size(&self) -> u64 {
        self.size
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        if offset >= self.size || buf.is_empty() {
            return Ok(0);
        }
        let n = usize::try_from((self.size - offset).min(buf.len() as u64)).unwrap_or(buf.len());
        let buf = &mut buf[..n];
        // The page's thread may not wait on a lock (a contended one traps
        // there), and a worker holds this one across its fetches: so the page
        // reads past the cache while a worker is in it.
        let mut cache = if oag_raceplay::web_thread::on_worker() {
            self.cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        } else {
            match self.cache.try_lock() {
                Ok(cache) => cache,
                Err(std::sync::TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
                Err(std::sync::TryLockError::WouldBlock) => {
                    Self::fetch_uncounted(offset, buf)?;
                    return Ok(n);
                }
            }
        };
        if n >= DIRECT {
            Self::fetch(&mut cache, offset, buf)?;
            return Ok(n);
        }
        let mut done = 0;
        while done < n {
            let at = offset + done as u64;
            let block = self.block(&mut cache, at / BLOCK)?;
            let within = usize::try_from(at % BLOCK).unwrap_or(0);
            let take = (block.len() - within).min(n - done);
            buf[done..done + take].copy_from_slice(&block[within..within + take]);
            done += take;
        }
        Ok(n)
    }
}
