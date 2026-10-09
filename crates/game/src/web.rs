//! `oag-game` in the browser: the same body as the desktop binary
//! (`main_body.rs`), built for `wasm32-unknown-unknown` as the cdylib
//! `wasm-bindgen` wraps (`just web`). The page in `web/` reads the disc image
//! the player picks and hands its bytes to [`start`]. See docs/tools/web.md.

include!("main_body.rs");

/// Where the picked image is registered, and so the path the run is told.
const MOUNT_DIR: &str = "/web";

/// Boots the game on the image the page read: `name` is the file's own name,
/// `image` its bytes.
///
/// The bytes are copied into the module's memory once and the page's copy can
/// go; see `oag_disc::mount`. Resolves once the event loop has been handed to
/// the browser, which then drives every frame; rejects with the reason when
/// there is no WebGPU or the image does not open.
///
/// # Errors
/// The WebGPU adapter or device could not be had, or `run` failed before the
/// window opened (an image that is not a disc image, say).
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn start(name: String, image: js_sys::Uint8Array) -> Result<(), wasm_bindgen::JsValue> {
    web_log::install();
    let path = format!("{MOUNT_DIR}/{}", name.replace('/', "_"));
    oag_disc::mount::register(path.clone(), std::sync::Arc::new(image.to_vec()));
    drop(image);
    log::info!("web: {path} registered");
    let started = async {
        crate::gpu::web::prepare().await?;
        run(Cli::parse_from(["oag-game", &path, "--no-audio"]))
    };
    started
        .await
        .map_err(|why| wasm_bindgen::JsValue::from_str(&format!("{why:#}")))
}

/// The browser console as the log sink: there is no terminal and no file.
mod web_log {
    use log::{Level, LevelFilter, Log, Metadata, Record};

    struct Console;

    impl Log for Console {
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() <= Level::Warn
                || (metadata.level() <= Level::Info && metadata.target().starts_with("oag"))
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

    /// Installs the console logger and a panic hook that logs through it.
    pub(super) fn install() {
        if log::set_logger(&Console).is_ok() {
            log::set_max_level(LevelFilter::Info);
        }
        std::panic::set_hook(Box::new(|info| {
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!("panic: {info}")));
        }));
    }
}
