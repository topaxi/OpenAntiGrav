//! `oag-game` in the browser: the same body as the desktop binary
//! (`main_body.rs`), built for `wasm32-unknown-unknown` as the cdylib
//! `wasm-bindgen` wraps (`just web`). The page in `web/` reads the disc image
//! the player picks and hands it to [`start`]. See docs/tools/web.md.

include!("main_body.rs");

/// Where the picked image is registered, and so the path the run is told.
const MOUNT_DIR: &str = "/web";

#[path = "web_image.rs"]
mod web_image;

/// Registers the page's image under `/web/<name>` and returns that path.
/// `image` is as [`start`] describes it.
///
/// `image` undefined or null means the page already mounted this file (its
/// first `inspect`): nothing is registered again. An in-memory image
/// (`?read=memory`) is a copy in the module's memory, which never shrinks, so a
/// second registration would double what that fallback holds.
fn mount_image(name: &str, image: &wasm_bindgen::JsValue) -> Result<String, wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    let path = format!("{MOUNT_DIR}/{}", name.replace('/', "_"));
    if image.is_undefined() || image.is_null() {
        return Ok(path);
    }
    let blob: std::sync::Arc<dyn oag_disc::mount::Blob> =
        match image.dyn_ref::<js_sys::Uint8Array>() {
            Some(bytes) => {
                log::info!("web: {path} held in memory ({} bytes)", bytes.length());
                std::sync::Arc::new(bytes.to_vec())
            }
            None => {
                let sliced = web_image::Sliced::new(image.clone())?;
                log::info!(
                    "web: {path} read in slices ({} bytes)",
                    oag_disc::mount::Blob::size(&sliced)
                );
                std::sync::Arc::new(sliced)
            }
        };
    oag_disc::mount::register(path.clone(), blob);
    Ok(path)
}

/// What an encrypted PS3 image is waiting for, asked before anything boots so
/// the page can show a key field and look up a remembered key.
///
/// `key` is what the player supplied, if anything: a `.dkey`'s bytes or the
/// key's hex text. Resolves to `{ state, serial, keyHex }`: `state` is
/// `"ready"` (no key needed, or the key opens it), `"missing"`, `"malformed"`
/// or `"wrong"`; `serial` the disc's serial when it reads; `keyHex` the
/// accepted key as hex, for the page to remember in this browser and nowhere
/// else. The key is never logged.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn inspect(
    name: String,
    image: wasm_bindgen::JsValue,
    key: Option<Vec<u8>>,
) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
    use oag_disc::ps3_probe::{KeyCheck, probe};
    let path = mount_image(&name, &image)?;
    let probed = probe(std::path::Path::new(&path), key.as_deref())
        .map_err(|why| wasm_bindgen::JsValue::from_str(&format!("{why:#}")))?;
    let (state, key_hex) = match &probed.check {
        KeyCheck::NotNeeded => ("ready", None),
        KeyCheck::Accepted(key) => ("ready", Some(key.to_hex())),
        KeyCheck::Missing => ("missing", None),
        KeyCheck::Malformed => ("malformed", None),
        KeyCheck::Wrong => ("wrong", None),
    };
    let out = js_sys::Object::new();
    let set = |field: &str, value: wasm_bindgen::JsValue| {
        let _ = js_sys::Reflect::set(&out, &field.into(), &value);
    };
    set("state", state.into());
    set(
        "serial",
        probed
            .serial
            .map_or(wasm_bindgen::JsValue::NULL, Into::into),
    );
    set(
        "keyHex",
        key_hex.map_or(wasm_bindgen::JsValue::NULL, Into::into),
    );
    Ok(out.into())
}

/// Boots the game on the image the page picked: `name` is the file's own name,
/// `image` either its bytes (a `Uint8Array`, copied into the module's memory
/// once) or a reader object `{ size, read(offset, length) }` that returns a
/// slice of the file synchronously (`web_image`), `log` the page's `?log=`
/// parameter (`debug`, `trace`; absent is `info` for this project's own lines
/// and `warn` for the rest) and `key` the disc key of an encrypted PS3 image,
/// as `inspect` takes it.
///
/// The key is mounted as a file beside the image (`<stem>.dkey`), which is
/// where `DiscImage::open` looks for one natively, so the race load's worker
/// finds it through the shared mount table with no message of its own.
///
/// The promise resolves once the event loop has been handed to the browser,
/// which then drives every frame, and rejects with the reason when there is no
/// WebGPU or the image does not open (a file that is not a disc image, say).
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn start(
    name: String,
    image: wasm_bindgen::JsValue,
    log: Option<String>,
    key: Option<Vec<u8>>,
) -> js_sys::Promise {
    web_log::install(log.as_deref().and_then(|level| level.parse().ok()));
    let path = match mount_image(&name, &image) {
        Ok(path) => path,
        Err(why) => return js_sys::Promise::reject(&why),
    };
    if let Some(key) = key {
        use oag_disc::ps3_probe::{KeyCheck, probe};
        let sibling = std::path::Path::new(&path).with_extension("dkey");
        match probe(std::path::Path::new(&path), Some(&key)) {
            Ok(probed) => match probed.check {
                KeyCheck::Accepted(key) => {
                    oag_disc::mount::register(
                        sibling,
                        std::sync::Arc::new(key.to_hex().into_bytes()),
                    );
                }
                KeyCheck::NotNeeded => {}
                _ => {
                    return js_sys::Promise::reject(&wasm_bindgen::JsValue::from_str(
                        "that key does not open this disc",
                    ));
                }
            },
            Err(why) => {
                return js_sys::Promise::reject(&wasm_bindgen::JsValue::from_str(&format!(
                    "{why:#}"
                )));
            }
        }
    }
    wasm_bindgen_futures::future_to_promise(async move {
        let started = match crate::gpu::web::prepare().await {
            Ok(()) => run(Cli::parse_from(["oag-game", &path])),
            Err(why) => Err(why),
        };
        started
            .map(|()| wasm_bindgen::JsValue::UNDEFINED)
            .map_err(|why| wasm_bindgen::JsValue::from_str(&format!("{why:#}")))
    })
}

/// The browser console as the log sink: there is no terminal and no file.
mod web_log {
    use log::{Level, LevelFilter, Log, Metadata, Record};

    struct Console {
        /// How far this project's own lines go; everything else stops at warn.
        ours: LevelFilter,
    }

    impl Log for Console {
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() <= Level::Warn
                || (metadata.level() <= self.ours && metadata.target().starts_with("oag"))
        }

        fn log(&self, record: &Record) {
            if !self.enabled(record.metadata()) {
                return;
            }
            let line = wasm_bindgen::JsValue::from_str(&format!(
                "{} {}: {}",
                record.level(),
                record.target(),
                record.args()
            ));
            match record.level() {
                Level::Error => web_sys::console::error_1(&line),
                Level::Warn => web_sys::console::warn_1(&line),
                _ => web_sys::console::log_1(&line),
            }
        }

        fn flush(&self) {}
    }

    /// Installs the console logger and a panic hook that logs through it and puts
    /// the panic on the page.
    pub(super) fn install(ours: Option<LevelFilter>) {
        let ours = ours.unwrap_or(LevelFilter::Info);
        if log::set_logger(Box::leak(Box::new(Console { ours }))).is_ok() {
            log::set_max_level(ours.max(LevelFilter::Warn));
        }
        std::panic::set_hook(Box::new(|info| {
            let text = format!("panic: {info}");
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&text));
            crate::gpu::web::fatal(&text);
        }));
    }
}
