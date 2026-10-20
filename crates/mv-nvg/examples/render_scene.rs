//! The render scene drawn through the Rust FFI, as a program.
//!
//! SDL needs the main thread, which `cargo test` does not run tests on, so
//! the render lives here (`cargo run --example render_scene -- --out PNG`)
//! and `tests/render.rs` drives it and judges the PNG. Numbers mirror the
//! twin's scene: 400x200 logical at density 2 (800x400 PNG).
//!
//! Colors mirror the twin's `nvg.RGB` bytes (`255.0 / 255.0`, allowed); the
//! degenerate path keeps its exact sub-0.01px coordinates (allowed).
#![allow(clippy::eq_op, clippy::excessive_precision)]

use std::ffi::c_void;

use glow::HasContext as _;
use mv_nvg::{Context, PlacedGlyph};
use mv_sys::{kb, kb_safe};

const LOGICAL_W: f32 = 400.0;
const LOGICAL_H: f32 = 200.0;
const DENSITY: f32 = 2.0;
const PX_W: u32 = 800;
const PX_H: u32 = 400;

const LINE_48: &str = "MathViz To Va AV Ty 123";
const OFFSETS: [f32; 4] = [0.0, 0.25, 0.5, 0.75];

const SEMI: &[u8] = include_bytes!("../../../assets/fonts/WorkSans-SemiBold.ttf");
const BOLD: &[u8] = include_bytes!("../../../assets/fonts/WorkSans-Bold.ttf");

/// Shape `text` at `size` (kb call sequence of shape.odin): push font,
/// begin LTR, shape UTF-8 with source-index ids, end, iterate runs and
/// glyphs; advances scaled by size/units-per-em, no tracking.
fn shape(
    ctx: &mut kb_safe::ShapeContext,
    font: &mut kb::font,
    units_per_em: f32,
    size: f32,
    text: &str,
) -> (Vec<PlacedGlyph>, f64) {
    ctx.push_font(font);
    ctx.begin(kb::direction::KBTS_DIRECTION_LTR, kb::language::DONT_KNOW);
    ctx.shape_utf8(text, kb::user_id_generation_mode::SOURCE_INDEX);
    ctx.end();
    let scale = size / units_per_em;
    let mut glyphs = Vec::new();
    let mut pen = 0.0f32;
    while let Some(mut run) = ctx.shape_run() {
        assert_eq!(
            run.font(),
            font as *const kb::font,
            "shaped run fell back to another font"
        );
        while let Some(g) = run.glyph_next() {
            let (sc, _) = ctx.shape_codepoint(g.user_id_or_codepoint_index);
            let _ = sc.UserId;
            glyphs.push(PlacedGlyph {
                index: g.id as i32,
                x: pen + g.offset_x as f32 * scale,
                y: -(g.offset_y as f32) * scale,
            });
            // No tracking in this scene, so attached marks take no spacing:
            // adding zero either way.
            pen += g.advance_x as f32 * scale;
        }
    }
    ctx.pop_font();
    (glyphs, pen as f64)
}

