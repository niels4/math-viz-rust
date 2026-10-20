//! Struct sizes and field offsets against the C headers (`csrc/sizes.c`).
//!
//! Every struct that crosses the FFI must match the C layout exactly;
//! `sizes.c` exports `sizeof`/`offsetof` from the C side and this test
//! compares them to Rust's.

use core::ffi::{c_float, c_int};
use core::mem::{offset_of, size_of};

use mv_sys::{clay as c, kb as k, nvg as n};

unsafe extern "C" {
    fn mv_sys_sizeof_NVGcolor() -> usize;
    fn mv_sys_offset_NVGcolor_r() -> usize;
    fn mv_sys_offset_NVGcolor_g() -> usize;
    fn mv_sys_offset_NVGcolor_b() -> usize;
    fn mv_sys_offset_NVGcolor_a() -> usize;
    fn mv_sys_sizeof_NVGpaint() -> usize;
    fn mv_sys_offset_NVGpaint_xform() -> usize;
    fn mv_sys_offset_NVGpaint_extent() -> usize;
    fn mv_sys_offset_NVGpaint_radius() -> usize;
    fn mv_sys_offset_NVGpaint_feather() -> usize;
    fn mv_sys_offset_NVGpaint_innerColor() -> usize;
    fn mv_sys_offset_NVGpaint_outerColor() -> usize;
    fn mv_sys_offset_NVGpaint_image() -> usize;
    fn mv_sys_sizeof_NVGscissor() -> usize;
    fn mv_sys_offset_NVGscissor_xform() -> usize;
    fn mv_sys_offset_NVGscissor_extent() -> usize;
    fn mv_sys_sizeof_NVGvertex() -> usize;
    fn mv_sys_offset_NVGvertex_x() -> usize;
    fn mv_sys_offset_NVGvertex_y() -> usize;
    fn mv_sys_offset_NVGvertex_u() -> usize;
    fn mv_sys_offset_NVGvertex_v() -> usize;
    fn mv_sys_sizeof_NVGplacedGlyph() -> usize;
    fn mv_sys_offset_NVGplacedGlyph_index() -> usize;
    fn mv_sys_offset_NVGplacedGlyph_x() -> usize;
    fn mv_sys_offset_NVGplacedGlyph_y() -> usize;
    fn mv_sys_sizeof_NVGglyphPosition() -> usize;
    fn mv_sys_offset_NVGglyphPosition_str() -> usize;
    fn mv_sys_offset_NVGglyphPosition_x() -> usize;
    fn mv_sys_offset_NVGglyphPosition_minx() -> usize;
    fn mv_sys_offset_NVGglyphPosition_maxx() -> usize;
    fn mv_sys_sizeof_Clay_String() -> usize;
    fn mv_sys_offset_Clay_String_isStaticallyAllocated() -> usize;
    fn mv_sys_offset_Clay_String_length() -> usize;
    fn mv_sys_offset_Clay_String_chars() -> usize;
    fn mv_sys_sizeof_Clay_Arena() -> usize;
    fn mv_sys_offset_Clay_Arena_nextAllocation() -> usize;
    fn mv_sys_offset_Clay_Arena_capacity() -> usize;
    fn mv_sys_offset_Clay_Arena_memory() -> usize;
    fn mv_sys_sizeof_Clay_Dimensions() -> usize;
    fn mv_sys_offset_Clay_Dimensions_width() -> usize;
    fn mv_sys_offset_Clay_Dimensions_height() -> usize;
    fn mv_sys_sizeof_Clay_Vector2() -> usize;
    fn mv_sys_offset_Clay_Vector2_x() -> usize;
    fn mv_sys_offset_Clay_Vector2_y() -> usize;
    fn mv_sys_sizeof_Clay_Color() -> usize;
    fn mv_sys_offset_Clay_Color_r() -> usize;
    fn mv_sys_offset_Clay_Color_g() -> usize;
    fn mv_sys_offset_Clay_Color_b() -> usize;
    fn mv_sys_offset_Clay_Color_a() -> usize;
    fn mv_sys_sizeof_Clay_BoundingBox() -> usize;
    fn mv_sys_offset_Clay_BoundingBox_x() -> usize;
    fn mv_sys_offset_Clay_BoundingBox_y() -> usize;
    fn mv_sys_offset_Clay_BoundingBox_width() -> usize;
    fn mv_sys_offset_Clay_BoundingBox_height() -> usize;
    fn mv_sys_sizeof_KbtsGlyph() -> usize;
    fn mv_sys_offset_KbtsGlyph_Id() -> usize;
    fn mv_sys_offset_KbtsGlyph_AdvanceX() -> usize;
    fn mv_sys_offset_KbtsGlyph_OffsetX() -> usize;
    fn mv_sys_offset_KbtsGlyph_OffsetY() -> usize;
    fn mv_sys_offset_KbtsGlyph_AttachGlyph() -> usize;
    fn mv_sys_offset_KbtsGlyph_UserIdOrCodepointIndex() -> usize;
    fn mv_sys_sizeof_KbtsRun() -> usize;
    fn mv_sys_offset_KbtsRun_Font() -> usize;
    fn mv_sys_offset_KbtsRun_Glyphs() -> usize;
    fn mv_sys_sizeof_KbtsShapeCodepoint() -> usize;
    fn mv_sys_offset_KbtsShapeCodepoint_UserId() -> usize;
    fn mv_sys_sizeof_KbtsFontInfo2_1() -> usize;
    fn mv_sys_offset_KbtsFontInfo2_1_UnitsPerEm() -> usize;
}

