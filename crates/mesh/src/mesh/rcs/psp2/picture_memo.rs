//! Every material picture one model build decoded, by path.
//!
//! The planners each keep their own map of what they decoded, and a picture one of them binds
//! is often another's too: on Omega's Tech De Ra track, `emissive_add` and `normal_map` read
//! the same three textures (two of 89 MB stored), so the build read and decoded each twice.
//! [`scoped`] opens one memo for a whole build and [`picture`] answers from it, so a second
//! planner asking for a path takes the first one's texture without reading the archive again.
//!
//! The decode is a function of the path and its bytes alone, and every planner of one build
//! reads through the same `textures`, so a shared answer is the one each planner would have
//! decoded itself. Each answer counts its `.gnf` form again in the asking build's report, as
//! a decode would have.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use super::{Report, Textures, decode_texture_form};
use crate::mesh::{GnfForm, ModelTexture};

/// Path to what decoding it gave: the texture and its `.gnf` form, or `None` for a picture
/// that did not read or decode.
type Memo = HashMap<String, Option<(Arc<ModelTexture>, Option<GnfForm>)>>;

thread_local! {
    static MEMO: RefCell<Option<Memo>> = const { RefCell::new(None) };
}

/// Runs `f` with [`picture`] memoised on this thread. A nested call gets a memo of its own,
/// and the outer one is back when it returns.
pub(super) fn scoped<R>(f: impl FnOnce() -> R) -> R {
    struct Restore(Option<Memo>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let outer = self.0.take();
            MEMO.with_borrow_mut(|memo| *memo = outer);
        }
    }
    let outer = MEMO.with_borrow_mut(|memo| memo.replace(Memo::new()));
    let _restore = Restore(outer);
    f()
}

/// The picture at `path`, read and decoded once per build (offered to the texture sink as
/// [`super::decode_material_texture`] does), or `None` when it does not read or decode.
pub(super) fn picture(
    path: &str,
    textures: Textures<'_>,
    report: &mut Report,
) -> Option<Arc<ModelTexture>> {
    let known = MEMO.with_borrow(|memo| memo.as_ref().map(|memo| memo.get(path).cloned()));
    let found = match known {
        Some(Some(found)) => found,
        open => {
            let found = textures(path)
                .and_then(|blob| decode_texture_form(path, &blob))
                .map(|(texture, form)| {
                    let texture = crate::mesh_render::offer_to_texture_sink(texture);
                    (Arc::new(texture), form)
                });
            if open.is_some() {
                MEMO.with_borrow_mut(|memo| {
                    if let Some(memo) = memo.as_mut() {
                        memo.insert(path.to_string(), found.clone());
                    }
                });
            }
            found
        }
    };
    let (texture, form) = found?;
    if let Some(form) = form {
        report.gnf.record(form);
    }
    Some(texture)
}