fn main() {
    let out = {
        let mut args = std::env::args().skip(1);
        let mut out: Option<String> = None;
        while let Some(a) = args.next() {
            if a == "--out" {
                out = args.next();
            }
        }
        // On the web there are no CLI args; the file lands in MEMFS and the
        // harness screenshots the canvas instead.
        out.unwrap_or_else(|| "/tmp/render_scene.png".to_string())
    };

    let sdl = sdl3::init().unwrap();
    let video = sdl.video().unwrap();
    {
        let attr = video.gl_attr();
        // WebGL2 on the web (GLES3), OpenGL 3.3 core natively.
        #[cfg(target_os = "emscripten")]
        {
            attr.set_context_profile(sdl3::video::GLProfile::GLES);
            attr.set_context_version(3, 0);
        }
        #[cfg(not(target_os = "emscripten"))]
        {
            attr.set_context_profile(sdl3::video::GLProfile::Core);
            attr.set_context_version(3, 3);
        }
        attr.set_stencil_size(8);
    }
    let mut builder = video.window("t0c render", PX_W, PX_H);
    builder.opengl();
    builder.position_centered();
    // A hidden window has no drawable on the web; the canvas shows the frame.
    #[cfg(not(target_os = "emscripten"))]
    builder.hidden();
    let window = builder.build().unwrap();
    println!(
        "rust: window={:?} drawable={:?}",
        window.size(),
        window.size_in_pixels()
    );
    assert_eq!(
        window.size_in_pixels(),
        (PX_W, PX_H),
        "drawable must be 800x400 for the planned density"
    );
    let _gl_ctx = window.gl_create_context().unwrap();
    let gl = unsafe {
        glow::Context::from_loader_function(|s| {
            video
                .gl_get_proc_address(s)
                .map(|f| f as *const c_void)
                .unwrap_or(core::ptr::null())
        })
    };

    #[cfg(target_os = "emscripten")]
    let mut vg = Context::gles3(
        mv_sys::nvg::CreateFlag::ANTI_ALIAS as core::ffi::c_int
            | mv_sys::nvg::CreateFlag::STENCIL_STROKES as core::ffi::c_int,
    );
    #[cfg(not(target_os = "emscripten"))]
    let mut vg = Context::gl3(
        mv_sys::nvg::CreateFlag::ANTI_ALIAS as core::ffi::c_int
            | mv_sys::nvg::CreateFlag::STENCIL_STROKES as core::ffi::c_int,
    );

    let mut shape_ctx = kb_safe::ShapeContext::new().expect("shape context");
    let mut semi = kb_safe::font_from_memory(SEMI, 0);
    assert!(kb_safe::font_is_valid(&mut semi));
    let mut bold = kb_safe::font_from_memory(BOLD, 0);
    assert!(kb_safe::font_is_valid(&mut bold));
    let semi_upm = kb_safe::font_info(&mut semi).UnitsPerEm as f32;
    let bold_upm = kb_safe::font_info(&mut bold).UnitsPerEm as f32;

    let semi_id = vg.create_font_mem("Work Sans 600", SEMI, false);
    let bold_id = vg.create_font_mem("Work Sans 700", BOLD, false);

    let white = [1.0, 1.0, 1.0, 1.0];
    unsafe {
        gl.clear_color(30.0 / 255.0, 35.0 / 255.0, 48.0 / 255.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT | glow::STENCIL_BUFFER_BIT);
    }
    vg.set_gl(gl);
    vg.begin_frame(LOGICAL_W, LOGICAL_H, DENSITY);
    vg.begin_path();
    vg.rect(0.0, 0.0, LOGICAL_W, LOGICAL_H);
    vg.fill_color([30.0 / 255.0, 35.0 / 255.0, 48.0 / 255.0, 1.0]);
    vg.fill();

    let (placed48, advance48) = shape(&mut shape_ctx, &mut semi, semi_upm, 48.0, LINE_48);
    println!("rust: advance48={advance48:.3}");
    vg.font_face_id(semi_id);
    vg.font_size(48.0);
    vg.fill_color(white);
    for off in OFFSETS {
        vg.text_glyphs(16.0 + off, 64.0, &placed48);
    }

    let (placed_bold, advance_bold) = shape(&mut shape_ctx, &mut bold, bold_upm, 48.0, LINE_48);
    println!("rust: advance48bold={advance_bold:.3}");
    vg.font_face_id(bold_id);
    vg.font_size(48.0);
    vg.fill_color(white);
    for off in OFFSETS {
        vg.text_glyphs(16.0 + off, 132.0, &placed_bold);
    }

    let (placed15, _) = shape(&mut shape_ctx, &mut semi, semi_upm, 15.0, LINE_48);
    vg.font_face_id(semi_id);
    vg.font_size(15.0);
    vg.fill_color([200.0 / 255.0, 200.0 / 255.0, 200.0 / 255.0, 1.0]);
    vg.text_glyphs(16.0, 164.0, &placed15);

    vg.begin_path();
    vg.rounded_rect(16.5, 172.5, 100.0, 20.0, 6.0);
    vg.fill_color([70.0 / 255.0, 130.0 / 255.0, 180.0 / 255.0, 1.0]);
    vg.fill();
    vg.stroke_color(white);
    vg.stroke_width(1.0);
    vg.stroke();

    vg.scissor(130.0, 174.0, 36.0, 16.0);
    vg.begin_path();
    vg.move_to(130.0, 182.0);
    vg.line_to(200.0, 182.0);
    vg.stroke_color([255.0 / 255.0, 127.0 / 255.0, 80.0 / 255.0, 1.0]);
    vg.stroke_width(3.0);
    vg.stroke();
    vg.reset_scissor();

    vg.begin_path();
    vg.circle(240.0, 182.0, 7.0);
    vg.stroke_color([64.0 / 255.0, 224.0 / 255.0, 208.0 / 255.0, 1.0]);
    vg.stroke_width(3.0);
    vg.stroke_feathered(1.5);

    vg.begin_path();
    vg.move_to(290.0, 182.0);
    vg.line_to(290.00005, 182.00005);
    vg.line_to(290.00009, 182.00002);
    vg.stroke_color([255.0 / 255.0, 80.0 / 255.0, 80.0 / 255.0, 1.0]);
    vg.stroke_width(2.0);
    vg.stroke();

    vg.end_frame();
    kb_safe::free_font(&mut semi);
    kb_safe::free_font(&mut bold);

    vg.finish();
    let px = vg.read_pixels(PX_W as i32, PX_H as i32);
    let stride = (PX_W * 4) as usize;
    let mut top_down = vec![0u8; px.len()];
    for y in 0..PX_H as usize {
        top_down[y * stride..(y + 1) * stride]
            .copy_from_slice(&px[(PX_H as usize - 1 - y) * stride..(PX_H as usize - y) * stride]);
    }

    // Present to the canvas (the web build's framebuffer); the harness
    // screenshots the element. Readback above already took the bytes.
    window.gl_swap_window();

    let file = std::fs::File::create(&out).unwrap();
    let mut enc = png::Encoder::new(file, PX_W, PX_H);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .unwrap()
        .write_image_data(&top_down)
        .unwrap();
    println!("rust: wrote {out}");

    // On the web the harness screenshots the presented frame after main
    // returns; the SDL window, context and GL context are intentionally
    // leaked: dropping them would destroy the window (sdl3's Drop calls
    // SDL_DestroyWindow), and nothing draws again, so there is nothing to
    // clean up.
    #[cfg(target_os = "emscripten")]
    {
        core::mem::forget(_gl_ctx);
        core::mem::forget(window);
        core::mem::forget(video);
        core::mem::forget(sdl);
    }
}