#[test]
fn struct_sizes_match_c() {
    unsafe {
        assert_eq!(size_of::<n::Color>(), mv_sys_sizeof_NVGcolor());
        assert_eq!(size_of::<n::Paint>(), mv_sys_sizeof_NVGpaint());
        assert_eq!(size_of::<n::ScissorT>(), mv_sys_sizeof_NVGscissor());
        assert_eq!(size_of::<n::Vertex>(), mv_sys_sizeof_NVGvertex());
        assert_eq!(size_of::<n::PlacedGlyph>(), mv_sys_sizeof_NVGplacedGlyph());
        assert_eq!(
            size_of::<n::GlyphPosition>(),
            mv_sys_sizeof_NVGglyphPosition()
        );
        assert_eq!(size_of::<c::String>(), mv_sys_sizeof_Clay_String());
        assert_eq!(size_of::<c::Arena>(), mv_sys_sizeof_Clay_Arena());
        assert_eq!(size_of::<c::Dimensions>(), mv_sys_sizeof_Clay_Dimensions());
        assert_eq!(size_of::<c::Vector2>(), mv_sys_sizeof_Clay_Vector2());
        assert_eq!(size_of::<c::Color>(), mv_sys_sizeof_Clay_Color());
        assert_eq!(
            size_of::<c::BoundingBox>(),
            mv_sys_sizeof_Clay_BoundingBox()
        );
        assert_eq!(size_of::<k::glyph>(), mv_sys_sizeof_KbtsGlyph());
        assert_eq!(size_of::<k::run>(), mv_sys_sizeof_KbtsRun());
        assert_eq!(
            size_of::<k::shape_codepoint>(),
            mv_sys_sizeof_KbtsShapeCodepoint()
        );
        assert_eq!(
            size_of::<k::font_info2_1>(),
            mv_sys_sizeof_KbtsFontInfo2_1()
        );
    }
}

