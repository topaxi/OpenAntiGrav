//! The system clipboard: pasting a disc key into the chooser, and copying a benchmark result out.
//!
//! Desktop builds read and write it through `arboard`. **Android writes it** through JNI
//! (`oag_mobile::clipboard`) **and reads nothing**: winit's
//! `NativeActivity` backend has no clipboard and this project carries no JNI
//! shim, so [`AVAILABLE`] is `false` there and the key prompt offers its keypad
//! and, beside the image, a `.dkey` file instead. **The web build reads nothing
//! either**: the browser's clipboard is asynchronous and asks permission, and a
//! key file beside the image does not exist there; see docs/tools/web.md.

/// Whether this build can write a clipboard: every target but iOS, where the benchmark's COPY TO
/// CLIPBOARD is left off its screen.
pub(crate) const CAN_WRITE: bool = cfg!(not(target_os = "ios"));

/// Whether this build can read a clipboard.
pub(crate) const AVAILABLE: bool = cfg!(not(any(
    target_os = "android",
    target_os = "ios",
    target_arch = "wasm32"
)));

/// How a write went, once the platform knows: at once on a desktop and a phone, when the
/// browser's promise settles on the web.
#[derive(Debug, Clone, Default)]
pub(crate) struct Outcome(std::sync::Arc<std::sync::OnceLock<bool>>);

impl Outcome {
    /// A write whose result is already known.
    pub(crate) fn known(took: bool) -> Self {
        let outcome = Self::default();
        outcome.settle(took);
        outcome
    }

    /// Records the answer. The first one stands.
    pub(crate) fn settle(&self, took: bool) {
        let _ = self.0.set(took);
    }

    /// Whether the system took the text, or `None` while it has not said.
    pub(crate) fn get(&self) -> Option<bool> {
        self.0.get().copied()
    }
}

static CTRL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Records whether a Control key is down, from the window's modifier events.
pub(crate) fn set_ctrl_held(held: bool) {
    CTRL.store(held, std::sync::atomic::Ordering::Relaxed);
}

/// Whether a Control key is down.
pub(crate) fn ctrl_held() -> bool {
    CTRL.load(std::sync::atomic::Ordering::Relaxed)
}

/// The clipboard's text, or `None` when it holds none or cannot be read.
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
pub(crate) fn read() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// See the module doc.
#[cfg(any(target_os = "android", target_os = "ios", target_arch = "wasm32"))]
pub(crate) fn read() -> Option<String> {
    None
}

/// Puts `text` on the clipboard as plain text; the [`Outcome`] says whether the system took it.
///
/// The desktop's clipboard object is kept for the life of the process: on Linux the text is served
/// by the process that owns the selection, so a clipboard dropped at once would paste nothing.
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
pub(crate) fn write(text: &str) -> Outcome {
    static HELD: std::sync::Mutex<Option<arboard::Clipboard>> = std::sync::Mutex::new(None);
    let mut held = HELD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if held.is_none() {
        *held = arboard::Clipboard::new().ok();
    }
    Outcome::known(
        held.as_mut()
            .is_some_and(|clipboard| clipboard.set_text(text.to_owned()).is_ok()),
    )
}

/// See the module doc.
#[cfg(target_os = "android")]
pub(crate) fn write(text: &str) -> Outcome {
    Outcome::known(oag_mobile::clipboard::write(text).is_ok())
}

/// The browser's `navigator.clipboard.writeText`, which needs a secure page and may need the
/// click that asked. Called through `js_sys` because `web-sys`'s clipboard is behind an unstable
/// cfg. The answer is the promise's: a refusal (permission, no gesture) settles as not taken.
#[cfg(target_arch = "wasm32")]
pub(crate) fn write(text: &str) -> Outcome {
    let Some(promise) = write_promise(text) else {
        return Outcome::known(false);
    };
    let outcome = Outcome::default();
    let settled = outcome.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = wasm_bindgen_futures::JsFuture::from(promise).await;
        if let Err(why) = &result {
            log::warn!("the browser refused the clipboard write: {why:?}");
        }
        settled.settle(result.is_ok());
    });
    outcome
}

/// `writeText`'s promise, or `None` where the page has no clipboard or the call threw.
#[cfg(target_arch = "wasm32")]
fn write_promise(text: &str) -> Option<js_sys::Promise> {
    use wasm_bindgen::JsCast;
    let navigator = web_sys::window()?.navigator();
    let clipboard = js_sys::Reflect::get(&navigator, &"clipboard".into()).ok()?;
    if clipboard.is_undefined() || clipboard.is_null() {
        return None;
    }
    js_sys::Reflect::get(&clipboard, &"writeText".into())
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?
        .call1(&clipboard, &text.into())
        .ok()?
        .dyn_into::<js_sys::Promise>()
        .ok()
}

/// See the module doc.
#[cfg(target_os = "ios")]
pub(crate) fn write(_text: &str) -> Outcome {
    Outcome::known(false)
}

#[cfg(test)]
mod tests {
    /// The benchmark's COPY TO CLIPBOARD is offered on every target but iOS, which has no write.
    #[test]
    fn the_clipboard_can_be_written_on_every_target_but_ios() {
        assert_eq!(super::CAN_WRITE, cfg!(not(target_os = "ios")));
    }
}
