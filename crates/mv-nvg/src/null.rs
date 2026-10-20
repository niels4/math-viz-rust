//! The null context: the `Context` of a test that never draws.
//!
//! The Odin tests build `new(App)` with a nil `vg`; the Rust tests build
//! [`Context`] inverses [`Context::null`]. A method called on one hands
//! NanoVG a null context, which the C side tolerates the way it tolerates
//! the Odin tests' nil (no test here draws: drawing needs GL, which a
//! `cargo test` thread cannot start).

use super::Context;
use mv_sys::nvg as sys;

impl Context {
    /// A context that owns nothing: `raw` is null.
    ///
    /// The context of a test that never draws (the Odin tests' nil `vg`).
    /// Every other constructor hands back a live backend context; this one
    /// only fills the `App` shape so headless tests (`headless_app`) build.
    pub fn null() -> Self {
        Self {
            raw: std::ptr::null_mut::<sys::Context>(),
            gl: None,
        }
    }
}