#[test]
fn struct_offsets_match_c() {
    unsafe {
        // NVGcolor is a [c_float; 4] alias: r, g, b, a at 0, 4, 8, 12.
        assert_eq!(0, mv_sys_offset_NVGcolor_r());
        assert_eq!(size_of::<c_float>(), mv_sys_offset_NVGcolor_g());
        assert_eq!(2 * size_of::<c_float>(), mv_sys_offset_NVGcolor_b());
        assert_eq!(3 * size_of::<c_float>(), mv_sys_offset_NVGcolor_a());
        assert_eq!(offset_of!(n::Paint, xform), mv_sys_offset_NVGpaint_xform());
        assert_eq!(
            offset_of!(n::Paint, extent),
            mv_sys_offset_NVGpaint_extent()
        );
        assert_eq!(
            offset_of!(n::Paint, radius),
            mv_sys_offset_NVGpaint_radius()
        );
        assert_eq!(
            offset_of!(n::Paint, feather),
            mv_sys_offset_NVGpaint_feather()
        );
        assert_eq!(
            offset_of!(n::Paint, innerColor),
            mv_sys_offset_NVGpaint_innerColor()
        );
        assert_eq!(
            offset_of!(n::Paint, outerColor),
            mv_sys_offset_NVGpaint_outerColor()
        );
        assert_eq!(offset_of!(n::Paint, image), mv_sys_offset_NVGpaint_image());
        assert_eq!(
            offset_of!(n::ScissorT, xform),
            mv_sys_offset_NVGscissor_xform()
        );
        assert_eq!(
            offset_of!(n::ScissorT, extent),
            mv_sys_offset_NVGscissor_extent()
        );
        // NVGvertex is a [c_float; 4] alias: x, y, u, v at 0, 4, 8, 12.
        assert_eq!(0, mv_sys_offset_NVGvertex_x());
        assert_eq!(size_of::<c_float>(), mv_sys_offset_NVGvertex_y());
        assert_eq!(2 * size_of::<c_float>(), mv_sys_offset_NVGvertex_u());
        assert_eq!(3 * size_of::<c_float>(), mv_sys_offset_NVGvertex_v());
        assert_eq!(
            offset_of!(n::PlacedGlyph, index),
            mv_sys_offset_NVGplacedGlyph_index()
        );
        assert_eq!(
            offset_of!(n::PlacedGlyph, x),
            mv_sys_offset_NVGplacedGlyph_x()
        );
        assert_eq!(
            offset_of!(n::PlacedGlyph, y),
            mv_sys_offset_NVGplacedGlyph_y()
        );
        assert_eq!(
            offset_of!(n::GlyphPosition, str),
            mv_sys_offset_NVGglyphPosition_str()
        );
        assert_eq!(
            offset_of!(n::GlyphPosition, x),
            mv_sys_offset_NVGglyphPosition_x()
        );
        assert_eq!(
            offset_of!(n::GlyphPosition, minx),
            mv_sys_offset_NVGglyphPosition_minx()
        );
        assert_eq!(
            offset_of!(n::GlyphPosition, maxx),
            mv_sys_offset_NVGglyphPosition_maxx()
        );
        assert_eq!(
            offset_of!(c::String, isStaticallyAllocated),
            mv_sys_offset_Clay_String_isStaticallyAllocated()
        );
        assert_eq!(
            offset_of!(c::String, length),
            mv_sys_offset_Clay_String_length()
        );
        assert_eq!(
            offset_of!(c::String, chars),
            mv_sys_offset_Clay_String_chars()
        );
        assert_eq!(
            offset_of!(c::Arena, nextAllocation),
            mv_sys_offset_Clay_Arena_nextAllocation()
        );
        assert_eq!(
            offset_of!(c::Arena, capacity),
            mv_sys_offset_Clay_Arena_capacity()
        );
        assert_eq!(
            offset_of!(c::Arena, memory),
            mv_sys_offset_Clay_Arena_memory()
        );
        assert_eq!(
            offset_of!(c::Dimensions, width),
            mv_sys_offset_Clay_Dimensions_width()
        );
        assert_eq!(
            offset_of!(c::Dimensions, height),
            mv_sys_offset_Clay_Dimensions_height()
        );
        // Vector2 is a [c_float; 2] alias: x at 0, y at one element size.
        assert_eq!(0, mv_sys_offset_Clay_Vector2_x());
        assert_eq!(size_of::<c_float>(), mv_sys_offset_Clay_Vector2_y());
        // Clay Color is a [c_float; 4] alias: channels at 0, 4, 8, 12.
        assert_eq!(0, mv_sys_offset_Clay_Color_r());
        assert_eq!(size_of::<c_float>(), mv_sys_offset_Clay_Color_g());
        assert_eq!(2 * size_of::<c_float>(), mv_sys_offset_Clay_Color_b());
        assert_eq!(3 * size_of::<c_float>(), mv_sys_offset_Clay_Color_a());
        assert_eq!(
            offset_of!(c::BoundingBox, x),
            mv_sys_offset_Clay_BoundingBox_x()
        );
        assert_eq!(
            offset_of!(c::BoundingBox, y),
            mv_sys_offset_Clay_BoundingBox_y()
        );
        assert_eq!(
            offset_of!(c::BoundingBox, width),
            mv_sys_offset_Clay_BoundingBox_width()
        );
        assert_eq!(
            offset_of!(c::BoundingBox, height),
            mv_sys_offset_Clay_BoundingBox_height()
        );
        assert_eq!(offset_of!(k::glyph, Id), mv_sys_offset_KbtsGlyph_Id());
        assert_eq!(
            offset_of!(k::glyph, AdvanceX),
            mv_sys_offset_KbtsGlyph_AdvanceX()
        );
        assert_eq!(
            offset_of!(k::glyph, OffsetX),
            mv_sys_offset_KbtsGlyph_OffsetX()
        );
        assert_eq!(
            offset_of!(k::glyph, OffsetY),
            mv_sys_offset_KbtsGlyph_OffsetY()
        );
        assert_eq!(
            offset_of!(k::glyph, AttachGlyph),
            mv_sys_offset_KbtsGlyph_AttachGlyph()
        );
        assert_eq!(
            offset_of!(k::glyph, UserIdOrCodepointIndex),
            mv_sys_offset_KbtsGlyph_UserIdOrCodepointIndex()
        );
        assert_eq!(offset_of!(k::run, Font), mv_sys_offset_KbtsRun_Font());
        assert_eq!(offset_of!(k::run, Glyphs), mv_sys_offset_KbtsRun_Glyphs());
        assert_eq!(
            offset_of!(k::shape_codepoint, UserId),
            mv_sys_offset_KbtsShapeCodepoint_UserId()
        );
        assert_eq!(
            offset_of!(k::font_info2_1, UnitsPerEm),
            mv_sys_offset_KbtsFontInfo2_1_UnitsPerEm()
        );
    }
}

#[test]
fn scalar_widths_match_c() {
    assert_eq!(size_of::<c_int>(), 4);
    assert_eq!(size_of::<c_float>(), 4);
    assert_eq!(size_of::<usize>(), size_of::<*const u8>());
}
