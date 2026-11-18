//! `oag-game` in the browser: the cdylib `wasm-bindgen` wraps, built for
//! `wasm32-unknown-unknown` (`just web`). The exports are one-line calls into
//! [`oag_game::app::web`], which documents each. See docs/tools/web.md.

use wasm_bindgen::prelude::wasm_bindgen;

// Same allocator as `main.rs`: it has to be the process's own to see anything.
#[cfg(feature = "perf-probe")]
#[global_allocator]
static ALLOCATOR: oag_gpu::perfprobe::Counting = oag_gpu::perfprobe::Counting;

/// What an encrypted PS3 image is waiting for: resolves to `{ state, serial,
/// keyHex }`. See `oag_game::app::web::inspect`.
#[wasm_bindgen]
pub fn inspect(
    name: String,
    image: wasm_bindgen::JsValue,
    key: Option<Vec<u8>>,
) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
    oag_game::app::web::inspect(name, image, key)
}

/// Boots the game on the image the page picked. See `oag_game::app::web::start`.
#[wasm_bindgen]
pub fn start(
    name: String,
    image: wasm_bindgen::JsValue,
    log: Option<String>,
    key: Option<Vec<u8>>,
) -> js_sys::Promise {
    oag_game::app::web::start(name, image, log, key)
}

/// Raises one failure the page reports, on demand. See
/// `oag_game::app::web::fault_test`.
#[cfg(feature = "fault-test")]
#[wasm_bindgen]
pub fn fault_test(what: &str) {
    oag_game::app::web::fault_test(what);
}
