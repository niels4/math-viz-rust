// The Odin app's C libraries through cc: Clay, NanoVG (with fontstash and stb_truetype)
// and kb_text_shape. One static lib per family. Native GL3 comes from the OpenGL framework,
// Emscripten GLES3 from emsdk (cc picks emcc for the emscripten target when its env is
// sourced); both targets build with -O2 and floating-point contraction off, as the Odin
// build does, so native and web lay out and rasterize with the same arithmetic.
fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let emscripten = target == "wasm32-unknown-emscripten";

    let mut base = cc::Build::new();
    base.opt_level(2)
        .flag("-ffp-contract=off")
        .warnings(false)
        .define("NVG_NO_STB", None);

    // NanoVG core (fontstash and stb_truetype implementations live inside).
    {
        let mut b = base.clone();
        b.file("../../vendor/nanovg/nanovg.c")
            .include("../../vendor/nanovg");
        b.compile("mv_nanovg");
    }

    // NanoVG GL backend: the GL header first, then the backend selection.
    {
        let mut b = base.clone();
        b.file("csrc/nvg_gl_impl.c").include("../../vendor/nanovg");
        b.compile("mv_nanovg_gl");
    }

    // Clay (header + implementation TU).
    {
        let mut b = base.clone();
        b.file("csrc/clay_impl.c").include("../../vendor/clay");
        b.compile("mv_clay");
    }

    // kb_text_shape (the shaper: one C file).
    {
        let mut b = base.clone();
        // kbts_un follows the pointer size, but upstream defaults 32-bit
        // targets it does not know (wasm32) to 64-bit, which mismatches
        // Rust's pointer-sized `un` at the FFI boundary; pin it.
        if emscripten {
            b.define("KB_TEXT_SHAPE_POINTER_SIZE", "4");
        }
        b.file("../../vendor/kb_text_shape/kb_text_shape.c")
            .include("../../vendor/kb_text_shape");
        b.compile("mv_kbts");
    }

    // The sizeof/offsetof exporters for the struct-size test.
    {
        let mut b = base.clone();
        b.file("csrc/sizes.c")
            .include("../../vendor/nanovg")
            .include("../../vendor/clay")
            .include("../../vendor/kb_text_shape");
        b.compile("mv_sizes");
    }

    if target.contains("apple-darwin") {
        println!("cargo:rustc-link-lib=framework=OpenGL");
    } else if !emscripten && std::env::var("CARGO_CFG_UNIX").is_ok() {
        // Unverified path (no Linux builder on this run yet): system GL for nanovg_gl.
        println!("cargo:rustc-link-lib=GL");
    }

    println!("cargo:rerun-if-changed=csrc/");
    println!("cargo:rerun-if-changed=../../vendor/");
}
