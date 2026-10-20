//! The framebuffer link guard: create, bind, read and delete through the wrapper.
//!
//! The `nvg*` framebuffer symbols' bodies live behind
//! `NANOVG_GL_IMPLEMENTATION` in `csrc/nvg_gl_impl.c`; without it nothing the
//! binary links defines `nvgluCreateFramebuffer` et al, and `cargo test`
//! dead-strips the unreached path, so the suite stays green while the
//! production binary fails to link. This test calls every one, so a missing
//! define breaks the test binary's link loudly on both targets (`cargo test`
//! natively, `cargo build --tests --target wasm32-unknown-emscripten` for the
//! link). The body runs only with `MV_TEST_DISPLAY=1`: framebuffer creation
//! needs a live GL context, and SDL video cannot start on a `cargo test`
//! thread (the render test drives its example binary for the same reason).

use mv_nvg::{Context, Framebuffer};

#[test]
fn framebuffer_create_bind_read_delete() {
    if std::env::var("MV_TEST_DISPLAY").is_err() {
        eprintln!("skipping: needs a display (MV_TEST_DISPLAY=1)");
        return;
    }
    // The GL3 backend compiles natively, GLES3 on the web; the test links
    // on both targets either way.
    #[cfg(target_os = "emscripten")]
    let mut vg = Context::gles3(0);
    #[cfg(not(target_os = "emscripten"))]
    let mut vg = Context::gl3(0);
    let fb = Framebuffer::create(&mut vg, 16, 16, 0);
    fb.bind();
    let _ = fb.image();
    let px = fb.read_pixels();
    assert_eq!(px.len(), 16 * 16 * 4);
    fb.delete();
    Framebuffer::bind_default();
}
