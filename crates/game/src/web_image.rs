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
//! A race load reads on a Web Worker (`oag_thread::web`), where the
//! page's reader object does not exist: `web/worker.js` puts its own there as
//! `oagImageReader`, a `FileReaderSync` over the same `File`, and each thread
//! picks up its realm's reader on first use. The page's thread and a worker
//! each have a cache of their own ([`Shape`]), and the page's thread never
//! waits on a lock (see [`Sliced::read_at`]). See docs/tools/web.md.

use std::io;
use std::sync::Mutex;

use wasm_bindgen::{JsCast, JsValue};

/// How one realm's cache cuts the image up. The page's thread and a worker read
/// differently and are cached apart, because each undid the other's cache:
/// the worker's 200 MiB of race load evicted every block the front end had
/// read, so leaving a race re-read the menus' data through synchronous
/// requests on the page's thread (41 reads, about 1.5 s frozen), and the page's
/// scattered small reads (a table of contents, a name, a sprite) each pulled a
/// whole 1 MiB block, 119 MiB for the HD boot.
struct Shape {
    /// The unit a miss fetches. A fetch costs a fixed request plus the copy
    /// out of the response, which was measured at about 110 MB/s, so a size
    /// that wastes nothing on scattered reads wins on the page and the fixed
    /// part wins on a worker's sequential ones. Chosen, not measured, except
    /// for those two costs.
    block: u64,
    /// Blocks kept, least recently used dropped first.
    blocks: usize,
    /// A read at least this long skips the cache and is fetched exactly.
    direct: usize,
}

/// The page's thread: 64 MiB of 128 KiB blocks.
const PAGE: Shape = Shape {
    block: 128 << 10,
    blocks: 512,
    direct: 512 << 10,
};

/// A worker, whose reads are the race load's long ones: 32 MiB of 1 MiB blocks.
const WORKER: Shape = Shape {
    block: 1 << 20,
    blocks: 32,
    direct: 4 << 20,
};

/// The page's reader behind a block cache.
pub(crate) struct Sliced {
    size: u64,
    /// The page's thread's blocks, and a worker's: see [`Shape`].
    page: Mutex<Cache>,
    worker: Mutex<Cache>,
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
            page: Mutex::new(Cache::default()),
            worker: Mutex::new(Cache::default()),
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
        log::debug!(
            "web: image fetch at {offset}, {} bytes, on {}",
            into.len(),
            if oag_thread::web::on_worker() {
                "a worker"
            } else {
                "the page"
            }
        );
        Ok(())
    }

    /// The cached block `index`, fetched on a miss.
    fn block<'a>(&self, cache: &'a mut Cache, shape: &Shape, index: u64) -> io::Result<&'a [u8]> {
        cache.clock += 1;
        let now = cache.clock;
        if let Some(at) = cache.blocks.iter().position(|(i, _, _)| *i == index) {
            cache.blocks[at].2 = now;
            return Ok(&cache.blocks[at].1);
        }
        let start = index * shape.block;
        let len = usize::try_from(shape.block.min(self.size - start)).unwrap_or(usize::MAX);
        let mut bytes = vec![0; len].into_boxed_slice();
        Self::fetch(cache, start, &mut bytes)?;
        if cache.blocks.len() >= shape.blocks
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
        // there), and a worker holds its own across its fetches. The two have
        // a cache each (see [`Shape`]), so the page only ever meets a held lock
        // while a worker is in *its* cache and the page is not - which is
        // never: the page does not touch the worker's. `try_lock` stays for
        // the page's own, which nothing else holds, and as a guard.
        let (lock, shape) = if oag_thread::web::on_worker() {
            (&self.worker, &WORKER)
        } else {
            (&self.page, &PAGE)
        };
        let mut cache = if oag_thread::web::on_worker() {
            lock.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        } else {
            match lock.try_lock() {
                Ok(cache) => cache,
                Err(std::sync::TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
                Err(std::sync::TryLockError::WouldBlock) => {
                    Self::fetch_uncounted(offset, buf)?;
                    return Ok(n);
                }
            }
        };
        if n >= shape.direct {
            Self::fetch(&mut cache, offset, buf)?;
            return Ok(n);
        }
        let mut done = 0;
        while done < n {
            let at = offset + done as u64;
            let block = self.block(&mut cache, shape, at / shape.block)?;
            let within = usize::try_from(at % shape.block).unwrap_or(0);
            let take = (block.len() - within).min(n - done);
            buf[done..done + take].copy_from_slice(&block[within..within + take]);
            done += take;
        }
        Ok(n)
    }
}
